use std::collections::{BTreeSet, HashMap};

use chrono::{Datelike, Duration, Local, NaiveDate};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::State;
use uuid::Uuid;

use crate::{
    domain::{
        money::Money,
        models::{BillForSchedule, OptionalCommitmentForSchedule, OptionalCommitmentKind, PaycheckForSchedule, PaymentType, Priority},
    },
    error::{AppError, AppResult},
    scheduler::{
        self,
        recurrence::{
            generate_monthly_bill_dates, generate_pay_dates, BusinessDayRule, PayScheduleRule, PaymentWindowRule,
        },
        BucketStatus, ScheduleInput, ScheduleResult,
    },
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

fn payment_type_from(value: &str) -> PaymentType {
    if value == "autopay" { PaymentType::Autopay } else { PaymentType::Manual }
}

fn priority_from(value: &str) -> Priority {
    match value {
        "essential" => Priority::Essential,
        "flexible" => Priority::Flexible,
        _ => Priority::Normal,
    }
}

fn bucket_status_string(value: BucketStatus) -> &'static str {
    match value {
        BucketStatus::Healthy => "healthy",
        BucketStatus::Tight => "tight",
        BucketStatus::Shortage => "shortage",
    }
}

fn format_money_cents(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    format!("{}${}.{:02}", sign, abs / 100, abs % 100)
}

fn funding_source_string(value: crate::domain::models::FundingSourceType) -> &'static str {
    match value {
        crate::domain::models::FundingSourceType::CurrentCash => "current_cash",
        crate::domain::models::FundingSourceType::Paycheck => "paycheck",
    }
}

fn reason_code_string(value: crate::domain::models::ScheduleReasonCode) -> &'static str {
    use crate::domain::models::ScheduleReasonCode::*;
    match value {
        CurrentCash => "current_cash",
        LatestEligiblePaycheck => "latest_eligible_paycheck",
        AutopayLatestEligiblePaycheck => "autopay_latest_eligible_paycheck",
        StableExistingAssignment => "stable_existing_assignment",
        MovedEarlierToProtectBuffer => "moved_earlier_to_protect_buffer",
        UserLock => "user_lock",
        SplitAcrossPaychecks => "split_across_paychecks",
        ReservedAcrossPaychecks => "reserved_across_paychecks",
        AllocatedWithShortage => "allocated_with_shortage",
        PartialFunding => "partial_funding",
    }
}

fn primary_account_id(conn: &Connection) -> AppResult<String> {
    conn.query_row(
        "SELECT id FROM accounts WHERE is_primary_bill_account=1 AND is_active=1 LIMIT 1",
        [],
        |r| r.get(0),
    )
    .optional()?
    .ok_or_else(|| AppError::Validation("primary bill account is not configured".into()))
}

fn estimate_for_template(conn: &Connection, template_id: &str, fallback: i64, window: i64) -> AppResult<i64> {
    let mut stmt = conn.prepare(
        "SELECT total_paid FROM (
            SELECT SUM(p.amount_cents) AS total_paid, MAX(p.paid_date) AS paid_date
            FROM payments p
            JOIN bill_occurrences o ON o.id=p.bill_occurrence_id
            WHERE o.bill_template_id=?1
            GROUP BY o.id
            ORDER BY paid_date DESC
            LIMIT ?2
        )",
    )?;
    let values = stmt
        .query_map(params![template_id, window.max(1)], |r| r.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    // Delegate to the scheduler's estimator so there is exactly one rounding rule.
    // A second, truncating average here drifts a cent from what the tests assert.
    let history = values.into_iter().map(Money::cents).collect::<Vec<_>>();
    Ok(scheduler::estimate_variable_bill(&history, Money::cents(fallback.max(0))).value())
}

fn ensure_template_occurrences(conn: &Connection, template_id: &str, through: NaiveDate) -> AppResult<()> {
    let row = conn
        .query_row(
            "SELECT name,category_id,amount_type,fixed_amount_cents,fallback_estimate_cents,estimate_window_count,
                    recurrence_type,due_day,payment_type,priority,pay_earliest_days_before
             FROM bill_templates WHERE id=?1 AND is_active=1",
            [template_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<i64>>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, Option<i64>>(7)?,
                    r.get::<_, String>(8)?,
                    r.get::<_, String>(9)?,
                    r.get::<_, i64>(10)?,
                ))
            },
        )
        .optional()?;

    let Some((name, category_id, amount_type, fixed, fallback, window, recurrence, due_day, payment_type, priority, earliest_days)) = row else {
        return Ok(());
    };

    let estimate = if amount_type == "variable" {
        estimate_for_template(conn, template_id, fallback.unwrap_or(0), window)?
    } else {
        fixed.unwrap_or(0)
    };

    if recurrence == "monthly" {
        let day = u32::try_from(due_day.unwrap_or(1).clamp(1, 31)).unwrap_or(1);
        let planning = today();
        let dates = generate_monthly_bill_dates(
            planning,
            through,
            day,
            BusinessDayRule::PriorBusinessDay,
            PaymentWindowRule::DaysBeforeDue {
                earliest_days_before: u32::try_from(earliest_days.max(0)).unwrap_or(31),
                latest_days_before: 0,
            },
            &BTreeSet::new(),
        );
        for generated in dates {
            conn.execute(
                "INSERT OR IGNORE INTO bill_occurrences(
                    id,bill_template_id,name_snapshot,category_id,due_date,latest_payment_date,earliest_payment_date,
                    estimated_amount_cents,status,payment_type_snapshot,priority_snapshot,is_one_time
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'upcoming',?9,?10,0)",
                params![
                    Uuid::new_v4().to_string(),
                    template_id,
                    name,
                    category_id,
                    date_string(generated.due_date),
                    date_string(generated.latest_payment_date),
                    date_string(generated.earliest_payment_date),
                    estimate,
                    payment_type,
                    priority,
                ],
            )?;
        }

        // `INSERT OR IGNORE` cannot refresh a row that already exists, so a new
        // rolling average would otherwise only reach occurrences beyond the
        // pre-generated horizon. Push the re-estimate onto future occurrences that
        // the household has not already pinned or started paying.
        if amount_type == "variable" {
            conn.execute(
                "UPDATE bill_occurrences
                    SET estimated_amount_cents=?2, updated_at=CURRENT_TIMESTAMP
                  WHERE bill_template_id=?1
                    AND status IN ('upcoming','scheduled')
                    AND due_date>=date('now','localtime')
                    AND manual_amount_override_cents IS NULL
                    AND actual_required_amount_cents IS NULL
                    AND estimated_amount_cents<>?2
                    AND NOT EXISTS (SELECT 1 FROM payments p WHERE p.bill_occurrence_id=bill_occurrences.id)",
                params![template_id, estimate],
            )?;
        }
    }
    Ok(())
}

