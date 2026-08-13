import { invoke } from "@tauri-apps/api/core";
import type {
  AppBootstrap,
  DashboardSummary,
  OnboardingPayload,
  BillDetail,
  BillListItem,
  MarkPaidPayload,
  PaycheckItem,
  PlannerView,
  SaveBillPayload,
  SavePaycheckPayload,
  DeletePaycheckPayload,
  PaycheckScheduleItem,
  SavePaycheckSchedulePayload,
} from "./types";

const isTauri = () => "__TAURI_INTERNALS__" in window;

const browserBootstrap: AppBootstrap = {
  appVersion: "1.0.1-browser-preview",
  onboardingComplete: true,
  users: [
    { id: "preview-jonathan", displayName: "Jonathan" },
    { id: "preview-tiffany", displayName: "Tiffany" },
  ],
  accounts: [
    {
      id: "preview-checking",
      name: "Checking",
      accountType: "checking",
      bookBalanceCents: 834718,
      isPrimaryBillAccount: true,
    },
  ],
  settings: {
    householdName: "Household",
    protectedBufferCents: 50000,
    defaultPlanningHorizonDays: 90,
    aiEnabled: false,
  },
  databasePath: "Browser preview only",
  backupDirectory: "Browser preview only",
};

const browserDashboard: DashboardSummary = {
  currentCashCents: 834718,
  safeToSpendCents: 94996,
  reservedBillsCents: 173016,
  protectedBufferCents: 50000,
  upcomingBillCount: 5,
  nextPaycheckDate: "2026-08-14",
  nextPaycheckOwner: "Jonathan",
  nextPaycheckAmountCents: 268012,
};

export async function getBootstrap(): Promise<AppBootstrap> {
  if (!isTauri()) return browserBootstrap;
  return invoke<AppBootstrap>("get_app_bootstrap");
}

export async function completeOnboarding(payload: OnboardingPayload): Promise<AppBootstrap> {
  if (!isTauri()) return browserBootstrap;
  return invoke<AppBootstrap>("complete_onboarding", { payload });
}

export async function getDashboardSummary(): Promise<DashboardSummary> {
  if (!isTauri()) return browserDashboard;
  return invoke<DashboardSummary>("get_dashboard_summary");
}

export async function createBackup(): Promise<string> {
  if (!isTauri()) return "Browser preview: backup not created";
  return invoke<string>("create_backup");
}


const previewBills: BillListItem[] = [
  { id:"b-mortgage",name:"Mortgage",categoryId:"housing",categoryName:"Housing",amountType:"fixed",amountCents:110000,dueDay:15,recurrenceType:"monthly",paymentType:"manual",priority:"essential",canSplit:false,assignedUserId:null,assignedUserName:null,nextOccurrenceId:"o-mortgage",nextDueDate:"2026-08-15",nextPayByDate:"2026-08-14",nextStatus:"scheduled",assignedPaycheckDate:"2026-08-14",assignedPaycheckOwner:"Jonathan" },
  { id:"b-electric",name:"Electric",categoryId:"utilities",categoryName:"Utilities",amountType:"variable",amountCents:18422,dueDay:18,recurrenceType:"monthly",paymentType:"manual",priority:"essential",canSplit:true,assignedUserId:null,assignedUserName:null,nextOccurrenceId:"o-electric",nextDueDate:"2026-08-18",nextPayByDate:"2026-08-18",nextStatus:"scheduled",assignedPaycheckDate:"2026-08-14",assignedPaycheckOwner:"Jonathan" },
  { id:"b-internet",name:"Internet",categoryId:"utilities",categoryName:"Utilities",amountType:"fixed",amountCents:9000,dueDay:22,recurrenceType:"monthly",paymentType:"manual",priority:"normal",canSplit:false,assignedUserId:null,assignedUserName:null,nextOccurrenceId:"o-internet",nextDueDate:"2026-08-22",nextPayByDate:"2026-08-21",nextStatus:"scheduled",assignedPaycheckDate:"2026-08-14",assignedPaycheckOwner:"Jonathan" },
];

