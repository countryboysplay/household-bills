export type PageKey =
  | "dashboard"
  | "planner"
  | "bills"
  | "calendar"
  | "spending"
  | "savings"
  | "history"
  | "reports"
  | "settings";

export interface UserProfile {
  id: string;
  displayName: string;
}

export interface AccountSummary {
  id: string;
  name: string;
  accountType: string;
  bookBalanceCents: number;
  isPrimaryBillAccount: boolean;
}

export interface HouseholdSettings {
  householdName: string;
  protectedBufferCents: number;
  defaultPlanningHorizonDays: number;
  aiEnabled: boolean;
}

export interface AppBootstrap {
  appVersion: string;
  onboardingComplete: boolean;
  users: UserProfile[];
  accounts: AccountSummary[];
  settings: HouseholdSettings | null;
  databasePath: string;
  backupDirectory: string;
}

export interface DashboardSummary {
  currentCashCents: number;
  safeToSpendCents: number;
  reservedBillsCents: number;
  protectedBufferCents: number;
  upcomingBillCount: number;
  nextPaycheckDate: string | null;
  nextPaycheckOwner: string | null;
  nextPaycheckAmountCents: number | null;
}

export interface OnboardingPayload {
  householdName: string;
  protectedBufferCents: number;
  primaryAccountName: string;
  primaryAccountBalanceCents: number;
  users: string[];
}

export interface BillListItem {
  id: string;
  name: string;
  categoryId: string | null;
  categoryName: string;
  amountType: "fixed" | "variable";
  amountCents: number;
  dueDay: number | null;
  recurrenceType: "monthly" | "one_time";
  paymentType: "manual" | "autopay";
  priority: "essential" | "normal" | "flexible";
  canSplit: boolean;
  assignedUserId: string | null;
  assignedUserName: string | null;
  nextOccurrenceId: string | null;
  nextDueDate: string | null;
  nextPayByDate: string | null;
  nextStatus: string | null;
  assignedPaycheckDate: string | null;
  assignedPaycheckOwner: string | null;
}

export interface SaveBillPayload {
  id?: string | null;
  name: string;
  categoryId?: string | null;
  amountType: "fixed" | "variable";
  amountCents: number;
  dueDay?: number | null;
  recurrenceType: "monthly" | "one_time";
  oneTimeDueDate?: string | null;
  paymentType: "manual" | "autopay";
  priority: "essential" | "normal" | "flexible";
  canSplit: boolean;
  assignedUserId?: string | null;
  payEarliestDaysBefore?: number | null;
  notes?: string | null;
}

export interface PaymentHistoryItem {
  id: string;
  paidDate: string;
  amountCents: number;
  paidBy: string;
  note: string | null;
}

export interface BillDetail {
  bill: BillListItem;
  notes: string | null;
  paymentHistory: PaymentHistoryItem[];
}

export interface MarkPaidPayload {
  occurrenceId: string;
  amountCents: number;
  paidDate: string;
  paidByUserId: string;
  paymentMethod?: string | null;
  note?: string | null;
  isPartial: boolean;
}

export interface PaycheckItem {
  id: string;
  userId: string;
  ownerName: string;
  payDate: string;
  projectedAmountCents: number;
  expectedAmountCents: number | null;
  actualAmountCents: number | null;
  effectiveAmountCents: number;
  status: "projected" | "updated" | "received" | "skipped";
}

export interface SavePaycheckPayload {
  id?: string | null;
  userId: string;
  payDate: string;
  projectedAmountCents: number;
  expectedAmountCents?: number | null;
  actualAmountCents?: number | null;
  status: "projected" | "updated" | "received" | "skipped";
  note?: string | null;
}

export interface DeletePaycheckPayload {
  id: string;
}

export interface PaycheckScheduleItem {
  id: string;
  userId: string;
  ownerName: string;
  frequency: "weekly" | "biweekly" | "semimonthly" | "monthly";
  defaultProjectedAmountCents: number;
  anchorDate: string | null;
  firstDay: number | null;
  secondDay: number | null;
  dayOfMonth: number | null;
  weekendHolidayRule: "exact" | "prior_business_day" | "next_business_day";
  nextPayDate: string | null;
}

export interface SavePaycheckSchedulePayload {
  id?: string | null;
  userId: string;
  frequency: "weekly" | "biweekly" | "semimonthly" | "monthly";
  defaultProjectedAmountCents: number;
  anchorDate?: string | null;
  firstDay?: number | null;
  secondDay?: number | null;
  dayOfMonth?: number | null;
  weekendHolidayRule?: "exact" | "prior_business_day" | "next_business_day" | null;
}

