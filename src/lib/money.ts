export const formatMoney = (cents: number): string =>
  new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
  }).format(cents / 100);

export const dollarsToCents = (value: string): number => {
  const cleaned = value.replace(/[$,\s]/g, "");
  const parsed = Number(cleaned);
  if (!Number.isFinite(parsed)) throw new Error("Enter a valid dollar amount.");
  return Math.round(parsed * 100);
};
