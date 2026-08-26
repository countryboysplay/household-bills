import { useEffect, useMemo, useState } from "react";
import { archiveBill, getBillDetail, getPaymentGuidance, listBills, markBillPaid, saveBill } from "../lib/backend";
import { dollarsToCents, formatMoney } from "../lib/money";
import { todayText } from "../lib/dates";
import type { BillDetail, BillListItem, PaymentGuidanceView, SaveBillPayload, UserProfile } from "../lib/types";

type BillFormState = {
  id?: string;
  name: string;
  categoryId: string;
  amountType: "fixed" | "variable";
  amount: string;
  recurrenceType: "monthly" | "one_time";
  dueDay: string;
  oneTimeDueDate: string;
  paymentType: "manual" | "autopay";
  priority: "essential" | "normal" | "flexible";
  canSplit: boolean;
  assignedUserId: string;
  earliestDays: string;
  notes: string;
};

const emptyForm = (): BillFormState => ({
  name:"",categoryId:"utilities",amountType:"fixed",amount:"",recurrenceType:"monthly",dueDay:"15",oneTimeDueDate:"",
  paymentType:"manual",priority:"normal",canSplit:false,assignedUserId:"",earliestDays:"31",notes:"",
});

const categoryOptions = [
  ["housing","Housing"],["utilities","Utilities"],["insurance","Insurance"],["debt","Debt"],["other","Other"],
] as const;