export interface PlannerBill {
  occurrenceId: string;
  name: string;
  amountCents: number;
  dueDate: string;
  paymentType: string;
  priority: string;
  status: string;
  paymentDate: string;
  reasonCode: string;
}

export interface PlannerCommitment { id:string; name:string; kind:string; requestedAmountCents:number; effectiveAmountCents:number; reducedByCents:number }
export interface PlannerPaycheck {
  id: string;
  ownerName: string;
  payDate: string;
  amountCents: number;
  status: string;
  billsTotalCents: number;
  commitmentsTotalCents: number;
  safeRemainingCents: number;
  bills: PlannerBill[];
  commitments: PlannerCommitment[];
}

export interface PlannerWarning {
  code: string;
  message: string;
  date: string | null;
  amountCents: number | null;
}

export interface PlannerView {
  planningDate: string;
  protectedBufferCents: number;
  currentCashCents: number;
  currentCashSafeCents: number;
  paychecks: PlannerPaycheck[];
  warnings: PlannerWarning[];
  unresolvedBillCount: number;
}

export interface DashboardBill {
  id: string;
  name: string;
  dueDate: string;
  payByDate: string;
  amountCents: number;
  status: string;
  paymentType: string;
}

export interface DashboardPaycheck {
  id: string;
  ownerName: string;
  payDate: string;
  amountCents: number;
  billsCents: number;
  safeCents: number;
  status: string;
}

export interface CategorySpend {
  categoryId: string;
  categoryName: string;
  amountCents: number;
}

export interface ActivityItem {
  id: string;
  occurredAt: string;
  userName: string | null;
  eventType: string;
  entityType: string | null;
  entityId: string | null;
  summary: string;
}

export interface DashboardAlert {
  code: string;
  title: string;
  message: string;
  tone: string;
}

export interface DashboardData {
  currentCashCents: number;
  reservedBillsCents: number;
  safeToSpendCents: number;
  protectedBufferCents: number;
  nextPaycheck: DashboardPaycheck | null;
  upcomingBills: DashboardBill[];
  paychecks: DashboardPaycheck[];
  monthIncomeCents: number;
  monthBillPaymentsCents: number;
  monthEverydaySpendingCents: number;
  monthNetCents: number;
  categorySpending: CategorySpend[];
  recentActivity: ActivityItem[];
  alerts: DashboardAlert[];
  savingsTotalCents: number;
  debtTotalCents: number;
}

export interface AccountView {
  id: string;
  name: string;
  accountType: string;
  balanceCents: number;
  isPrimary: boolean;
  lastReconciledAt: string | null;
}

export interface CategoryItem {
  id: string;
  name: string;
  kind: string;
}

export interface TransactionItem {
  id: string;
  accountId: string;
  accountName: string;
  transactionDate: string;
  description: string;
  categoryId: string | null;
  categoryName: string;
  amountCents: number;
  transactionType: string;
  status: string;
  source: string;
  note: string | null;
}

export interface SpendingView {
  accounts: AccountView[];
  categories: CategoryItem[];
  transactions: TransactionItem[];
  monthIncomeCents: number;
  monthSpendingCents: number;
  monthNetCents: number;
  categorySpending: CategorySpend[];
}

export interface AddTransactionPayload {
  accountId: string;
  transactionDate: string;
  description: string;
  categoryId?: string | null;
  amountCents: number;
  direction: "expense" | "income";
  userId?: string | null;
  note?: string | null;
}

export interface ReconcilePayload {
  accountId: string;
  actualBalanceCents: number;
  userId?: string | null;
  note?: string | null;
}

export interface ReconcileResult {
  appBalanceBeforeCents: number;
  actualBalanceCents: number;
  differenceCents: number;
}

export interface CalendarEvent {
  id: string;
  date: string;
  eventType: "paycheck" | "bill" | "payment";
  title: string;
  subtitle: string;
  amountCents: number;
  status: string;
  dueDate: string | null;
  payByDate: string | null;
}

export interface CalendarData {
  events: CalendarEvent[];
}

export interface PaymentHistoryRow {
  id: string;
  paidDate: string;
  billName: string;
  amountCents: number;
  paidBy: string;
  note: string | null;
}

