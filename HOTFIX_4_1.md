# Hotfix 4.1

Fixes the Phase 4 Calendar frontend type error. `getCalendarData` returns `CalendarData { events }`; Calendar now stores `data.events` rather than passing the wrapper object to React `setEvents`.

No database schema or financial-engine behavior changed.