export function Bills({ users, onChanged }: { users: UserProfile[]; onChanged: () => Promise<void> | void }) {
  const [bills,setBills]=useState<BillListItem[]>([]);
  const [selectedId,setSelectedId]=useState<string | null>(null);
  const [detail,setDetail]=useState<BillDetail | null>(null);
  const [guidance,setGuidance]=useState<PaymentGuidanceView | null>(null);
  const [search,setSearch]=useState("");
  const [filter,setFilter]=useState<"all"|"autopay"|"unpaid">("all");
  const [form,setForm]=useState<BillFormState | null>(null);
  const [payMode,setPayMode]=useState<"full"|"partial"|null>(null);
  const [paymentAmount,setPaymentAmount]=useState("");
  const [paymentDate,setPaymentDate]=useState(todayText());
  const [paidBy,setPaidBy]=useState(users[0]?.id??"");
  const [error,setError]=useState("");
  const [busy,setBusy]=useState(false);

  // `preferred === null` is an explicit "select nothing in particular" (used after
  // archiving). Only `undefined` means "keep whatever is selected" — collapsing the
  // two would re-select the bill that was just archived and request a missing row.
  const load=async(preferred?:string|null)=>{
    const [rows,paymentGuidance]=await Promise.all([listBills(),getPaymentGuidance()]);
    setBills(rows);
    setGuidance(paymentGuidance);
    const keepCurrent=preferred===undefined
      ? (selectedId && rows.some(r=>r.id===selectedId) ? selectedId : null)
      : preferred;
    const next=keepCurrent ?? rows[0]?.id ?? null;
    setSelectedId(next);
    if(next) setDetail(await getBillDetail(next)); else setDetail(null);
  };
  useEffect(()=>{
    void load().catch(e=>setError(e instanceof Error?e.message:String(e)));
  },[]);
  useEffect(()=>{
    if(selectedId) void getBillDetail(selectedId).then(setDetail).catch(e=>setError(e instanceof Error?e.message:String(e)));
  },[selectedId]);

  const shown=useMemo(()=>bills.filter(b=>{
    const matches=b.name.toLowerCase().includes(search.toLowerCase())||b.categoryName.toLowerCase().includes(search.toLowerCase());
    if(!matches)return false;
    if(filter==="autopay")return b.paymentType==="autopay";
    if(filter==="unpaid")return b.nextStatus!=="paid";
    return true;
  }),[bills,search,filter]);

  const edit=(bill:BillListItem)=>setForm({
    id:bill.id,name:bill.name,categoryId:bill.categoryId??"other",amountType:bill.amountType,amount:(bill.amountCents/100).toFixed(2),
    recurrenceType:bill.recurrenceType,dueDay:String(bill.dueDay??15),oneTimeDueDate:bill.recurrenceType==="one_time"?(bill.nextDueDate??""):"",
    paymentType:bill.paymentType,priority:bill.priority,canSplit:bill.canSplit,assignedUserId:bill.assignedUserId??"",earliestDays:String(bill.payEarliestDaysBefore),notes:detail?.bill.id===bill.id?(detail.notes??""):"",
  });

  const submitBill=async()=>{
    if(!form)return;
    setBusy(true);setError("");
    try{
      const payload:SaveBillPayload={id:form.id??null,name:form.name,categoryId:form.categoryId,amountType:form.amountType,amountCents:dollarsToCents(form.amount),
        dueDay:form.recurrenceType==="monthly"?Number(form.dueDay):null,recurrenceType:form.recurrenceType,oneTimeDueDate:form.recurrenceType==="one_time"?form.oneTimeDueDate:null,
        paymentType:form.paymentType,priority:form.priority,canSplit:form.canSplit,assignedUserId:form.assignedUserId||null,payEarliestDaysBefore:form.earliestDays.trim()===""||!Number.isFinite(Number(form.earliestDays))?31:Number(form.earliestDays),notes:form.notes||null};
      const id=await saveBill(payload); setForm(null); await load(id); await onChanged();
    }catch(e){setError(e instanceof Error?e.message:String(e));}finally{setBusy(false)}
  };

  const openPay=(mode:"full"|"partial")=>{
    if(!detail?.bill.nextOccurrenceId)return;
    setPayMode(mode); setPaymentAmount((detail.bill.remainingAmountCents/100).toFixed(2)); setPaymentDate(todayText()); setPaidBy(users[0]?.id??"");
  };
  const submitPayment=async()=>{
    if(!detail?.bill.nextOccurrenceId||!payMode)return;
    setBusy(true);setError("");
    try{
      await markBillPaid({occurrenceId:detail.bill.nextOccurrenceId,amountCents:dollarsToCents(paymentAmount),paidDate:paymentDate,paidByUserId:paidBy,paymentMethod:"Checking",note:null,isPartial:payMode==="partial"});
      setPayMode(null); await load(detail.bill.id); await onChanged();
    }catch(e){setError(e instanceof Error?e.message:String(e));}finally{setBusy(false)}
  };

  const doArchive=async()=>{
    if(!detail||!confirm(`Archive ${detail.bill.name}? Its history will be kept.`))return;
    setBusy(true);setError("");
    try{
      await archiveBill(detail.bill.id);
      setSelectedId(null);
      await load(null);
      await onChanged();
    }catch(e){setError(e instanceof Error?e.message:String(e));}finally{setBusy(false)}
  };

  const selectedGuidance = detail?.bill.nextOccurrenceId ? guidance?.items.find(g=>g.occurrenceId===detail.bill.nextOccurrenceId) ?? null : null;

  return <div className="page bills-page">
    <header className="page-header"><div><h1>Bills</h1><p>Manage recurring and one-time household bills.</p></div><div className="header-actions"><button className="primary" onClick={()=>setForm(emptyForm())}>＋ Add Bill</button></div></header>
    {error&&<div className="page-error">{error}</div>}
    <div className="bill-toolbar"><input value={search} onChange={e=>setSearch(e.target.value)} placeholder="Search bills…"/><div className="segmented"><button className={filter==="all"?"active":""} onClick={()=>setFilter("all")}>All</button><button className={filter==="autopay"?"active":""} onClick={()=>setFilter("autopay")}>Autopay</button><button className={filter==="unpaid"?"active":""} onClick={()=>setFilter("unpaid")}>Unpaid</button></div></div>
    <div className="bills-layout">
      <section className="bill-table card">
        <div className="bill-table-head"><span>Bill</span><span>Amount</span><span>Due</span><span>Type</span><span>Status</span><span>Latest Funding</span></div>
        {shown.length===0&&<div className="empty-inline">No bills yet. Add your first bill to start the schedule.</div>}
        {shown.map(b=><button className={`bill-table-row ${selectedId===b.id?"selected":""}`} key={b.id} onClick={()=>setSelectedId(b.id)}>
          <span className="bill-name-cell"><i>{iconFor(b.categoryId)}</i><span><strong>{b.name}</strong><small>{b.categoryName}</small></span></span>
          <span><strong>{formatMoney(b.amountCents)}</strong><small>{b.amountType==="variable"?"Estimated":"Fixed"}</small></span>
          <span><strong>{b.recurrenceType==="monthly"?ordinal(b.dueDay??1):formatShortDate(b.nextDueDate)}</strong><small>{b.nextDueDate?`Due: ${formatShortDate(b.nextDueDate)}`:""}{b.nextPayByDate&&b.nextDueDate&&b.nextPayByDate!==b.nextDueDate?` • Plan by ${formatShortDate(b.nextPayByDate)}`:""}</small></span>
          <span><em className={`type-chip ${b.paymentType}`}>{b.paymentType==="autopay"?"Autopay":"Manual"}</em></span>
          <span><em className="status-chip good">{b.nextStatus??"Active"}</em></span>
          <span><strong>{b.assignedPaycheckDate?formatShortDate(b.assignedPaycheckDate):"Not scheduled"}</strong><small>{b.assignedPaycheckOwner??""}</small></span>
        </button>)}
      </section>
      <aside className="bill-detail card">
        {!detail?<div className="empty-inline">Select a bill to view details.</div>:<>
          <div className="bill-detail-title"><div className="detail-icon">{iconFor(detail.bill.categoryId)}</div><div><h2>{detail.bill.name}</h2><p>{detail.bill.categoryName}</p></div><span className="active-chip">Active</span></div>
          <div className="detail-actions"><button className="primary" disabled={!detail.bill.nextOccurrenceId} onClick={()=>openPay("full")}>✓ Mark Paid</button><button className="outline" onClick={()=>edit(detail.bill)}>Edit</button><button className="outline" onClick={doArchive}>Archive</button></div>
          <dl className="detail-dl"><dt>Next due</dt><dd>{formatLongDate(detail.bill.nextDueDate)}</dd>{selectedGuidance&&<><dt>{selectedGuidance.paymentType==="autopay"?"Expected draft":"Payment plan"}</dt><dd>{selectedGuidance.paymentActions.length>1?<div className="bill-action-plan">{selectedGuidance.paymentActions.map((a,i)=><span key={`${a.paymentDate}-${i}`}><strong>{formatMoney(a.amountCents)}</strong> on {formatLongDate(a.paymentDate)}</span>)}</div>:<strong>{formatLongDate(selectedGuidance.paymentActions[0]?.paymentDate??selectedGuidance.recommendedPaymentDate)}</strong>}</dd><dt>Funding plan</dt><dd>{billFundingText(selectedGuidance)}</dd></>}{detail.bill.nextPayByDate&&detail.bill.nextDueDate&&detail.bill.nextPayByDate!==detail.bill.nextDueDate&&<><dt>Latest safe date</dt><dd>{formatLongDate(detail.bill.nextPayByDate)}</dd></>}<dt>Amount</dt><dd>{formatMoney(detail.bill.amountCents)}{detail.bill.amountType==="variable"?" est.":""}</dd><dt>Payment type</dt><dd>{detail.bill.paymentType}</dd><dt>Priority</dt><dd>{detail.bill.priority}</dd><dt>Latest funding</dt><dd>{detail.bill.assignedPaycheckDate?`${formatShortDate(detail.bill.assignedPaycheckDate)} • ${detail.bill.assignedPaycheckOwner??""}`:"Not scheduled"}</dd><dt>Responsibility</dt><dd>{detail.bill.assignedUserName??"Shared"}</dd></dl>
          <div className="detail-section"><div className="card-title"><h2>Payment History</h2></div>{detail.paymentHistory.length===0?<p className="muted-text">No payments recorded yet.</p>:detail.paymentHistory.slice(0,8).map(p=><div className="history-line" key={p.id}><span>{formatShortDate(p.paidDate)}</span><strong>{formatMoney(p.amountCents)}</strong><span>{p.paidBy}</span></div>)}</div>
          {detail.notes&&<div className="detail-section"><div className="card-title"><h2>Notes</h2></div><p className="muted-text">{detail.notes}</p></div>}
          {detail.bill.nextOccurrenceId&&detail.bill.canSplit&&<button className="text-action" onClick={()=>openPay("partial")}>Record a partial payment</button>}
        </>}
      </aside>
    </div>
    {form&&<div className="modal-backdrop"><div className="modal bill-form-modal"><div className="modal-header"><div><h2>{form.id?"Edit Bill":"Add Bill"}</h2><p>Enter the details the scheduler needs.</p></div><button onClick={()=>setForm(null)}>×</button></div>
      <div className="form-sections"><section><h3>Basic Information</h3><div className="editor-grid"><label>Bill Name<input value={form.name} onChange={e=>setForm({...form,name:e.target.value})}/></label><label>Category<select value={form.categoryId} onChange={e=>setForm({...form,categoryId:e.target.value})}>{categoryOptions.map(x=><option value={x[0]} key={x[0]}>{x[1]}</option>)}</select></label>
      <label>Amount Type<select value={form.amountType} onChange={e=>setForm({...form,amountType:e.target.value as BillFormState["amountType"]})}><option value="fixed">Fixed</option><option value="variable">Variable / Estimated</option></select></label><label>{form.amountType==="variable"?"Current Estimate":"Amount"}<input value={form.amount} onChange={e=>setForm({...form,amount:e.target.value})} placeholder="0.00"/></label>
      <label>Recurrence<select value={form.recurrenceType} onChange={e=>setForm({...form,recurrenceType:e.target.value as BillFormState["recurrenceType"]})}><option value="monthly">Monthly</option><option value="one_time">One-Time</option></select></label>{form.recurrenceType==="monthly"?<label>Due Day<input type="number" min="1" max="31" value={form.dueDay} onChange={e=>setForm({...form,dueDay:e.target.value})}/></label>:<label>Due Date<input type="date" value={form.oneTimeDueDate} onChange={e=>setForm({...form,oneTimeDueDate:e.target.value})}/></label>}
      <label>Payment Type<select value={form.paymentType} onChange={e=>setForm({...form,paymentType:e.target.value as BillFormState["paymentType"]})}><option value="manual">Manual</option><option value="autopay">Autopay</option></select></label><label>Priority<select value={form.priority} onChange={e=>setForm({...form,priority:e.target.value as BillFormState["priority"]})}><option value="essential">Essential</option><option value="normal">Normal</option><option value="flexible">Flexible</option></select></label>
      <label>Responsible For<select value={form.assignedUserId} onChange={e=>setForm({...form,assignedUserId:e.target.value})}><option value="">Shared</option>{users.map(u=><option value={u.id} key={u.id}>{u.displayName}</option>)}</select></label><label>May Pay Up To<input type="number" min="0" max="365" value={form.earliestDays} onChange={e=>setForm({...form,earliestDays:e.target.value})}/><small>days before due date</small></label></div>
      <label className="check-line"><input type="checkbox" checked={form.canSplit} onChange={e=>setForm({...form,canSplit:e.target.checked})}/> Allow partial payments to the bill provider</label><label>Notes<textarea value={form.notes} onChange={e=>setForm({...form,notes:e.target.value})} rows={3}/></label></section></div>
      <div className="modal-footer"><button className="outline" onClick={()=>setForm(null)}>Cancel</button><button className="primary" disabled={busy} onClick={submitBill}>{busy?"Saving…":"Save Bill"}</button></div></div></div>}
    {payMode&&detail&&<div className="modal-backdrop"><div className="modal payment-modal"><div className="modal-header"><div><h2>{payMode==="full"?"Mark as Paid":"Partial Payment"}</h2><p>{detail.bill.name}</p></div><button onClick={()=>setPayMode(null)}>×</button></div><div className="editor-grid"><label>Amount<input value={paymentAmount} onChange={e=>setPaymentAmount(e.target.value)}/></label><label>Date<input type="date" value={paymentDate} onChange={e=>setPaymentDate(e.target.value)}/></label><label>Paid By<select value={paidBy} onChange={e=>setPaidBy(e.target.value)}>{users.map(u=><option value={u.id} key={u.id}>{u.displayName}</option>)}</select></label></div><div className="modal-footer"><button className="outline" onClick={()=>setPayMode(null)}>Cancel</button><button className="primary" disabled={busy} onClick={submitPayment}>{busy?"Saving…":payMode==="full"?"Mark Paid":"Record Partial Payment"}</button></div></div></div>}
  </div>;
}

const billFundingText=(g:PaymentGuidanceView["items"][number])=>g.fundingSources.length?g.fundingSources.map(f=>f.sourceType==="current_cash"?`Current cash ${formatMoney(f.amountCents)}`:`${f.ownerName??"Paycheck"} ${f.payDate?formatShortDate(f.payDate):""} ${formatMoney(f.amountCents)}`).join(" + "):"Not funded yet";
const iconFor=(id:string|null)=>id==="housing"?"⌂":id==="utilities"?"ϟ":id==="insurance"?"♢":id==="debt"?"▣":"●";
const ordinal=(n:number)=>`${n}${n%100>=11&&n%100<=13?"th":n%10===1?"st":n%10===2?"nd":n%10===3?"rd":"th"}`;
const formatShortDate=(s:string|null)=>s?new Date(`${s}T12:00:00`).toLocaleDateString("en-US",{month:"short",day:"numeric"}):"—";
const formatLongDate=(s:string|null)=>s?new Date(`${s}T12:00:00`).toLocaleDateString("en-US",{month:"long",day:"numeric",year:"numeric"}):"—";
