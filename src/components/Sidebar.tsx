import type { PageKey } from "../lib/types";
import { Icon } from "./Icon";

const nav: { key: PageKey; label: string; icon: string }[] = [
  { key: "dashboard", label: "Dashboard", icon: "dashboard" },
  { key: "planner", label: "Paycheck Planner", icon: "planner" },
  { key: "bills", label: "Bills", icon: "bills" },
  { key: "calendar", label: "Calendar", icon: "calendar" },
  { key: "spending", label: "Spending", icon: "spending" },
  { key: "savings", label: "Savings & Debt", icon: "savings" },
  { key: "history", label: "History", icon: "history" },
  { key: "reports", label: "Reports", icon: "reports" },
];

export function Sidebar({ page, onPage, profileName, version }: { page: PageKey; onPage: (p: PageKey) => void; profileName: string; version: string }) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark">⌂</div>
        <div><strong>Household Bills</strong><span>v{version.replace("-browser-preview", "")}</span></div>
      </div>
      <nav className="primary-nav">
        {nav.map(item => (
          <button key={item.key} className={page === item.key ? "active" : ""} onClick={() => onPage(item.key)}>
            <Icon name={item.icon}/><span>{item.label}</span>
          </button>
        ))}
      </nav>
      <div className="nav-separator"/>
      <nav className="secondary-nav">
        <button className={page === "settings" ? "active" : ""} onClick={() => onPage("settings")}><Icon name="settings"/><span>Settings</span></button>
      </nav>
      <div className="sidebar-spacer"/>
      <div className="system-status">
        <div><i className="status-dot"/>All systems normal</div>
        <p>Database: <span>Local (SQLite)</span></p>
        <p>Backups: <span>Enabled</span></p>
        <p>Storage: <span>Local Only</span></p>
      </div>
      <div className="profile-card"><div className="avatar">{profileName.slice(0,1).toUpperCase()}</div><div><strong>{profileName}</strong><span>Household</span></div><span className="chevron">›</span></div>
    </aside>
  );
}
