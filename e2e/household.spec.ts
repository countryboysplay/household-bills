import { expect, test, type Page } from "@playwright/test";

// One household, built up step by step; each step checks the exact money the
// app shows. Dates are relative to today so the suite never goes stale.
test.describe.configure({ mode: "serial" });

const pad = (n: number) => String(n).padStart(2, "0");
const isoDaysFromToday = (days: number) => {
  const d = new Date();
  d.setDate(d.getDate() + days);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
};

let page: Page;

test.beforeAll(async ({ browser }) => {
  page = await browser.newPage();
  page.on("dialog", (dialog) => void dialog.accept()); // archive / restore confirmations
});
test.afterAll(async () => { await page.close(); });

const nav = (name: string) => page.locator("aside").getByRole("button", { name, exact: true }).click();
const modal = () => page.locator(".modal");
const stat = (label: string) => page.locator(".stat-card", { hasText: label }).locator("strong");
const sidebarStatus = () => page.locator(".system-status > div").first();

async function addBill(fields: { name: string; category: string; amount: string; recurrence?: string; dueDate?: string; dueDay?: string; priority?: string }) {
  await nav("Bills");
  await page.getByRole("button", { name: "＋ Add Bill" }).click();
  const m = modal();
  await m.getByLabel("Bill Name").fill(fields.name);
  await m.getByLabel("Category").selectOption(fields.category);
  await m.getByLabel("Amount", { exact: true }).fill(fields.amount);
  if (fields.recurrence) await m.getByLabel("Recurrence").selectOption(fields.recurrence);
  if (fields.dueDay) await m.getByLabel("Due Day").fill(fields.dueDay);
  if (fields.dueDate) await m.getByLabel(/Due Date/).fill(fields.dueDate);
  if (fields.priority) await m.getByLabel("Priority").selectOption(fields.priority);
  await m.getByRole("button", { name: "Save Bill" }).click();
  await expect(m).toHaveCount(0);
}

async function openBill(name: string) {
  await nav("Bills");
  await page.locator(".bill-table-row", { hasText: name }).click();
}

test("onboarding sets cash and the protected buffer", async () => {
  await page.goto("/");
  await page.getByLabel("Protected cash buffer").fill("500.00");
  await page.getByLabel("Current account balance").fill("2,000.00");
  await page.getByRole("button", { name: "Create Household" }).click();
  await expect(stat("Current Cash")).toHaveText("$2,000.00");
  await expect(stat("Safe to Spend")).toHaveText("$1,500.00");
  await expect(sidebarStatus()).toHaveText("All systems normal");
});

test("a pay schedule creates paychecks", async () => {
  await nav("Paycheck Planner");
  await page.getByRole("button", { name: "Add / Change Schedule" }).click();
  const m = modal();
  await m.getByLabel("Pay Frequency").selectOption("biweekly");
  await m.getByLabel("Normal / Projected Amount").fill("1500.00");
  await m.getByLabel("Next Pay Date").fill(isoDaysFromToday(7));
  await m.getByRole("button", { name: "Save Schedule & Recalculate" }).click();
  await expect(m).toHaveCount(0);
  await expect(page.getByText("Every 2 Weeks")).toBeVisible();
});

test("a bill the household cannot cover raises a shortage, and archiving it releases the money", async () => {
  await addBill({ name: "Big Test", category: "other", amount: "5000.00", recurrence: "one_time", dueDate: isoDaysFromToday(3), priority: "essential" });
  await expect(sidebarStatus()).toContainText("Plan needs attention");
  await nav("Dashboard");
  await expect(stat("Safe to Spend")).toHaveText("$0.00");
  // Most severe alert first: $2,000 cash cannot cover a $5,000 bill due before payday.
  await expect(page.locator(".alerts .alert").first()).toContainText("Projected negative balance");

  await openBill("Big Test");
  await page.getByRole("button", { name: "Archive" }).click();
  await expect(page.locator(".bill-table-row", { hasText: "Big Test" })).toHaveCount(0);
  await expect(sidebarStatus()).toHaveText("All systems normal");
  await nav("Dashboard");
  await expect(stat("Safe to Spend")).toHaveText("$1,500.00");
});

test("paying a bill deducts it exactly and files it under the bill's category", async () => {
  await addBill({ name: "Phone", category: "utilities", amount: "85.25", dueDay: "28" });
  await openBill("Phone");
  await page.getByRole("button", { name: "✓ Mark Paid" }).click();
  await modal().getByRole("button", { name: "Mark Paid" }).click();
  await expect(modal()).toHaveCount(0);
  await nav("Dashboard");
  await expect(stat("Current Cash")).toHaveText("$1,914.75");
  await nav("Spending");
  await expect(page.locator(".transaction-table-row", { hasText: "Phone" })).toContainText("Utilities");
});

test("activity times are shown in local time", async () => {
  await nav("History");
  const stamp = (await page.locator(".history-item time").first().innerText()).trim(); // e.g. "Nov 5, 2026, 4:00 PM"
  const minutesOff = Math.abs(Date.parse(stamp) - Date.now()) / 60_000;
  expect(minutesOff, `history shows "${stamp}"`).toBeLessThan(10);
});

test("a yearly bill shows its schedule and next due date", async () => {
  const firstDue = isoDaysFromToday(200);
  await addBill({ name: "Car Registration", category: "other", amount: "180.00", recurrence: "annual", dueDate: firstDue });
  const row = page.locator(".bill-table-row", { hasText: "Car Registration" });
  await expect(row).toContainText("Yearly");
  await expect(row).toContainText("Due:");
});

test("an extra debt payment larger than the balance is refused", async () => {
  await nav("Savings & Debt");
  await page.getByRole("button", { name: "＋ Add Debt" }).click();
  let m = modal();
  await m.getByLabel("Name").fill("Store Card");
  await m.getByLabel("Current Balance").fill("300");
  await m.getByLabel("APR %").fill("18");
  await m.getByLabel("Minimum Payment").fill("25");
  await m.getByLabel("Due Day").fill("5");
  await m.getByRole("button").last().click();
  await expect(m).toHaveCount(0);

  await page.getByRole("button", { name: "Extra Payment" }).first().click();
  m = modal();
  await m.getByLabel("Amount").fill("500");
  await m.getByRole("button", { name: "Record" }).click();
  await expect(m).toContainText("more than the remaining Store Card balance of $300.00");
  await m.getByLabel("Amount").fill("300");
  await m.getByRole("button", { name: "Record" }).click();
  await expect(m).toHaveCount(0);
  await nav("Dashboard");
  await expect(stat("Current Cash")).toHaveText("$1,614.75");
});
