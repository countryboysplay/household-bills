import { FormEvent, useEffect, useMemo, useState } from "react";
import { addTransaction, getSpendingView, reconcileAccount } from "../lib/backend";
import type { SpendingView, UserProfile } from "../lib/types";
import { dollarsToCents, formatMoney } from "../lib/money";

export function Spending({ users, onChanged }: { users: UserProfile[]; onChanged: () => Promise<void> | void }) {
  const [data, setData] = useState<SpendingView | null>(null);
  const [error, setError] = useState("");
  const [search, setSearch] = useState("");
  const [showAdd, setShowAdd] = useState(false);
  const [showReconcile, setShowReconcile] = useState(false);
  const [activeAccountId, setActiveAccountId] = useState("");

  const load = async () => {
    try { setError(""); const next = await getSpendingView(); setData(next); if (!activeAccountId && next.accounts[0]) setActiveAccountId(next.accounts[0].id); }
    catch (e) { setError(e instanceof Error ? e.message : String(e)); }
  };
  useEffect(() => { void load(); }, []);

  const visible = useMemo(() => {
    if (!data) return [];
    const q = search.trim().toLowerCase();
    return data.transactions.filter(t => !q || `${t.description} ${t.categoryName} ${t.accountName}`.toLowerCase().includes(q));
  }, [data, search]);

  if (!data) return <div className="page"><header className="page-header"><div><h1>Spending & Balance Reconciliation</h1><p>Keep your manually tracked balances accurate.</p></div></header>{error ? <div className="page-error">{error}</div> : <div className="loading-inline">Loading spending…</div>}</div>;

  const primary = data.accounts.find(a => a.isPrimary) ?? data.accounts[0];
  const spendTotal = data.categorySpending.reduce((s,x)=>s+x.amountCents,0);
  return <div className="page spending-page">
    <header className="page-header"><div><h1>Spending & Balance Reconciliation</h1><p>Track important purchases and keep the app aligned with your real balance without connecting a bank.</p></div><div className="header-actions"><button className="outline" onClick={()=>setShowAdd(true)}>＋ Add Transaction</button><button className="primary" onClick={()=>{setActiveAccountId(primary?.id ?? "");setShowReconcile(true)}}>↻ Reconcile Balance</button></div></header>
    {error && <div className="page-error">{error}</div>}

    <div className="account-card-grid">
      {data.accounts.map(a => <button key={a.id} className={`account-card ${a.id===activeAccountId?"selected":""}`} onClick={()=>setActiveAccountId(a.id)}><span>{a.isPrimary?"Primary bill account":friendly(a.accountType)}</span><strong className={a.balanceCents<0?"negative-money":""}>{formatMoney(a.balanceCents)}</strong><b>{a.name}</b><small>{a.lastReconciledAt?`Last reconciled ${formatDateTime(a.lastReconciledAt)}`:"Not reconciled yet"}</small></button>)}
    </div>

    <div className="spending-layout">
      <main>
        <div className="spending-summary-row"><div><span>This Month Income</span><strong className="positive-money">{formatMoney(data.monthIncomeCents)}</strong></div><div><span>This Month Spending</span><strong className="negative-money">-{formatMoney(data.monthSpendingCents).replace("-","")}</strong></div><div><span>Net Cash Flow</span><strong className={data.monthNetCents<0?"negative-money":"positive-money"}>{formatMoney(data.monthNetCents)}</strong></div></div>
        <section className="card transaction-card">
          <div className="transaction-toolbar"><div><h2>Transactions</h2><p>Manual entries, bill payments, paycheck deposits, and reconciliation adjustments.</p></div><input value={search} onChange={e=>setSearch(e.target.value)} placeholder="Search transactions…"/></div>
          <div className="transaction-table-head"><span>Date</span><span>Description</span><span>Category</span><span>Account</span><span>Amount</span><span>Source</span></div>
          {visible.length ? visible.map(t => <div className="transaction-table-row" key={t.id}><span>{formatShortDate(t.transactionDate)}</span><span><strong>{t.description}</strong>{t.note && <small>{t.note}</small>}</span><span>{t.categoryName}</span><span>{t.accountName}</span><span className={t.amountCents<0?"negative-money":"positive-money"}>{formatMoney(t.amountCents)}</span><span><i className="source-chip">{friendly(t.source)}</i></span></div>) : <div className="empty-inline">No transactions match the current search.</div>}
        </section>
      </main>

      <aside className="spending-rail">
        <section className="card reconcile-card"><div className="card-title"><h2>Balance Reconciliation</h2></div>{primary ? <><span className="muted-label">{primary.name}</span><strong className="reconcile-balance">{formatMoney(primary.balanceCents)}</strong><p>This is the balance Household Bills currently believes is in the account. Compare it with your actual bank balance whenever the numbers drift.</p><button className="primary" onClick={()=>{setActiveAccountId(primary.id);setShowReconcile(true)}}>Reconcile Now</button></> : <p>No account configured.</p>}</section>
        <section className="card"><div className="card-title"><h2>Spending Summary</h2><span>This month</span></div>{data.categorySpending.length ? <div className="category-spend-list">{data.categorySpending.slice(0,7).map(x=><div key={x.categoryId} className="category-spend-row"><span>{x.categoryName}</span><div><i style={{width:`${spendTotal?Math.max(5,(x.amountCents/spendTotal)*100):0}%`}}/></div><b>{formatMoney(x.amountCents)}</b><small>{spendTotal?Math.round((x.amountCents/spendTotal)*100):0}%</small></div>)}</div> : <p className="empty-inline">No spending yet this month.</p>}</section>
        <section className="card reconciliation-explainer"><h2>How reconciliation works</h2><p>Enter the real account balance. Household Bills records the difference as a reconciliation adjustment and uses the real balance from that point forward. You do not have to enter every small debit-card purchase.</p></section>
      </aside>
    </div>

    {showAdd && <AddTransactionModal data={data} users={users} onClose={()=>setShowAdd(false)} onSaved={async()=>{setShowAdd(false);await load();await onChanged()}}/>}
    {showReconcile && <ReconcileModal data={data} users={users} accountId={activeAccountId} onClose={()=>setShowReconcile(false)} onSaved={async()=>{setShowReconcile(false);await load();await onChanged()}}/>}
  </div>;
}

