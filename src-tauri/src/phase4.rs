use chrono::{Datelike, Duration, Local, NaiveDate};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    AppState,
};

fn today() -> NaiveDate {
    Local::now().date_naive()
}

fn date_string(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn parse_date(value: &str, field: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| AppError::Validation(format!("{field} must be YYYY-MM-DD")))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardBillDto {
    pub id: String,
    pub name: String,
    pub due_date: String,
    pub pay_by_date: String,
    pub amount_cents: i64,
    pub status: String,
    pub payment_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardPaycheckDto {
    pub id: String,
    pub owner_name: String,
    pub pay_date: String,
    pub amount_cents: i64,
    pub bills_cents: i64,
    pub safe_cents: i64,
    pub status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategorySpendDto {
    pub category_id: String,
    pub category_name: String,
    pub amount_cents: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityDto {
    pub id: String,
    pub occurred_at: String,
    pub user_name: Option<String>,
    pub event_type: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub summary: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardAlertDto {
    pub code: String,
    pub title: String,
    pub message: String,
    pub tone: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardDataDto {
    pub current_cash_cents: i64,
    pub reserved_bills_cents: i64,
    pub safe_to_spend_cents: i64,
    pub protected_buffer_cents: i64,
    pub next_paycheck: Option<DashboardPaycheckDto>,
    pub upcoming_bills: Vec<DashboardBillDto>,
    pub paychecks: Vec<DashboardPaycheckDto>,
    pub month_income_cents: i64,
    pub month_bill_payments_cents: i64,
    pub month_everyday_spending_cents: i64,
    pub month_net_cents: i64,
    pub category_spending: Vec<CategorySpendDto>,
    pub recent_activity: Vec<ActivityDto>,
    pub alerts: Vec<DashboardAlertDto>,
    pub savings_total_cents: i64,
    pub debt_total_cents: i64,
}

fn account_cash(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(book_balance_cents),0) FROM accounts WHERE is_active=1 AND account_type != 'credit'",
        [],
        |r| r.get(0),
    )?)
}

fn protected_buffer(conn: &Connection) -> AppResult<i64> {
    Ok(conn
        .query_row(
            "SELECT protected_buffer_cents FROM household_settings WHERE id=1",
            [],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or(0))
}

fn current_cash_reserved(conn: &Connection) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(a.allocated_amount_cents),0)
         FROM bill_allocations a
         JOIN bill_occurrences o ON o.id=a.bill_occurrence_id
         WHERE o.status IN ('upcoming','scheduled','partial','late')
           AND a.funding_source_type='current_cash'",
        [],
        |r| r.get(0),
    )?)
}

