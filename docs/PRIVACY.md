# Cairn privacy policy

Cairn is a personal task app that runs on your own Mac. There is no Cairn server and no Cairn account.

## Google Calendar

If you connect Google Calendar, Cairn asks for two permissions:

- `calendar.calendarlist.readonly`, to show the names of your calendars so you can pick one;
- `calendar.events`, to create, update and remove the events Cairn makes for your own tasks (due times and a daily
  "due today" summary), work blocks, and the events you add on Cairn's Events page in the one calendar you picked.
  An event's Google description also lists the issues linked to it: each line has the issue key, its title and a
  `taskapp://issue/` link, so the issue titles of linked issues go to Google.

Cairn does not read, change or delete any other event, and does not read any other calendar. Events it makes are
marked as Cairn's own.

## Where your data goes

- Your tasks and settings stay in a database on your Mac.
- The Google sign-in token is stored in your Mac's Keychain.
- Data goes only to Google Calendar (the events above) and, if you turn on export, to a folder or Git remote you choose.
- Nothing is sent to the authors of Cairn or to anyone else. There is no analytics or tracking.

## Stopping

Turn the calendar off in Cairn's Settings, or remove Cairn's access at https://myaccount.google.com/permissions.
Removing access stops all calendar writes; deleting the Cairn calendar removes its events.

Questions: open an issue at https://github.com/ZachFara/taskapp/issues.
