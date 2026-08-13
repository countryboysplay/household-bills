import { useEffect, useState } from "react";
import { getBootstrap, completeOnboarding, getDashboardSummary, checkForUpdate, installUpdate } from "./lib/backend";
import type { AppBootstrap, DashboardSummary, OnboardingPayload, PageKey, UpdateStatus } from "./lib/types";
import { Sidebar } from "./components/Sidebar";
import { Dashboard } from "./pages/Dashboard";
import { Spending } from "./pages/Spending";
import { Calendar } from "./pages/Calendar";
import { History } from "./pages/History";
import { Onboarding } from "./pages/Onboarding";
import { Placeholder } from "./pages/Placeholder";
import { SavingsDebt } from "./pages/SavingsDebt";
import { Settings } from "./pages/Settings";
import { Reports } from "./pages/Reports";
import { Bills } from "./pages/Bills";
import { Planner } from "./pages/Planner";

export default function App() {
  const [bootstrap, setBootstrap] = useState<AppBootstrap | null>(null);
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [page, setPage] = useState<PageKey>("dashboard");
  const [error, setError] = useState("");
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus | null>(null);
  const [installingUpdate, setInstallingUpdate] = useState(false);

  const load = async () => {
    try {
      const b = await getBootstrap();
      setBootstrap(b);
      if (b.onboardingComplete) setSummary(await getDashboardSummary());
    } catch (e) { setError(e instanceof Error ? e.message : String(e)); }
  };

  useEffect(() => { void load(); }, []);

  useEffect(() => {
    if (!bootstrap?.onboardingComplete) return;
    const timer = window.setTimeout(() => {
      void checkForUpdate().then(setUpdateStatus).catch(() => { /* Offline is fine. */ });
    }, 2500);
    return () => window.clearTimeout(timer);
  }, [bootstrap?.onboardingComplete]);

  const finishOnboarding = async (payload: OnboardingPayload) => {
    const b = await completeOnboarding(payload);
    setBootstrap(b);
    setSummary(await getDashboardSummary());
  };

  if (error) return <div className="fatal"><h1>Household Bills could not start</h1><pre>{error}</pre></div>;
  if (!bootstrap) return <div className="loading-screen"><div className="spinner"/><p>Opening Household Bills…</p></div>;
  if (!bootstrap.onboardingComplete) return <Onboarding onComplete={finishOnboarding}/>;

  const profile = bootstrap.users[0]?.displayName ?? "Household";
  const refreshSummary = async () => setSummary(await getDashboardSummary());
  const applyUpdate = async () => {
    if (!updateStatus?.available || installingUpdate) return;
    if (!window.confirm(`Install Household Bills ${updateStatus.version}? A local database backup will be created first, then the app will restart.`)) return;
    try {
      setInstallingUpdate(true);
      await installUpdate();
    } catch (e) {
      setInstallingUpdate(false);
      window.alert(e instanceof Error ? e.message : String(e));
    }
  };

  return <div className="app-shell"><Sidebar page={page} onPage={setPage} profileName={profile} version={bootstrap.appVersion}/><div className="content-shell">
    {updateStatus?.available && <div className="update-banner"><div><strong>Household Bills {updateStatus.version} is available</strong><span>Your financial data will be backed up before the signed update installs.</span></div><button className="primary" onClick={()=>void applyUpdate()} disabled={installingUpdate}>{installingUpdate ? "Installing…" : "Install Update"}</button><button className="update-dismiss" aria-label="Dismiss update notice" onClick={()=>setUpdateStatus(null)}>×</button></div>}
    {page === "dashboard" ? <Dashboard profileName={profile} onNavigate={setPage}/> :
     page === "planner" ? <Planner users={bootstrap.users} onChanged={refreshSummary}/> :
     page === "bills" ? <Bills users={bootstrap.users} onChanged={refreshSummary}/> :
     page === "spending" ? <Spending users={bootstrap.users} onChanged={refreshSummary}/> :
     page === "calendar" ? <Calendar/> :
     page === "history" ? <History/> :
     page === "savings" ? <SavingsDebt users={bootstrap.users} onChanged={refreshSummary}/> :
     page === "reports" ? <Reports/> :
     page === "settings" ? <Settings onChanged={load}/> :
     <Placeholder page={page}/>}</div></div>;
}
