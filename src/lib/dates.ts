/**
 * Date helpers for the app's `YYYY-MM-DD` wire format.
 *
 * The backend decides "today" with `chrono::Local`, so the frontend has to agree.
 * `new Date().toISOString()` formats in UTC and returns *tomorrow* from early
 * evening onward in US timezones, which silently defaults date pickers to the
 * wrong day. Everything here stays in local time.
 */

/** Format a `Date` as local `YYYY-MM-DD`. */
export const dateText = (d: Date): string =>
  `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;

/** Today in local time as `YYYY-MM-DD`. */
export const todayText = (): string => dateText(new Date());

/**
 * Parse a `YYYY-MM-DD` string into a local `Date`.
 *
 * Anchored at noon so that DST shifts and timezone offsets can never move the
 * value onto an adjacent day.
 */
export const parseDateText = (value: string): Date => new Date(`${value}T12:00:00`);

/**
 * Parse a SQLite `CURRENT_TIMESTAMP` value (`YYYY-MM-DD HH:MM:SS`, always UTC)
 * into a `Date`. Without the `Z`, browsers read it as local time and every
 * activity timestamp shifts by the UTC offset.
 */
export const parseDbTimestamp = (value: string): Date => {
  const iso = value.includes("T") ? value : value.replace(" ", "T");
  return new Date(/[Zz]|[+-]\d\d:?\d\d$/.test(iso) || iso.length <= 10 ? iso : `${iso}Z`);
};