function AddTransactionModal({data,users,onClose,onSaved}:{data:SpendingView;users:UserProfile[];onClose:()=>void;onSaved:()=>void|Promise<void>}){
  const [direction,setDirection]=useState<"expense"|"income">("expense"); const [accountId,setAccountId]=useState(data.accounts[0]?.id??""); const [date,setDate]=useState(todayText()); const [description,setDescription]=useState(""); const [categoryId,setCategoryId]=useState("other"); const [amount,setAmount]=useState(""); const [userId,setUserId]=useState(users[0]?.id??""); const [note,setNote]=useState(""); const [error,setError]=useState(""); const [saving,setSaving]=useState(false);
  const submit=async(e:FormEvent)=>{e.preventDefault();try{setSaving(true);setError("");await addTransaction({accountId,transactionDate:date,description,categoryId,amountCents:dollarsToCents(amount),direction,userId:userId||null,note:note||null});await onSaved()}catch(err){setError(err instanceof Error?err.message:String(err));setSaving(false)}};
  return <div className="modal-backdrop"><form className="modal payment-modal" onSubmit={submit}><div className="modal-header"><div><h2>Add Transaction</h2><p>Record an important household purchase or income item.</p></div><button type="button" onClick={onClose}>×</button></div>{error&&<div className="form-error">{error}</div>}<div className="editor-grid"><label>Type<select value={direction} onChange={e=>setDirection(e.target.value as "expense"|"income")}><option value="expense">Expense</option><option value="income">Income</option></select></label><label>Account<select value={accountId} onChange={e=>setAccountId(e.target.value)}>{data.accounts.map(a=><option key={a.id} value={a.id}>{a.name}</option>)}</select></label><label>Date<input type="date" value={date} onChange={e=>setDate(e.target.value)} required/></label><label>Amount<input value={amount} onChange={e=>setAmount(e.target.value)} placeholder="$0.00" required/></label><label>Description<input value={description} onChange={e=>setDescription(e.target.value)} placeholder="Kroger, gas, school supplies…" required/></label><label>Category<select value={categoryId} onChange={e=>setCategoryId(e.target.value)}>{data.categories.filter(c=>direction==="income"?c.kind==="income":c.kind!=="income").map(c=><option key={c.id} value={c.id}>{c.name}</option>)}</select></label><label>Entered By<select value={userId} onChange={e=>setUserId(e.target.value)}>{users.map(u=><option key={u.id} value={u.id}>{u.displayName}</option>)}</select></label><label>Note<input value={note} onChange={e=>setNote(e.target.value)} placeholder="Optional"/></label></div><div className="modal-footer"><button type="button" className="outline" onClick={onClose}>Cancel</button><button className="primary" disabled={saving}>{saving?"Saving…":"Save Transaction"}</button></div></form></div>
}