fn ensure_all_occurrences(conn: &Connection, through: NaiveDate) -> AppResult<()> {
    let ids = {
        let mut stmt = conn.prepare("SELECT id FROM bill_templates WHERE is_active=1 AND archived_at IS NULL")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let ids = rows.collect::<Result<Vec<_>, _>>()?;
        ids
    };
    for id in ids {
        ensure_template_occurrences(conn, &id, through)?;
    }
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BillListItemDto {
    pub id: String,
    pub name: String,
    pub category_id: Option<String>,
    pub category_name: String,
    pub amount_type: String,
    pub amount_cents: i64,
    pub due_day: Option<i64>,
    pub recurrence_type: String,
    pub payment_type: String,
    pub priority: String,
    pub can_split: bool,
    pub assigned_user_id: Option<String>,
    pub assigned_user_name: Option<String>,
    pub next_occurrence_id: Option<String>,
    pub next_due_date: Option<String>,
    pub next_pay_by_date: Option<String>,
    pub next_status: Option<String>,
    pub assigned_paycheck_date: Option<String>,
    pub assigned_paycheck_owner: Option<String>,
    /// The bill's configured "may pay up to N days before due" window.
    pub pay_earliest_days_before: i64,
    /// Amount still owed on the next occurrence, net of payments already recorded.
    /// `amount_cents` remains the full bill amount for display.
    pub remaining_amount_cents: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveBillPayload {
    pub id: Option<String>,
    pub name: String,
    pub category_id: Option<String>,
    pub amount_type: String,
    pub amount_cents: i64,
    pub due_day: Option<i64>,
    pub recurrence_type: String,
    pub one_time_due_date: Option<String>,
    pub payment_type: String,
    pub priority: String,
    pub can_split: bool,
    pub assigned_user_id: Option<String>,
    pub pay_earliest_days_before: Option<i64>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentHistoryDto {
    pub id: String,
    pub paid_date: String,
    pub amount_cents: i64,
    pub paid_by: String,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BillDetailDto {
    pub bill: BillListItemDto,
    pub notes: Option<String>,
    pub payment_history: Vec<PaymentHistoryDto>,
}

fn list_bills_from_conn(conn: &Connection) -> AppResult<Vec<BillListItemDto>> {
    ensure_all_occurrences(&conn, today() + Duration::days(120))?;
    let mut stmt = conn.prepare(
        "SELECT t.id,t.name,t.category_id,COALESCE(c.name,'Other'),t.amount_type,
                CASE WHEN t.amount_type='fixed' THEN COALESCE(t.fixed_amount_cents,0) ELSE COALESCE(t.fallback_estimate_cents,0) END,
                t.due_day,t.recurrence_type,t.payment_type,t.priority,t.can_split,t.assigned_user_id,u.display_name,
                COALESCE(t.pay_earliest_days_before,31)
         FROM bill_templates t
         LEFT JOIN categories c ON c.id=t.category_id
         LEFT JOIN users u ON u.id=t.assigned_user_id
         WHERE t.is_active=1 AND t.archived_at IS NULL
         ORDER BY COALESCE(t.due_day,99), t.name",
    )?;
    let base = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?, r.get::<_, String>(3)?,
                r.get::<_, String>(4)?, r.get::<_, i64>(5)?, r.get::<_, Option<i64>>(6)?, r.get::<_, String>(7)?,
                r.get::<_, String>(8)?, r.get::<_, String>(9)?, r.get::<_, i64>(10)? == 1,
                r.get::<_, Option<String>>(11)?, r.get::<_, Option<String>>(12)?, r.get::<_, i64>(13)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut out = Vec::new();
    for (id,name,category_id,category_name,amount_type,base_amount,due_day,recurrence_type,payment_type,priority,can_split,assigned_user_id,assigned_user_name,pay_earliest_days_before) in base {
        let next = conn.query_row(
            "SELECT o.id,o.due_date,o.latest_payment_date,o.status,p.pay_date,u.display_name,o.estimated_amount_cents,
                    MAX(COALESCE(o.actual_required_amount_cents,o.manual_amount_override_cents,o.estimated_amount_cents)
                        -COALESCE((SELECT SUM(pay.amount_cents) FROM payments pay WHERE pay.bill_occurrence_id=o.id),0),0)
             FROM bill_occurrences o
             LEFT JOIN bill_allocations a ON a.bill_occurrence_id=o.id
             LEFT JOIN paycheck_occurrences p ON p.id=a.paycheck_occurrence_id
             LEFT JOIN income_sources i ON i.id=p.income_source_id
             LEFT JOIN users u ON u.id=i.user_id
             WHERE o.bill_template_id=?1 AND o.status IN ('upcoming','scheduled','partial','late')
             ORDER BY o.due_date, p.pay_date DESC, a.created_at LIMIT 1",
            [id.as_str()],
            |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,Option<String>>(4)?,r.get::<_,Option<String>>(5)?,r.get::<_,i64>(6)?,r.get::<_,i64>(7)?)),
        ).optional()?;
        let (next_occurrence_id,next_due_date,next_pay_by_date,next_status,assigned_paycheck_date,assigned_paycheck_owner,amount_cents,remaining_amount_cents) = match next {
            Some((oid,d,pay_by,s,pd,po,a,rem)) => (Some(oid),Some(d),Some(pay_by),Some(s),pd,po,a,rem),
            None => (None,None,None,None,None,None,base_amount,base_amount),
        };
        out.push(BillListItemDto { id,name,category_id,category_name,amount_type,amount_cents,due_day,recurrence_type,payment_type,priority,can_split,assigned_user_id,assigned_user_name,next_occurrence_id,next_due_date,next_pay_by_date,next_status,assigned_paycheck_date,assigned_paycheck_owner,pay_earliest_days_before,remaining_amount_cents });
    }
    Ok(out)
}


#[tauri::command]
pub fn list_bills(state: State<'_, AppState>) -> AppResult<Vec<BillListItemDto>> {
    let conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    list_bills_from_conn(&conn)
}
#[tauri::command]
pub fn save_bill(payload: SaveBillPayload, state: State<'_, AppState>) -> AppResult<String> {
    if payload.name.trim().is_empty() { return Err(AppError::Validation("bill name is required".into())); }
    if payload.amount_cents < 0 { return Err(AppError::Validation("bill amount cannot be negative".into())); }
    if !matches!(payload.amount_type.as_str(), "fixed" | "variable") { return Err(AppError::Validation("amount type must be fixed or variable".into())); }
    if !matches!(payload.payment_type.as_str(), "manual" | "autopay") { return Err(AppError::Validation("payment type must be manual or autopay".into())); }
    if !matches!(payload.priority.as_str(), "essential" | "normal" | "flexible") { return Err(AppError::Validation("invalid bill priority".into())); }
    if !matches!(payload.recurrence_type.as_str(), "monthly" | "one_time") { return Err(AppError::Validation("recurrence must be monthly or one_time".into())); }
    if payload.recurrence_type == "monthly" && !(1..=31).contains(&payload.due_day.unwrap_or(0)) { return Err(AppError::Validation("monthly due day must be between 1 and 31".into())); }

    let mut conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    let tx = conn.transaction()?;
    let id = payload.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let fixed = if payload.amount_type == "fixed" { Some(payload.amount_cents) } else { None };
    let fallback = if payload.amount_type == "variable" { Some(payload.amount_cents) } else { None };
    let recurrence_json = if payload.recurrence_type == "one_time" {
        let due = payload.one_time_due_date.as_deref().ok_or_else(|| AppError::Validation("one-time due date is required".into()))?;
        parse_date(due, "one-time due date")?;
        json!({"dueDate": due}).to_string()
    } else { "{}".to_string() };
    let earliest = payload.pay_earliest_days_before.unwrap_or(31).clamp(0, 365);

    if payload.id.is_some() {
        tx.execute(
            "UPDATE bill_templates SET name=?2,category_id=?3,amount_type=?4,fixed_amount_cents=?5,fallback_estimate_cents=?6,
                recurrence_type=?7,recurrence_config_json=?8,payment_type=?9,priority=?10,can_split=?11,assigned_user_id=?12,
                due_day=?13,pay_earliest_days_before=?14,notes=?15,updated_at=CURRENT_TIMESTAMP WHERE id=?1",
            params![id,payload.name.trim(),payload.category_id,payload.amount_type,fixed,fallback,payload.recurrence_type,recurrence_json,payload.payment_type,payload.priority,if payload.can_split {1}else{0},payload.assigned_user_id,payload.due_day,earliest,payload.notes],
        )?;
        // A one-time bill has exactly one occurrence, and editing it means replacing
        // that occurrence even once its due date has passed — otherwise the edit is
        // silently discarded. Monthly bills keep the future-only rule so that
        // past-due occurrences are not erased. Neither branch ever removes an
        // occurrence with recorded payments.
        let regenerable: &str = if payload.recurrence_type == "one_time" {
            "SELECT id FROM bill_occurrences
              WHERE bill_template_id=?1 AND status!='paid'
                AND NOT EXISTS (SELECT 1 FROM payments p WHERE p.bill_occurrence_id=bill_occurrences.id)"
        } else {
            "SELECT id FROM bill_occurrences
              WHERE bill_template_id=?1 AND due_date>=date('now','localtime')
                AND status IN ('upcoming','scheduled')
                AND NOT EXISTS (SELECT 1 FROM payments p WHERE p.bill_occurrence_id=bill_occurrences.id)"
        };
        tx.execute(
            &format!("DELETE FROM bill_allocations WHERE bill_occurrence_id IN ({regenerable})"),
            [id.as_str()],
        )?;
        tx.execute(
            &format!("DELETE FROM bill_occurrences WHERE id IN ({regenerable})"),
            [id.as_str()],
        )?;
    } else {
        tx.execute(
            "INSERT INTO bill_templates(id,name,category_id,amount_type,fixed_amount_cents,fallback_estimate_cents,estimate_window_count,
                recurrence_type,recurrence_config_json,due_rule_json,payment_type,priority,payment_window_type,payment_window_config_json,
                can_split,assigned_user_id,is_active,notes,due_day,pay_earliest_days_before)
             VALUES(?1,?2,?3,?4,?5,?6,6,?7,?8,'{}',?9,?10,'custom','{}',?11,?12,1,?13,?14,?15)",
            params![id,payload.name.trim(),payload.category_id,payload.amount_type,fixed,fallback,payload.recurrence_type,recurrence_json,payload.payment_type,payload.priority,if payload.can_split {1}else{0},payload.assigned_user_id,payload.notes,payload.due_day,earliest],
        )?;
    }

    if payload.recurrence_type == "one_time" {
        let due_text = payload.one_time_due_date.as_deref().unwrap_or_default();
        let due = parse_date(due_text, "one-time due date")?;
        let earliest_date = (due - Duration::days(earliest)).min(due);
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO bill_occurrences(id,bill_template_id,name_snapshot,category_id,due_date,latest_payment_date,earliest_payment_date,
             estimated_amount_cents,status,payment_type_snapshot,priority_snapshot,is_one_time)
             VALUES(?1,?2,?3,?4,?5,?5,?6,?7,'upcoming',?8,?9,1)",
            params![Uuid::new_v4().to_string(),id,payload.name.trim(),payload.category_id,due_text,date_string(earliest_date),payload.amount_cents,payload.payment_type,payload.priority],
        )?;
        // The unique (template, due date) index turns a collision into a silent
        // no-op. The only rows the delete above leaves behind are ones with
        // recorded payments, so say that plainly instead of reporting success.
        if inserted == 0 {
            return Err(AppError::Validation(
                "this bill already has an occurrence on that date with recorded payments. Remove the payment first, or choose a different due date.".into(),
            ));
        }
    }
    tx.execute(
        "INSERT INTO activity_log(id,user_id,event_type,entity_type,entity_id,summary) VALUES(?1,?2,'bill_saved','bill',?3,?4)",
        params![Uuid::new_v4().to_string(),payload.assigned_user_id,id,format!("Saved bill {}",payload.name.trim())],
    )?;
    tx.commit()?;
    ensure_template_occurrences(&conn, &id, today()+Duration::days(120))?;
    drop(conn);
    let _ = run_scheduler_internal(&state)?;
    Ok(id)
}

