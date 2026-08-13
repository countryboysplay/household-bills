import { useEffect, useState } from "react";
import {
  deletePaycheck,
  getPlanner,
  getPaymentGuidance,
  listPaychecks,
  listPaycheckSchedules,
  runScheduler,
  savePaycheck,
  savePaycheckSchedule,
} from "../lib/backend";
import { dollarsToCents, formatMoney } from "../lib/money";
import type {
  PaycheckItem,
  PaycheckScheduleItem,
  PlannerView,
  PaymentGuidanceView,
  SavePaycheckPayload,
  SavePaycheckSchedulePayload,
  UserProfile,
} from "../lib/types";

type PayForm = {
  id?: string;
  userId: string;
  payDate: string;
  projected: string;
  expected: string;
  actual: string;
  status: "projected" | "updated" | "received" | "skipped";
  note: string;
};

type ScheduleForm = {
  id?: string;
  userId: string;
  frequency: "one_time" | "weekly" | "biweekly" | "semimonthly" | "monthly";
  payDate: string;
  projected: string;
  expected: string;
  actual: string;
  status: "projected" | "updated" | "received" | "skipped";
  note: string;
  anchorDate: string;
  firstDay: string;
  secondDay: string;
  dayOfMonth: string;
  weekendHolidayRule: "exact" | "prior_business_day" | "next_business_day";
};

const emptyScheduleForm = (userId: string): ScheduleForm => ({
  userId,
  frequency: "one_time",
  payDate: new Date().toISOString().slice(0, 10),
  projected: "",
  expected: "",
  actual: "",
  status: "projected",
  note: "",
  anchorDate: new Date().toISOString().slice(0, 10),
  firstDay: "1",
  secondDay: "15",
  dayOfMonth: "1",
  weekendHolidayRule: "prior_business_day",
});

const scheduleToForm = (schedule: PaycheckScheduleItem): ScheduleForm => ({
  id: schedule.id,
  userId: schedule.userId,
  frequency: schedule.frequency,
  payDate: schedule.nextPayDate ?? new Date().toISOString().slice(0, 10),
  projected: (schedule.defaultProjectedAmountCents / 100).toFixed(2),
  expected: "",
  actual: "",
  status: "projected",
  note: "",
  anchorDate: schedule.anchorDate ?? schedule.nextPayDate ?? new Date().toISOString().slice(0, 10),
  firstDay: String(schedule.firstDay ?? 1),
  secondDay: String(schedule.secondDay ?? 15),
  dayOfMonth: String(schedule.dayOfMonth ?? 1),
  weekendHolidayRule: schedule.weekendHolidayRule,
});