function ReconcileModal({data,users,accountId,onClose,onSaved}:{data:SpendingView;users:UserProfile[];accountId:string;onClose:()=>void;onSaved:()=>void|Promise<void>}){
  const account=data.accounts.find(a=>a.id===accountId)??data.accounts[0]; const [actual,setActual]=useState(account?String((account.balanceCents/100).toFixed(2)):""); const [userId,setUserId]=useState(users[0]?.id??""); const [note,setNote]=useState(""); const [error,setError]=useState(""); const [saving,setSaving]=useState(false); const actualCents=(()=>{try{return dollarsToCents(actual)}catch{return account?.balanceCents??0}})(); const difference=actualCents-(account?.balanceCents??0);
  const submit=async(e:FormEvent)=>{e.preventDefault();if(!account)return;try{setSaving(true);setError("");await reconcileAccount({accountId:account.id,actualBalanceCents:dollarsToCents(actual),userId:userId||null,note:note||null});await onSaved()}catch(err){setError(err instanceof Error?err.message:String(err));setSaving(false)}};
  return <div className="modal-backdrop"><form className="modal payment-modal" onSubmit={submit}><div className="modal-header"><div><h2>Reconcile {account?.name??"Account"}</h2><p>Replace the app’s tracked balance with the real balance you see at your bank.</p></div><button type="button" onClick={onClose}>×</button></div>{error&&<div className="form-error">{error}</div>}<div className="reconcile-modal-body"><div className="reconcile-comparison"><div><span>App balance</span><strong>{formatMoney(account?.balanceCents??0)}</strong></div><div><span>Actual balance</span><strong>{formatMoney(actualCents)}</strong></div><div><span>Difference</span><strong className={difference<0?"negative-money":difference>0?"positive-money":""}>{formatMoney(difference)}</strong></div></div><div className="editor-grid"><label>Actual Balance<input value={actual} onChange={e=>setActual(e.target.value)} required/></label><label>Reconciled By<select value={userId} onChange={e=>setUserId(e.target.value)}>{users.map(u=><option key={u.id} value={u.id}>{u.displayName}</option>)}</select></label><label className="full-span">Note<input value={note} onChange={e=>setNote(e.target.value)} placeholder="Optional note about the difference"/></label></div>{difference<0&&<p className="reconcile-note">The {formatMoney(Math.abs(difference))} difference will be recorded as untracked spending so the forecast stays accurate.</p>}{difference>0&&<p className="reconcile-note">The {formatMoney(difference)} increase will be recorded as a balance reconciliation adjustment.</p>}</div><div className="modal-footer"><button type="button" className="outline" onClick={onClose}>Cancel</button><button className="primary" disabled={saving}>{saving?"Reconciling…":"Reconcile Balance"}</button></div></form></div>
}

function friendly(v:string){return v.replaceAll("_"," ").replace(/\b\w/g,c=>c.toUpperCase())}
function formatShortDate(v:string){return new Date(`${v}T12:00:00`).toLocaleDateString("en-US",{month:"short",day:"numeric"})}
function formatDateTime(v:string){const d=new Date(v.includes("T")?v:v.replace(" ","T"));return Number.isNaN(d.getTime())?v:d.toLocaleDateString("en-US",{month:"short",day:"numeric",year:"numeric"})}
function todayText(){const d=new Date();return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,"0")}-${String(d.getDate()).padStart(2,"0")}`}
