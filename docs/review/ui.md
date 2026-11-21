# UI review (Sonnet, read-only). Worktree review-ui @ plan-1. Screens: .scratch/review/ui-shots/ (7 PNGs, mock mode, 0 console problems)
Repro tests were a scratch vitest file (since deleted), run with SQLX_OFFLINE=true.

## Important
1. ui/src/components/views/GanttView.svelte:~100 (up(), apply). Dragging a bar always writes `due: {Date: end}`.
   a) Timed due (DateTime utc+tz) is replaced by a date-only due: time of day and zone are lost, and the Calendar
      reminder semantics change (15 min event becomes digest entry). Repro: issue start 2026-12-01, due DateTime 2026-12-03T15:00Z NY,
      drag +28px (Week) -> update(1,{"start_date":"2026-12-03","due":{"Date":"2026-12-05"}}).
   b) Estimate-only bar (start + estimate, no due) gets a due invented from the estimate. Repro: start 12-01, 480 min, drag +2d ->
      due {Date:"2026-12-04"}.
   Fix: when moving, patch only start_date, and for due shift the existing Date/DateTime by d days (keep time and tz); skip due when the row is from_estimate.
2. GanttView.svelte apply(): sequential issues.update loop with no try/catch. If the 2nd write fails, the 1st stays written, load() is not
   called (stale bars), the error is an unhandled rejection and nothing is shown. Repro: relation 1 blocks 2, make 2nd update reject {message:'boom'},
   confirm toast -> 1 unhandled rejection, 'boom' not in DOM. Fix: try/catch, show error, always load().
3. Keymap Enter is global: ListView registers `enter` -> navigate to the selected row, and keymap only ignores input/textarea/select/contenteditable.
   Enter on the Description `div role=button` (or any focused button, e.g. Comment, picker items) also navigates away to /issue/<selected row>.
   Repro: ListView + Description mounted, j, focus .md, Enter -> location.pathname '/issue/ZF-1' while the description should enter edit mode.
   Fix: in keymap ignore Enter/space when target is button/a/[role=button]/[role=option-menu], or only fire list.open when target is body.
4. Keyboard-first is List-only: j/k/enter/s/p are registered only in ListView.svelte:124-129. Table, Board, Gantt, Calendar, panel have none
   (only cmd+k and c are global, Shell.svelte:24). `l`, `/`, `g i`, `g v` exist in defaultKeymap but are never registered. Fix: register per view or
   hoist selection into the store; remove unused keymap entries.

## Minor
5. Gantt bars and Calendar chips do not open the panel (ledger item, confirmed): bars carry data-row, ViewPage.pick looks for data-key/data-issue
   (GanttView.svelte:~158, CalendarView eventClick only selects a block). Panel for an issue outside the view shows `#id` and the not-found text (App.svelte panelKey).
6. Rollup rows are draggable in effect? down() blocks 'rollup' (guard checked), but breaking that guard is not noticed by any test (see table).
7. Esc is handled twice: ListView `esc` (closes picker) and App window handler (closes panel), so one Esc closes both. Esc inside the panel's title/description editor
   closes the panel and drops the unsaved draft (change/blur does not fire on unmount). Not run; from code (App.svelte onkeydown, Description.svelte).
8. Markdown (markdown.ts): script, event handlers, javascript: and data: links, iframe, style are stripped (9 payloads checked, output in .scratch/review/d.txt) - safe from XSS.
   But `<form action=...><input>` survives, `<img src=https://...>` survives (remote fetch on render, a tracking beacon when an agent writes the description via MCP),
   and rendered `<a href=https://...>` has no click interception (only Sidebar/PinnedPage/ProjectHeader use openExternal), so in the Tauri webview a click would navigate
   the app window. Not reproduced in Tauri. Fix: DOMPurify FORBID_TAGS form,input,img (or allow only data images) and a click handler on `.md a` calling openExternal.
9. api.mock.ts divergences from core: mock shiftDependents (line ~358) mutates issues before the user confirms, so the "declining applies nothing" test only
   proves update() is not called, not that the data is unchanged. Mock estimate bar end = start+ceil(est/480) days, core = start+ceil(est/480)-1, and mock has no (due+estimate, no start) case.
10. Gantt: `.gantt{height:100vh}` inside main plus the Edit view button makes the page scroll by a button's height; group rows reuse the default blue bar style (looks like a task bar, screenshot view-1-Gantt.png); mock data shows no critical path or arrows.

## Checked, no finding
Palette/quick-add: Esc, failed create keeps text. Shortcuts ignored while typing in input/textarea/select/contenteditable (cmd+k and esc still work). Calendar timed due uses the due's own zone.
Servers: only the harness's own, stopped by shot.ts; none left running.

## Break-test table
| Behaviour broken | Test that caught it |
|---|---|
| markdown sanitizer removed | IssueDetail.test.ts (1 failure) |
| keymap typing guard removed | keymap tests (3 failures) |
| openExternal allow-list widened to anything | openExternal / PinnedPage / Sidebar tests (6 failures) |
| Gantt shift applied without the confirm toast | GanttView tests (2 failures) |
| Gantt critical flag dropped | gantt.test / GanttView tests (1 failure) |
| QuickAdd closes and loses text on failed create | QuickAdd.test.ts (1 failure) |
| App Esc closes panel even with a dialog open | Shell/App tests (1 failure) |
| Gantt drag distance forced to 0 | GanttView tests (3 failures) |
| CommandPalette stale-search guard (`mine !== seq`) removed | none |
| Gantt rollup rows made draggable | none |
Not covered by any test (found by review): finding 1a/1b/2/3.
