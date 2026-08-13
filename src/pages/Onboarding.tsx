import { useState } from "react";
import { dollarsToCents } from "../lib/money";
import type { OnboardingPayload } from "../lib/types";

export function Onboarding({ onComplete }: { onComplete: (payload: OnboardingPayload) => Promise<void> }) {
  const [household, setHousehold] = useState("Household");
  const [buffer, setBuffer] = useState("500.00");
  const [balance, setBalance] = useState("0.00");
  const [account, setAccount] = useState("Checking");
  const [first, setFirst] = useState("Jonathan");
  const [second, setSecond] = useState("Tiffany");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  const submit = async () => {
    try {
      setSaving(true); setError("");
      await onComplete({ householdName: household.trim() || "Household", protectedBufferCents: dollarsToCents(buffer), primaryAccountName: account.trim() || "Checking", primaryAccountBalanceCents: dollarsToCents(balance), users: [first.trim(), second.trim()].filter(Boolean) });
    } catch (e) { setError(e instanceof Error ? e.message : String(e)); }
    finally { setSaving(false); }
  };

  return <div className="onboarding"><div className="onboarding-card"><div className="onboarding-logo">⌂</div><h1>Set up your household</h1><p>Household Bills keeps your financial plan local on this PC. Start with the basics. You can change all of this later.</p><div className="form-grid"><label>Household name<input value={household} onChange={e=>setHousehold(e.target.value)}/></label><label>Protected cash buffer<input value={buffer} onChange={e=>setBuffer(e.target.value)} inputMode="decimal"/></label><label>Primary bill account<input value={account} onChange={e=>setAccount(e.target.value)}/></label><label>Current account balance<input value={balance} onChange={e=>setBalance(e.target.value)} inputMode="decimal"/></label><label>First profile<input value={first} onChange={e=>setFirst(e.target.value)}/></label><label>Second profile<input value={second} onChange={e=>setSecond(e.target.value)}/></label></div>{error && <div className="form-error">{error}</div>}<button className="primary large" disabled={saving} onClick={submit}>{saving ? "Setting up…" : "Create Household"}</button><small>Your database and backups stay local on this PC unless you explicitly export a report.</small></div></div>;
}