export function Planner({ users, onChanged }: { users: UserProfile[]; onChanged: () => Promise<void> | void }) {
  const [planner, setPlanner] = useState<PlannerView | null>(null);
  const [guidance, setGuidance] = useState<PaymentGuidanceView | null>(null);
  const [paychecks, setPaychecks] = useState<PaycheckItem[]>([]);
  const [schedules, setSchedules] = useState<PaycheckScheduleItem[]>([]);
  const [form, setForm] = useState<PayForm | null>(null);
  const [scheduleForm, setScheduleForm] = useState<ScheduleForm | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [planningWindowDays, setPlanningWindowDays] = useState<30 | 60 | 90>(30);

  const load = async () => {
    setLoading(true);
    setError("");
    try {
      const [p, checks, recurring, paymentGuidance] = await Promise.all([getPlanner(), listPaychecks(), listPaycheckSchedules(), getPaymentGuidance()]);
      setPlanner(p);
      setPaychecks(checks);
      setSchedules(recurring);
      setGuidance(paymentGuidance);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { void load(); }, []);

  const openNew = () => setScheduleForm(emptyScheduleForm(users[0]?.id ?? ""));

  const editSchedule = (schedule: PaycheckScheduleItem) => setScheduleForm(scheduleToForm(schedule));

  const editPaycheck = (id: string) => {
    const p = paychecks.find(x => x.id === id);
    if (!p) return;
    setForm({
      id: p.id,
      userId: p.userId,
      payDate: p.payDate,
      projected: (p.projectedAmountCents / 100).toFixed(2),
      expected: p.expectedAmountCents == null ? "" : (p.expectedAmountCents / 100).toFixed(2),
      actual: p.actualAmountCents == null ? "" : (p.actualAmountCents / 100).toFixed(2),
      status: p.status,
      note: "",
    });
  };

  const save = async () => {
    if (!form) return;
    setBusy(true);
    setError("");
    try {
      const payload: SavePaycheckPayload = {
        id: form.id ?? null,
        userId: form.userId,
        payDate: form.payDate,
        projectedAmountCents: dollarsToCents(form.projected || "0"),
        expectedAmountCents: form.expected ? dollarsToCents(form.expected) : null,
        actualAmountCents: form.actual ? dollarsToCents(form.actual) : null,
        status: form.status,
        note: form.note || null,
      };
      await savePaycheck(payload);
      setForm(null);
      await load();
      await onChanged();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const saveNewOrSchedule = async () => {
    if (!scheduleForm) return;
    setBusy(true);
    setError("");
    try {
      if (scheduleForm.frequency === "one_time") {
        const payload: SavePaycheckPayload = {
          id: null,
          userId: scheduleForm.userId,
          payDate: scheduleForm.payDate,
          projectedAmountCents: dollarsToCents(scheduleForm.projected || "0"),
          expectedAmountCents: scheduleForm.expected ? dollarsToCents(scheduleForm.expected) : null,
          actualAmountCents: scheduleForm.actual ? dollarsToCents(scheduleForm.actual) : null,
          status: scheduleForm.status,
          note: scheduleForm.note || null,
        };
        await savePaycheck(payload);
      } else {
        const payload: SavePaycheckSchedulePayload = {
          id: scheduleForm.id ?? null,
          userId: scheduleForm.userId,
          frequency: scheduleForm.frequency,
          defaultProjectedAmountCents: dollarsToCents(scheduleForm.projected || "0"),
          anchorDate: scheduleForm.frequency === "weekly" || scheduleForm.frequency === "biweekly" ? scheduleForm.anchorDate : null,
          firstDay: scheduleForm.frequency === "semimonthly" ? Number(scheduleForm.firstDay) : null,
          secondDay: scheduleForm.frequency === "semimonthly" ? Number(scheduleForm.secondDay) : null,
          dayOfMonth: scheduleForm.frequency === "monthly" ? Number(scheduleForm.dayOfMonth) : null,
          weekendHolidayRule: scheduleForm.weekendHolidayRule,
        };
        await savePaycheckSchedule(payload);
      }
      setScheduleForm(null);
      await load();
      await onChanged();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const removePaycheck = async () => {
    if (!form?.id) return;
    const paycheck = paychecks.find(p => p.id === form.id);
    const label = paycheck ? `${paycheck.ownerName} paycheck on ${formatDate(paycheck.payDate)}` : "this paycheck";
    if (!window.confirm(`Remove ${label}?${paycheck?.status === "received" ? " The posted deposit will also be reversed from the app balance." : ""}`)) return;
    setBusy(true);
    setError("");
    try {
      await deletePaycheck({ id: form.id });
      setForm(null);
      await load();
      await onChanged();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const recalc = async () => {
    setBusy(true);
    setError("");
    try {
      setPlanner(await runScheduler());
      const [checks, recurring, paymentGuidance] = await Promise.all([listPaychecks(), listPaycheckSchedules(), getPaymentGuidance()]);
      setPaychecks(checks);
      setSchedules(recurring);
      setGuidance(paymentGuidance);
      await onChanged();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const windowCutoff = new Date();
  windowCutoff.setHours(23, 59, 59, 999);
  windowCutoff.setDate(windowCutoff.getDate() + planningWindowDays);
  const isInsidePlanningWindow = (date: string | null | undefined) => {
    if (!date) return true;
    return new Date(`${date}T12:00:00`) <= windowCutoff;
  };
  const visiblePaychecks = planner?.paychecks.filter(p => isInsidePlanningWindow(p.payDate)) ?? [];
  const visibleGuidanceItems = guidance?.items.filter(g => {
    const firstActionDate = g.paymentActions[0]?.paymentDate;
    return isInsidePlanningWindow(firstActionDate ?? g.recommendedPaymentDate ?? g.dueDate);
  }) ?? [];
  const visibleWarnings = planner?.warnings.filter(w => isInsidePlanningWindow(w.date)) ?? [];
  const visibleNeedsFundingCount = visibleGuidanceItems.filter(g => g.actionStatus === "needs_funding").length;
  const visibleDueNowCount = visibleGuidanceItems.filter(g => ["pay_today", "draft_today", "overdue_action"].includes(g.actionStatus)).length;

  return <div className="page planner-page">
    <header className="page-header">
      <div><h1>Paycheck Planner</h1><p>See what each paycheck needs to cover and what remains safe to spend.</p></div>
      <div className="header-actions">
        <button className="outline" onClick={recalc} disabled={busy || loading}>↻ Recalculate</button>
        <button className="primary" onClick={openNew}>＋ Paycheck</button>
      </div>
    </header>

    {error && <div className="page-error">{error}</div>}

    {loading && !planner ? <div className="card empty-inline">Building your paycheck plan… You can still use <strong>+ Paycheck</strong> above to enter your next check.</div> : null}
    {!loading && !planner ? <div className="card empty-inline">The planner could not load. The error above should explain why. You can still add a paycheck and then retry.</div> : null}

    {planner ? <>
      <div className="planner-summary">
        <div><span>Current Cash</span><strong>{formatMoney(planner.currentCashCents)}</strong><small>Tracked account balance before future paychecks</small></div>
        <div><span>Safe Now</span><strong>{formatMoney(planner.currentCashSafeCents)}</strong></div>
        <div><span>Protected Buffer</span><strong>{formatMoney(planner.protectedBufferCents)}</strong></div>
        <div><span>Needs Funding ({planningWindowDays} Days)</span><strong>{visibleNeedsFundingCount}</strong></div>
      </div>

      <section className="pay-schedule-section card">
        <div className="pay-schedule-heading">
          <div><h2>Pay Schedules</h2><p>Set the normal schedule once. You can still change the amount of any individual paycheck when commissions or deductions change it.</p></div>
          <button className="outline" onClick={openNew}>Add / Change Schedule</button>
        </div>
        <div className="pay-schedule-list">
          {schedules.length === 0 ? <div className="pay-schedule-empty">No recurring pay schedule is configured yet. Use <strong>+ Paycheck</strong> and choose Weekly, Every 2 Weeks, Twice a Month, or Monthly.</div> : schedules.map(s => <button className="pay-schedule-card" key={s.id} onClick={() => editSchedule(s)}>
            <span><strong>{s.ownerName}</strong><small>{frequencyLabel(s.frequency)}</small></span>
            <span><small>Normal check</small><b>{formatMoney(s.defaultProjectedAmountCents)}</b></span>
            <span><small>Next payday</small><b>{s.nextPayDate ? formatDate(s.nextPayDate) : "Not generated"}</b></span>
            <em>Edit Schedule</em>
          </button>)}
        </div>
      </section>

      <section className="planner-window-bar card">
        <div><strong>Planning Window</strong><small>Show only the paychecks and bill actions you need to plan for in the next 30, 60, or 90 days.</small></div>
        <div className="segmented planner-window-filter" role="group" aria-label="Planning window">
          {([30, 60, 90] as const).map(days => <button key={days} className={planningWindowDays === days ? "active" : ""} onClick={() => setPlanningWindowDays(days)}>{days} Days</button>)}
        </div>
      </section>

      <section className="card planner-payment-instructions">
        <div className="card-title"><div><h2>Payment Instructions</h2><p>This is when you actually pay each bill. Paycheck cards below show where the money is reserved.</p></div><span>{guidance ? `${visibleDueNowCount} action${visibleDueNowCount===1?"":"s"} due now • ${visibleGuidanceItems.length} in ${planningWindowDays} days` : "Loading…"}</span></div>
        {!guidance ? <p className="empty-inline">Building payment instructions…</p> : visibleGuidanceItems.length === 0 ? <div className="guidance-all-clear"><span>✓</span><div><strong>No unpaid bills need action.</strong><small>There are no current payment instructions.</small></div></div> : <div className="planner-guidance-list">{visibleGuidanceItems.map(g=><div className={`planner-guidance-row ${g.actionStatus}`} key={g.occurrenceId}><div><strong>{g.paymentType==="autopay"?`Autopay: ${g.billName}`:`Pay ${g.billName}`}</strong>{g.paymentActions.length>1?<div className="planner-action-plan"><span>{formatMoney(g.remainingAmountCents)} remaining • Due {formatDate(g.dueDate)}</span>{g.paymentActions.map((a,i)=><small key={`${a.paymentDate}-${i}`}>Pay <b>{formatMoney(a.amountCents)}</b> on {formatDate(a.paymentDate)}</small>)}</div>:<span>{formatMoney(g.paymentActions[0]?.amountCents??g.remainingAmountCents)} on {formatDate(g.paymentActions[0]?.paymentDate??g.recommendedPaymentDate)} • Due {formatDate(g.dueDate)}</span>}<small>{fundingSummary(g)}</small></div><em>{guidanceStatusLabel(g.actionStatus)}</em></div>)}</div>}
      </section>

      {visibleWarnings.length > 0 && <section className="planner-alerts">
        {visibleWarnings.slice(0, 3).map((w, i) => <div className="planner-alert" key={`${w.code}-${i}`}><strong>⚠ {friendlyWarning(w.code)}</strong><span>{friendlyWarningMessage(w, planner.protectedBufferCents)}</span></div>)}
      </section>}

      <section className="paycheck-card-grid">
        {visiblePaychecks.length === 0 ? <div className="card empty-inline">No paychecks fall within the next {planningWindowDays} days. Choose a longer planning window or click <strong>+ Paycheck</strong> to add a check.</div> : visiblePaychecks.map((p, index) => {
          const health = p.status.split(":")[1] ?? "healthy";
          return <article className={`paycheck-card ${health}`} key={p.id}>
            <button className="paycheck-card-head" onClick={() => editPaycheck(p.id)}><span className="pay-index">{index + 1}</span><span><strong>{formatDate(p.payDate)}</strong><small>{p.ownerName}</small></span><b>{formatMoney(p.amountCents)}</b></button>
            <div className="paycheck-metrics"><span>Scheduled Bills <b>{formatMoney(p.billsTotalCents)}</b></span><span>Savings / Extra Debt <b>{formatMoney(p.commitmentsTotalCents)}</b></span><span>Safe to Spend <b className={health === "shortage" || health === "tight" ? "warn-text" : "good-text"}>{formatMoney(p.safeRemainingCents)}</b></span></div>
            <div className="paycheck-bills">{p.bills.length === 0 ? <p>No bills assigned.</p> : p.bills.map(b => { const isReserve = b.reasonCode === "reserved_across_paychecks"; const paymentDay = isReserve && b.paymentDate === p.payDate; return <div className="planner-bill" key={`${p.id}-${b.occurrenceId}-${b.amountCents}`}><span><i>{b.paymentType === "autopay" ? "↻" : isReserve ? "◫" : "▣"}</i><strong>{isReserve ? `Reserve for ${b.name}` : b.name}</strong><small>{isReserve ? (paymentDay ? `Final reserve • Pay bill today • Due ${formatDate(b.dueDate)}` : `Pay by ${formatDate(b.paymentDate)} • Due ${formatDate(b.dueDate)}`) : `Due ${formatDate(b.dueDate)}`}</small></span><b>{formatMoney(b.amountCents)}</b></div> })}</div>{p.commitments.length > 0 && <div className="paycheck-commitments">{p.commitments.map(c => <div className="planner-commitment" key={c.id}><span><i>{c.kind.includes("Debt") ? "↘" : "＋"}</i><strong>{c.kind.includes("Debt") ? `Extra payment: ${c.name}` : `Save: ${c.name}`}</strong><small>{c.reducedByCents > 0 ? `Reduced by ${formatMoney(c.reducedByCents)} to protect bills/buffer` : "Planned optional commitment"}</small></span><b>{formatMoney(c.effectiveAmountCents)}</b></div>)}</div>}
            <div className="paycheck-footer"><span className={`health-chip ${health}`}>{healthLabel(health)}</span><button onClick={() => editPaycheck(p.id)}>Update Paycheck</button></div>
          </article>;
        })}
      </section>

      <section className="planner-bottom-grid">
        <div className="card"><div className="card-title"><h2>Scheduler Rules</h2></div><ul className="rule-list"><li>Bills start with the latest eligible paycheck before they are due.</li><li>The protected buffer is a balance floor, never an expense.</li><li>When needed, eligible bills move earlier to prevent a shortage.</li><li>Autopay dates stay fixed even when a different paycheck funds them.</li><li>Optional savings and extra debt payments are reduced before required bills or the protected buffer.</li></ul></div>
        <div className="card"><div className="card-title"><h2>Variable Paychecks</h2></div><p className="muted-text">Your recurring schedule creates the dates automatically. When you know the real amount for a specific check, open that paycheck and enter the expected or actual amount. The plan recalculates immediately.</p><button className="outline" onClick={openNew}>Add Paycheck / Schedule</button></div>
      </section>
    </> : null}

    {scheduleForm && <div className="modal-backdrop"><div className="modal paycheck-modal schedule-modal">
      <div className="modal-header"><div><h2>{scheduleForm.id ? "Edit Pay Schedule" : "Add Paycheck"}</h2><p>Choose whether this is a one-time check or a recurring paycheck schedule.</p></div><button onClick={() => setScheduleForm(null)}>×</button></div>
      <div className="editor-grid">
        <label>Person<select value={scheduleForm.userId} onChange={e => { const existing = schedules.find(s => s.userId === e.target.value); setScheduleForm(existing ? scheduleToForm(existing) : { ...emptyScheduleForm(e.target.value), frequency: scheduleForm.frequency }); }}>{users.map(u => <option value={u.id} key={u.id}>{u.displayName}</option>)}</select></label>
        <label>Pay Frequency<select value={scheduleForm.frequency} onChange={e => setScheduleForm({ ...scheduleForm, frequency: e.target.value as ScheduleForm["frequency"] })}><option value="one_time">One-Time / Manual</option><option value="weekly">Weekly</option><option value="biweekly">Every 2 Weeks</option><option value="semimonthly">Twice a Month</option><option value="monthly">Monthly</option></select></label>
        <label>Normal / Projected Amount<input value={scheduleForm.projected} onChange={e => setScheduleForm({ ...scheduleForm, projected: e.target.value })} placeholder="0.00"/><small>Use the normal take-home amount. Individual checks can be changed later.</small></label>
        {scheduleForm.frequency === "one_time" && <label>Pay Date<input type="date" value={scheduleForm.payDate} onChange={e => setScheduleForm({ ...scheduleForm, payDate: e.target.value })}/></label>}
        {(scheduleForm.frequency === "weekly" || scheduleForm.frequency === "biweekly") && <label>Next Pay Date<input type="date" value={scheduleForm.anchorDate} onChange={e => setScheduleForm({ ...scheduleForm, anchorDate: e.target.value })}/><small>This anchors all future paycheck dates.</small></label>}
        {scheduleForm.frequency === "semimonthly" && <><label>First Pay Day<input type="number" min="1" max="31" value={scheduleForm.firstDay} onChange={e => setScheduleForm({ ...scheduleForm, firstDay: e.target.value })}/><small>Example: 1</small></label><label>Second Pay Day<input type="number" min="1" max="31" value={scheduleForm.secondDay} onChange={e => setScheduleForm({ ...scheduleForm, secondDay: e.target.value })}/><small>Example: 15 or 31 for end of month</small></label></>}
        {scheduleForm.frequency === "monthly" && <label>Pay Day of Month<input type="number" min="1" max="31" value={scheduleForm.dayOfMonth} onChange={e => setScheduleForm({ ...scheduleForm, dayOfMonth: e.target.value })}/><small>Dates beyond the end of a short month are clamped automatically.</small></label>}
        {scheduleForm.frequency !== "one_time" && <label>Weekend Date Rule<select value={scheduleForm.weekendHolidayRule} onChange={e => setScheduleForm({ ...scheduleForm, weekendHolidayRule: e.target.value as ScheduleForm["weekendHolidayRule"] })}><option value="prior_business_day">Pay on Prior Business Day</option><option value="next_business_day">Pay on Next Business Day</option><option value="exact">Use Exact Calendar Date</option></select></label>}
        {scheduleForm.frequency === "one_time" && <><label>Expected Amount<input value={scheduleForm.expected} onChange={e => setScheduleForm({ ...scheduleForm, expected: e.target.value })} placeholder="Optional"/></label><label>Actual Deposited Amount<input value={scheduleForm.actual} onChange={e => setScheduleForm({ ...scheduleForm, actual: e.target.value })} placeholder="Enter on payday"/></label><label>Status<select value={scheduleForm.status} onChange={e => setScheduleForm({ ...scheduleForm, status: e.target.value as ScheduleForm["status"] })}><option value="projected">Projected</option><option value="updated">Updated Estimate</option><option value="received">Received</option><option value="skipped">Skipped</option></select></label></>}
      </div>
      {scheduleForm.frequency === "one_time" ? <label>Note<textarea rows={2} value={scheduleForm.note} onChange={e => setScheduleForm({ ...scheduleForm, note: e.target.value })}/></label> : <div className="schedule-help"><strong>Recurring schedule</strong><span>The app will automatically create future paycheck dates. Open any individual paycheck later to enter a commission-adjusted expected amount or the actual deposited amount.</span></div>}
      <div className="modal-footer"><button className="outline" onClick={() => setScheduleForm(null)}>Cancel</button><button className="primary" disabled={busy} onClick={saveNewOrSchedule}>{busy ? "Saving…" : scheduleForm.frequency === "one_time" ? "Save & Recalculate" : "Save Schedule & Recalculate"}</button></div>
    </div></div>}

    {form && <div className="modal-backdrop"><div className="modal paycheck-modal">
      <div className="modal-header"><div><h2>Update Paycheck</h2><p>Update this individual check without changing the recurring pay schedule.</p></div><button onClick={() => setForm(null)}>×</button></div>
      <div className="editor-grid">
        <label>Person<select value={form.userId} disabled>{users.map(u => <option value={u.id} key={u.id}>{u.displayName}</option>)}</select></label>
        <label>Pay Date<input type="date" value={form.payDate} onChange={e => setForm({ ...form, payDate: e.target.value })}/></label>
        <label>Normal / Projected Amount<input value={form.projected} onChange={e => setForm({ ...form, projected: e.target.value })} placeholder="0.00"/></label>
        <label>Expected Amount<input value={form.expected} onChange={e => setForm({ ...form, expected: e.target.value })} placeholder="Optional"/><small>Use this when you know the upcoming check will differ because of commissions.</small></label>
        <label>Actual Deposited Amount<input value={form.actual} onChange={e => setForm({ ...form, actual: e.target.value })} placeholder="Enter on payday"/></label>
        <label>Status<select value={form.status} onChange={e => setForm({ ...form, status: e.target.value as PayForm["status"] })}><option value="projected">Projected</option><option value="updated">Updated Estimate</option><option value="received">Received</option><option value="skipped">Skipped</option></select></label>
      </div>
      <label>Note<textarea rows={2} value={form.note} onChange={e => setForm({ ...form, note: e.target.value })}/></label>
      <div className="modal-footer paycheck-modal-footer">{form.id ? <button className="danger-outline" disabled={busy} onClick={removePaycheck}>Remove Paycheck</button> : <span/>}<div className="modal-footer-actions"><button className="outline" onClick={() => setForm(null)}>Cancel</button><button className="primary" disabled={busy} onClick={save}>{busy ? "Saving…" : "Save & Recalculate"}</button></div></div>
    </div></div>}
  </div>;
}

const guidanceStatusLabel=(v:string)=>v==="pay_today"?"Pay Today":v==="draft_today"?"Drafts Today":v==="overdue_action"?"Overdue":v==="needs_funding"?"Needs Funding":v==="coming_up"?"Coming Up":"Scheduled";
const fundingSummary=(g:PaymentGuidanceView["items"][number])=>g.fundingSources.length?"Funded from "+g.fundingSources.map(f=>f.sourceType==="current_cash"?`current cash ${formatMoney(f.amountCents)}`:`${f.ownerName??"Paycheck"} ${f.payDate?formatDate(f.payDate):""} ${formatMoney(f.amountCents)}`).join(" + "):"No funding source assigned yet";
const formatDate = (s: string) => new Date(`${s}T12:00:00`).toLocaleDateString("en-US", { month: "short", day: "numeric", year: new Date(s).getFullYear() === new Date().getFullYear() ? undefined : "numeric" });
const healthLabel = (h: string) => h === "shortage" ? "Shortage" : h === "tight" ? "Tight" : "Healthy";
const frequencyLabel = (frequency: PaycheckScheduleItem["frequency"]) => frequency === "weekly" ? "Weekly" : frequency === "biweekly" ? "Every 2 Weeks" : frequency === "semimonthly" ? "Twice a Month" : "Monthly";
const friendlyWarning = (c: string) => c.includes("Negative") ? "Negative Balance" : c.includes("BelowBuffer") ? "Below Protected Buffer" : c.includes("Shortage") ? "Funding Shortage" : "Schedule Notice";
const friendlyWarningMessage = (w: PlannerView["warnings"][number], protectedBufferCents: number) => {
  const when = w.date ? formatDate(w.date) : "the current plan";
  if (w.code.includes("ProjectedNegative")) {
    return `Projected balance is ${formatMoney(w.amountCents ?? 0)} below $0.00 on ${when}.`;
  }
  if (w.code.includes("ProjectedBelowBuffer")) {
    return `Projected balance is ${formatMoney(w.amountCents ?? 0)} below your ${formatMoney(protectedBufferCents)} protected buffer on ${when}.`;
  }
  if (w.code.includes("FundingShortage") || w.code.includes("PartialFunding")) {
    return w.amountCents != null ? `${w.message.replace(/\d+ cents/g, formatMoney(w.amountCents))}` : w.message;
  }
  return w.message.replace(/(\d+) cents/g, (_, cents: string) => formatMoney(Number(cents)));
};
