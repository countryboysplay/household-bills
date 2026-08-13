import { useEffect, useMemo, useState } from "react";
import { getCalendarData } from "../lib/backend";
import type { CalendarEvent } from "../lib/types";
import { formatMoney } from "../lib/money";

export function Calendar() {
  const now = new Date();
  const [month, setMonth] = useState(new Date(now.getFullYear(), now.getMonth(), 1));
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [selected, setSelected] = useState(todayText());
  const [error, setError] = useState("");

  const gridStart = useMemo(() => {
    const d = new Date(month.getFullYear(), month.getMonth(), 1);
    d.setDate(d.getDate() - d.getDay());
    return d;
  }, [month]);
  const gridEnd = useMemo(() => { const d = new Date(gridStart); d.setDate(d.getDate()+41); return d; }, [gridStart]);

  useEffect(() => {
    setError("");
    getCalendarData(dateText(gridStart), dateText(gridEnd)).then(data => setEvents(data.events)).catch(e=>setError(e instanceof Error?e.message:String(e)));
  }, [gridStart.getTime(), gridEnd.getTime()]);

  const days = Array.from({length:42},(_,i)=>{const d=new Date(gridStart);d.setDate(d.getDate()+i);return d});
  const byDate = useMemo(()=>{const m=new Map<string,CalendarEvent[]>();for(const e of events){const arr=m.get(e.date)??[];arr.push(e);m.set(e.date,arr)}return m},[events]);
  const selectedEvents=byDate.get(selected)??[];
  const income=events.filter(e=>e.eventType==="paycheck"&&e.date.slice(0,7)===dateText(month).slice(0,7)).reduce((s,e)=>s+e.amountCents,0);
  const bills=events.filter(e=>e.eventType==="bill"&&e.date.slice(0,7)===dateText(month).slice(0,7)).reduce((s,e)=>s+e.amountCents,0);

  return <div className="page calendar-page">
    <header className="page-header"><div><h1>Calendar</h1><p>See paychecks, bill due dates, and conservative pay-by dates in one place.</p></div><div className="header-actions"><button className="outline" onClick={()=>{const d=new Date();setMonth(new Date(d.getFullYear(),d.getMonth(),1));setSelected(todayText())}}>Today</button></div></header>
    {error&&<div className="page-error">{error}</div>}
    <div className="calendar-layout">
      <main className="card calendar-card">
        <div className="calendar-toolbar"><div><button onClick={()=>setMonth(new Date(month.getFullYear(),month.getMonth()-1,1))}>‹</button><button onClick={()=>setMonth(new Date(month.getFullYear(),month.getMonth()+1,1))}>›</button></div><h2>{month.toLocaleDateString("en-US",{month:"long",year:"numeric"})}</h2><div className="calendar-view-label">Month</div></div>
        <div className="calendar-weekdays">{["Sunday","Monday","Tuesday","Wednesday","Thursday","Friday","Saturday"].map(x=><span key={x}>{x}</span>)}</div>
        <div className="calendar-grid">{days.map(d=>{const key=dateText(d);const current=d.getMonth()===month.getMonth();const dayEvents=byDate.get(key)??[];return <button key={key} className={`calendar-day ${current?"":"outside"} ${key===selected?"selected":""} ${key===todayText()?"today":""}`} onClick={()=>setSelected(key)}><span className="calendar-day-number">{d.getDate()}</span><div className="calendar-events">{dayEvents.slice(0,4).map(e=><div key={`${e.eventType}-${e.id}`} className={`calendar-event ${e.eventType} ${e.status==="paid"?"paid":""}`}><strong>{e.eventType==="paycheck"?`Paycheck • ${e.subtitle}`:e.title}</strong><span>{formatMoney(e.amountCents)}</span></div>)}{dayEvents.length>4&&<small>+{dayEvents.length-4} more</small>}</div></button>})}</div>
      </main>
      <aside className="calendar-rail">
        <section className="card day-detail-card"><div className="card-title"><h2>Day Details</h2></div><h3>{prettyFullDate(selected)}</h3>{selectedEvents.length ? selectedEvents.map(e=><div className={`day-event-detail ${e.eventType}`} key={`${e.eventType}-${e.id}`}><div><strong>{e.eventType==="paycheck"?`${e.subtitle} Paycheck`:e.title}</strong><span>{e.status}</span></div><b>{formatMoney(e.amountCents)}</b>{e.eventType==="bill"&&e.dueDate&&<small>Actual due date {prettyDate(e.dueDate)}{e.payByDate&&e.payByDate!==e.dueDate?` • Latest safe date ${prettyDate(e.payByDate)}`:""}</small>}{e.eventType==="payment"&&e.dueDate&&<small>Recommended action • Bill due {prettyDate(e.dueDate)}</small>}</div>) : <p className="muted-text">Nothing is scheduled for this day.</p>}</section>
        <section className="card"><div className="card-title"><h2>Month Summary</h2><span>{month.toLocaleDateString("en-US",{month:"long"})}</span></div><dl className="month-summary"><dt>Scheduled income</dt><dd className="positive-money">{formatMoney(income)}</dd><dt>Bills due</dt><dd className="negative-money">-{formatMoney(bills).replace("-","")}</dd><dt>Difference</dt><dd className={income-bills<0?"negative-money":"positive-money"}>{formatMoney(income-bills)}</dd></dl></section>
        <section className="card calendar-legend"><h2>Legend</h2><div><i className="legend-paycheck"/> Paycheck</div><div><i className="legend-bill"/> Bill due</div><div><i className="legend-payment"/> Recommended payment</div><div><i className="legend-paid"/> Paid bill</div><p>When a weekend changes the recommended payment date, the actual due date stays on the calendar and the earlier pay-by date appears in the details.</p></section>
      </aside>
    </div>
  </div>
}

function dateText(d:Date){return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,"0")}-${String(d.getDate()).padStart(2,"0")}`}
function todayText(){return dateText(new Date())}
function prettyDate(v:string){return new Date(`${v}T12:00:00`).toLocaleDateString("en-US",{month:"short",day:"numeric"})}
function prettyFullDate(v:string){return new Date(`${v}T12:00:00`).toLocaleDateString("en-US",{weekday:"long",month:"long",day:"numeric",year:"numeric"})}
