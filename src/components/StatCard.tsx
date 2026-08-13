import type { ReactNode } from "react";

export function StatCard({ icon, title, value, meta, tone = "blue" }: { icon: ReactNode; title: string; value: string; meta: string; tone?: "blue"|"green"|"orange"|"purple" }) {
  return <section className="stat-card"><div className={`stat-icon ${tone}`}>{icon}</div><div><span className="eyebrow">{title}</span><strong>{value}</strong><small>{meta}</small></div></section>;
}