export interface ReconciliationHistory {
  id: string;
  performedAt: string;
  accountName: string;
  beforeCents: number;
  actualCents: number;
  differenceCents: number;
  performedBy: string | null;
}

export interface HistoryData {
  activity: ActivityItem[];
  payments: PaymentHistoryRow[];
  reconciliations: ReconciliationHistory[];
}

export interface SavingsGoalItem {
  id: string;
  name: string;
  goalType: "savings" | "emergency" | "sinking_fund";
  targetAmountCents: number;
  targetDate: string | null;
  currentAmountCents: number;
  plannedContributionCents: number;
  contributionFrequency: "per_paycheck" | "monthly" | "manual";
  notes: string | null;
}
export interface DebtItem {
  id: string;
  name: string;
  balanceCents: number;
  aprBasisPoints: number;
  minimumPaymentCents: number;
  plannedExtraPaymentCents: number;
  dueDay: number | null;
}
export interface GoalActivityItem { id:string; date:string; itemName:string; activityType:"savings"|"debt"; amountCents:number; personName:string|null }
export interface DebtStrategy { strategy:"snowball"|"avalanche"; payoffMonths:number; totalInterestCents:number }
export interface SavingsDebtView {
  goals:SavingsGoalItem[]; debts:DebtItem[]; recentActivity:GoalActivityItem[];
  totalSavedCents:number; totalDebtCents:number; plannedSavingsPerPaycheckCents:number; plannedExtraDebtMonthlyCents:number; strategies:DebtStrategy[];
}
export interface SaveSavingsGoalPayload { id?:string|null; name:string; goalType:SavingsGoalItem["goalType"]; targetAmountCents:number; targetDate?:string|null; currentAmountCents:number; plannedContributionCents:number; contributionFrequency:SavingsGoalItem["contributionFrequency"]; notes?:string|null }
export interface SaveDebtPayload { id?:string|null; name:string; balanceCents:number; aprBasisPoints:number; minimumPaymentCents:number; plannedExtraPaymentCents:number; dueDay?:number|null }
export interface MoneyActionPayload { id:string; amountCents:number; date:string; userId?:string|null; note?:string|null }

export interface SettingsUser { id:string; displayName:string }
export interface SettingsView { appVersion:string; householdName:string; protectedBufferCents:number; planningHorizonDays:number; primaryAccountId:string; primaryAccountName:string; backupRetentionCount:number; users:SettingsUser[]; databasePath:string; backupDirectory:string; exportDirectory:string }
export interface SaveSettingsPayload { householdName:string; protectedBufferCents:number; planningHorizonDays:number; primaryAccountName:string; backupRetentionCount:number; users:SettingsUser[] }
export interface BackupItem { fileName:string; createdAt:string; sizeBytes:number }

export interface ReportCategory { name:string; amountCents:number }
export interface ReportMonth { month:string; incomeCents:number; spendingCents:number; netCents:number }
export interface ReportsView { startDate:string; endDate:string; incomeCents:number; billPaymentsCents:number; everydaySpendingCents:number; savingsCents:number; debtPaymentsCents:number; netCents:number; categories:ReportCategory[]; months:ReportMonth[] }

export interface PaymentFundingSource {
  sourceType: "current_cash" | "paycheck";
  paycheckId: string | null;
  ownerName: string | null;
  payDate: string | null;
  amountCents: number;
  reasonCode: string | null;
  recommendedPaymentDate: string | null;
}

export interface PaymentAction {
  paymentDate: string;
  amountCents: number;
  actionStatus: "overdue_action" | "pay_today" | "draft_today" | "coming_up" | "scheduled";
}

export interface PaymentGuidanceItem {
  occurrenceId: string;
  billName: string;
  remainingAmountCents: number;
  dueDate: string;
  payByDate: string;
  recommendedPaymentDate: string;
  paymentType: "manual" | "autopay";
  priority: "essential" | "normal" | "flexible";
  status: string;
  canSplitPayment: boolean;
  fundedAmountCents: number;
  fundingComplete: boolean;
  actionStatus: "needs_funding" | "overdue_action" | "pay_today" | "draft_today" | "coming_up" | "scheduled";
  fundingSources: PaymentFundingSource[];
  paymentActions: PaymentAction[];
}

export interface PaymentGuidanceView {
  asOfDate: string;
  payNowCount: number;
  needsAttentionCount: number;
  items: PaymentGuidanceItem[];
}

export interface UpdateStatus {
  available: boolean;
  currentVersion: string;
  version: string | null;
}