#[tauri::command]
pub fn get_bill_detail(id: String, state: State<'_, AppState>) -> AppResult<BillDetailDto> {
    let conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    let bills = list_bills_from_conn(&conn)?;
    let bill = bills.into_iter().find(|b| b.id == id).ok_or_else(|| AppError::Validation("bill not found".into()))?;
    let notes = conn.query_row("SELECT notes FROM bill_templates WHERE id=?1", [id.as_str()], |r| r.get::<_,Option<String>>(0)).optional()?.flatten();
    let mut stmt = conn.prepare(
        "SELECT p.id,p.paid_date,p.amount_cents,u.display_name,p.note
         FROM payments p JOIN bill_occurrences o ON o.id=p.bill_occurrence_id JOIN users u ON u.id=p.paid_by_user_id
         WHERE o.bill_template_id=?1 ORDER BY p.paid_date DESC,p.created_at DESC LIMIT 24",
    )?;
    let history = stmt.query_map([id.as_str()], |r| Ok(PaymentHistoryDto { id:r.get(0)?,paid_date:r.get(1)?,amount_cents:r.get(2)?,paid_by:r.get(3)?,note:r.get(4)? }))?.collect::<Result<Vec<_>,_>>()?;
    Ok(BillDetailDto { bill, notes, payment_history:history })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkPaidPayload {
    pub occurrence_id: String,
    pub amount_cents: i64,
    pub paid_date: String,
    pub paid_by_user_id: String,
    pub payment_method: Option<String>,
    pub note: Option<String>,
    pub is_partial: bool,
}

#[tauri::command]
pub fn mark_bill_paid(payload: MarkPaidPayload, state: State<'_, AppState>) -> AppResult<()> {
    if payload.amount_cents <= 0 { return Err(AppError::Validation("payment amount must be greater than zero".into())); }
    parse_date(&payload.paid_date, "paid date")?;
    let mut conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    let tx = conn.transaction()?;
    let (status,name,estimated,manual,actual): (String,String,i64,Option<i64>,Option<i64>) = tx.query_row(
        "SELECT status,name_snapshot,estimated_amount_cents,manual_amount_override_cents,actual_required_amount_cents FROM bill_occurrences WHERE id=?1",
        [payload.occurrence_id.as_str()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))
        .optional()?.ok_or_else(|| AppError::Validation("bill occurrence not found".into()))?;
    if status == "paid" { return Err(AppError::Validation("this bill is already marked paid".into())); }
    let already_paid: i64 = tx.query_row("SELECT COALESCE(SUM(amount_cents),0) FROM payments WHERE bill_occurrence_id=?1", [payload.occurrence_id.as_str()], |r| r.get(0))?;
    let target = actual.or(manual).unwrap_or(estimated).max(already_paid);
    let payment_id = Uuid::new_v4().to_string();
    let account_id = primary_account_id(&tx)?;
    tx.execute(
        "INSERT INTO payments(id,bill_occurrence_id,account_id,amount_cents,paid_date,paid_by_user_id,payment_method,note) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![payment_id,payload.occurrence_id,account_id,payload.amount_cents,payload.paid_date,payload.paid_by_user_id,payload.payment_method,payload.note],
    )?;
    tx.execute("UPDATE accounts SET book_balance_cents=book_balance_cents-?2,updated_at=CURRENT_TIMESTAMP WHERE id=?1", params![account_id,payload.amount_cents])?;
    tx.execute(
        "INSERT INTO transactions(id,account_id,transaction_date,description,amount_cents,transaction_type,status,source,created_by_user_id,note,source_entity_type,source_entity_id)
         VALUES(?1,?2,?3,?4,?5,'bill_payment','cleared','bill_payment',?6,?7,'payment',?8)",
        params![Uuid::new_v4().to_string(),account_id,payload.paid_date,name,-payload.amount_cents,payload.paid_by_user_id,payload.note,payment_id],
    )?;
    let new_total = already_paid + payload.amount_cents;
    let fully_paid = !payload.is_partial || new_total >= target;
    if fully_paid {
        tx.execute("UPDATE bill_occurrences SET status='paid',actual_required_amount_cents=?2,updated_at=CURRENT_TIMESTAMP WHERE id=?1", params![payload.occurrence_id,new_total])?;
        tx.execute("DELETE FROM bill_allocations WHERE bill_occurrence_id=?1", [payload.occurrence_id.as_str()])?;
    } else {
        tx.execute("UPDATE bill_occurrences SET status='partial',updated_at=CURRENT_TIMESTAMP WHERE id=?1", [payload.occurrence_id.as_str()])?;
    }
    tx.execute(
        "INSERT INTO activity_log(id,user_id,event_type,entity_type,entity_id,summary) VALUES(?1,?2,'bill_payment','bill_occurrence',?3,?4)",
        params![Uuid::new_v4().to_string(),payload.paid_by_user_id,payload.occurrence_id,format!("Recorded payment for {name}")],
    )?;
    tx.commit()?;
    drop(conn);
    let _ = run_scheduler_internal(&state)?;
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaycheckDto {
    pub id: String,
    pub user_id: String,
    pub owner_name: String,
    pub pay_date: String,
    pub projected_amount_cents: i64,
    pub expected_amount_cents: Option<i64>,
    pub actual_amount_cents: Option<i64>,
    pub effective_amount_cents: i64,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePaycheckPayload {
    pub id: Option<String>,
    pub user_id: String,
    pub pay_date: String,
    pub projected_amount_cents: i64,
    pub expected_amount_cents: Option<i64>,
    pub actual_amount_cents: Option<i64>,
    pub status: String,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePaycheckPayload {
    pub id: String,
}

fn manual_income_source(conn: &Connection, user_id: &str) -> AppResult<String> {
    if let Some(id) = conn.query_row("SELECT id FROM income_sources WHERE user_id=?1 AND schedule_type='manual' AND is_active=1 LIMIT 1", [user_id], |r| r.get::<_,String>(0)).optional()? {
        return Ok(id);
    }
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO income_sources(id,user_id,name,schedule_type,schedule_config_json,default_projected_amount_cents,weekend_holiday_rule,is_active) VALUES(?1,?2,'Manual Paychecks','manual','{}',0,'prior_business_day',1)",
        params![id,user_id],
    )?;
    Ok(id)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaycheckScheduleDto {
    pub id: String,
    pub user_id: String,
    pub owner_name: String,
    pub frequency: String,
    pub default_projected_amount_cents: i64,
    pub anchor_date: Option<String>,
    pub first_day: Option<u32>,
    pub second_day: Option<u32>,
    pub day_of_month: Option<u32>,
    pub weekend_holiday_rule: String,
    pub next_pay_date: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePaycheckSchedulePayload {
    pub id: Option<String>,
    pub user_id: String,
    pub frequency: String,
    pub default_projected_amount_cents: i64,
    pub anchor_date: Option<String>,
    pub first_day: Option<u32>,
    pub second_day: Option<u32>,
    pub day_of_month: Option<u32>,
    pub weekend_holiday_rule: Option<String>,
}

fn business_day_rule_from(value: &str) -> BusinessDayRule {
    match value {
        "exact" => BusinessDayRule::Exact,
        "next_business_day" => BusinessDayRule::NextBusinessDay,
        _ => BusinessDayRule::PriorBusinessDay,
    }
}

fn pay_schedule_rule(schedule_type: &str, config_json: &str) -> AppResult<PayScheduleRule> {
    let config: serde_json::Value = serde_json::from_str(config_json)
        .map_err(|_| AppError::Validation("paycheck schedule configuration is invalid".into()))?;
    match schedule_type {
        "weekly" => {
            let anchor = config.get("anchorDate").and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Validation("weekly schedules require a next pay date".into()))?;
            Ok(PayScheduleRule::Weekly { anchor: parse_date(anchor, "next pay date")? })
        }
        "biweekly" => {
            let anchor = config.get("anchorDate").and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Validation("biweekly schedules require a next pay date".into()))?;
            Ok(PayScheduleRule::Biweekly { anchor: parse_date(anchor, "next pay date")? })
        }
        "semimonthly" => {
            let first_day = config.get("firstDay").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let second_day = config.get("secondDay").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            if !(1..=31).contains(&first_day) || !(1..=31).contains(&second_day) || first_day == second_day {
                return Err(AppError::Validation("twice-monthly schedules require two different days from 1 through 31".into()));
            }
            Ok(PayScheduleRule::SemiMonthly { first_day, second_day })
        }
        "monthly" => {
            let day = config.get("dayOfMonth").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            if !(1..=31).contains(&day) {
                return Err(AppError::Validation("monthly schedules require a pay day from 1 through 31".into()));
            }
            Ok(PayScheduleRule::Monthly { day })
        }
        _ => Err(AppError::Validation("unsupported paycheck frequency".into())),
    }
}

fn ensure_income_source_occurrences(conn: &Connection, source_id: &str, through: NaiveDate) -> AppResult<()> {
    let source: Option<(String, String, String, i64, String, i64)> = conn.query_row(
        "SELECT user_id,schedule_type,schedule_config_json,default_projected_amount_cents,weekend_holiday_rule,is_active FROM income_sources WHERE id=?1",
        [source_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
    ).optional()?;
    let Some((user_id, schedule_type, config_json, projected, weekend_rule, active)) = source else { return Ok(()); };
    if active != 1 || schedule_type == "manual" { return Ok(()); }

    let rule = pay_schedule_rule(&schedule_type, &config_json)?;
    let dates = generate_pay_dates(
        rule,
        today(),
        through,
        business_day_rule_from(&weekend_rule),
        &BTreeSet::new(),
    );
    for date in dates {
        let date_text = date_string(date);
        let existing: Option<String> = conn.query_row(
            "SELECT p.id FROM paycheck_occurrences p JOIN income_sources i ON i.id=p.income_source_id WHERE i.user_id=?1 AND (p.pay_date=?2 OR p.scheduled_pay_date=?2) LIMIT 1",
            params![user_id, date_text],
            |r| r.get(0),
        ).optional()?;
        if existing.is_some() { continue; }
        conn.execute(
            "INSERT INTO paycheck_occurrences(id,income_source_id,pay_date,scheduled_pay_date,projected_amount_cents,status,is_date_override,posted_to_account) VALUES(?1,?2,?3,?3,?4,'projected',0,0)",
            params![Uuid::new_v4().to_string(), source_id, date_text, projected.max(0)],
        )?;
    }
    Ok(())
}

fn ensure_all_paycheck_occurrences(conn: &Connection, through: NaiveDate) -> AppResult<()> {
    let source_ids = {
        let mut stmt = conn.prepare("SELECT id FROM income_sources WHERE is_active=1 AND schedule_type<>'manual'")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    for source_id in source_ids {
        ensure_income_source_occurrences(conn, &source_id, through)?;
    }
    Ok(())
}

#[tauri::command]
pub fn list_paycheck_schedules(state: State<'_, AppState>) -> AppResult<Vec<PaycheckScheduleDto>> {
    let conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    ensure_all_paycheck_occurrences(&conn, today() + Duration::days(365))?;
    let mut stmt = conn.prepare(
        "SELECT i.id,i.user_id,u.display_name,i.schedule_type,i.schedule_config_json,i.default_projected_amount_cents,i.weekend_holiday_rule,
                (SELECT MIN(p.pay_date) FROM paycheck_occurrences p WHERE p.income_source_id=i.id AND p.pay_date>=?1 AND p.status<>'skipped')
         FROM income_sources i JOIN users u ON u.id=i.user_id
         WHERE i.is_active=1 AND i.schedule_type<>'manual' ORDER BY u.display_name",
    )?;
    let today_text = date_string(today());
    let items = stmt.query_map(params![today_text], |r| {
        let config_text: String = r.get(4)?;
        let config: serde_json::Value = serde_json::from_str(&config_text).unwrap_or_else(|_| json!({}));
        Ok(PaycheckScheduleDto {
            id: r.get(0)?,
            user_id: r.get(1)?,
            owner_name: r.get(2)?,
            frequency: r.get(3)?,
            default_projected_amount_cents: r.get(5)?,
            anchor_date: config.get("anchorDate").and_then(|v| v.as_str()).map(str::to_string),
            first_day: config.get("firstDay").and_then(|v| v.as_u64()).map(|v| v as u32),
            second_day: config.get("secondDay").and_then(|v| v.as_u64()).map(|v| v as u32),
            day_of_month: config.get("dayOfMonth").and_then(|v| v.as_u64()).map(|v| v as u32),
            weekend_holiday_rule: r.get(6)?,
            next_pay_date: r.get(7)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

#[tauri::command]
pub fn save_paycheck_schedule(payload: SavePaycheckSchedulePayload, state: State<'_, AppState>) -> AppResult<String> {
    if payload.default_projected_amount_cents < 0 {
        return Err(AppError::Validation("normal paycheck amount cannot be negative".into()));
    }
    if !matches!(payload.frequency.as_str(), "weekly" | "biweekly" | "semimonthly" | "monthly") {
        return Err(AppError::Validation("select weekly, every 2 weeks, twice monthly, or monthly".into()));
    }
    let weekend_rule = payload.weekend_holiday_rule.clone().unwrap_or_else(|| "prior_business_day".into());
    if !matches!(weekend_rule.as_str(), "exact" | "prior_business_day" | "next_business_day") {
        return Err(AppError::Validation("invalid weekend/holiday rule".into()));
    }

    let config = match payload.frequency.as_str() {
        "weekly" | "biweekly" => {
            let anchor = payload.anchor_date.as_deref().ok_or_else(|| AppError::Validation("enter the next pay date".into()))?;
            parse_date(anchor, "next pay date")?;
            json!({"anchorDate": anchor})
        }
        "semimonthly" => {
            let first = payload.first_day.unwrap_or(0);
            let second = payload.second_day.unwrap_or(0);
            if !(1..=31).contains(&first) || !(1..=31).contains(&second) || first == second {
                return Err(AppError::Validation("enter two different paycheck days from 1 through 31".into()));
            }
            json!({"firstDay": first, "secondDay": second})
        }
        "monthly" => {
            let day = payload.day_of_month.unwrap_or(0);
            if !(1..=31).contains(&day) {
                return Err(AppError::Validation("enter a monthly paycheck day from 1 through 31".into()));
            }
            json!({"dayOfMonth": day})
        }
        _ => unreachable!(),
    };

    let mut conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    let tx = conn.transaction()?;
    let owner: String = tx.query_row("SELECT display_name FROM users WHERE id=?1", [payload.user_id.as_str()], |r| r.get(0))
        .optional()?.ok_or_else(|| AppError::Validation("household member was not found".into()))?;
    let existing_for_user: Option<String> = tx.query_row(
        "SELECT id FROM income_sources WHERE user_id=?1 AND is_active=1 AND schedule_type<>'manual' LIMIT 1",
        [payload.user_id.as_str()], |r| r.get(0),
    ).optional()?;
    let id = payload.id.clone().or(existing_for_user).unwrap_or_else(|| Uuid::new_v4().to_string());

    let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM income_sources WHERE id=?1)", [id.as_str()], |r| r.get::<_, i64>(0)).unwrap_or(0) == 1;
    if exists {
        tx.execute(
            "UPDATE income_sources SET user_id=?2,name=?3,schedule_type=?4,schedule_config_json=?5,default_projected_amount_cents=?6,weekend_holiday_rule=?7,is_active=1,updated_at=CURRENT_TIMESTAMP,archived_at=NULL WHERE id=?1",
            params![id,payload.user_id,format!("{} Paycheck", owner),payload.frequency,config.to_string(),payload.default_projected_amount_cents,weekend_rule],
        )?;
        tx.execute(
            "DELETE FROM bill_allocations WHERE paycheck_occurrence_id IN (SELECT id FROM paycheck_occurrences WHERE income_source_id=?1 AND pay_date>=?2 AND status='projected' AND expected_amount_cents IS NULL AND actual_amount_cents IS NULL AND posted_to_account=0 AND is_date_override=0)",
            params![id,date_string(today())],
        )?;
        tx.execute(
            "DELETE FROM paycheck_occurrences WHERE income_source_id=?1 AND pay_date>=?2 AND status='projected' AND expected_amount_cents IS NULL AND actual_amount_cents IS NULL AND posted_to_account=0 AND is_date_override=0",
            params![id,date_string(today())],
        )?;
    } else {
        tx.execute(
            "INSERT INTO income_sources(id,user_id,name,schedule_type,schedule_config_json,default_projected_amount_cents,weekend_holiday_rule,is_active) VALUES(?1,?2,?3,?4,?5,?6,?7,1)",
            params![id,payload.user_id,format!("{} Paycheck", owner),payload.frequency,config.to_string(),payload.default_projected_amount_cents,weekend_rule],
        )?;
    }
    tx.execute(
        "INSERT INTO activity_log(id,user_id,event_type,entity_type,entity_id,summary) VALUES(?1,?2,'paycheck_schedule_saved','income_source',?3,?4)",
        params![Uuid::new_v4().to_string(),payload.user_id,id,format!("Saved {} paycheck schedule",payload.frequency)],
    )?;
    tx.commit()?;
    ensure_income_source_occurrences(&conn, &id, today() + Duration::days(365))?;
    drop(conn);
    let _ = run_scheduler_internal(&state)?;
    Ok(id)
}

#[tauri::command]
pub fn list_paychecks(state: State<'_, AppState>) -> AppResult<Vec<PaycheckDto>> {
    let conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    ensure_all_paycheck_occurrences(&conn, today() + Duration::days(180))?;
    let start = date_string(today()-Duration::days(45));
    let end = date_string(today()+Duration::days(180));
    let mut stmt = conn.prepare(
        "SELECT p.id,i.user_id,u.display_name,p.pay_date,p.projected_amount_cents,p.expected_amount_cents,p.actual_amount_cents,
                COALESCE(p.actual_amount_cents,p.expected_amount_cents,p.projected_amount_cents),p.status
         FROM paycheck_occurrences p JOIN income_sources i ON i.id=p.income_source_id JOIN users u ON u.id=i.user_id
         WHERE p.pay_date BETWEEN ?1 AND ?2 ORDER BY p.pay_date",
    )?;
    let items = stmt.query_map(params![start,end], |r| Ok(PaycheckDto { id:r.get(0)?,user_id:r.get(1)?,owner_name:r.get(2)?,pay_date:r.get(3)?,projected_amount_cents:r.get(4)?,expected_amount_cents:r.get(5)?,actual_amount_cents:r.get(6)?,effective_amount_cents:r.get(7)?,status:r.get(8)? }))?.collect::<Result<Vec<_>,_>>()?;
    Ok(items)
}

#[tauri::command]
pub fn save_paycheck(payload: SavePaycheckPayload, state: State<'_, AppState>) -> AppResult<String> {
    let pay_date = parse_date(&payload.pay_date,"pay date")?;
    if payload.status == "received" && pay_date > today() { return Err(AppError::Validation("a paycheck cannot be marked received before its deposit date".into())); }
    if payload.projected_amount_cents < 0 || payload.expected_amount_cents.unwrap_or(0) < 0 || payload.actual_amount_cents.unwrap_or(0) < 0 { return Err(AppError::Validation("paycheck amounts cannot be negative".into())); }
    if !matches!(payload.status.as_str(), "projected"|"updated"|"received"|"skipped") { return Err(AppError::Validation("invalid paycheck status".into())); }
    if payload.status == "received" && payload.actual_amount_cents.is_none() { return Err(AppError::Validation("received paychecks require the actual deposited amount".into())); }
    let mut conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    let tx = conn.transaction()?;
    let id = payload.id.clone().unwrap_or_else(||Uuid::new_v4().to_string());
    let existing_meta: Option<(String,String,i64,i64)> = tx.query_row(
        "SELECT income_source_id,pay_date,COALESCE(actual_amount_cents,expected_amount_cents,projected_amount_cents),posted_to_account FROM paycheck_occurrences WHERE id=?1",
        [id.as_str()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
    let source_id = if let Some((source_id,_,_,_)) = &existing_meta { source_id.clone() } else { manual_income_source(&tx,&payload.user_id)? };

    // One paycheck occurrence per person/date. Accidental duplicates make the
    // planner double-count income and are almost always data-entry mistakes.
    // A skipped occurrence does not block a replacement entry for that date.
    let duplicate: Option<String> = tx.query_row(
        "SELECT p.id FROM paycheck_occurrences p
         JOIN income_sources i ON i.id=p.income_source_id
         WHERE i.user_id=?1 AND p.pay_date=?2 AND p.id<>?3 AND p.status<>'skipped'
         LIMIT 1",
        params![payload.user_id,payload.pay_date,id],
        |r| r.get(0),
    ).optional()?;
    if duplicate.is_some() {
        let owner: String = tx.query_row("SELECT display_name FROM users WHERE id=?1",[payload.user_id.as_str()],|r|r.get(0))
            .optional()?.unwrap_or_else(||"This person".into());
        return Err(AppError::Validation(format!(
            "A paycheck for {owner} on {} already exists. Open the existing paycheck to update or remove it instead.",
            payload.pay_date
        )));
    }
    let existing: Option<(i64,i64)> = existing_meta.as_ref().map(|(_,_,effective,posted)| (*effective,*posted));
    if matches!(existing, Some((_, 1))) && payload.status != "received" {
        return Err(AppError::Validation("a received paycheck cannot be changed back to a projected status; edit its actual amount instead".into()));
    }
    if payload.id.is_some() {
        let date_override = existing_meta.as_ref().map(|(_,old_date,_,_)| old_date != &payload.pay_date).unwrap_or(false);
        tx.execute(
            "UPDATE paycheck_occurrences SET income_source_id=?2,pay_date=?3,projected_amount_cents=?4,expected_amount_cents=?5,actual_amount_cents=?6,status=?7,note=?8,is_date_override=CASE WHEN ?9=1 THEN 1 ELSE is_date_override END,updated_at=CURRENT_TIMESTAMP WHERE id=?1",
            params![id,source_id,payload.pay_date,payload.projected_amount_cents,payload.expected_amount_cents,payload.actual_amount_cents,payload.status,payload.note,if date_override {1} else {0}],
        )?;
    } else {
        tx.execute(
            "INSERT INTO paycheck_occurrences(id,income_source_id,pay_date,projected_amount_cents,expected_amount_cents,actual_amount_cents,status,note,posted_to_account,is_date_override) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,0,1)",
            params![id,source_id,payload.pay_date,payload.projected_amount_cents,payload.expected_amount_cents,payload.actual_amount_cents,payload.status,payload.note],
        )?;
    }
    let effective = payload.actual_amount_cents.or(payload.expected_amount_cents).unwrap_or(payload.projected_amount_cents);
    if payload.status == "received" {
        let account_id = primary_account_id(&tx)?;
        match existing {
            Some((old_effective,posted)) if posted == 1 => {
                let delta = effective-old_effective;
                if delta != 0 {
                    tx.execute("UPDATE accounts SET book_balance_cents=book_balance_cents+?2,updated_at=CURRENT_TIMESTAMP WHERE id=?1",params![account_id,delta])?;
                }
                tx.execute("UPDATE transactions SET amount_cents=?2,transaction_date=?3,description=?4,created_by_user_id=?5,updated_at=CURRENT_TIMESTAMP WHERE source_entity_type='paycheck' AND source_entity_id=?1",params![id,effective,payload.pay_date,format!("Paycheck deposit"),payload.user_id])?;
            }
            _ => {
                tx.execute("UPDATE accounts SET book_balance_cents=book_balance_cents+?2,updated_at=CURRENT_TIMESTAMP WHERE id=?1",params![account_id,effective])?;
                tx.execute(
                    "INSERT INTO transactions(id,account_id,transaction_date,description,amount_cents,transaction_type,status,source,created_by_user_id,source_entity_type,source_entity_id)
                     VALUES(?1,?2,?3,'Paycheck deposit',?4,'income','cleared','paycheck',?5,'paycheck',?6)",
                    params![Uuid::new_v4().to_string(),account_id,payload.pay_date,effective,payload.user_id,id],
                )?;
                tx.execute("UPDATE paycheck_occurrences SET posted_to_account=1 WHERE id=?1",[id.as_str()])?;
            }
        }
    }
    tx.execute(
        "INSERT INTO activity_log(id,user_id,event_type,entity_type,entity_id,summary) VALUES(?1,?2,'paycheck_saved','paycheck',?3,?4)",
        params![Uuid::new_v4().to_string(),payload.user_id,id,format!("Saved paycheck for {}",payload.pay_date)],
    )?;
    tx.commit()?;
    drop(conn);
    let _ = run_scheduler_internal(&state)?;
    Ok(id)
}

#[tauri::command]
pub fn delete_paycheck(payload: DeletePaycheckPayload, state: State<'_, AppState>) -> AppResult<()> {
    let mut conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    let tx = conn.transaction()?;

    let existing: Option<(String, String, i64, i64)> = tx.query_row(
        "SELECT i.user_id,p.pay_date,COALESCE(p.actual_amount_cents,p.expected_amount_cents,p.projected_amount_cents),p.posted_to_account
         FROM paycheck_occurrences p JOIN income_sources i ON i.id=p.income_source_id WHERE p.id=?1",
        [payload.id.as_str()],
        |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
    ).optional()?;
    let Some((user_id, pay_date, effective_amount, posted)) = existing else {
        return Err(AppError::Validation("paycheck was not found".into()));
    };

    // Scheduler allocations are derived data and will be rebuilt immediately.
    tx.execute("DELETE FROM bill_allocations WHERE paycheck_occurrence_id=?1",[payload.id.as_str()])?;

    if posted == 1 {
        let account_id = primary_account_id(&tx)?;
        tx.execute(
            "UPDATE accounts SET book_balance_cents=book_balance_cents-?2,updated_at=CURRENT_TIMESTAMP WHERE id=?1",
            params![account_id,effective_amount],
        )?;
        tx.execute(
            "DELETE FROM transactions WHERE source_entity_type='paycheck' AND source_entity_id=?1",
            [payload.id.as_str()],
        )?;
    }

    tx.execute("DELETE FROM paycheck_occurrences WHERE id=?1",[payload.id.as_str()])?;
    tx.execute(
        "INSERT INTO activity_log(id,user_id,event_type,entity_type,entity_id,summary) VALUES(?1,?2,'paycheck_deleted','paycheck',?3,?4)",
        params![Uuid::new_v4().to_string(),user_id,payload.id,format!("Removed paycheck for {pay_date}")],
    )?;
    tx.commit()?;
    drop(conn);
    let _ = run_scheduler_internal(&state)?;
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerBillDto {
    pub occurrence_id: String,
    pub name: String,
    pub amount_cents: i64,
    pub due_date: String,
    pub payment_type: String,
    pub priority: String,
    pub status: String,
    pub payment_date: String,
    pub reason_code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerCommitmentDto {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub requested_amount_cents: i64,
    pub effective_amount_cents: i64,
    pub reduced_by_cents: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerPaycheckDto {
    pub id: String,
    pub owner_name: String,
    pub pay_date: String,
    pub amount_cents: i64,
    pub status: String,
    pub bills_total_cents: i64,
    pub commitments_total_cents: i64,
    pub safe_remaining_cents: i64,
    pub bills: Vec<PlannerBillDto>,
    pub commitments: Vec<PlannerCommitmentDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerWarningDto {
    pub code: String,
    pub message: String,
    pub date: Option<String>,
    pub amount_cents: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannerViewDto {
    pub planning_date: String,
    pub protected_buffer_cents: i64,
    pub current_cash_cents: i64,
    pub current_cash_safe_cents: i64,
    pub paychecks: Vec<PlannerPaycheckDto>,
    pub warnings: Vec<PlannerWarningDto>,
    pub unresolved_bill_count: usize,
}

fn build_schedule_input(conn: &Connection) -> AppResult<ScheduleInput> {
    let planning_date = today();
    let horizon_days: i64 = conn.query_row("SELECT default_planning_horizon_days FROM household_settings WHERE id=1",[],|r|r.get(0)).optional()?.unwrap_or(90);
    // The Paycheck Planner always offers 30 / 60 / 90 day filters, so load at least
    // 90 days of deterministic schedule data even if another app setting uses a
    // shorter default planning horizon. The UI then filters this stable result.
    let planner_horizon_days = horizon_days.max(90);
    ensure_all_occurrences(conn,planning_date+Duration::days(planner_horizon_days))?;
    let starting_cash: i64 = conn.query_row("SELECT COALESCE(SUM(book_balance_cents),0) FROM accounts WHERE is_active=1 AND account_type!='credit'",[],|r|r.get(0))?;
    let (buffer,tight): (i64,i64) = conn.query_row("SELECT protected_buffer_cents,tight_headroom_cents FROM household_settings WHERE id=1",[],|r|Ok((r.get(0)?,r.get(1)?))).optional()?.unwrap_or((50_000,10_000));
    let end = date_string(planning_date+Duration::days(planner_horizon_days));
    let start = date_string(planning_date);
    let mut p_stmt = conn.prepare(
        "SELECT id,pay_date,COALESCE(actual_amount_cents,expected_amount_cents,projected_amount_cents)
         FROM paycheck_occurrences WHERE status IN ('projected','updated') AND pay_date BETWEEN ?1 AND ?2 ORDER BY pay_date",
    )?;
    let paychecks = p_stmt.query_map(params![start,end],|r|{
        let date:String=r.get(1)?;
        let parsed=NaiveDate::parse_from_str(&date,"%Y-%m-%d").map_err(|_|rusqlite::Error::InvalidQuery)?;
        Ok(PaycheckForSchedule{id:r.get(0)?,pay_date:parsed,amount:Money::cents(r.get(2)?)})
    })?.collect::<Result<Vec<_>,_>>()?;

    let mut b_stmt = conn.prepare(
        "SELECT o.id,o.due_date,o.earliest_payment_date,o.latest_payment_date,
                MAX(COALESCE(o.actual_required_amount_cents,o.manual_amount_override_cents,o.estimated_amount_cents)-COALESCE((SELECT SUM(amount_cents) FROM payments p WHERE p.bill_occurrence_id=o.id),0),0),
                o.payment_type_snapshot,o.priority_snapshot,t.can_split,
                (SELECT a.paycheck_occurrence_id FROM bill_allocations a WHERE a.bill_occurrence_id=o.id AND a.is_locked=1 LIMIT 1),
                (SELECT a.paycheck_occurrence_id FROM bill_allocations a WHERE a.bill_occurrence_id=o.id AND a.paycheck_occurrence_id IS NOT NULL ORDER BY a.updated_at DESC LIMIT 1)
         FROM bill_occurrences o LEFT JOIN bill_templates t ON t.id=o.bill_template_id
         WHERE o.status IN ('upcoming','scheduled','partial','late') AND o.latest_payment_date<=?1 AND o.due_date>=date(?2,'-30 day')
         GROUP BY o.id ORDER BY o.due_date",
    )?;
    let bills = b_stmt.query_map(params![end,start],|r|{
        let parse = |s:String| NaiveDate::parse_from_str(&s,"%Y-%m-%d").map_err(|_|rusqlite::Error::InvalidQuery);
        Ok(BillForSchedule{
            id:r.get(0)?,due_date:parse(r.get(1)?)?,earliest_payment_date:parse(r.get(2)?)?,latest_payment_date:parse(r.get(3)?)?,
            amount:Money::cents(r.get(4)?),payment_type:payment_type_from(&r.get::<_,String>(5)?),priority:priority_from(&r.get::<_,String>(6)?),
            can_split:r.get::<_,i64>(7)?==1,locked_paycheck_id:r.get(8)?,existing_paycheck_id:r.get(9)?,
        })
    })?.collect::<Result<Vec<_>,_>>()?;

    // Phase 5 optional savings and extra debt payments ride on top of the required-bill
    // plan. The scheduler is allowed to reduce these before it jeopardizes bills or the
    // protected buffer. Recording an actual contribution/payment remains a separate user action.
    let mut optional_commitments = Vec::<OptionalCommitmentForSchedule>::new();
    {
        let mut stmt = conn.prepare("SELECT id,goal_type,target_amount_cents,current_amount_cents,planned_contribution_cents,COALESCE(contribution_frequency,'per_paycheck') FROM savings_goals WHERE is_active=1 AND planned_contribution_cents>0")?;
        let rows = stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,i64>(3)?,r.get::<_,i64>(4)?,r.get::<_,String>(5)?)))?.collect::<Result<Vec<_>,_>>()?;
        for (goal_id,goal_type,target,current,amount,frequency) in rows {
            if target > 0 && current >= target { continue; }
            let kind = if goal_type == "sinking_fund" { OptionalCommitmentKind::SinkingFund } else { OptionalCommitmentKind::OptionalSavings };
            if frequency == "per_paycheck" {
                for p in &paychecks { optional_commitments.push(OptionalCommitmentForSchedule{id:format!("savings:{goal_id}:{}",p.id),scheduled_date:p.pay_date,target_paycheck_id:Some(p.id.clone()),amount:Money::cents(amount),minimum_amount:Money::ZERO,kind}); }
            } else if frequency == "monthly" {
                let mut seen = BTreeSet::<(i32,u32)>::new();
                for p in &paychecks { let key=(p.pay_date.year(),p.pay_date.month()); if seen.insert(key) { optional_commitments.push(OptionalCommitmentForSchedule{id:format!("savings:{goal_id}:{}",p.id),scheduled_date:p.pay_date,target_paycheck_id:Some(p.id.clone()),amount:Money::cents(amount),minimum_amount:Money::ZERO,kind}); } }
            }
        }
    }
    {
        let mut stmt = conn.prepare("SELECT id,planned_payment_cents FROM debts WHERE is_active=1 AND balance_cents>0 AND planned_payment_cents>0")?;
        let rows = stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?)))?.collect::<Result<Vec<_>,_>>()?;
        for (debt_id,amount) in rows {
            let mut seen = BTreeSet::<(i32,u32)>::new();
            for p in &paychecks { let key=(p.pay_date.year(),p.pay_date.month()); if seen.insert(key) { optional_commitments.push(OptionalCommitmentForSchedule{id:format!("debt:{debt_id}:{}",p.id),scheduled_date:p.pay_date,target_paycheck_id:Some(p.id.clone()),amount:Money::cents(amount),minimum_amount:Money::ZERO,kind:OptionalCommitmentKind::ExtraDebt}); } }
        }
    }
    Ok(ScheduleInput{planning_date,starting_cash:Money::cents(starting_cash),protected_buffer:Money::cents(buffer),tight_headroom:Money::cents(tight),paychecks,bills,optional_commitments})
}

fn persist_schedule(conn: &mut Connection, input: &ScheduleInput, result: &ScheduleResult) -> AppResult<()> {
    let tx = conn.transaction()?;
    for bill in &input.bills {
        tx.execute("DELETE FROM bill_allocations WHERE bill_occurrence_id=?1 AND is_locked=0",[bill.id.as_str()])?;
        tx.execute("UPDATE bill_occurrences SET status=CASE WHEN status='partial' THEN 'partial' ELSE 'upcoming' END,scheduled_payment_date=NULL,updated_at=CURRENT_TIMESTAMP WHERE id=?1 AND status!='paid'",[bill.id.as_str()])?;
    }
    let unresolved: std::collections::HashSet<&str> = result.unresolved_bill_ids.iter().map(String::as_str).collect();
    for allocation in &result.allocations {
        // A locked allocation survives the DELETE above, so re-inserting the
        // scheduler's own row for the same occurrence would leave two rows and
        // double-count the bill. The user's lock wins.
        let locked: i64 = tx.query_row(
            "SELECT COUNT(*) FROM bill_allocations WHERE bill_occurrence_id=?1 AND is_locked=1",
            [allocation.bill_id.as_str()],
            |r| r.get(0),
        )?;
        if locked > 0 { continue; }
        tx.execute(
            "INSERT INTO bill_allocations(id,bill_occurrence_id,paycheck_occurrence_id,funding_source_type,allocated_amount_cents,source,is_locked,reason_code,recommended_payment_date)
             VALUES(?1,?2,?3,?4,?5,'scheduler',0,?6,?7)",
            params![Uuid::new_v4().to_string(),allocation.bill_id,allocation.paycheck_id,funding_source_string(allocation.funding_source_type),allocation.amount.value(),reason_code_string(allocation.reason_code),date_string(allocation.payment_date)],
        )?;
        tx.execute(
            "UPDATE bill_occurrences SET status=CASE WHEN status='partial' THEN 'partial' ELSE 'scheduled' END,
             scheduled_payment_date=CASE WHEN scheduled_payment_date IS NULL OR scheduled_payment_date>?2 THEN ?2 ELSE scheduled_payment_date END,
             updated_at=CURRENT_TIMESTAMP WHERE id=?1",
            params![allocation.bill_id,date_string(allocation.payment_date)],
        )?;
    }
    for bill_id in unresolved {
        let has_alloc: i64 = tx.query_row("SELECT COUNT(*) FROM bill_allocations WHERE bill_occurrence_id=?1",[bill_id],|r|r.get(0))?;
        if has_alloc>0 { tx.execute("UPDATE bill_occurrences SET status='partial',updated_at=CURRENT_TIMESTAMP WHERE id=?1 AND status!='paid'",[bill_id])?; }
    }
    tx.commit()?;
    Ok(())
}

pub(crate) fn run_scheduler_internal(state: &AppState) -> AppResult<PlannerViewDto> {
    let mut conn = state.db.lock().map_err(|_| AppError::Validation("database lock poisoned".into()))?;
    let horizon_days: i64 = conn.query_row("SELECT default_planning_horizon_days FROM household_settings WHERE id=1",[],|r|r.get(0)).optional()?.unwrap_or(90);
    ensure_all_paycheck_occurrences(&conn, today() + Duration::days(horizon_days.max(180)))?;
    let input=build_schedule_input(&conn)?;
    let result=scheduler::build_plan(&input);
    persist_schedule(&mut conn,&input,&result)?;
    planner_from_result(&conn,&input,&result)
}

fn planner_from_result(conn:&Connection,input:&ScheduleInput,result:&ScheduleResult)->AppResult<PlannerViewDto>{
    let mut owner_by_paycheck=HashMap::<String,(String,String)>::new();
    {
        let mut stmt=conn.prepare("SELECT p.id,u.display_name,p.status FROM paycheck_occurrences p JOIN income_sources i ON i.id=p.income_source_id JOIN users u ON u.id=i.user_id")?;
        for row in stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))? { let (id,name,status)=row?; owner_by_paycheck.insert(id,(name,status)); }
    }
    let mut bill_meta=HashMap::<String,(String,String,String,String)>::new();
    {
        let mut stmt=conn.prepare("SELECT id,name_snapshot,due_date,payment_type_snapshot,priority_snapshot FROM bill_occurrences")?;
        for row in stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?)))? { let (id,n,d,p,pr)=row?; bill_meta.insert(id,(n,d,p,pr)); }
    }
    let mut commitment_names = HashMap::<String,String>::new();
    {
        let mut stmt=conn.prepare("SELECT id,name FROM savings_goals WHERE is_active=1")?;
        for row in stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))? { let (id,name)=row?; commitment_names.insert(format!("savings:{id}"),name); }
        let mut stmt=conn.prepare("SELECT id,name FROM debts WHERE is_active=1")?;
        for row in stmt.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))? { let (id,name)=row?; commitment_names.insert(format!("debt:{id}"),name); }
    }
    let commitment_label = |id:&str| -> String {
        let mut parts=id.split(':'); let kind=parts.next().unwrap_or(""); let entity=parts.next().unwrap_or("");
        commitment_names.get(&format!("{kind}:{entity}")).cloned().unwrap_or_else(|| if kind=="debt" {"Extra debt payment".into()} else {"Savings contribution".into()})
    };
    let mut commitments_by_paycheck=HashMap::<String,Vec<PlannerCommitmentDto>>::new();
    for c in &result.commitment_adjustments {
        if let Some(pid)=&c.funding_bucket_id {
            if pid == "__current_cash__" { continue; }
            commitments_by_paycheck.entry(pid.clone()).or_default().push(PlannerCommitmentDto{
                id:c.commitment_id.clone(), name:commitment_label(&c.commitment_id), kind:format!("{:?}",c.kind),
                requested_amount_cents:c.requested_amount.value(), effective_amount_cents:c.effective_amount.value(), reduced_by_cents:c.reduced_by.value()
            });
        }
    }

    let mut allocations_by_paycheck=HashMap::<String,Vec<PlannerBillDto>>::new();
    for a in &result.allocations {
        if let Some(pid)=&a.paycheck_id {
            if let Some((name,due,payment_type,priority))=bill_meta.get(&a.bill_id) {
                allocations_by_paycheck.entry(pid.clone()).or_default().push(PlannerBillDto{occurrence_id:a.bill_id.clone(),name:name.clone(),amount_cents:a.amount.value(),due_date:due.clone(),payment_type:payment_type.clone(),priority:priority.clone(),status:"scheduled".into(),payment_date:date_string(a.payment_date),reason_code:reason_code_string(a.reason_code).into()});
            }
        }
    }
    let mut paychecks=Vec::new();
    for b in &result.bucket_summaries {
        if b.funding_source_type != crate::domain::models::FundingSourceType::Paycheck { continue; }
        let (owner,status)=owner_by_paycheck.get(&b.bucket_id).cloned().unwrap_or(("Household".into(),"projected".into()));
        let mut bills=allocations_by_paycheck.remove(&b.bucket_id).unwrap_or_default();
        bills.sort_by(|a,b|a.due_date.cmp(&b.due_date));
        let commitments=commitments_by_paycheck.remove(&b.bucket_id).unwrap_or_default();
        paychecks.push(PlannerPaycheckDto{id:b.bucket_id.clone(),owner_name:owner,pay_date:date_string(b.date),amount_cents:b.gross_amount.value(),status:format!("{}:{}",status,bucket_status_string(b.status)),bills_total_cents:b.bill_allocations.value(),commitments_total_cents:b.optional_commitments.value(),safe_remaining_cents:b.remaining_headroom.value(),bills,commitments});
    }
    let current_cash_safe=result.bucket_summaries.iter().find(|b|b.funding_source_type==crate::domain::models::FundingSourceType::CurrentCash).map(|b|b.remaining_headroom.value()).unwrap_or_else(||scheduler::safe_to_spend(input.starting_cash,input.protected_buffer).value());
    let warnings=result.warnings.iter().map(|w| {
        let bill_name = w.entity_id.as_ref().and_then(|id| bill_meta.get(id)).map(|meta| meta.0.as_str());
        let message = match w.code {
            scheduler::WarningCode::FundingShortage => format!(
                "{} needs {} more reserved by {}.",
                bill_name.unwrap_or("A bill"),
                format_money_cents(w.amount.map(|m| m.value()).unwrap_or(0)),
                w.date.map(date_string).unwrap_or_else(|| "its deadline".into())
            ),
            scheduler::WarningCode::PartialFunding => format!(
                "{} still needs {} reserved before its payment deadline.",
                bill_name.unwrap_or("A bill"),
                format_money_cents(w.amount.map(|m| m.value()).unwrap_or(0))
            ),
            scheduler::WarningCode::NoEligibleFundingSource => format!(
                "{} has no paycheck or available cash inside its allowed payment window.",
                bill_name.unwrap_or("A bill")
            ),
            scheduler::WarningCode::BillPastDue => format!(
                "{} is past due{}. It is scheduled from the soonest available money.",
                bill_name.unwrap_or("A bill"),
                w.date.map(|d| format!(" (was due {})", date_string(d))).unwrap_or_default()
            ),
            scheduler::WarningCode::OptionalCommitmentReduced => {
                let name=w.entity_id.as_deref().map(&commitment_label).unwrap_or_else(||"Optional savings/debt plan".into());
                format!("{name} was reduced by {} to protect required bills and the cash buffer.",format_money_cents(w.amount.map(|m|m.value()).unwrap_or(0)))
            },
            _ => w.message.clone(),
        };
        PlannerWarningDto{code:format!("{:?}",w.code),message,date:w.date.map(date_string),amount_cents:w.amount.map(|m|m.value())}
    }).collect();
    Ok(PlannerViewDto{planning_date:date_string(input.planning_date),protected_buffer_cents:input.protected_buffer.value(),current_cash_cents:input.starting_cash.value(),current_cash_safe_cents:current_cash_safe,paychecks,warnings,unresolved_bill_count:result.unresolved_bill_ids.len()})
}

#[tauri::command]
pub fn run_scheduler(state: State<'_, AppState>) -> AppResult<PlannerViewDto> { run_scheduler_internal(&state) }

#[tauri::command]
pub fn get_planner(state: State<'_, AppState>) -> AppResult<PlannerViewDto> { run_scheduler_internal(&state) }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveBillPayload { pub id:String }

#[tauri::command]
pub fn archive_bill(payload:ArchiveBillPayload,state:State<'_,AppState>)->AppResult<()> {
    let mut conn=state.db.lock().map_err(|_|AppError::Validation("database lock poisoned".into()))?;
    let tx=conn.transaction()?;
    tx.execute("DELETE FROM bill_allocations WHERE bill_occurrence_id IN (SELECT id FROM bill_occurrences WHERE bill_template_id=?1 AND status IN ('upcoming','scheduled','partial','late'))",[payload.id.as_str()])?;
    tx.execute("DELETE FROM bill_occurrences WHERE bill_template_id=?1 AND status IN ('upcoming','scheduled','late')",[payload.id.as_str()])?;
    tx.execute("UPDATE bill_templates SET is_active=0,archived_at=CURRENT_TIMESTAMP,updated_at=CURRENT_TIMESTAMP WHERE id=?1",[payload.id.as_str()])?;
    tx.commit()?;
    drop(conn);
    let _=run_scheduler_internal(&state)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn test_conn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        db::initialize(&mut conn).unwrap();
        db::complete_onboarding(
            &mut conn,
            "Test Household",
            Money::cents(50_000),
            "Checking",
            Money::cents(150_000),
            &["Jonathan".into(), "Tiffany".into()],
        ).unwrap();
        conn
    }

    #[test]
    fn phase3_monthly_template_generates_real_occurrences() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO bill_templates(id,name,category_id,amount_type,fixed_amount_cents,recurrence_type,payment_type,priority,can_split,is_active,due_day,pay_earliest_days_before) VALUES('electric','Electric','utilities','fixed',18422,'monthly','manual','essential',0,1,18,31)",
            [],
        ).unwrap();
        ensure_template_occurrences(&conn, "electric", today() + Duration::days(95)).unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM bill_occurrences WHERE bill_template_id='electric'", [], |r| r.get(0)).unwrap();
        assert!(count >= 2);
    }

    #[test]
    fn recurring_paycheck_source_generates_future_occurrences() {
        let conn = test_conn();
        let user: String = conn.query_row("SELECT id FROM users ORDER BY created_at LIMIT 1", [], |r| r.get(0)).unwrap();
        conn.execute(
            "INSERT INTO income_sources(id,user_id,name,schedule_type,schedule_config_json,default_projected_amount_cents,weekend_holiday_rule,is_active) VALUES('income-test',?1,'Test Paycheck','biweekly',?2,200000,'exact',1)",
            params![user, json!({"anchorDate": date_string(today())}).to_string()],
        ).unwrap();
        ensure_income_source_occurrences(&conn, "income-test", today() + Duration::days(35)).unwrap();
        let dates = {
            let mut stmt = conn.prepare("SELECT pay_date FROM paycheck_occurrences WHERE income_source_id='income-test' ORDER BY pay_date").unwrap();
            let rows = stmt.query_map([], |r| r.get::<_, String>(0)).unwrap();
            rows.collect::<Result<Vec<_>, _>>().unwrap()
        };
        assert!(dates.len() >= 3);
        let first = parse_date(&dates[0], "date").unwrap();
        let second = parse_date(&dates[1], "date").unwrap();
        assert_eq!((second-first).num_days(), 14);
    }

    #[test]
    fn variable_estimate_uses_recorded_payment_history() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO bill_templates(id,name,category_id,amount_type,fallback_estimate_cents,estimate_window_count,recurrence_type,payment_type,priority,can_split,is_active,due_day,pay_earliest_days_before) VALUES('water','Water','utilities','variable',10000,6,'monthly','manual','normal',0,1,5,31)",
            [],
        ).unwrap();
        let user: String = conn.query_row("SELECT id FROM users ORDER BY created_at LIMIT 1", [], |r| r.get(0)).unwrap();
        let account: String = conn.query_row("SELECT id FROM accounts WHERE is_primary_bill_account=1", [], |r| r.get(0)).unwrap();
        for (index, amount) in [8_000_i64, 10_000, 12_000].into_iter().enumerate() {
            let occurrence = format!("past-{index}");
            let month = index + 1;
            let due = format!("2026-{month:02}-05");
            conn.execute(
                "INSERT INTO bill_occurrences(id,bill_template_id,name_snapshot,due_date,latest_payment_date,earliest_payment_date,estimated_amount_cents,status,payment_type_snapshot,priority_snapshot,is_one_time) VALUES(?1,'water','Water',?2,?2,?3,10000,'paid','manual','normal',0)",
                params![occurrence,due,format!("2025-12-{:02}", 5 + index)],
            ).unwrap();
            conn.execute(
                "INSERT INTO payments(id,bill_occurrence_id,account_id,amount_cents,paid_date,paid_by_user_id) VALUES(?1,?2,?3,?4,?5,?6)",
                params![format!("payment-{index}"),occurrence,account,amount,due,user],
            ).unwrap();
        }
        assert_eq!(estimate_for_template(&conn, "water", 10_000, 6).unwrap(), 10_000);
    }
}