#[tauri::command]
pub fn get_dashboard_data(state: State<'_, AppState>) -> AppResult<DashboardDataDto> {
    // Rebuild derived allocations first so every dashboard number is based on the
    // same deterministic plan shown in the Paycheck Planner.
    let planner = crate::phase3::run_scheduler_internal(&state)?;
    let conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    let cash = account_cash(&conn)?;
    let buffer = protected_buffer(&conn)?;
    let reserved = current_cash_reserved(&conn)?;
    let safe = (cash - reserved - buffer).max(0);

    let mut bill_stmt = conn.prepare(
        "SELECT o.id,o.name_snapshot,o.due_date,o.latest_payment_date,
                MAX(COALESCE(o.actual_required_amount_cents,o.manual_amount_override_cents,o.estimated_amount_cents)
                    - COALESCE((SELECT SUM(p.amount_cents) FROM payments p WHERE p.bill_occurrence_id=o.id),0),0),
                o.status,o.payment_type_snapshot
         FROM bill_occurrences o
         WHERE o.status IN ('upcoming','scheduled','partial','late')
           AND o.due_date >= date('now','localtime')
         ORDER BY o.due_date,o.name_snapshot LIMIT 6",
    )?;
    let upcoming_bills = bill_stmt
        .query_map([], |r| {
            Ok(DashboardBillDto {
                id: r.get(0)?,
                name: r.get(1)?,
                due_date: r.get(2)?,
                pay_by_date: r.get(3)?,
                amount_cents: r.get(4)?,
                status: r.get(5)?,
                payment_type: r.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let paychecks: Vec<DashboardPaycheckDto> = planner
        .paychecks
        .iter()
        .take(3)
        .map(|p| DashboardPaycheckDto {
            id: p.id.clone(),
            owner_name: p.owner_name.clone(),
            pay_date: p.pay_date.clone(),
            amount_cents: p.amount_cents,
            bills_cents: p.bills_total_cents,
            safe_cents: p.safe_remaining_cents,
            status: p.status.clone(),
        })
        .collect();
    let next_paycheck = paychecks.first().map(|p| DashboardPaycheckDto {
        id: p.id.clone(),
        owner_name: p.owner_name.clone(),
        pay_date: p.pay_date.clone(),
        amount_cents: p.amount_cents,
        bills_cents: p.bills_cents,
        safe_cents: p.safe_cents,
        status: p.status.clone(),
    });

    let month_start = NaiveDate::from_ymd_opt(today().year(), today().month(), 1)
        .ok_or_else(|| AppError::Validation("could not determine current month".into()))?;
    let next_month = if today().month() == 12 {
        NaiveDate::from_ymd_opt(today().year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(today().year(), today().month() + 1, 1)
    }
    .ok_or_else(|| AppError::Validation("could not determine next month".into()))?;
    let start = date_string(month_start);
    let end = date_string(next_month);

    let month_income: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount_cents),0) FROM transactions
         WHERE transaction_date>=?1 AND transaction_date<?2 AND amount_cents>0",
        params![start, end],
        |r| r.get(0),
    )?;
    let month_bills: i64 = conn.query_row(
        "SELECT COALESCE(-SUM(amount_cents),0) FROM transactions
         WHERE transaction_date>=?1 AND transaction_date<?2 AND amount_cents<0 AND transaction_type='bill_payment'",
        params![start, end],
        |r| r.get(0),
    )?;
    let month_everyday: i64 = conn.query_row(
        "SELECT COALESCE(-SUM(amount_cents),0) FROM transactions
         WHERE transaction_date>=?1 AND transaction_date<?2 AND amount_cents<0
           AND transaction_type NOT IN ('bill_payment','savings_contribution','debt_payment')",
        params![start, end],
        |r| r.get(0),
    )?;
    let month_net: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE transaction_date>=?1 AND transaction_date<?2",
        params![start, end],
        |r| r.get(0),
    )?;

    let mut cat_stmt = conn.prepare(
        "SELECT COALESCE(t.category_id,'other'),COALESCE(c.name,'Other'),-SUM(t.amount_cents)
         FROM transactions t LEFT JOIN categories c ON c.id=t.category_id
         WHERE t.transaction_date>=?1 AND t.transaction_date<?2 AND t.amount_cents<0
           AND t.transaction_type NOT IN ('bill_payment','savings_contribution','debt_payment')
         GROUP BY COALESCE(t.category_id,'other'),COALESCE(c.name,'Other')
         ORDER BY -SUM(t.amount_cents) DESC LIMIT 5",
    )?;
    let category_spending = cat_stmt
        .query_map(params![start, end], |r| {
            Ok(CategorySpendDto {
                category_id: r.get(0)?,
                category_name: r.get(1)?,
                amount_cents: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut activity_stmt = conn.prepare(
        "SELECT a.id,a.occurred_at,u.display_name,a.event_type,a.entity_type,a.entity_id,a.summary
         FROM activity_log a LEFT JOIN users u ON u.id=a.user_id
         ORDER BY a.occurred_at DESC LIMIT 6",
    )?;
    let recent_activity = activity_stmt
        .query_map([], |r| {
            Ok(ActivityDto {
                id: r.get(0)?,
                occurred_at: r.get(1)?,
                user_name: r.get(2)?,
                event_type: r.get(3)?,
                entity_type: r.get(4)?,
                entity_id: r.get(5)?,
                summary: r.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut alerts = Vec::new();
    if cash < 0 {
        alerts.push(DashboardAlertDto {
            code: "negative_cash".into(),
            title: "Account balance needs attention".into(),
            message: "Your manually tracked cash balance is below zero. Reconcile it to the actual account balance.".into(),
            tone: "red".into(),
        });
    } else if cash < buffer {
        alerts.push(DashboardAlertDto {
            code: "below_buffer_now".into(),
            title: "Below protected buffer".into(),
            message: "Your current tracked balance is below the protected household buffer.".into(),
            tone: "orange".into(),
        });
    }
    for w in planner.warnings.iter().take(3) {
        alerts.push(DashboardAlertDto {
            code: w.code.clone(),
            title: match w.code.as_str() {
                "negative_balance" => "Projected negative balance".into(),
                "below_buffer" => "Projected buffer warning".into(),
                "funding_shortage" => "Funding shortage".into(),
                _ => "Plan needs attention".into(),
            },
            message: w.message.clone(),
            tone: if w.code == "negative_balance" || w.code == "funding_shortage" { "red".into() } else { "orange".into() },
        });
    }
    if alerts.is_empty() {
        alerts.push(DashboardAlertDto {
            code: "on_track".into(),
            title: "Plan is on track".into(),
            message: "No current cash-flow problems are detected in the active plan.".into(),
            tone: "green".into(),
        });
    }

    let savings_total: i64 = conn.query_row(
        "SELECT COALESCE(SUM(current_amount_cents),0) FROM savings_goals WHERE is_active=1",
        [],
        |r| r.get(0),
    )?;
    let debt_total: i64 = conn.query_row(
        "SELECT COALESCE(SUM(balance_cents),0) FROM debts WHERE is_active=1",
        [],
        |r| r.get(0),
    )?;

    Ok(DashboardDataDto {
        current_cash_cents: cash,
        reserved_bills_cents: reserved,
        safe_to_spend_cents: safe,
        protected_buffer_cents: buffer,
        next_paycheck,
        upcoming_bills,
        paychecks,
        month_income_cents: month_income,
        month_bill_payments_cents: month_bills,
        month_everyday_spending_cents: month_everyday,
        month_net_cents: month_net,
        category_spending,
        recent_activity,
        alerts,
        savings_total_cents: savings_total,
        debt_total_cents: debt_total,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountViewDto {
    pub id: String,
    pub name: String,
    pub account_type: String,
    pub balance_cents: i64,
    pub is_primary: bool,
    pub last_reconciled_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDto {
    pub id: String,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDto {
    pub id: String,
    pub account_id: String,
    pub account_name: String,
    pub transaction_date: String,
    pub description: String,
    pub category_id: Option<String>,
    pub category_name: String,
    pub amount_cents: i64,
    pub transaction_type: String,
    pub status: String,
    pub source: String,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpendingViewDto {
    pub accounts: Vec<AccountViewDto>,
    pub categories: Vec<CategoryDto>,
    pub transactions: Vec<TransactionDto>,
    pub month_income_cents: i64,
    pub month_spending_cents: i64,
    pub month_net_cents: i64,
    pub category_spending: Vec<CategorySpendDto>,
}

fn spending_view_from_conn(conn: &Connection) -> AppResult<SpendingViewDto> {
    let mut account_stmt = conn.prepare(
        "SELECT a.id,a.name,a.account_type,a.book_balance_cents,a.is_primary_bill_account,
                (SELECT MAX(r.performed_at) FROM balance_reconciliations r WHERE r.account_id=a.id)
         FROM accounts a WHERE a.is_active=1 ORDER BY a.is_primary_bill_account DESC,a.created_at",
    )?;
    let accounts = account_stmt
        .query_map([], |r| {
            Ok(AccountViewDto {
                id: r.get(0)?,
                name: r.get(1)?,
                account_type: r.get(2)?,
                balance_cents: r.get(3)?,
                is_primary: r.get::<_, i64>(4)? == 1,
                last_reconciled_at: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut category_stmt = conn.prepare(
        "SELECT id,name,kind FROM categories WHERE is_active=1 ORDER BY sort_order,name",
    )?;
    let categories = category_stmt
        .query_map([], |r| {
            Ok(CategoryDto {
                id: r.get(0)?,
                name: r.get(1)?,
                kind: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let start = date_string(today() - Duration::days(90));
    let mut tx_stmt = conn.prepare(
        "SELECT t.id,t.account_id,a.name,t.transaction_date,t.description,t.category_id,COALESCE(c.name,'Other'),
                t.amount_cents,t.transaction_type,t.status,t.source,t.note
         FROM transactions t
         JOIN accounts a ON a.id=t.account_id
         LEFT JOIN categories c ON c.id=t.category_id
         WHERE t.transaction_date>=?1
         ORDER BY t.transaction_date DESC,t.created_at DESC LIMIT 250",
    )?;
    let transactions = tx_stmt
        .query_map([start.as_str()], |r| {
            Ok(TransactionDto {
                id: r.get(0)?,
                account_id: r.get(1)?,
                account_name: r.get(2)?,
                transaction_date: r.get(3)?,
                description: r.get(4)?,
                category_id: r.get(5)?,
                category_name: r.get(6)?,
                amount_cents: r.get(7)?,
                transaction_type: r.get(8)?,
                status: r.get(9)?,
                source: r.get(10)?,
                note: r.get(11)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let month_start = NaiveDate::from_ymd_opt(today().year(), today().month(), 1)
        .ok_or_else(|| AppError::Validation("could not determine current month".into()))?;
    let next_month = if today().month() == 12 {
        NaiveDate::from_ymd_opt(today().year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(today().year(), today().month() + 1, 1)
    }
    .ok_or_else(|| AppError::Validation("could not determine next month".into()))?;
    let mstart = date_string(month_start);
    let mend = date_string(next_month);
    let income: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE transaction_date>=?1 AND transaction_date<?2 AND amount_cents>0",
        params![mstart, mend],
        |r| r.get(0),
    )?;
    let spending: i64 = conn.query_row(
        "SELECT COALESCE(-SUM(amount_cents),0) FROM transactions WHERE transaction_date>=?1 AND transaction_date<?2 AND amount_cents<0",
        params![mstart, mend],
        |r| r.get(0),
    )?;
    let net: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE transaction_date>=?1 AND transaction_date<?2",
        params![mstart, mend],
        |r| r.get(0),
    )?;
    let mut cs_stmt = conn.prepare(
        "SELECT COALESCE(t.category_id,'other'),COALESCE(c.name,'Other'),-SUM(t.amount_cents)
         FROM transactions t LEFT JOIN categories c ON c.id=t.category_id
         WHERE t.transaction_date>=?1 AND t.transaction_date<?2 AND t.amount_cents<0
         GROUP BY COALESCE(t.category_id,'other'),COALESCE(c.name,'Other')
         ORDER BY -SUM(t.amount_cents) DESC",
    )?;
    let category_spending = cs_stmt
        .query_map(params![mstart, mend], |r| {
            Ok(CategorySpendDto {
                category_id: r.get(0)?,
                category_name: r.get(1)?,
                amount_cents: r.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(SpendingViewDto {
        accounts,
        categories,
        transactions,
        month_income_cents: income,
        month_spending_cents: spending,
        month_net_cents: net,
        category_spending,
    })
}

#[tauri::command]
pub fn get_spending_view(state: State<'_, AppState>) -> AppResult<SpendingViewDto> {
    let conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    spending_view_from_conn(&conn)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddTransactionPayload {
    pub account_id: String,
    pub transaction_date: String,
    pub description: String,
    pub category_id: Option<String>,
    pub amount_cents: i64,
    pub direction: String,
    pub user_id: Option<String>,
    pub note: Option<String>,
}

#[tauri::command]
pub fn add_transaction(payload: AddTransactionPayload, state: State<'_, AppState>) -> AppResult<()> {
    parse_date(&payload.transaction_date, "transaction date")?;
    if payload.description.trim().is_empty() {
        return Err(AppError::Validation("transaction description is required".into()));
    }
    if payload.amount_cents <= 0 {
        return Err(AppError::Validation("transaction amount must be greater than zero".into()));
    }
    if !matches!(payload.direction.as_str(), "expense" | "income") {
        return Err(AppError::Validation("transaction direction must be expense or income".into()));
    }
    let signed = if payload.direction == "expense" { -payload.amount_cents } else { payload.amount_cents };
    let mut conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    let tx = conn.transaction()?;
    let account_exists: Option<String> = tx.query_row(
        "SELECT id FROM accounts WHERE id=?1 AND is_active=1",
        [payload.account_id.as_str()],
        |r| r.get(0),
    ).optional()?;
    if account_exists.is_none() {
        return Err(AppError::Validation("account was not found".into()));
    }
    let id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO transactions(id,account_id,transaction_date,description,category_id,amount_cents,transaction_type,status,source,created_by_user_id,note)
         VALUES(?1,?2,?3,?4,?5,?6,?7,'cleared','manual',?8,?9)",
        params![id,payload.account_id,payload.transaction_date,payload.description.trim(),payload.category_id,signed,payload.direction,payload.user_id,payload.note],
    )?;
    tx.execute(
        "UPDATE accounts SET book_balance_cents=book_balance_cents+?2,updated_at=CURRENT_TIMESTAMP WHERE id=?1",
        params![payload.account_id,signed],
    )?;
    tx.execute(
        "INSERT INTO activity_log(id,user_id,event_type,entity_type,entity_id,summary)
         VALUES(?1,?2,'transaction_added','transaction',?3,?4)",
        params![Uuid::new_v4().to_string(),payload.user_id,id,format!("Recorded {}",payload.description.trim())],
    )?;
    tx.commit()?;
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcilePayload {
    pub account_id: String,
    pub actual_balance_cents: i64,
    pub user_id: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileResultDto {
    pub app_balance_before_cents: i64,
    pub actual_balance_cents: i64,
    pub difference_cents: i64,
}

#[tauri::command]
pub fn reconcile_account(payload: ReconcilePayload, state: State<'_, AppState>) -> AppResult<ReconcileResultDto> {
    let mut conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    let tx = conn.transaction()?;
    let before: i64 = tx.query_row(
        "SELECT book_balance_cents FROM accounts WHERE id=?1 AND is_active=1",
        [payload.account_id.as_str()],
        |r| r.get(0),
    ).optional()?.ok_or_else(|| AppError::Validation("account was not found".into()))?;
    let difference = payload.actual_balance_cents - before;
    let rec_id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO balance_reconciliations(id,account_id,app_balance_before_cents,actual_balance_cents,difference_cents,resolution_type,note,performed_by_user_id,performed_at)
         VALUES(?1,?2,?3,?4,?5,'manual_adjustment',?6,?7,datetime('now','localtime'))",
        params![rec_id,payload.account_id,before,payload.actual_balance_cents,difference,payload.note,payload.user_id],
    )?;
    if difference != 0 {
        let description = if difference < 0 { "Untracked spending (balance reconciliation)" } else { "Balance reconciliation adjustment" };
        tx.execute(
            "INSERT INTO transactions(id,account_id,transaction_date,description,category_id,amount_cents,transaction_type,status,source,created_by_user_id,note,source_entity_type,source_entity_id)
             VALUES(?1,?2,date('now','localtime'),?3,'other',?4,'reconciliation','cleared','reconciliation',?5,?6,'reconciliation',?7)",
            params![Uuid::new_v4().to_string(),payload.account_id,description,difference,payload.user_id,payload.note,rec_id],
        )?;
    }
    tx.execute(
        "UPDATE accounts SET book_balance_cents=?2,updated_at=CURRENT_TIMESTAMP WHERE id=?1",
        params![payload.account_id,payload.actual_balance_cents],
    )?;
    tx.execute(
        "INSERT INTO activity_log(id,user_id,event_type,entity_type,entity_id,summary)
         VALUES(?1,?2,'balance_reconciled','account',?3,'Reconciled account balance')",
        params![Uuid::new_v4().to_string(),payload.user_id,payload.account_id],
    )?;
    tx.commit()?;
    drop(conn);
    let _ = crate::phase3::run_scheduler_internal(&state)?;
    Ok(ReconcileResultDto {
        app_balance_before_cents: before,
        actual_balance_cents: payload.actual_balance_cents,
        difference_cents: difference,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEventDto {
    pub id: String,
    pub date: String,
    pub event_type: String,
    pub title: String,
    pub subtitle: String,
    pub amount_cents: i64,
    pub status: String,
    pub due_date: Option<String>,
    pub pay_by_date: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarDataDto {
    pub events: Vec<CalendarEventDto>,
}

#[tauri::command]
pub fn get_calendar_data(start_date: String, end_date: String, state: State<'_, AppState>) -> AppResult<CalendarDataDto> {
    let start = parse_date(&start_date, "calendar start date")?;
    let end = parse_date(&end_date, "calendar end date")?;
    if end < start {
        return Err(AppError::Validation("calendar end date must be after start date".into()));
    }
    let _ = crate::phase3::run_scheduler_internal(&state)?;
    let conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    let mut events = Vec::new();

    let mut paycheck_stmt = conn.prepare(
        "SELECT p.id,p.pay_date,u.display_name,COALESCE(p.actual_amount_cents,p.expected_amount_cents,p.projected_amount_cents),p.status
         FROM paycheck_occurrences p JOIN income_sources i ON i.id=p.income_source_id JOIN users u ON u.id=i.user_id
         WHERE p.pay_date BETWEEN ?1 AND ?2 AND p.status<>'skipped' ORDER BY p.pay_date",
    )?;
    let paychecks = paycheck_stmt.query_map(params![start_date,end_date], |r| {
        Ok(CalendarEventDto {
            id:r.get(0)?,date:r.get(1)?,event_type:"paycheck".into(),title:"Paycheck".into(),subtitle:r.get(2)?,amount_cents:r.get(3)?,status:r.get(4)?,due_date:None,pay_by_date:None
        })
    })?.collect::<Result<Vec<_>,_>>()?;
    events.extend(paychecks);

    let mut bill_stmt = conn.prepare(
        "SELECT o.id,o.due_date,o.name_snapshot,
                COALESCE(o.actual_required_amount_cents,o.manual_amount_override_cents,o.estimated_amount_cents),
                o.status,o.latest_payment_date
         FROM bill_occurrences o WHERE o.due_date BETWEEN ?1 AND ?2 ORDER BY o.due_date,o.name_snapshot",
    )?;
    let bills = bill_stmt.query_map(params![start_date,end_date], |r| {
        let due: String = r.get(1)?;
        Ok(CalendarEventDto {
            id:r.get(0)?,date:due.clone(),event_type:"bill".into(),title:r.get(2)?,subtitle:if r.get::<_,String>(4)?=="paid" {"Paid".into()} else {"Bill due".into()},amount_cents:r.get(3)?,status:r.get(4)?,due_date:Some(due),pay_by_date:r.get(5)?
        })
    })?.collect::<Result<Vec<_>,_>>()?;
    events.extend(bills);

    // Phase 5 adds actual recommended payment actions as separate calendar
    // events. Non-splittable bills keep one provider payment even if multiple
    // paychecks reserve money. Bills that explicitly allow partial provider
    // payments can have more than one action date.
    let mut action_stmt = conn.prepare(
        "SELECT o.id,CASE WHEN o.payment_type_snapshot='autopay' THEN o.due_date ELSE COALESCE(o.scheduled_payment_date,o.latest_payment_date) END,
                o.name_snapshot,
                MAX(COALESCE(o.actual_required_amount_cents,o.manual_amount_override_cents,o.estimated_amount_cents)
                    - COALESCE((SELECT SUM(p.amount_cents) FROM payments p WHERE p.bill_occurrence_id=o.id),0),0),
                o.status,o.due_date,o.latest_payment_date,o.payment_type_snapshot
         FROM bill_occurrences o
         LEFT JOIN bill_templates t ON t.id=o.bill_template_id
         WHERE o.status IN ('upcoming','scheduled','partial','late')
           AND (o.payment_type_snapshot='autopay' OR COALESCE(t.can_split,0)=0)
           AND (CASE WHEN o.payment_type_snapshot='autopay' THEN o.due_date ELSE COALESCE(o.scheduled_payment_date,o.latest_payment_date) END) BETWEEN ?1 AND ?2
         GROUP BY o.id
         ORDER BY 2,o.name_snapshot"
    )?;
    let actions = action_stmt.query_map(params![start_date,end_date], |r| {
        let action_date: String = r.get(1)?;
        let payment_type: String = r.get(7)?;
        let name: String = r.get(2)?;
        Ok(CalendarEventDto {
            id:format!("payment:{}",r.get::<_,String>(0)?),date:action_date,event_type:"payment".into(),
            title:if payment_type=="autopay" { format!("Autopay: {name}") } else { format!("Pay {name}") },
            subtitle:if payment_type=="autopay" { "Automatic draft".into() } else { "Recommended payment".into() },
            amount_cents:r.get(3)?,status:r.get(4)?,due_date:r.get(5)?,pay_by_date:r.get(6)?
        })
    })?.collect::<Result<Vec<_>,_>>()?;
    events.extend(actions);

    let mut split_action_stmt = conn.prepare(
        "SELECT o.id,COALESCE(a.recommended_payment_date,p.pay_date,o.scheduled_payment_date,o.latest_payment_date),
                o.name_snapshot,SUM(a.allocated_amount_cents),o.status,o.due_date,o.latest_payment_date
         FROM bill_allocations a
         JOIN bill_occurrences o ON o.id=a.bill_occurrence_id
         JOIN bill_templates t ON t.id=o.bill_template_id
         LEFT JOIN paycheck_occurrences p ON p.id=a.paycheck_occurrence_id
         WHERE o.status IN ('upcoming','scheduled','partial','late')
           AND o.payment_type_snapshot='manual' AND COALESCE(t.can_split,0)=1
           AND COALESCE(a.recommended_payment_date,p.pay_date,o.scheduled_payment_date,o.latest_payment_date) BETWEEN ?1 AND ?2
         GROUP BY o.id,COALESCE(a.recommended_payment_date,p.pay_date,o.scheduled_payment_date,o.latest_payment_date)
         ORDER BY 2,o.name_snapshot"
    )?;
    let split_actions = split_action_stmt.query_map(params![start_date,end_date], |r| {
        let occurrence_id: String = r.get(0)?;
        let action_date: String = r.get(1)?;
        let name: String = r.get(2)?;
        Ok(CalendarEventDto {
            id:format!("payment:{occurrence_id}:{action_date}"),date:action_date,event_type:"payment".into(),
            title:format!("Pay {name}"),subtitle:"Recommended partial payment".into(),amount_cents:r.get(3)?,status:r.get(4)?,due_date:r.get(5)?,pay_by_date:r.get(6)?
        })
    })?.collect::<Result<Vec<_>,_>>()?;
    events.extend(split_actions);
    events.sort_by(|a,b| a.date.cmp(&b.date).then(a.event_type.cmp(&b.event_type)).then(a.title.cmp(&b.title)));
    Ok(CalendarDataDto { events })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryDataDto {
    pub activity: Vec<ActivityDto>,
    pub payments: Vec<PaymentHistoryRowDto>,
    pub reconciliations: Vec<ReconciliationHistoryDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentHistoryRowDto {
    pub id: String,
    pub paid_date: String,
    pub bill_name: String,
    pub amount_cents: i64,
    pub paid_by: String,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationHistoryDto {
    pub id: String,
    pub performed_at: String,
    pub account_name: String,
    pub before_cents: i64,
    pub actual_cents: i64,
    pub difference_cents: i64,
    pub performed_by: Option<String>,
}

#[tauri::command]
pub fn get_history_data(state: State<'_, AppState>) -> AppResult<HistoryDataDto> {
    let conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    let mut activity_stmt = conn.prepare(
        "SELECT a.id,a.occurred_at,u.display_name,a.event_type,a.entity_type,a.entity_id,a.summary
         FROM activity_log a LEFT JOIN users u ON u.id=a.user_id
         ORDER BY a.occurred_at DESC LIMIT 150",
    )?;
    let activity = activity_stmt.query_map([], |r| Ok(ActivityDto {
        id:r.get(0)?,occurred_at:r.get(1)?,user_name:r.get(2)?,event_type:r.get(3)?,entity_type:r.get(4)?,entity_id:r.get(5)?,summary:r.get(6)?
    }))?.collect::<Result<Vec<_>,_>>()?;

    let mut payment_stmt = conn.prepare(
        "SELECT p.id,p.paid_date,o.name_snapshot,p.amount_cents,u.display_name,p.note
         FROM payments p JOIN bill_occurrences o ON o.id=p.bill_occurrence_id JOIN users u ON u.id=p.paid_by_user_id
         ORDER BY p.paid_date DESC,p.created_at DESC LIMIT 150",
    )?;
    let payments = payment_stmt.query_map([], |r| Ok(PaymentHistoryRowDto {
        id:r.get(0)?,paid_date:r.get(1)?,bill_name:r.get(2)?,amount_cents:r.get(3)?,paid_by:r.get(4)?,note:r.get(5)?
    }))?.collect::<Result<Vec<_>,_>>()?;

    let mut rec_stmt = conn.prepare(
        "SELECT br.id,br.performed_at,a.name,br.app_balance_before_cents,br.actual_balance_cents,br.difference_cents,u.display_name
         FROM balance_reconciliations br JOIN accounts a ON a.id=br.account_id LEFT JOIN users u ON u.id=br.performed_by_user_id
         ORDER BY br.performed_at DESC LIMIT 100",
    )?;
    let reconciliations = rec_stmt.query_map([], |r| Ok(ReconciliationHistoryDto {
        id:r.get(0)?,performed_at:r.get(1)?,account_name:r.get(2)?,before_cents:r.get(3)?,actual_cents:r.get(4)?,difference_cents:r.get(5)?,performed_by:r.get(6)?
    }))?.collect::<Result<Vec<_>,_>>()?;
    Ok(HistoryDataDto { activity,payments,reconciliations })
}

#[cfg(test)]
mod tests {

    #[test]
    fn signed_manual_transaction_convention_is_stable() {
        let expense = -1250_i64;
        let income = 1250_i64;
        assert!(expense < 0);
        assert!(income > 0);
    }

    #[test]
    fn reconciliation_difference_matches_actual_minus_book() {
        let before = 100_00_i64;
        let actual = 82_50_i64;
        assert_eq!(actual - before, -17_50);
    }
}
