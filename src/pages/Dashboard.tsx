import { useEffect, useState } from "react";
import { getDashboardData, getPaymentGuidance } from "../lib/backend";
import type { DashboardData, PageKey, PaymentGuidanceItem, PaymentGuidanceView } from "../lib/types";
import { formatMoney } from "../lib/money";

export function Dashboard({ profileName, onNavigate }: { profileName: string; onNavigate: (page: PageKey) => void }) {
  const [data, setData] = useState<DashboardData | null>(null);
  const [guidance, setGuidance] = useState<PaymentGuidanceView | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    Promise.all([getDashboardData(), getPaymentGuidance()])
      .then(([dashboard, paymentGuidance]) => { setData(dashboard); setGuidance(paymentGuidance); })
      .catch(e => setError(e instanceof Error ? e.message : String(e)));
  }, []);

  if (error) return <div className="page"><div className="page-error">{error}</div></div>;
  if (!data) return <div className="loading-inline">Loading household dashboard…</div>;

  const totalOut = data.monthBillPaymentsCents + data.monthEverydaySpendingCents;
  const maxCashFlow = Math.max(data.monthIncomeCents, data.monthBillPaymentsCents, data.monthEverydaySpendingCents, Math.abs(data.monthNetCents), 1);
  const spendTotal = data.categorySpending.reduce((sum, x) => sum + x.amountCents, 0);

  return <div className="page dashboard-page">
    <header className="page-header">
      <div><h1>Good {greeting()}, {profileName}</h1><p>Here’s your household financial overview for today.</p></div>
      <div className="header-actions"><button className="outline" onClick={() => onNavigate("history")}>View History</button><button className="primary" onClick={() => onNavigate("spending")}>Reconcile Balance</button></div>
    </header>

    <div className="dashboard-layout">
      <main className="dashboard-main">
        <div className="stats-grid">
          <DashboardStat tone="green" icon="＄" label="Current Cash" value={formatMoney(data.currentCashCents)} detail="Across manually tracked cash accounts"/>
          <DashboardStat tone="blue" icon="▦" label="Next Paycheck" value={data.nextPaycheck ? formatDate(data.nextPaycheck.payDate) : "Not scheduled"} detail={data.nextPaycheck ? `${data.nextPaycheck.ownerName} • ${formatMoney(data.nextPaycheck.amountCents)}` : "Add a paycheck in the planner"}/>
          <DashboardStat tone="orange" icon="◇" label="Safe to Spend" value={formatMoney(data.safeToSpendCents)} detail={`${formatMoney(data.reservedBillsCents)} currently reserved • ${formatMoney(data.protectedBufferCents)} buffer`}/>
          <DashboardStat tone="purple" icon="◎" label="Month Net" value={formatMoney(data.monthNetCents)} detail={`${formatMoney(data.monthIncomeCents)} income • ${formatMoney(totalOut)} out`}/>
        </div>

        <section className="card pay-guidance-card">
          <div className="card-title pay-guidance-title"><div><h2>What to Pay</h2><p>The actual payment action, separate from money being reserved from earlier paychecks.</p></div><button className="link-button" onClick={() => onNavigate("planner")}>Open full plan</button></div>
          {!guidance ? <p className="empty-inline">Loading payment instructions…</p> : guidance.items.length === 0 ? <div className="guidance-all-clear"><span>✓</span><div><strong>No unpaid bills need an action.</strong><small>Your current bill plan is clear.</small></div></div> : <div className="pay-guidance-list">
            {guidance.items.slice(0,5).map(item => <PaymentInstruction key={item.occurrenceId} item={item}/>)}
          </div>}
        </section>

        <div className="dashboard-grid three phase4-dashboard-grid">
          <section className="card upcoming-card">
            <div className="card-title"><h2>Upcoming Bills</h2><button className="link-button" onClick={() => onNavigate("bills")}>View all</button></div>
            {data.upcomingBills.length ? data.upcomingBills.map(b => <div className="bill-row" key={b.id}>
              <span className="bill-icon">{b.paymentType === "autopay" ? "↻" : "▣"}</span>
              <div><strong>{b.name}</strong><small>Due {formatShortDate(b.dueDate)}{b.payByDate !== b.dueDate ? ` • plan by ${formatShortDate(b.payByDate)}` : ""}</small></div>
              <b>{formatMoney(b.amountCents)}</b>
            </div>) : <p className="empty-inline">No unpaid bills are currently scheduled.</p>}
          </section>

          <section className="card">
            <div className="card-title"><h2>Cash Flow</h2><span>This month</span></div>
            <div className="cashflow-bars">
              <CashBar label="Income" value={data.monthIncomeCents} max={maxCashFlow} tone="positive"/>
              <CashBar label="Bills" value={data.monthBillPaymentsCents} max={maxCashFlow} tone="negative"/>
              <CashBar label="Spending" value={data.monthEverydaySpendingCents} max={maxCashFlow} tone="negative2"/>
              <CashBar label="Net" value={data.monthNetCents} max={maxCashFlow} tone={data.monthNetCents >= 0 ? "positive2" : "negative"}/>
            </div>
          </section>

          <section className="card paycheck-overview">
            <div className="card-title"><h2>Paycheck Overview</h2><button className="link-button" onClick={() => onNavigate("planner")}>Open planner</button></div>
            {data.paychecks.length ? data.paychecks.map(p => <div className={`paycheck-mini ${p.status.includes("tight") || p.safeCents < data.protectedBufferCents ? "warning" : ""}`} key={p.id}>
              <div><strong>{formatDate(p.payDate)}</strong><span>{p.ownerName}</span><b>{formatMoney(p.amountCents)}</b></div>
              <dl><dt>Reserved</dt><dd>{formatMoney(p.billsCents)}</dd><dt>Safe remaining</dt><dd className={p.safeCents < data.protectedBufferCents ? "warn-text" : "good-text"}>{formatMoney(p.safeCents)}</dd></dl>
            </div>) : <p className="empty-inline">No upcoming paychecks yet.</p>}
          </section>
        </div>

        <div className="dashboard-grid lower phase4-lower-grid">
          <section className="card">
            <div className="card-title"><h2>Spending by Category</h2><button className="link-button" onClick={() => onNavigate("spending")}>Open spending</button></div>
            {data.categorySpending.length ? <div className="category-spend-list">{data.categorySpending.map((x, i) => <div key={x.categoryId} className="category-spend-row"><span>{x.categoryName}</span><div><i style={{width:`${spendTotal ? Math.max(6,(x.amountCents/spendTotal)*100) : 0}%`}}/></div><b>{formatMoney(x.amountCents)}</b><small>{spendTotal ? Math.round((x.amountCents/spendTotal)*100) : 0}%</small></div>)}</div> : <p className="empty-inline">No everyday spending has been recorded this month.</p>}
          </section>

          <section className="card">
            <div className="card-title"><h2>Savings & Debt</h2><button className="link-button" onClick={() => onNavigate("savings")}>View details</button></div>
            <div className="simple-money-pair"><div><span>Savings tracked</span><strong>{formatMoney(data.savingsTotalCents)}</strong></div><div><span>Debt tracked</span><strong>{formatMoney(data.debtTotalCents)}</strong></div></div>
            <p className="muted-text">Goals, sinking funds, debt balances, and optional extra-payment plans are available in Savings & Debt.</p>
          </section>

          <section className="card">
            <div className="card-title"><h2>Recent Activity</h2><button className="link-button" onClick={() => onNavigate("history")}>View all</button></div>
            {data.recentActivity.length ? data.recentActivity.map(a => <div className="activity-row" key={a.id}><div><strong>{a.summary}</strong><small>{a.userName ?? "Household"}</small></div><span className="activity-type">{friendlyEvent(a.eventType)}</span><span>{formatTimestamp(a.occurredAt)}</span></div>) : <p className="empty-inline">No household activity yet.</p>}
          </section>
        </div>
      </main>

      <aside className="dashboard-rail">
        <section className="card alerts"><div className="card-title"><h2>Alerts</h2></div>{data.alerts.map((a, i) => <div className={`alert ${a.tone}`} key={`${a.code}-${i}`}><span>{a.tone === "green" ? "✓" : a.tone === "red" ? "!" : "△"}</span><div><strong>{a.title}</strong><p>{a.message}</p></div></div>)}</section>
        <section className="card quick-actions"><h2>Quick Actions</h2><button onClick={() => onNavigate("bills")}>＋ Add or manage bill</button><button onClick={() => onNavigate("planner")}>＋ Update paycheck</button><button onClick={() => onNavigate("spending")}>＋ Add transaction</button><button onClick={() => onNavigate("spending")}>↻ Reconcile balance</button></section>
        <section className="card plan-rules-card"><div className="card-title"><h2>Planning Rules</h2><span>Deterministic</span></div><p>Bills are scheduled from paycheck dates, due dates, available household cash, and your protected buffer. Reserved money and actual payment dates are shown separately.</p><button className="outline" onClick={() => onNavigate("settings")}>Review Settings</button></section>
      </aside>
    </div>
  </div>;
}

