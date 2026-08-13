import type { ReactNode } from "react";

const paths: Record<string, ReactNode> = {
  dashboard: <><path d="M3 11 12 3l9 8"/><path d="M5 10v10h14V10"/><path d="M9 20v-6h6v6"/></>,
  planner: <><rect x="4" y="5" width="16" height="15" rx="2"/><path d="M8 3v4M16 3v4M4 9h16"/><path d="M8 13h3v3H8z"/></>,
  bills: <><path d="M6 3h12v18l-3-2-3 2-3-2-3 2z"/><path d="M9 8h6M9 12h6M9 16h4"/></>,
  calendar: <><rect x="3" y="5" width="18" height="16" rx="2"/><path d="M8 3v4M16 3v4M3 10h18"/></>,
  spending: <><path d="M4 7h16v11H4z"/><path d="M7 7V5h10v2M8 12h4"/><circle cx="17" cy="13" r="1"/></>,
  savings: <><path d="M5 8c0-2 3-4 7-4s7 2 7 4v7c0 3-3 5-7 5s-7-2-7-5z"/><path d="M9 10h6M12 8v4"/></>,
  history: <><path d="M3 12a9 9 0 1 0 3-6.7L3 8"/><path d="M3 3v5h5M12 7v6l4 2"/></>,
  reports: <><path d="M4 20V10M10 20V4M16 20v-7M22 20H2"/></>,
  settings: <><circle cx="12" cy="12" r="3"/><path d="M19 12a7 7 0 0 0-.1-1l2-1.6-2-3.4-2.5 1A7 7 0 0 0 15 6l-.4-2.7h-4L10 6a7 7 0 0 0-1.4 1L6 6 4 9.4 6.1 11a7 7 0 0 0 0 2L4 14.6 6 18l2.6-1a7 7 0 0 0 1.4 1l.5 2.7h4L15 18a7 7 0 0 0 1.4-1l2.6 1 2-3.4-2.1-1.6c.1-.3.1-.7.1-1z"/></>,
  bell: <><path d="M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9"/><path d="M10 21h4"/></>,
};

export function Icon({ name, size = 20 }: { name: string; size?: number }) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">{paths[name]}</svg>;
}
