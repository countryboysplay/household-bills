import type { PageKey } from "../lib/types";

const content: Record<PageKey, { title: string; description: string }> = {
  dashboard: { title: "Dashboard", description: "Household overview" },
  planner: { title: "Paycheck Planner", description: "Assign upcoming bills to the right paycheck and protect your cash buffer." },
  bills: { title: "Bills", description: "Recurring and one-time bills, payment status, estimates, and schedule assignments." },
  calendar: { title: "Calendar", description: "Paydays, due dates, recommended payment dates, and autopay drafts." },
  spending: { title: "Spending", description: "Manual transactions and fast balance reconciliation without a bank connection." },
  savings: { title: "Savings & Debt", description: "Savings goals, sinking funds, and debt payoff planning." },
  history: { title: "History", description: "Payments, paycheck changes, reconciliations, and important household activity." },
  reports: { title: "Reports", description: "Household cash-flow and spending reports." },
  settings: { title: "Settings", description: "Household, backup, and scheduling settings." },
};

export function Placeholder({ page }: { page: PageKey }) {
  const c = content[page];
  return <div className="page placeholder-page"><header className="page-header"><div><h1>{c.title}</h1><p>{c.description}</p></div></header><section className="card empty-state"><div className="empty-icon">⌁</div><h2>{c.title} foundation is wired</h2><p>This Phase 1 build includes the navigation and persistence foundation. The full approved screen is scheduled for its implementation phase.</p><button className="primary">Continue with Phase 1</button></section></div>;
}