function PaymentInstruction({item}:{item:PaymentGuidanceItem}) {
  const badge = item.actionStatus === "pay_today" ? "PAY TODAY" : item.actionStatus === "draft_today" ? "DRAFTS TODAY" : item.actionStatus === "overdue_action" ? "ACTION OVERDUE" : item.actionStatus === "needs_funding" ? "NEEDS FUNDING" : item.actionStatus === "coming_up" ? "COMING UP" : "SCHEDULED";
  const funding = fundingText(item);
  const actions = item.paymentActions.length ? item.paymentActions : [{paymentDate:item.recommendedPaymentDate,amountCents:item.remainingAmountCents,actionStatus:item.actionStatus}];
  return <div className={`payment-instruction ${item.actionStatus}`}>
    <div className="payment-instruction-main"><span className="payment-action-badge">{badge}</span><div><strong>{item.billName}</strong>{actions.length===1?<p>{item.paymentType === "autopay" ? "Autopay drafts" : "Pay"} <b>{formatMoney(actions[0].amountCents)}</b> on <b>{formatDate(actions[0].paymentDate)}</b> • Due {formatDate(item.dueDate)}</p>:<div className="payment-action-plan"><p><b>{formatMoney(item.remainingAmountCents)}</b> remaining • Due {formatDate(item.dueDate)} • Partial payments enabled</p>{actions.map((a,i)=><small key={`${a.paymentDate}-${i}`}><b>Pay {formatMoney(a.amountCents)}</b> on {formatDate(a.paymentDate)}</small>)}</div>}<small>{funding}</small></div></div>
    <span className={item.fundingComplete ? "funded-chip" : "funding-chip"}>{item.fundingComplete ? "Fully funded" : `${formatMoney(item.fundedAmountCents)} funded`}</span>
  </div>;
}

