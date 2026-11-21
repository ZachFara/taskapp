# taskapp-gcal

Writes the app's notifications to one Google Calendar calendar. Sync is one way: the app overwrites any manual
edit to its own events and recreates any it finds deleted. Events carry `extendedProperties.private`
`{"taskapp": "1", "managed_key": "<key>"}`; nothing else in the calendar is touched.

## One-time Google setup (owner)

1. Create a Google Cloud project and enable the Google Calendar API.
2. OAuth consent screen: external, scopes `calendar.events` and `calendar.calendarlist.readonly`.
3. Create an OAuth client of type Desktop app.
4. **Publish the app to production** (unverified is fine). Never leave it in Testing: Testing expires refresh
   tokens after 7 days and the sync would silently stop. Expect a one-time "unverified app" warning at sign-in.
5. Build the app with `GOOGLE_CLIENT_ID` (and `GOOGLE_CLIENT_SECRET`, which Google issues for Desktop clients) set.
   The refresh token lives in the macOS Keychain, service `taskapp.gcal`.
6. In Settings, connect, pick the calendar, set `gcal_enabled`. Optionally set the `timezone` setting to an IANA
   name (default UTC); it fixes when the 08:00 digest alert fires.
7. Use the Google Calendar app on the phone for notifications.

## Digest reminder probe (owner's step, not done)

The probe needs a real Google login, so it has not been run. Steps, on a throwaway test calendar:

1. In the Google Calendar web UI create an all-day event and set the notification "On the day at 8:00 AM".
2. Read it back: `GET /calendar/v3/calendars/<id>/events/<eventId>` and look at `reminders.overrides`.
3. If a positive `minutes` value can express 08:00 on the day (minutes are counted before midnight, so it
   would have to be negative), return it from `planner::digest_reminder_minutes()` and drop the alert event.
   If not, the current fallback is already correct.

Current path: the fallback. `digest_reminder_minutes()` returns `None`; each digest stays all-day with no
reminder, and the planner adds a 15-minute timed event at 08:00 local titled the same, `managed_key`
`digest-alert:<date>`, with a popup at the event time. Record the probe result here once run.