const previewPlanner: PlannerView = {
  planningDate:"2026-08-12",protectedBufferCents:50000,currentCashCents:834718,currentCashSafeCents:94996,unresolvedBillCount:0,warnings:[],
  paychecks:[
    {id:"p1",ownerName:"Jonathan",payDate:"2026-08-14",amountCents:268012,status:"projected:healthy",billsTotalCents:137422,commitmentsTotalCents:0,safeRemainingCents:130590,commitments:[],bills:[
      {occurrenceId:"o-mortgage",name:"Mortgage",amountCents:110000,dueDate:"2026-08-15",paymentType:"manual",priority:"essential",status:"scheduled",paymentDate:"2026-08-14",reasonCode:"latest_eligible_paycheck"},
      {occurrenceId:"o-electric",name:"Electric",amountCents:18422,dueDate:"2026-08-18",paymentType:"manual",priority:"essential",status:"scheduled",paymentDate:"2026-08-14",reasonCode:"latest_eligible_paycheck"},
      {occurrenceId:"o-internet",name:"Internet",amountCents:9000,dueDate:"2026-08-22",paymentType:"manual",priority:"normal",status:"scheduled",paymentDate:"2026-08-14",reasonCode:"latest_eligible_paycheck"},
    ]},
    {id:"p2",ownerName:"Tiffany",payDate:"2026-08-21",amountCents:217546,status:"updated:tight",billsTotalCents:171400,commitmentsTotalCents:0,safeRemainingCents:46146,commitments:[],bills:[]},
  ]
};

export async function listBills(): Promise<BillListItem[]> {
  if (!isTauri()) return previewBills;
  return invoke<BillListItem[]>("list_bills");
}
export async function saveBill(payload: SaveBillPayload): Promise<string> {
  if (!isTauri()) return payload.id ?? "preview-bill";
  return invoke<string>("save_bill", { payload });
}
export async function getBillDetail(id: string): Promise<BillDetail> {
  if (!isTauri()) return { bill: previewBills.find(b=>b.id===id) ?? previewBills[0], notes:null, paymentHistory:[] };
  return invoke<BillDetail>("get_bill_detail", { id });
}
export async function markBillPaid(payload: MarkPaidPayload): Promise<void> {
  if (!isTauri()) return;
  return invoke<void>("mark_bill_paid", { payload });
}
export async function archiveBill(id: string): Promise<void> {
  if (!isTauri()) return;
  return invoke<void>("archive_bill", { payload: { id } });
}
export async function listPaychecks(): Promise<PaycheckItem[]> {
  if (!isTauri()) return [];
  return invoke<PaycheckItem[]>("list_paychecks");
}
export async function savePaycheck(payload: SavePaycheckPayload): Promise<string> {
  if (!isTauri()) return payload.id ?? "preview-paycheck";
  return invoke<string>("save_paycheck", { payload });
}
export async function deletePaycheck(payload: DeletePaycheckPayload): Promise<void> {
  if (!isTauri()) return;
  return invoke<void>("delete_paycheck", { payload });
}
export async function listPaycheckSchedules(): Promise<PaycheckScheduleItem[]> {
  if (!isTauri()) return [];
  return invoke<PaycheckScheduleItem[]>("list_paycheck_schedules");
}
export async function savePaycheckSchedule(payload: SavePaycheckSchedulePayload): Promise<string> {
  if (!isTauri()) return payload.id ?? "preview-paycheck-schedule";
  return invoke<string>("save_paycheck_schedule", { payload });
}
export async function getPlanner(): Promise<PlannerView> {
  if (!isTauri()) return previewPlanner;
  return invoke<PlannerView>("get_planner");
}
export async function runScheduler(): Promise<PlannerView> {
  if (!isTauri()) return previewPlanner;
  return invoke<PlannerView>("run_scheduler");
}

export async function getDashboardData(): Promise<import("./types").DashboardData> {
  if (!isTauri()) throw new Error("Dashboard data requires the desktop app.");
  return invoke<import("./types").DashboardData>("get_dashboard_data");
}

