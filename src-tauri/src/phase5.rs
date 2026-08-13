use std::{collections::BTreeMap, fs};

use chrono::{Duration, Local, NaiveDate};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::{
    backup,
    error::{AppError, AppResult},
    AppState,
};

fn today() -> NaiveDate { Local::now().date_naive() }
fn date_string(d: NaiveDate) -> String { d.format("%Y-%m-%d").to_string() }
fn parse_date(v: &str, field: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(v, "%Y-%m-%d").map_err(|_| AppError::Validation(format!("{field} is invalid")))
}
fn primary_account_id(conn: &Connection) -> AppResult<String> {
    conn.query_row("SELECT id FROM accounts WHERE is_primary_bill_account=1 AND is_active=1 LIMIT 1", [], |r| r.get(0))
        .optional()?.ok_or_else(|| AppError::Validation("primary bill account was not found".into()))
}


// ---------- Explicit bill payment guidance ----------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentFundingSourceDto {
    pub source_type: String,
    pub paycheck_id: Option<String>,
    pub owner_name: Option<String>,
    pub pay_date: Option<String>,
    pub amount_cents: i64,
    pub reason_code: Option<String>,
    pub recommended_payment_date: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentActionDto {
    pub payment_date: String,
    pub amount_cents: i64,
    pub action_status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentGuidanceItemDto {
    pub occurrence_id: String,
    pub bill_name: String,
    pub remaining_amount_cents: i64,
    pub due_date: String,
    pub pay_by_date: String,
    pub recommended_payment_date: String,
    pub payment_type: String,
    pub priority: String,
    pub status: String,
    pub can_split_payment: bool,
    pub funded_amount_cents: i64,
    pub funding_complete: bool,
    pub action_status: String,
    pub funding_sources: Vec<PaymentFundingSourceDto>,
    pub payment_actions: Vec<PaymentActionDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentGuidanceViewDto {
    pub as_of_date: String,
    pub pay_now_count: usize,
    pub needs_attention_count: usize,
    pub items: Vec<PaymentGuidanceItemDto>,
}

fn payment_action_status(payment_date: &str, payment_type: &str, as_of: NaiveDate) -> AppResult<String> {
    let action_date = parse_date(payment_date, "recommended payment date")?;
    Ok((if action_date < as_of {
        "overdue_action"
    } else if action_date == as_of {
        if payment_type == "autopay" { "draft_today" } else { "pay_today" }
    } else if action_date <= as_of + Duration::days(7) {
        "coming_up"
    } else {
        "scheduled"
    }).to_string())
}

fn payment_guidance_from_conn(conn: &Connection) -> AppResult<PaymentGuidanceViewDto> {
    let as_of = today();
    let as_of_text = date_string(as_of);
    let mut occurrence_stmt = conn.prepare(
        "SELECT o.id,o.name_snapshot,o.due_date,o.latest_payment_date,
                COALESCE(o.scheduled_payment_date,o.latest_payment_date),
                MAX(COALESCE(o.actual_required_amount_cents,o.manual_amount_override_cents,o.estimated_amount_cents)
                    - COALESCE((SELECT SUM(p.amount_cents) FROM payments p WHERE p.bill_occurrence_id=o.id),0),0),
                o.payment_type_snapshot,o.priority_snapshot,o.status,COALESCE(t.can_split,0)
         FROM bill_occurrences o
         LEFT JOIN bill_templates t ON t.id=o.bill_template_id
         WHERE o.status IN ('upcoming','scheduled','partial','late')
           AND o.due_date >= date(?1,'-31 day')
         GROUP BY o.id
         ORDER BY COALESCE(o.scheduled_payment_date,o.latest_payment_date),o.due_date,o.name_snapshot"
    )?;
    let occurrences = occurrence_stmt.query_map([as_of_text.as_str()], |r| {
        Ok((
            r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?,
            r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, i64>(5)?,
            r.get::<_, String>(6)?, r.get::<_, String>(7)?, r.get::<_, String>(8)?,
            r.get::<_, i64>(9)? == 1,
        ))
    })?.collect::<Result<Vec<_>,_>>()?;

    let mut items = Vec::new();
    for (occurrence_id,bill_name,due_date,pay_by_date,scheduled_date,remaining_amount,payment_type,priority,status,can_split) in occurrences {
        if remaining_amount <= 0 { continue; }
        let default_payment_date = if payment_type == "autopay" { due_date.clone() } else { scheduled_date };
        let mut funding_stmt = conn.prepare(
            "SELECT a.funding_source_type,a.paycheck_occurrence_id,u.display_name,p.pay_date,a.allocated_amount_cents,a.reason_code,a.recommended_payment_date
             FROM bill_allocations a
             LEFT JOIN paycheck_occurrences p ON p.id=a.paycheck_occurrence_id
             LEFT JOIN income_sources i ON i.id=p.income_source_id
             LEFT JOIN users u ON u.id=i.user_id
             WHERE a.bill_occurrence_id=?1
             ORDER BY CASE WHEN a.funding_source_type='current_cash' THEN 0 ELSE 1 END,p.pay_date,a.created_at"
        )?;
        let funding_sources = funding_stmt.query_map([occurrence_id.as_str()], |r| {
            Ok(PaymentFundingSourceDto {
                source_type:r.get(0)?, paycheck_id:r.get(1)?, owner_name:r.get(2)?, pay_date:r.get(3)?, amount_cents:r.get(4)?, reason_code:r.get(5)?, recommended_payment_date:r.get(6)?
            })
        })?.collect::<Result<Vec<_>,_>>()?;
        let funded_amount: i64 = funding_sources.iter().map(|x| x.amount_cents).sum();
        let funding_complete = funded_amount >= remaining_amount;

        let mut action_amounts = BTreeMap::<String, i64>::new();
        if payment_type == "autopay" || !can_split {
            action_amounts.insert(default_payment_date.clone(), remaining_amount);
        } else if !funding_sources.is_empty() {
            for source in &funding_sources {
                let action_date = source.recommended_payment_date.clone()
                    .or_else(|| source.pay_date.clone())
                    .unwrap_or_else(|| default_payment_date.clone());
                *action_amounts.entry(action_date).or_default() += source.amount_cents;
            }
        } else {
            action_amounts.insert(default_payment_date.clone(), remaining_amount);
        }

        let mut payment_actions = Vec::new();
        for (payment_date, amount_cents) in action_amounts {
            payment_actions.push(PaymentActionDto {
                action_status: payment_action_status(&payment_date, &payment_type, as_of)?,
                payment_date,
                amount_cents,
            });
        }
        payment_actions.sort_by(|a,b| a.payment_date.cmp(&b.payment_date));
        let recommended_payment_date = payment_actions.first().map(|a|a.payment_date.clone()).unwrap_or(default_payment_date);
        let action_status = if !funding_complete {
            "needs_funding".to_string()
        } else {
            payment_actions.first().map(|a|a.action_status.clone()).unwrap_or_else(||"scheduled".into())
        };
        items.push(PaymentGuidanceItemDto {
            occurrence_id,bill_name,remaining_amount_cents:remaining_amount,due_date,pay_by_date,recommended_payment_date,
            payment_type,priority,status,can_split_payment:can_split,funded_amount_cents:funded_amount,funding_complete,action_status,funding_sources,payment_actions,
        });
    }
    let pay_now_count = items.iter().flat_map(|x|x.payment_actions.iter()).filter(|a| matches!(a.action_status.as_str(),"pay_today"|"draft_today"|"overdue_action")).count();
    let needs_attention_count = items.iter().filter(|x| !x.funding_complete || x.payment_actions.iter().any(|a|a.action_status=="overdue_action")).count();
    Ok(PaymentGuidanceViewDto { as_of_date:as_of_text,pay_now_count,needs_attention_count,items })
}

#[tauri::command]
pub fn get_payment_guidance(state: State<'_, AppState>) -> AppResult<PaymentGuidanceViewDto> {
    // Always refresh the deterministic allocations first. Guidance must never be
    // based on stale paycheck assignments.
    let _ = crate::phase3::run_scheduler_internal(&state)?;
    let conn = state.db.lock().map_err(|_|AppError::Validation("database lock poisoned".into()))?;
    payment_guidance_from_conn(&conn)
}

// ---------- Savings & Debt ----------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavingsGoalDto {
    pub id: String,
    pub name: String,
    pub goal_type: String,
    pub target_amount_cents: i64,
    pub target_date: Option<String>,
    pub current_amount_cents: i64,
    pub planned_contribution_cents: i64,
    pub contribution_frequency: String,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebtDto {
    pub id: String,
    pub name: String,
    pub balance_cents: i64,
    pub apr_basis_points: i64,
    pub minimum_payment_cents: i64,
    pub planned_extra_payment_cents: i64,
    pub due_day: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalActivityDto {
    pub id: String,
    pub date: String,
    pub item_name: String,
    pub activity_type: String,
    pub amount_cents: i64,
    pub person_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebtStrategyDto {
    pub strategy: String,
    pub payoff_months: i64,
    pub total_interest_cents: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavingsDebtViewDto {
    pub goals: Vec<SavingsGoalDto>,
    pub debts: Vec<DebtDto>,
    pub recent_activity: Vec<GoalActivityDto>,
    pub total_saved_cents: i64,
    pub total_debt_cents: i64,
    pub planned_savings_per_paycheck_cents: i64,
    pub planned_extra_debt_monthly_cents: i64,
    pub strategies: Vec<DebtStrategyDto>,
}

#[derive(Debug, Clone)]
struct SimDebt { balance: i64, apr_bp: i64, minimum: i64 }

fn simulate_debt_strategy(debts: &[DebtDto], avalanche: bool) -> DebtStrategyDto {
    let mut items: Vec<SimDebt> = debts.iter().filter(|d| d.balance_cents > 0).map(|d| SimDebt {
        balance: d.balance_cents,
        apr_bp: d.apr_basis_points.max(0),
        minimum: d.minimum_payment_cents.max(0),
    }).collect();
    let base_budget: i64 = items.iter().map(|d| d.minimum).sum::<i64>()
        + debts.iter().map(|d| d.planned_extra_payment_cents.max(0)).sum::<i64>();
    if items.is_empty() { return DebtStrategyDto { strategy: if avalanche {"avalanche"} else {"snowball"}.into(), payoff_months: 0, total_interest_cents: 0 }; }
    if base_budget <= 0 { return DebtStrategyDto { strategy: if avalanche {"avalanche"} else {"snowball"}.into(), payoff_months: -1, total_interest_cents: 0 }; }
    let mut interest_total = 0_i64;
    for month in 1..=600_i64 {
        for d in &mut items {
            if d.balance <= 0 { continue; }
            let interest = (d.balance.saturating_mul(d.apr_bp) + 60_000) / 120_000;
            d.balance += interest.max(0);
            interest_total += interest.max(0);
        }
        let mut budget = base_budget;
        for d in &mut items {
            if d.balance <= 0 { continue; }
            let pay = d.minimum.min(d.balance).min(budget);
            d.balance -= pay;
            budget -= pay;
        }
        while budget > 0 && items.iter().any(|d| d.balance > 0) {
            let target = items.iter().enumerate().filter(|(_,d)| d.balance > 0).min_by(|(_,a),(_,b)| {
                if avalanche {
                    b.apr_bp.cmp(&a.apr_bp).then(a.balance.cmp(&b.balance))
                } else {
                    a.balance.cmp(&b.balance).then(b.apr_bp.cmp(&a.apr_bp))
                }
            }).map(|(i,_)| i).unwrap();
            let pay = budget.min(items[target].balance);
            items[target].balance -= pay;
            budget -= pay;
        }
        if items.iter().all(|d| d.balance <= 0) {
            return DebtStrategyDto { strategy: if avalanche {"avalanche"} else {"snowball"}.into(), payoff_months: month, total_interest_cents: interest_total };
        }
    }
    DebtStrategyDto { strategy: if avalanche {"avalanche"} else {"snowball"}.into(), payoff_months: -1, total_interest_cents: interest_total }
}

fn savings_debt_view_from_conn(conn: &Connection) -> AppResult<SavingsDebtViewDto> {
    let mut goal_stmt = conn.prepare("SELECT id,name,goal_type,target_amount_cents,target_date,current_amount_cents,planned_contribution_cents,COALESCE(contribution_frequency,'per_paycheck'),notes FROM savings_goals WHERE is_active=1 ORDER BY created_at")?;
    let goals = goal_stmt.query_map([], |r| Ok(SavingsGoalDto { id:r.get(0)?,name:r.get(1)?,goal_type:r.get(2)?,target_amount_cents:r.get(3)?,target_date:r.get(4)?,current_amount_cents:r.get(5)?,planned_contribution_cents:r.get(6)?,contribution_frequency:r.get(7)?,notes:r.get(8)? }))?.collect::<Result<Vec<_>,_>>()?;
    let mut debt_stmt = conn.prepare("SELECT id,name,balance_cents,apr_basis_points,minimum_payment_cents,planned_payment_cents,due_day FROM debts WHERE is_active=1 ORDER BY created_at")?;
    let debts = debt_stmt.query_map([], |r| Ok(DebtDto { id:r.get(0)?,name:r.get(1)?,balance_cents:r.get(2)?,apr_basis_points:r.get(3)?,minimum_payment_cents:r.get(4)?,planned_extra_payment_cents:r.get(5)?,due_day:r.get(6)? }))?.collect::<Result<Vec<_>,_>>()?;

    let mut activity = Vec::new();
    {
        let mut stmt = conn.prepare("SELECT c.id,c.contribution_date,g.name,c.amount_cents,u.display_name FROM savings_contributions c JOIN savings_goals g ON g.id=c.goal_id LEFT JOIN users u ON u.id=c.contributed_by_user_id ORDER BY c.contribution_date DESC,c.created_at DESC LIMIT 12")?;
        for row in stmt.query_map([], |r| Ok(GoalActivityDto{id:r.get(0)?,date:r.get(1)?,item_name:r.get(2)?,activity_type:"savings".into(),amount_cents:r.get(3)?,person_name:r.get(4)?}))? { activity.push(row?); }
    }
    {
        let mut stmt = conn.prepare("SELECT p.id,p.payment_date,d.name,p.amount_cents,u.display_name FROM debt_payments p JOIN debts d ON d.id=p.debt_id LEFT JOIN users u ON u.id=p.paid_by_user_id ORDER BY p.payment_date DESC,p.created_at DESC LIMIT 12")?;
        for row in stmt.query_map([], |r| Ok(GoalActivityDto{id:r.get(0)?,date:r.get(1)?,item_name:r.get(2)?,activity_type:"debt".into(),amount_cents:r.get(3)?,person_name:r.get(4)?}))? { activity.push(row?); }
    }
    activity.sort_by(|a,b| b.date.cmp(&a.date)); activity.truncate(12);
    let total_saved = goals.iter().map(|g|g.current_amount_cents).sum();
    let total_debt = debts.iter().map(|d|d.balance_cents).sum();
    let planned_savings = goals.iter().filter(|g|g.contribution_frequency=="per_paycheck").map(|g|g.planned_contribution_cents).sum();
    let planned_extra = debts.iter().map(|d|d.planned_extra_payment_cents).sum();
    let strategies = vec![simulate_debt_strategy(&debts,false), simulate_debt_strategy(&debts,true)];
    Ok(SavingsDebtViewDto { goals, debts, recent_activity:activity, total_saved_cents:total_saved,total_debt_cents:total_debt,planned_savings_per_paycheck_cents:planned_savings,planned_extra_debt_monthly_cents:planned_extra,strategies })
}

#[tauri::command]
pub fn get_savings_debt_view(state: State<'_, AppState>) -> AppResult<SavingsDebtViewDto> {
    let conn=state.db.lock().map_err(|_|AppError::Validation("database lock poisoned".into()))?;
    savings_debt_view_from_conn(&conn)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct SaveGoalPayload { pub id: Option<String>, pub name:String, pub goal_type:String, pub target_amount_cents:i64, pub target_date:Option<String>, pub current_amount_cents:i64, pub planned_contribution_cents:i64, pub contribution_frequency:String, pub notes:Option<String> }

#[tauri::command]
pub fn save_savings_goal(payload: SaveGoalPayload, state: State<'_, AppState>) -> AppResult<String> {
    if payload.name.trim().is_empty(){return Err(AppError::Validation("goal name is required".into()));}
    if !matches!(payload.goal_type.as_str(),"savings"|"emergency"|"sinking_fund"){return Err(AppError::Validation("invalid savings goal type".into()));}
    if !matches!(payload.contribution_frequency.as_str(),"per_paycheck"|"monthly"|"manual"){return Err(AppError::Validation("invalid contribution frequency".into()));}
    if let Some(d)=payload.target_date.as_deref(){parse_date(d,"target date")?;}
    if [payload.target_amount_cents,payload.current_amount_cents,payload.planned_contribution_cents].iter().any(|x|*x<0){return Err(AppError::Validation("goal amounts cannot be negative".into()));}
    let mut conn=state.db.lock().map_err(|_|AppError::Validation("database lock poisoned".into()))?; let tx=conn.transaction()?;
    let id=payload.id.unwrap_or_else(||Uuid::new_v4().to_string());
    let exists: i64=tx.query_row("SELECT EXISTS(SELECT 1 FROM savings_goals WHERE id=?1)",[id.as_str()],|r|r.get(0))?;
    if exists==1 { tx.execute("UPDATE savings_goals SET name=?2,goal_type=?3,target_amount_cents=?4,target_date=?5,current_amount_cents=?6,planned_contribution_cents=?7,contribution_frequency=?8,notes=?9,updated_at=CURRENT_TIMESTAMP WHERE id=?1",params![id,payload.name.trim(),payload.goal_type,payload.target_amount_cents,payload.target_date,payload.current_amount_cents,payload.planned_contribution_cents,payload.contribution_frequency,payload.notes])?; }
    else { tx.execute("INSERT INTO savings_goals(id,name,goal_type,target_amount_cents,target_date,current_amount_cents,planned_contribution_cents,contribution_frequency,is_required_contribution,is_active,notes) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,0,1,?9)",params![id,payload.name.trim(),payload.goal_type,payload.target_amount_cents,payload.target_date,payload.current_amount_cents,payload.planned_contribution_cents,payload.contribution_frequency,payload.notes])?; }
    tx.execute("INSERT INTO activity_log(id,event_type,entity_type,entity_id,summary) VALUES(?1,'savings_goal_saved','savings_goal',?2,?3)",params![Uuid::new_v4().to_string(),id,format!("Saved savings goal {}",payload.name.trim())])?;
    tx.commit()?; drop(conn); let _=crate::phase3::run_scheduler_internal(&state)?; Ok(id)
}

#[derive(Debug, Deserialize)] #[serde(rename_all="camelCase")]
pub struct SaveDebtPayload { pub id:Option<String>, pub name:String, pub balance_cents:i64, pub apr_basis_points:i64, pub minimum_payment_cents:i64, pub planned_extra_payment_cents:i64, pub due_day:Option<i64> }

#[tauri::command]
pub fn save_debt(payload:SaveDebtPayload,state:State<'_,AppState>)->AppResult<String>{
    if payload.name.trim().is_empty(){return Err(AppError::Validation("debt name is required".into()));}
    if [payload.balance_cents,payload.apr_basis_points,payload.minimum_payment_cents,payload.planned_extra_payment_cents].iter().any(|x|*x<0){return Err(AppError::Validation("debt values cannot be negative".into()));}
    if let Some(d)=payload.due_day{if !(1..=31).contains(&d){return Err(AppError::Validation("debt due day must be 1 through 31".into()));}}
    let mut conn=state.db.lock().map_err(|_|AppError::Validation("database lock poisoned".into()))?;let tx=conn.transaction()?;let id=payload.id.unwrap_or_else(||Uuid::new_v4().to_string());
    let exists:i64=tx.query_row("SELECT EXISTS(SELECT 1 FROM debts WHERE id=?1)",[id.as_str()],|r|r.get(0))?;
    if exists==1{tx.execute("UPDATE debts SET name=?2,balance_cents=?3,apr_basis_points=?4,minimum_payment_cents=?5,planned_payment_cents=?6,due_day=?7,updated_at=CURRENT_TIMESTAMP WHERE id=?1",params![id,payload.name.trim(),payload.balance_cents,payload.apr_basis_points,payload.minimum_payment_cents,payload.planned_extra_payment_cents,payload.due_day])?;}
    else{tx.execute("INSERT INTO debts(id,name,balance_cents,apr_basis_points,minimum_payment_cents,planned_payment_cents,due_day,is_active) VALUES(?1,?2,?3,?4,?5,?6,?7,1)",params![id,payload.name.trim(),payload.balance_cents,payload.apr_basis_points,payload.minimum_payment_cents,payload.planned_extra_payment_cents,payload.due_day])?;}
    tx.execute("INSERT INTO activity_log(id,event_type,entity_type,entity_id,summary) VALUES(?1,'debt_saved','debt',?2,?3)",params![Uuid::new_v4().to_string(),id,format!("Saved debt {}",payload.name.trim())])?;tx.commit()?;drop(conn);let _=crate::phase3::run_scheduler_internal(&state)?;Ok(id)
}

#[derive(Debug, Deserialize)] #[serde(rename_all="camelCase")]
pub struct MoneyActionPayload { pub id:String, pub amount_cents:i64, pub date:String, pub user_id:Option<String>, pub note:Option<String> }

#[tauri::command]
pub fn record_savings_contribution(payload:MoneyActionPayload,state:State<'_,AppState>)->AppResult<()> {
    parse_date(&payload.date,"contribution date")?; if payload.amount_cents<=0{return Err(AppError::Validation("contribution must be greater than zero".into()));}
    let mut conn=state.db.lock().map_err(|_|AppError::Validation("database lock poisoned".into()))?;let tx=conn.transaction()?;let account=primary_account_id(&tx)?;
    let goal:String=tx.query_row("SELECT name FROM savings_goals WHERE id=?1 AND is_active=1",[payload.id.as_str()],|r|r.get(0)).optional()?.ok_or_else(||AppError::Validation("savings goal was not found".into()))?;
    let cid=Uuid::new_v4().to_string();tx.execute("INSERT INTO savings_contributions(id,goal_id,account_id,amount_cents,contribution_date,contributed_by_user_id,note) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![cid,payload.id,account,payload.amount_cents,payload.date,payload.user_id,payload.note])?;
    tx.execute("UPDATE savings_goals SET current_amount_cents=current_amount_cents+?2,updated_at=CURRENT_TIMESTAMP WHERE id=?1",params![payload.id,payload.amount_cents])?;
    tx.execute("UPDATE accounts SET book_balance_cents=book_balance_cents-?2,updated_at=CURRENT_TIMESTAMP WHERE id=?1",params![account,payload.amount_cents])?;
    tx.execute("INSERT INTO transactions(id,account_id,transaction_date,description,category_id,amount_cents,transaction_type,status,source,created_by_user_id,note,source_entity_type,source_entity_id) VALUES(?1,?2,?3,?4,'savings',?5,'savings_contribution','cleared','manual',?6,?7,'savings_goal',?8)",params![Uuid::new_v4().to_string(),account,payload.date,format!("Savings contribution: {goal}"),-payload.amount_cents,payload.user_id,payload.note,payload.id])?;
    tx.execute("INSERT INTO activity_log(id,user_id,event_type,entity_type,entity_id,summary) VALUES(?1,?2,'savings_contribution','savings_goal',?3,?4)",params![Uuid::new_v4().to_string(),payload.user_id,payload.id,format!("Added savings contribution to {goal}")])?;tx.commit()?;drop(conn);let _=crate::phase3::run_scheduler_internal(&state)?;Ok(())
}

#[tauri::command]
pub fn record_debt_payment(payload:MoneyActionPayload,state:State<'_,AppState>)->AppResult<()> {
    parse_date(&payload.date,"payment date")?; if payload.amount_cents<=0{return Err(AppError::Validation("payment must be greater than zero".into()));}
    let mut conn=state.db.lock().map_err(|_|AppError::Validation("database lock poisoned".into()))?;let tx=conn.transaction()?;let account=primary_account_id(&tx)?;
    let (name,balance):(String,i64)=tx.query_row("SELECT name,balance_cents FROM debts WHERE id=?1 AND is_active=1",[payload.id.as_str()],|r|Ok((r.get(0)?,r.get(1)?))).optional()?.ok_or_else(||AppError::Validation("debt was not found".into()))?;
    let actual=payload.amount_cents.min(balance.max(0)); if actual<=0{return Err(AppError::Validation("this debt is already paid off".into()));}
    let pid=Uuid::new_v4().to_string();tx.execute("INSERT INTO debt_payments(id,debt_id,account_id,amount_cents,payment_date,paid_by_user_id,is_extra,note) VALUES(?1,?2,?3,?4,?5,?6,1,?7)",params![pid,payload.id,account,actual,payload.date,payload.user_id,payload.note])?;
    tx.execute("UPDATE debts SET balance_cents=MAX(balance_cents-?2,0),updated_at=CURRENT_TIMESTAMP WHERE id=?1",params![payload.id,actual])?;
    tx.execute("UPDATE accounts SET book_balance_cents=book_balance_cents-?2,updated_at=CURRENT_TIMESTAMP WHERE id=?1",params![account,actual])?;
    tx.execute("INSERT INTO transactions(id,account_id,transaction_date,description,category_id,amount_cents,transaction_type,status,source,created_by_user_id,note,source_entity_type,source_entity_id) VALUES(?1,?2,?3,?4,'debt',?5,'debt_payment','cleared','manual',?6,?7,'debt',?8)",params![Uuid::new_v4().to_string(),account,payload.date,format!("Extra debt payment: {name}"),-actual,payload.user_id,payload.note,payload.id])?;
    tx.execute("INSERT INTO activity_log(id,user_id,event_type,entity_type,entity_id,summary) VALUES(?1,?2,'debt_payment','debt',?3,?4)",params![Uuid::new_v4().to_string(),payload.user_id,payload.id,format!("Recorded extra payment to {name}")])?;tx.commit()?;drop(conn);let _=crate::phase3::run_scheduler_internal(&state)?;Ok(())
}

#[derive(Debug, Deserialize)] #[serde(rename_all="camelCase")]
pub struct ArchiveItemPayload { pub id:String }
#[tauri::command]
pub fn archive_savings_goal(payload:ArchiveItemPayload,state:State<'_,AppState>)->AppResult<()> {
    {
        let conn=state.db.lock().map_err(|_|AppError::Validation("database lock poisoned".into()))?;
        conn.execute("UPDATE savings_goals SET is_active=0,archived_at=CURRENT_TIMESTAMP,updated_at=CURRENT_TIMESTAMP WHERE id=?1",[payload.id])?;
    }
    let _=crate::phase3::run_scheduler_internal(&state)?;
    Ok(())
}
#[tauri::command]
pub fn archive_debt(payload:ArchiveItemPayload,state:State<'_,AppState>)->AppResult<()> {
    {
        let conn=state.db.lock().map_err(|_|AppError::Validation("database lock poisoned".into()))?;
        conn.execute("UPDATE debts SET is_active=0,archived_at=CURRENT_TIMESTAMP,updated_at=CURRENT_TIMESTAMP WHERE id=?1",[payload.id])?;
    }
    let _=crate::phase3::run_scheduler_internal(&state)?;
    Ok(())
}

// ---------- Settings ----------

#[derive(Debug, Serialize)] #[serde(rename_all="camelCase")]
pub struct SettingsViewDto { pub app_version:String,pub household_name:String,pub protected_buffer_cents:i64,pub planning_horizon_days:i64,pub primary_account_id:String,pub primary_account_name:String,pub backup_retention_count:i64,pub users:Vec<crate::commands::UserProfileDto>,pub database_path:String,pub backup_directory:String,pub export_directory:String }
#[tauri::command]
pub fn get_settings_view(state:State<'_,AppState>)->AppResult<SettingsViewDto>{
    let conn=state.db.lock().map_err(|_|AppError::Validation("database lock poisoned".into()))?;
    let (household,buffer,horizon,account_id,account_name,retention):(String,i64,i64,String,String,i64)=conn.query_row("SELECT h.household_name,h.protected_buffer_cents,h.default_planning_horizon_days,a.id,a.name,h.backup_retention_count FROM household_settings h JOIN accounts a ON a.id=h.primary_account_id WHERE h.id=1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)))?;
    let mut stmt=conn.prepare("SELECT id,display_name FROM users WHERE is_active=1 ORDER BY created_at")?;let users=stmt.query_map([],|r|Ok(crate::commands::UserProfileDto{id:r.get(0)?,display_name:r.get(1)?}))?.collect::<Result<Vec<_>,_>>()?;
    Ok(SettingsViewDto{app_version:env!("CARGO_PKG_VERSION").into(),household_name:household,protected_buffer_cents:buffer,planning_horizon_days:horizon,primary_account_id:account_id,primary_account_name:account_name,backup_retention_count:retention,users,database_path:state.database_path.display().to_string(),backup_directory:state.backup_dir.display().to_string(),export_directory:state.export_dir.display().to_string()})
}

#[derive(Debug, Deserialize)] #[serde(rename_all="camelCase")]
pub struct UserNamePayload { pub id:String,pub display_name:String }
#[derive(Debug, Deserialize)] #[serde(rename_all="camelCase")]
pub struct SaveSettingsPayload { pub household_name:String,pub protected_buffer_cents:i64,pub planning_horizon_days:i64,pub primary_account_name:String,pub backup_retention_count:i64,pub users:Vec<UserNamePayload> }
#[tauri::command]
pub fn save_settings(payload:SaveSettingsPayload,state:State<'_,AppState>)->AppResult<()> {
    if payload.household_name.trim().is_empty()||payload.primary_account_name.trim().is_empty(){return Err(AppError::Validation("household and account names are required".into()));}
    if payload.protected_buffer_cents<0{return Err(AppError::Validation("protected buffer cannot be negative".into()));}
    if !matches!(payload.planning_horizon_days,30|60|90|180|365){return Err(AppError::Validation("planning horizon must be 30, 60, 90, 180, or 365 days".into()));}
    if !(3..=90).contains(&payload.backup_retention_count){return Err(AppError::Validation("backup retention must be between 3 and 90 backups".into()));}
    let mut conn=state.db.lock().map_err(|_|AppError::Validation("database lock poisoned".into()))?;let tx=conn.transaction()?;
    let aid=primary_account_id(&tx)?;tx.execute("UPDATE household_settings SET household_name=?1,protected_buffer_cents=?2,default_planning_horizon_days=?3,backup_retention_count=?4,updated_at=CURRENT_TIMESTAMP WHERE id=1",params![payload.household_name.trim(),payload.protected_buffer_cents,payload.planning_horizon_days,payload.backup_retention_count])?;tx.execute("UPDATE accounts SET name=?2,updated_at=CURRENT_TIMESTAMP WHERE id=?1",params![aid,payload.primary_account_name.trim()])?;
    for u in payload.users { if u.display_name.trim().is_empty(){return Err(AppError::Validation("household member names cannot be blank".into()));} tx.execute("UPDATE users SET display_name=?2,updated_at=CURRENT_TIMESTAMP WHERE id=?1",params![u.id,u.display_name.trim()])?; }
    tx.execute("INSERT INTO activity_log(id,event_type,entity_type,summary) VALUES(?1,'settings_saved','settings','Updated household settings')",[Uuid::new_v4().to_string()])?;tx.commit()?;drop(conn);backup::prune_backups(&state.backup_dir,payload.backup_retention_count as usize)?;let _=crate::phase3::run_scheduler_internal(&state)?;Ok(())
}

#[tauri::command]
pub fn open_app_folder(target:String,state:State<'_,AppState>)->AppResult<()> {
    let path = match target.as_str() {
        "data" => state.database_path.parent().ok_or_else(||AppError::Validation("application data folder was not found".into()))?.to_path_buf(),
        "backups" => state.backup_dir.clone(),
        "exports" => state.export_dir.clone(),
        _ => return Err(AppError::Validation("unknown application folder".into())),
    };
    fs::create_dir_all(&path)?;
    std::process::Command::new("explorer.exe").arg(&path).spawn()?;
    Ok(())
}

// ---------- Backup & Restore ----------

#[derive(Debug, Serialize)] #[serde(rename_all="camelCase")]
pub struct BackupDto { pub file_name:String,pub created_at:String,pub size_bytes:u64 }
#[tauri::command]
pub fn list_backups(state:State<'_,AppState>)->AppResult<Vec<BackupDto>> { Ok(backup::list_backups(&state.backup_dir)?.into_iter().map(|b|BackupDto{file_name:b.file_name,created_at:b.created_at,size_bytes:b.size_bytes}).collect()) }
#[tauri::command]
pub fn request_restore_backup(file_name:String,state:State<'_,AppState>)->AppResult<String>{
    // Make one last safety copy of the current database before scheduling a restore.
    // The selected historical backup is never modified.
    {
        let conn=state.db.lock().map_err(|_|AppError::Validation("database lock poisoned".into()))?;
        let _=backup::create_backup(&conn,&state.database_path,&state.backup_dir)?;
    }
    backup::schedule_restore(&state.backup_dir,&state.restore_marker,&file_name)?;
    Ok("Restore is ready. A safety backup of the current database was created. Close and reopen Household Bills to complete the restore.".into())
}

// ---------- Reports & CSV ----------

#[derive(Debug, Serialize)] #[serde(rename_all="camelCase")]
pub struct ReportCategoryDto { pub name:String,pub amount_cents:i64 }
#[derive(Debug, Serialize)] #[serde(rename_all="camelCase")]
pub struct ReportMonthDto { pub month:String,pub income_cents:i64,pub spending_cents:i64,pub net_cents:i64 }
#[derive(Debug, Serialize)] #[serde(rename_all="camelCase")]
pub struct ReportsViewDto { pub start_date:String,pub end_date:String,pub income_cents:i64,pub bill_payments_cents:i64,pub everyday_spending_cents:i64,pub savings_cents:i64,pub debt_payments_cents:i64,pub net_cents:i64,pub categories:Vec<ReportCategoryDto>,pub months:Vec<ReportMonthDto> }

fn reports_from_conn(conn:&Connection,start:&str,end:&str)->AppResult<ReportsViewDto>{
    parse_date(start,"report start date")?;parse_date(end,"report end date")?;if start>end{return Err(AppError::Validation("report start date must be before end date".into()));}
    let total=|where_sql:&str|->AppResult<i64>{let sql=format!("SELECT COALESCE(SUM(amount_cents),0) FROM transactions WHERE transaction_date BETWEEN ?1 AND ?2 AND {where_sql}");Ok(conn.query_row(&sql,params![start,end],|r|r.get(0))?)};
    let income=total("amount_cents>0")?;let bill=-total("amount_cents<0 AND transaction_type='bill_payment'")?;let savings=-total("amount_cents<0 AND transaction_type='savings_contribution'")?;let debt=-total("amount_cents<0 AND transaction_type='debt_payment'")?;let everyday=-total("amount_cents<0 AND transaction_type NOT IN ('bill_payment','savings_contribution','debt_payment')")?;let net=total("1=1")?;
    let mut cstmt=conn.prepare("SELECT COALESCE(c.name,'Other'),-SUM(t.amount_cents) FROM transactions t LEFT JOIN categories c ON c.id=t.category_id WHERE t.transaction_date BETWEEN ?1 AND ?2 AND t.amount_cents<0 GROUP BY COALESCE(c.name,'Other') ORDER BY -SUM(t.amount_cents) DESC")?;let categories=cstmt.query_map(params![start,end],|r|Ok(ReportCategoryDto{name:r.get(0)?,amount_cents:r.get(1)?}))?.collect::<Result<Vec<_>,_>>()?;
    let mut mstmt=conn.prepare("SELECT substr(transaction_date,1,7),COALESCE(SUM(CASE WHEN amount_cents>0 THEN amount_cents ELSE 0 END),0),COALESCE(-SUM(CASE WHEN amount_cents<0 THEN amount_cents ELSE 0 END),0),COALESCE(SUM(amount_cents),0) FROM transactions WHERE transaction_date BETWEEN ?1 AND ?2 GROUP BY substr(transaction_date,1,7) ORDER BY substr(transaction_date,1,7)")?;let months=mstmt.query_map(params![start,end],|r|Ok(ReportMonthDto{month:r.get(0)?,income_cents:r.get(1)?,spending_cents:r.get(2)?,net_cents:r.get(3)?}))?.collect::<Result<Vec<_>,_>>()?;
    Ok(ReportsViewDto{start_date:start.into(),end_date:end.into(),income_cents:income,bill_payments_cents:bill,everyday_spending_cents:everyday,savings_cents:savings,debt_payments_cents:debt,net_cents:net,categories,months})
}
#[tauri::command] pub fn get_reports_data(start_date:String,end_date:String,state:State<'_,AppState>)->AppResult<ReportsViewDto>{let conn=state.db.lock().map_err(|_|AppError::Validation("database lock poisoned".into()))?;reports_from_conn(&conn,&start_date,&end_date)}
fn csv_cell(v:&str)->String{if v.chars().any(|c| matches!(c,','|'"'|'\n')){format!("\"{}\"",v.replace('"',"\"\""))}else{v.into()}}
#[tauri::command]
pub fn export_report_csv(start_date:String,end_date:String,state:State<'_,AppState>)->AppResult<String>{
    let conn=state.db.lock().map_err(|_|AppError::Validation("database lock poisoned".into()))?;let report=reports_from_conn(&conn,&start_date,&end_date)?;fs::create_dir_all(&state.export_dir)?;let path=state.export_dir.join(format!("HouseholdBills_Report_{}_to_{}.csv",start_date,end_date));
    let mut out=String::from("Section,Name,Amount\n");for (section,name,value) in [("Summary","Income",report.income_cents),("Summary","Bill Payments",report.bill_payments_cents),("Summary","Everyday Spending",report.everyday_spending_cents),("Summary","Savings",report.savings_cents),("Summary","Debt Payments",report.debt_payments_cents),("Summary","Net Cash Flow",report.net_cents)]{out.push_str(&format!("{},{},{:.2}\n",section,csv_cell(name),value as f64/100.0));}
    out.push_str("\nCategory,Amount\n");for c in report.categories{out.push_str(&format!("{},{}\n",csv_cell(&c.name),format!("{:.2}",c.amount_cents as f64/100.0)));}
    out.push_str("\nMonth,Income,Spending,Net\n");for m in report.months{out.push_str(&format!("{},{:.2},{:.2},{:.2}\n",m.month,m.income_cents as f64/100.0,m.spending_cents as f64/100.0,m.net_cents as f64/100.0));}
    fs::write(&path,out)?;Ok(path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, domain::money::Money};

    #[test]
    fn strategy_comparison_prefers_avalanche_interest_in_common_case(){
        let debts=vec![
            DebtDto{id:"a".into(),name:"Card".into(),balance_cents:500_000,apr_basis_points:2400,minimum_payment_cents:10_000,planned_extra_payment_cents:15_000,due_day:Some(10)},
            DebtDto{id:"b".into(),name:"Loan".into(),balance_cents:800_000,apr_basis_points:600,minimum_payment_cents:15_000,planned_extra_payment_cents:0,due_day:Some(20)}
        ];
        let snow=simulate_debt_strategy(&debts,false);
        let ava=simulate_debt_strategy(&debts,true);
        assert!(ava.total_interest_cents<=snow.total_interest_cents);
    }

    #[test]
    fn payment_guidance_aggregates_reserves_but_keeps_one_payment_action() {
        let mut conn = Connection::open_in_memory().unwrap();
        db::initialize(&mut conn).unwrap();
        db::complete_onboarding(&mut conn,"Test Household",Money::cents(50_000),"Checking",Money::cents(100_000),&["Jonathan".into(),"Tiffany".into()]).unwrap();
        let users: Vec<String> = {
            let mut stmt=conn.prepare("SELECT id FROM users ORDER BY created_at").unwrap();
            stmt.query_map([],|r|r.get(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap()
        };
        let source_a=Uuid::new_v4().to_string(); let source_b=Uuid::new_v4().to_string();
        conn.execute("INSERT INTO income_sources(id,user_id,name,schedule_type,default_projected_amount_cents) VALUES(?1,?2,'Pay','weekly',67000)",params![source_a,users[0]]).unwrap();
        conn.execute("INSERT INTO income_sources(id,user_id,name,schedule_type,default_projected_amount_cents) VALUES(?1,?2,'Pay','biweekly',153000)",params![source_b,users[1]]).unwrap();
        let first=Uuid::new_v4().to_string(); let second=Uuid::new_v4().to_string();
        let pay1=date_string(today()-Duration::days(7)); let pay2=date_string(today());
        conn.execute("INSERT INTO paycheck_occurrences(id,income_source_id,pay_date,projected_amount_cents,status) VALUES(?1,?2,?3,153000,'projected')",params![first,source_b,pay1]).unwrap();
        conn.execute("INSERT INTO paycheck_occurrences(id,income_source_id,pay_date,projected_amount_cents,status) VALUES(?1,?2,?3,67000,'projected')",params![second,source_a,pay2]).unwrap();
        conn.execute("INSERT INTO bill_templates(id,name,category_id,amount_type,fixed_amount_cents,recurrence_type,payment_type,priority,can_split) VALUES('mortgage','Mortgage','housing','fixed',220000,'monthly','manual','essential',0)",[]).unwrap();
        let occ=Uuid::new_v4().to_string(); let due=date_string(today()+Duration::days(2)); let action=date_string(today());
        conn.execute("INSERT INTO bill_occurrences(id,bill_template_id,name_snapshot,category_id,due_date,latest_payment_date,earliest_payment_date,estimated_amount_cents,status,payment_type_snapshot,priority_snapshot,scheduled_payment_date) VALUES(?1,'mortgage','Mortgage','housing',?2,?3,?3,220000,'scheduled','manual','essential',?3)",params![occ,due,action]).unwrap();
        conn.execute("INSERT INTO bill_allocations(id,bill_occurrence_id,paycheck_occurrence_id,funding_source_type,allocated_amount_cents,source,reason_code) VALUES(?1,?2,?3,'paycheck',153000,'scheduler','reserved_across_paychecks')",params![Uuid::new_v4().to_string(),occ,first]).unwrap();
        conn.execute("INSERT INTO bill_allocations(id,bill_occurrence_id,paycheck_occurrence_id,funding_source_type,allocated_amount_cents,source,reason_code) VALUES(?1,?2,?3,'paycheck',67000,'scheduler','reserved_across_paychecks')",params![Uuid::new_v4().to_string(),occ,second]).unwrap();
        let view=payment_guidance_from_conn(&conn).unwrap();
        assert_eq!(view.items.len(),1);
        assert_eq!(view.items[0].funded_amount_cents,220_000);
        assert!(view.items[0].funding_complete);
        assert_eq!(view.items[0].recommended_payment_date,action);
        assert_eq!(view.items[0].action_status,"pay_today");
    }

    #[test]
    fn split_payment_guidance_keeps_each_provider_payment_date() {
        let mut conn = Connection::open_in_memory().unwrap();
        db::initialize(&mut conn).unwrap();
        db::complete_onboarding(&mut conn,"Test Household",Money::cents(50_000),"Checking",Money::cents(100_000),&["Jonathan".into(),"Tiffany".into()]).unwrap();
        let users: Vec<String> = {
            let mut stmt=conn.prepare("SELECT id FROM users ORDER BY created_at").unwrap();
            stmt.query_map([],|r|r.get(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap()
        };
        let source=Uuid::new_v4().to_string();
        conn.execute("INSERT INTO income_sources(id,user_id,name,schedule_type,default_projected_amount_cents) VALUES(?1,?2,'Pay','weekly',30000)",params![source,users[0]]).unwrap();
        let p1=Uuid::new_v4().to_string(); let p2=Uuid::new_v4().to_string();
        let d1=date_string(today()); let d2=date_string(today()+Duration::days(7));
        conn.execute("INSERT INTO paycheck_occurrences(id,income_source_id,pay_date,projected_amount_cents,status) VALUES(?1,?2,?3,30000,'projected')",params![p1,source,d1]).unwrap();
        conn.execute("INSERT INTO paycheck_occurrences(id,income_source_id,pay_date,projected_amount_cents,status) VALUES(?1,?2,?3,30000,'projected')",params![p2,source,d2]).unwrap();
        conn.execute("INSERT INTO bill_templates(id,name,category_id,amount_type,fixed_amount_cents,recurrence_type,payment_type,priority,can_split) VALUES('card','Card','debt','fixed',60000,'monthly','manual','normal',1)",[]).unwrap();
        let occ=Uuid::new_v4().to_string(); let due=date_string(today()+Duration::days(14));
        conn.execute("INSERT INTO bill_occurrences(id,bill_template_id,name_snapshot,category_id,due_date,latest_payment_date,earliest_payment_date,estimated_amount_cents,status,payment_type_snapshot,priority_snapshot,scheduled_payment_date) VALUES(?1,'card','Card','debt',?2,?2,?3,60000,'scheduled','manual','normal',?3)",params![occ,due,d1]).unwrap();
        conn.execute("INSERT INTO bill_allocations(id,bill_occurrence_id,paycheck_occurrence_id,funding_source_type,allocated_amount_cents,source,reason_code,recommended_payment_date) VALUES(?1,?2,?3,'paycheck',30000,'scheduler','split_across_paychecks',?4)",params![Uuid::new_v4().to_string(),occ,p1,d1]).unwrap();
        conn.execute("INSERT INTO bill_allocations(id,bill_occurrence_id,paycheck_occurrence_id,funding_source_type,allocated_amount_cents,source,reason_code,recommended_payment_date) VALUES(?1,?2,?3,'paycheck',30000,'scheduler','split_across_paychecks',?4)",params![Uuid::new_v4().to_string(),occ,p2,d2]).unwrap();
        let view=payment_guidance_from_conn(&conn).unwrap();
        assert_eq!(view.items.len(),1);
        assert_eq!(view.items[0].payment_actions.len(),2);
        assert_eq!(view.items[0].payment_actions[0].payment_date,d1);
        assert_eq!(view.items[0].payment_actions[0].amount_cents,30_000);
        assert_eq!(view.items[0].payment_actions[1].payment_date,d2);
        assert_eq!(view.items[0].payment_actions[1].amount_cents,30_000);
    }

}