function fundingText(item:PaymentGuidanceItem){
  if(!item.fundingSources.length)return "No funding source has been assigned yet.";
  return "Funded from " + item.fundingSources.map(f=>f.sourceType==="current_cash"?`current cash ${formatMoney(f.amountCents)}`:`${f.ownerName??"Paycheck"} ${f.payDate?formatShortDate(f.payDate):""} ${formatMoney(f.amountCents)}`).join(" + ");
}
function DashboardStat({tone,icon,label,value,detail}:{tone:string;icon:string;label:string;value:string;detail:string}){return <div className="stat-card"><div className={`stat-icon ${tone}`}>{icon}</div><div><span className="eyebrow">{label}</span><strong>{value}</strong><small>{detail}</small></div></div>}
function CashBar({label,value,max,tone}:{label:string;value:number;max:number;tone:string}){const magnitude=Math.abs(value);return <div className="cashflow-bar-item"><div className="cashflow-track"><i className={tone} style={{height:`${Math.max(4,(magnitude/max)*100)}%`}}/></div><strong>{formatMoney(value)}</strong><span>{label}</span></div>}
function formatDate(v:string){return new Date(`${v}T12:00:00`).toLocaleDateString("en-US",{weekday:"short",month:"short",day:"numeric"})}
function formatShortDate(v:string){return new Date(`${v}T12:00:00`).toLocaleDateString("en-US",{month:"short",day:"numeric"})}
function formatTimestamp(v:string){const normalized=v.includes("T")?v:v.replace(" ","T");const d=new Date(normalized);return Number.isNaN(d.getTime())?v:d.toLocaleDateString("en-US",{month:"short",day:"numeric"})}
function friendlyEvent(v:string){return v.replaceAll("_"," ").replace(/\b\w/g,c=>c.toUpperCase())}
function greeting(){const h=new Date().getHours();return h<12?"morning":h<18?"afternoon":"evening"}