export async function getSpendingView(): Promise<import("./types").SpendingView> {
  if (!isTauri()) throw new Error("Spending data requires the desktop app.");
  return invoke<import("./types").SpendingView>("get_spending_view");
}

export async function addTransaction(payload: import("./types").AddTransactionPayload): Promise<void> {
  if (!isTauri()) return;
  return invoke<void>("add_transaction", { payload });
}

export async function reconcileAccount(payload: import("./types").ReconcilePayload): Promise<import("./types").ReconcileResult> {
  if (!isTauri()) throw new Error("Balance reconciliation requires the desktop app.");
  return invoke<import("./types").ReconcileResult>("reconcile_account", { payload });
}

export async function getCalendarData(startDate: string, endDate: string): Promise<import("./types").CalendarData> {
  if (!isTauri()) throw new Error("Calendar data requires the desktop app.");
  return invoke<import("./types").CalendarData>("get_calendar_data", { startDate, endDate });
}

export async function getHistoryData(): Promise<import("./types").HistoryData> {
  if (!isTauri()) throw new Error("History data requires the desktop app.");
  return invoke<import("./types").HistoryData>("get_history_data");
}


export async function getPaymentGuidance(): Promise<import("./types").PaymentGuidanceView> {
  if (!isTauri()) throw new Error("Payment guidance requires the desktop app.");
  return invoke<import("./types").PaymentGuidanceView>("get_payment_guidance");
}

export async function getSavingsDebtView(): Promise<import("./types").SavingsDebtView> {
  if (!isTauri()) throw new Error("Savings and debt data requires the desktop app.");
  return invoke<import("./types").SavingsDebtView>("get_savings_debt_view");
}
export async function saveSavingsGoal(payload: import("./types").SaveSavingsGoalPayload): Promise<string> { return invoke<string>("save_savings_goal", { payload }); }
export async function saveDebt(payload: import("./types").SaveDebtPayload): Promise<string> { return invoke<string>("save_debt", { payload }); }
export async function recordSavingsContribution(payload: import("./types").MoneyActionPayload): Promise<void> { return invoke<void>("record_savings_contribution", { payload }); }
export async function recordDebtPayment(payload: import("./types").MoneyActionPayload): Promise<void> { return invoke<void>("record_debt_payment", { payload }); }
export async function archiveSavingsGoal(id:string): Promise<void> { return invoke<void>("archive_savings_goal", { payload:{id} }); }
export async function archiveDebt(id:string): Promise<void> { return invoke<void>("archive_debt", { payload:{id} }); }

export async function getSettingsView(): Promise<import("./types").SettingsView> { return invoke<import("./types").SettingsView>("get_settings_view"); }
export async function saveSettings(payload:import("./types").SaveSettingsPayload): Promise<void> { return invoke<void>("save_settings",{payload}); }
export async function openAppFolder(target:"data"|"backups"|"exports"): Promise<void> { return invoke<void>("open_app_folder",{target}); }
export async function listBackups(): Promise<import("./types").BackupItem[]> { return invoke<import("./types").BackupItem[]>("list_backups"); }
export async function requestRestoreBackup(fileName:string): Promise<string> { return invoke<string>("request_restore_backup",{fileName}); }

export async function getReportsData(startDate:string,endDate:string): Promise<import("./types").ReportsView> { return invoke<import("./types").ReportsView>("get_reports_data",{startDate,endDate}); }
export async function exportReportCsv(startDate:string,endDate:string): Promise<string> { return invoke<string>("export_report_csv",{startDate,endDate}); }

export async function checkForUpdate(): Promise<import("./types").UpdateStatus> {
  if (!isTauri()) return { available:false, currentVersion:"browser-preview", version:null };
  return invoke<import("./types").UpdateStatus>("check_for_update");
}

export async function installUpdate(): Promise<void> {
  if (!isTauri()) return;
  return invoke<void>("install_update");
}
