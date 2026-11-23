# Using Cairn

How to use the app, written from the code. If you are an agent asked "how do I do X in Cairn", read the section that
matches and answer from it. Every key, setting, CLI command and MCP tool named here exists in the code; where a
feature is only partly built, the text says so. The keyboard tables were generated from the command
registry (`ui/src/lib/commands/defs/**`); `ui/src/lib/commands/guide.test.ts` keeps them in sync.

Contents: [What it is](#what-cairn-is) - [First run](#first-run) - [The screen](#the-screen) - [Issues](#issues) -
[Selection and bulk edit](#selection-and-bulk-edit) - [Undo and redo](#undo-and-redo) - [Views](#views) -
[Layouts](#the-five-layouts) - [Projects, milestones, cycles](#projects-milestones-cycles) -
[Points and insights](#points-and-insights) - [Custom properties](#custom-properties) -
[Statuses, labels, icons, colours](#statuses-labels-icons-and-colours) - [Templates](#issue-templates) -
[Import, export, backups](#import-export-and-backups) - [Reminders and Google Calendar](#reminders-and-google-calendar) -
[Settings](#settings) - [Command menu](#the-command-menu) - [Keyboard reference](#keyboard-reference) -
[CLI](#cli-reference) - [MCP](#mcp-tools-reference) - [How do I ...](#how-do-i-) - [Limits](#limits-and-not-built-yet)

## What Cairn is

Cairn is a single-user, Linear-like task manager for the Mac. It has issues with custom properties, saved views
(list, board, table, timeline, calendar), projects, milestones, cycles, optional points and progress charts,
Apple Reminders capture, Google Calendar notifications, optional export to files, a CLI and an MCP server.

Where the data lives:
- One SQLite file: `~/Library/Application Support/taskapp/taskapp.db` (the folder name is kept from an earlier
  name). The app, the CLI and the MCP server all read and write this file. `--db <path>` points the CLI and the MCP
  server at another file.
- Backups go to a folder you choose in Settings > Backups (`snapshots/` and `nightly/` inside it).
- Only one thing lives in the browser, not the database: your recent command-menu choices (`cairn.recent`).
- Secrets: the Google refresh token lives in the macOS Keychain (service `taskapp.gcal`). The Linear token is read
  from the `LINEAR_API_KEY` environment variable and never stored.

Status: pre-release. There is no signed or notarized build; you run it from source (see README.md).

## First run

On first launch the app shows "Create your workspace": a workspace name, a workspace key (2 to 5 capital letters,
used for issue keys such as `ABC-1`) and a time zone, then "Create workspace" or "Start empty", or "Import from CSV"
to bring tasks in. The same step can be done from the terminal: `taskapp setup "Cairn" ABC --tz Europe/Berlin`
(MCP: `workspace_setup`). You can change the name, key and zone later in Settings > General. A new database starts
with five statuses (Backlog, Todo, In Progress, Done, Canceled) and three saved views (Active, Board, Timeline: all
projects).

## The screen

- **Sidebar (left).** Top: My issues. Then four collapsible sections with counts: Pinned, Views, Projects (with "All
  projects"), Cycles (with "All cycles"). Bottom: Settings. Drag a pin or a view to reorder it. Hover a view, project
  or cycle and press the pin icon to pin or unpin it. With "Sidebar sparklines" on (Settings > General) each project
  shows a small progress line. The sidebar has no Inbox link; `/inbox` exists but only shows "Inbox arrives in a later
  release".
- **Top bar.** The breadcrumb, the "Search or run a command" button (mod+k), a keyboard-shortcuts button and a theme
  button (light, dark, system).
- **View header.** Name, layout tabs (List, Board, Table, Timeline, Calendar), then the filter chips and the Add
  filter, Group, Sort and Display buttons; on the right Rename, Insights, Pin, a menu (Save as new view, New view...,
  Save as template, Reset to template, Delete view) and a New issue button.
- **Legend bar (bottom).** Key hints read from the registry: move (`j`/`k`), open (`enter`), status (`s`), commands
  (mod+k), plus the sync state ("Sync ok" or the sync error). The "select" hint is meant to be there but is not shown
  (see [Limits](#limits-and-not-built-yet)).
- **Issue panel.** Opening an issue (`enter`, or click a row) docks a panel on the right; drag its left edge to
  resize it (mouse only).
- **Pages.** Pinned (also the home page `/`), My issues (every issue that is not Done or Canceled, grouped by
  status), Views (saved views and view templates), Projects, a project page, Cycles, the all-projects timeline,
  Settings, and a design gallery (a developer page).

## Issues

An issue has: a key (`ABC-12`), title, description (markdown), status, priority, project, milestone, parent (so it is
a sub-issue), cycle, labels, start date, due date (a day, or a day and time), estimate (minutes), points (0 or more,
one decimal), custom properties, and created, updated, completed and archived times.

Priority numbers: 0 No priority, 1 Urgent, 2 High, 3 Medium, 4 Low. Status categories: Backlog, Unstarted, Started,
Completed, Canceled. A status belongs to one category.

### Create

- Press `c` (anywhere), or use the New issue button, to open quick-add. Type one line, press `enter` to create and
  close, `shift+enter` to create and keep the dialog open, `mod+enter` to create and open the issue. A row of chips
  shows what was understood; each chip is a menu that rewrites that token.
- From a group header in a list, or from a project page, the dialog starts with the project (or status) filled in.
- CLI: `taskapp add "Pay rent friday p1"`. MCP: `create_issue`.

### Quick-add grammar

Tokens can be anywhere in the line and are removed from the title. Words that do not parse stay in the title. If every
word is a token, the whole text is kept as the title. Matching ignores case (except labels, which match names ignoring
case). Dates are relative to today in the workspace time zone.

| Token | Meaning | Examples |
|---|---|---|
| `p0` to `p4` | Priority (0 none, 1 urgent, 2 high, 3 medium, 4 low). `p5` or a bare `p` stay in the title. | `p1` |
| `#slug` | Project by slug. An unknown slug goes back into the title; nothing is created. | `#website` |
| `@name` | Label by name. An unknown label goes back into the title. Repeat for several. | `@bug @urgent` |
| `~30m`, `~2h`, `~1d` | Estimate (1d = 480 minutes). Zero stays in the title. | `~90m` |
| `3pt`, `3pts`, `.5pt`, `pt3`, `3 pt` | Points, one decimal at most. Only when estimation is Points or Both (Settings). | `5pt` |
| `today`, `tomorrow` | Due date. | `tomorrow` |
| weekday (`mon`, `monday` ...) | Due on the next such day (today's weekday means seven days out). | `friday` |
| `next week` | Due next Monday. | `next week` |
| `in N days`, `in N weeks` | Due in N days or weeks. | `in 2 weeks` |
| `2026-12-01` | Due on that date. | `2026-12-01` |
| `dec 3`, `3 dec` | Due on that day; next year if it has passed. | `dec 3` |
| `by <date>`, `due <date>` | Due date with an explicit word. Without a valid date after it, the words stay in the title. | `by friday` |
| `start <date>` | Start date. | `start monday` |
| `at 3pm`, `at 9:15am`, `at 15:30` | Time of day on the due date (today if no date). A bare hour needs am or pm. | `fri at 3pm` |
| `/slug` first | Create from an issue template; the rest of the line fills in or overrides fields. Typing `/` opens the template list. | `/bug login fails p2` |

Quick-add has no token for status, parent, cycle or milestone. Example: `Pay rent friday p1 #home @bills ~30m 3pt`.

### Edit

- Rename in place in a list: `f2`. Everything else: focus the issue and press the key (see the keyboard tables):
  `s` status, `p` priority, `l` or `shift+l` labels, `m` project, `shift+c` cycle, `d` due, `shift+d` start, `e`
  estimate, `shift+e` points, `shift+p` parent, `a` property, `r` relation. Each opens a page in the command menu;
  type to filter or, for dates, estimates and points, type a value.
- In the issue panel or page: click the title to edit it, edit the description (Write and Preview tabs, Save and
  Cancel), and set fields in the Properties plate (status, priority, project, milestone, cycle, labels, start, due,
  due time, estimate, points, parent, custom properties).
- Table layout: edit cells in place (`enter` on a cell).
- Dragging: a board card to another column changes its group (status by default); calendar and timeline items can
  be dragged to a day (see the layouts).
- Open the full page: `mod+enter` (route `/issue/ABC-1`). Close the panel or page: `escape`. `[` and `]` step to the
  previous or next issue.
- Copy: `mod+c` copies the key, `mod+shift+c` the link. Pin: `.`. Duplicate: `mod+d`.

### Links in descriptions and comments

- A bare web link, or an `obsidian://open` link with only `vault` and `file`, in a description, comment or project description shows as a chip: an icon, the host
  and a short path (an Obsidian link reads `Obsidian: <note name>`). Hover shows the full URL. Click opens it in your
  browser or Obsidian. The chip is built from the URL alone: nothing is fetched. Other `obsidian://` actions (new, plugin
  commands) and other schemes (javascript:, file:) stay plain text and never open.
- A link you wrote with its own text (`[the spec](https://...)`) stays as that text.
- Edit the description to see the raw URL again. Preview in the editor shows the chip.
- Link previews (page title, summary, image) are not built: they need a network fetch from the app, and the setting
  to switch them on is left out until that fetch is decided. Links stay chips with no fetch.

### Sub-issues, relations, comments, history

- **Sub-issues.** In the issue panel, "Sub-issues" shows a progress bar and a field: type a title and press enter to
  create one, or pick an existing issue to attach. Or set a parent with `shift+p`. In a list the Sub-issues display
  option nests, flattens or hides them.
- **Relations.** Kinds: blocks, blocked by, related, duplicate. Add from the panel's Relations section, or with `r`.
  CLI: `relation add`. In the timeline, "blocks" relations draw as arrows and a critical path is highlighted.
- **Comments.** Markdown. Panel "Comments" section: type and press the Comment button. CLI `comment add`; MCP `add_comment`.
- **History.** The panel's History section lists every change with its source (UI, Agent, Cli, import, Undo, ...).
  CLI `history <key>`; MCP `get_history`.

### Archive

`#` archives, `delete` does the same (there is no hard delete). More than 5 selected issues asks first in the command
menu ("Archive N issues" or "Keep them"). Archived issues leave views unless the view's "Show archived" display option
is on. Restore with the archive button in the issue page header (or undo).

## Selection and bulk edit

- `x` selects the focused issue; `shift+j` / `shift+k` extend the selection; `mod+a` selects all in the view;
  `escape` clears it. Mouse: `mod`+click toggles a row, `shift`+click selects a range, a plain click opens the issue.
- Any issue key (status, priority, labels, project, cycle, dates, estimate, points, parent, property, relation,
  archive, duplicate, pin) acts on the whole selection as one undo step. The command menu shows "N issues selected".
- Right-click a row or card for a menu of the same actions.
- A number before a movement key repeats it: `5 j` moves five rows.
- MCP: `bulk_update` applies many patches (over 10 takes a database snapshot first).

## Undo and redo

Every change can be undone, whoever made it: `mod+z` or `u` undoes the newest step, `mod+shift+z` redoes. One CLI or
MCP call is one step. After a change a toast offers Undo. The toast "Nothing to undo" shows when the stack is empty.
Undo refuses when something changed since. Inside a text field `mod+z` is the field's own undo. CLI: `taskapp undo`,
`taskapp redo`; MCP: `undo`, `redo`, `undo_status`. An import is one step ("Undo import" button after it).

## Views

A saved view is a definition: filter, sort, group, layout, displayed fields, and display options. Edits to a view are
saved automatically (after a short pause). Filter changes show an "Undo" toast.

- **Make one.** Views page > New view, or the view menu > "New view..." (choose Blank, a copy of the current view, or
  a template), or "Save as new view" (copies the current one).
- **Layouts.** Tabs in the header, or `v l` list, `v b` board, `v t` table, `v g` timeline, `v c` calendar.
- **Filter.** "Add filter" (`f`) lists Status, Status category, Project, Label, Priority, Due and every custom property.
  Each filter is a chip: click it to change the operator and values; the x removes it. Operators: is, is not, is any
  of, is none of, contains, before, up to, after, from, is empty, is set. Chips combine with AND. A filter that
  uses OR or NOT (made by CLI or MCP) shows as one "Advanced filter" chip you can only remove. Filters are saved
  with the view (see [Limits](#limits-and-not-built-yet)). The full field list a view definition accepts: Title,
  Status, StatusCategory, Priority, Project, Milestone, Cycle, Label, Parent, StartDate, DueDate, Estimate, Points,
  CreatedAt, UpdatedAt, CompletedAt, Archived, Key, SubIssueProgress, RelationCount, CommentCount, and any custom
  property.
- **Group.** "Group" button (not on the calendar): choose any of Status, Status category, Priority, Project, Cycle,
  Due, Title, Points (if on) or a custom property. Group headers collapse; the list adds an issue to a group.
- **Sort.** One sort field plus a Descending switch.
- **Display** (button, or the command "Display options..."). Switch fields on or off and drag (or use the arrow
  buttons, or `alt+arrowup` / `alt+arrowdown` on a focused row) to reorder: Key, Priority, Status, Labels, Project,
  Milestone, Cycle, Parent, Start, Due, Estimate, Points, Sub-issues, Relations, Comments, Created, Updated, and your
  custom properties (listed first under "Custom properties"). "Reset to default" restores the layout's default fields.
  Options: Compact rows (list, table, board), Show empty groups (list, table), Show archived, Sub-issues Nested, Flat or
  Hidden (list), Show weekends (timeline, calendar). On a board the field list means the card fields.
- **Pin, rename, delete.** Header buttons and menu; or commands.
- **View templates.** A template is a view definition new views start from. "Save as template" in the view menu
  makes one; the Views page lists templates, deletes them and "Set as default" makes the New view dialog preselect one.
  A view made from a template can be put back with "Reset to template". CLI: `view templates`, `view save-as-template`,
  `view from-template`, `view reset`, `view delete-template`.

## The five layouts

- **List.** Rows grouped under headers. Click a row to open the panel; double-click opens the full page. Click the
  status glyph to change status; the disclosure arrow expands sub-issues. Group header has an add-issue button.
- **Board.** One column per group (status by default). Drag a card to another column to change that field. A button
  hides or shows empty columns. `h` / `l` move between columns.
- **Table.** One row per issue, one column per displayed field. Edit cells in place (`enter`, arrows, `escape`).
  Click a column heading to sort; heading buttons move or remove a column; "Add field" adds one; drag the heading
  edge to resize a column (mouse only).
- **Timeline (Gantt).** Bars from start to due date; an issue with only an estimate shows an estimated bar. Zoom: Day,
  Week, Month, Quarter (tabs, or `=` and `-`); `t` or the Today button scrolls to today. Drag a bar to move it, drag
  its handles to change start or end; if blocked issues must move too, a toast "Also moves ..." with Apply and Cancel
  appears. Drag an issue from the Unscheduled band onto a day. Milestones show as markers. There is also an
  all-projects timeline (`g t`).
- **Calendar.** Week or Month (tabs, or `w` and `m`), previous and next (`[` and `]`), today (`t`). Issues appear on
  their due date. Drag an issue from the "Issues to schedule" side list onto a day to set its due date, or onto a time
  slot to create a one-hour work block. Drag items to move or resize them.

## Projects, milestones, cycles

- **Projects.** Projects page > New project (name, slug). Settings > Projects also adds and deletes. A project page
  shows a header (name, description, "Open doc" when a doc link is set, progress, target), milestones, a layout tab
  strip (with its own filter, group, sort and display controls) and the project's issues; `shift+i` toggles the
  insights panel. Rename, pin and archive are in the header. Archiving asks first, keeps the issues, and can be
  restored. Fields: slug, name, description, doc link, start date, target date, icon, colour. A project can also scope
  a custom property.
- **Milestones.** On a project page, "Milestones": add a name and a date. An issue is assigned a milestone in the
  issue panel (after choosing the project) or with the "Change milestone" command. Milestones draw as markers on the timeline.
- **Cycles.** Fixed, numbered, non-overlapping date ranges. Cycles page > New cycle (optional name, start, end), or
  Settings > Cycles. The current cycle shows a burn-up chart, past cycles a spark line and the page a "Completed per
  cycle" bar chart, when Progress charts is on. "Move unfinished to next cycle" (button, or the command) moves unfinished issues
  to the next cycle. Set an issue's cycle with `shift+c`.
- **Pins.** Pin issues, projects, views and cycles (CLI also notes and URLs). The Pinned page lists them; `g h`.

## Points and insights

- **Estimation setting** (Settings > General): None, Points, Time or Both. Points show only for Points or Both; time
  estimates for Time or Both. None and Time hide points everywhere. **Point scale**: Fibonacci (0, 1, 2, 3, 5, 8, 13,
  21), Linear (1 to 10), Powers (1, 2, 4, 8, 16) or T-shirt (XS=1, S=2, M=3, L=5, XL=8). The scale is the list the
  `shift+e` page offers; any number to one decimal can be typed.
- **Insights panel** (`shift+i`, or the target button) on a view or project page. Headline numbers, one chart,
  and choices remembered per view, project or cycle: Measure (Points, Issues or Time), Chart (Burn-up line, Stacked
  area, Bars per week), Range (This cycle or Project dates or 30 days, 30, 90, All). Drag its left edge to resize it.
  Charts are rebuilt from the change history, per local day: scope (every issue that existed and is not canceled),
  started, completed. With a chart focused, left and right arrows step through days.
- **Progress charts** switch (Settings > General): burn-ups on project and cycle pages, sparklines on past cycles.
- CLI: `insights progress` and `insights velocity`; MCP: `insights_progress`, `insights_velocity`.

## Custom properties

Your own fields on every issue (or just one project's). Kinds: Text, Number, Date, Select, MultiSelect, Checkbox, Url,
NoteLink. Create in Settings > Properties (name, key like `effort`, kind, scope, and options for Select kinds), or
CLI `property create`, or MCP `create_property`. Set a value in the issue panel, in a table cell, with `a` (Select and
Checkbox only), or CLI `property set`. Show a property in views through the Display panel; filter, group and sort on
it. Changing a property's kind or deleting it takes a database snapshot first (and deleting removes every issue's
value).

## Statuses, labels, icons and colours

- **Statuses** (Settings > Statuses): grouped by category; add a name with an icon and colour; delete asks which status to
  move that status's issues to.
- **Labels** (Settings > Labels): name, icon, colour. Add to issues with `l`, the panel, or `@name` in quick-add.
- **Colours** are a fixed palette of 12 keys: gray, red, orange, amber, yellow, green, teal, cyan, blue, indigo, purple,
  pink. A `#rrggbb` value sent by CLI or MCP maps to the nearest key.
- **Icons**: `icon:<id>` from the built-in set or `emoji:<emoji>` (any 1 to 8 non-ASCII characters). Today the
  icon and colour picker exists only in Settings: the "icon and colour" button next to the name field when you add a
  status, label, project, cycle, template or property. There is no way yet to change an icon by clicking it on a
  project page, view header or sidebar row, and no "Change icon" command. Views have an icon and colour in the data
  (the sidebar shows them) but no screen sets them. CLI and MCP can set icons when creating projects, labels, cycles,
  statuses and templates.

## Issue templates

Settings > Templates. A template has a name, a slug (used as `/slug` in quick-add), icon, colour, a title prefix, default
fields (status, priority, project, labels, estimate, points, properties) and sub-issues (each with its own title and
fields). Creating from a template makes the issue and its sub-issues as one undo step. Use it with `/slug ...` in
quick-add, `taskapp template apply <id> [title]`, or MCP `apply_template`.

## Import, export and backups

- **CSV import** (Settings > Import, or first run): choose a file (header row first), map each column to a field
  (title, description, status, priority, project, labels, start date, due date, estimate in minutes, or a custom
  property), press Dry run to see the plan and what is skipped and why, then "Import N issues". "Undo import" reverts it.
  Title must be mapped. CLI: `taskapp import csv file.csv [--map COLUMN=FIELD] [--apply]` (without `--apply` it only
  prints the plan). MCP: `import_csv_plan` then `import_apply`.
- **Linear import**: the Import page shows the terminal commands: `taskapp import linear` (plan, nothing written) and
  `taskapp import linear --apply`, with the token in `LINEAR_API_KEY`. It is the only part that uses the network. MCP:
  `import_linear_plan`. A runbook for migrating with an agent is in docs/linear-migration.md. Needs your Linear key.
- **Export** (Settings > Export): turn it on, choose a folder and optionally a Git remote, "Export now". It writes one
  markdown file per issue and commits and pushes it. Needs your own Git remote (see docs/github-setup.md). Not verified
  against a real repository.
- **Backups** (Settings > Backups): set a folder, "Snapshot now" writes a copy of the database to `snapshots/`; a nightly
  copy goes to `nightly/`. The newest 14 nightly and 30 snapshot files are kept. Before risky changes (property type
  change or delete, a bulk update over 10 items) a snapshot is taken. To restore: quit the app, then
  `taskapp backup restore <file> --yes`. `taskapp backup list --dir <folder>` lists backups.

## Reminders and Google Calendar

- **Apple Reminders capture** (Settings > Reminders): press Authorize (macOS asks for access to Reminders), then pick
  the "Inbox list". Reminders you add to that list (phone, Siri, share sheet) are imported as issues: the title is
  parsed with the quick-add grammar, notes become the description, a due date becomes the due date (as described in docs/DESIGN.md). Needs the Swift
  helper (`scripts/build-helper.sh`) and your Apple account's Reminders. Not verified against a real list.
- **Google Calendar** (Settings > Calendar): "Connect Google Calendar", choose a calendar (use a dedicated one),
  turn on "Enable notifications". Sync is one way: the app writes events and overwrites manual edits to its own events.
  Needs your Google Cloud OAuth client and a Google login (setup steps in docs/gcal.md). Not verified against a real account.
- The legend bar shows the sync state; a sync error appears there.

## Settings

Settings is a list of sections: General, Statuses, Labels, Projects, Cycles, Properties, Templates, Import, Reminders,
Calendar, Export, Backups, and "Keyboard shortcuts" (opens the sheet). `mod+s` saves the open page.

| General setting | Values | Notes |
|---|---|---|
| Workspace name | text | Must not be empty. |
| Workspace key | 2 to 5 capital letters | New issues use it; existing keys keep working. |
| Time zone | IANA name | Calendar alerts land at this time of day. A change applies after restart. |
| Week starts on | Sunday, Monday (Saturday is offered but is refused, see Limits) | |
| Theme | System, Light, Dark | Also the top-bar button. |
| Estimation | None, Points, Time, Both | |
| Point scale | Fibonacci, Linear, Powers, T-shirt | Disabled when points are off. |
| Progress charts | on or off | Burn-ups and sparklines on projects and cycles. |
| Sidebar sparklines | on or off | A small progress line per project. |

Other stored settings (not all with a screen): backup folder, Reminders list, Google Calendar choice and switch, export
folder, remote and switch, the default view template.

### Themes

Settings > General > Look picks one of six looks: Survey map (the default), Linear, Jira, Cupertino, Obsidian and
Scholastic. Each has a light and a dark variant and also sets corner radius, row height and type face (system fonts
only). It works with the Light, Dark or System choice above it and applies at once. The command menu has "Change
theme..." with the same list.

## The command menu

- **Open:** `mod+k` or `mod+p` (works inside text fields), the "Search or run a command" button, or `/` (search).
- **Search issues.** Typing at the top of the menu lists matching issues first, under "Issues". Each word matches
  from its start, so `cras` finds "Crash on launch", and a key such as `ABC-12` is listed first. `/` or the
  "Search issues…" row opens the same search on its own, with nothing else in the list.
- **Use:** type to filter commands, issues, views and projects. `arrowdown` / `arrowup` (or `ctrl+n` / `ctrl+p`,
  `ctrl+j` / `ctrl+k`) move, `enter` runs, `tab` completes the highlighted label, `escape` closes.
- **Contextual actions.** With an issue focused, selected or open, an "Actions on ABC-1" (or "N issues selected")
  section lists the issue commands first. With nothing focused they are hidden. Commands that only apply somewhere
  (table, calendar, timeline, a project page, Cycles) are listed only there.
- **Nested pages.** Commands such as Set status, Set due date or Add or remove labels open a second step (a page).
  The page can filter, show a loading row while choices load, and some are multi-pick (labels stay open and show
  checks). Free-text pages (due date, start date, estimate, points) turn typed text into a "Preview" row that runs on
  `enter`. `backspace` on an empty filter goes back.
- **Recents.** With an empty query, the last five things you chose show under "Recent" (kept in the browser).
- **Sections:** Issue, Go to, Actions, Create, Settings and help.
- The keyboard sheet (`?`) lists every key by group and has a filter box.

## Keyboard reference

Keys are written in the registry's own syntax: a single key (`c`, `?`), a
combination (`mod+k`, `shift+j`, `alt+arrowup`), or a sequence of keys separated by a space (`g i` means press `g`,
then `i`, within one second; `escape` cancels). `mod` is Cmd on a Mac and Ctrl elsewhere. A number typed first is a
count (`5 j`). Plain keys do nothing while you type in a field; `escape` and the `mod+` keys marked below reach
through. While a dialog, menu or popover is open, plain keys belong to it. When two commands share a key the more
specific scope wins (editing a list title, then issue detail, then a layout, then global).

How this section is checked (`ui/src/lib/commands/guide.test.ts`; keep the table format so it works): it reads every table row in
this file whose first cell is a command id in backticks, and takes the key sequences from the third cell (each in
backticks). It fails when a registered command that has a shortcut has no row, when a row's keys differ from the
registry, or when a row names an unknown command. Rows whose first cell is not a backticked id are not checked.

### Global (live everywhere)

| Command id | Name | Keys | What it does |
|---|---|---|---|
| `palette.open` | Open command menu | `mod+k` or `mod+p` | Open the command menu. Works inside text fields. |
| `nav.search` | Search | `/` | Open the command menu for search. |
| `help.shortcuts` | Keyboard shortcuts | `?` | Open the keyboard shortcuts sheet (grouped, with a filter box). |
| `issue.create` | New issue | `c` | Open the quick-add dialog. |
| `history.undo` | Undo | `mod+z` or `u` | Undo the newest step. Inside a text field mod+z stays the field's own undo. |
| `history.redo` | Redo | `mod+shift+z` | Redo the step you undid. |
| `settings.save` | Save settings | `mod+s` | Save the open settings page. Works inside text fields. |
| `nav.my` | Go to My issues | `g m` | Go to My issues. |
| `nav.pinned` | Go to Pinned | `g h` | Go to Pinned. |
| `nav.projects` | Go to Projects | `g p` | Go to the project list. |
| `nav.cycles` | Go to Cycles | `g c` | Go to Cycles. |
| `nav.views` | Go to Views | `g v` | Go to Views. |
| `nav.timeline` | Go to Timeline | `g t` | Go to the all-projects timeline. |
| `nav.settings` | Go to Settings | `g s` | Go to Settings. |
| `nav.design` | Go to Design gallery | `g d` | Go to the design gallery (a developer page). |

### List, board and table: moving and selecting

| Command id | Name | Keys | What it does |
|---|---|---|---|
| `move.down` | Move down | `j` or `arrowdown` | Move focus to the next row or card. A number first repeats it (`5 j`). |
| `move.up` | Move up | `k` or `arrowup` | Move focus to the previous row or card. |
| `move.top` | Go to top | `g g` | Jump to the first row. |
| `move.bottom` | Go to bottom | `shift+g` | Jump to the last row. |
| `move.left` | Previous column | `h` | Board: focus moves to the previous column. |
| `move.right` | Next column | `l` | Board: focus moves to the next column. |
| `move.open` | Open issue | `enter` | Open the focused issue in the side panel; focus stays in the list. |
| `move.peek` | Peek at issue | `space` | Peek at the focused issue in the side panel; press again to close. |
| `move.escape` | Close or clear | `escape` | Clear the selection (when no issue is open). |
| `select.toggle` | Select issue | `x` | Select or unselect the focused issue. |
| `select.all` | Select all in view | `mod+a` | Select every issue in the view. |
| `select.extendDown` | Extend selection down | `shift+j` | Extend the selection down. |
| `select.extendUp` | Extend selection up | `shift+k` | Extend the selection up. |

### List, board and table: actions on the focused or selected issues

| Command id | Name | Keys | What it does |
|---|---|---|---|
| `issue.status` | Set status | `s` | Set status (menu page). |
| `issue.priority` | Set priority | `p` | Set priority (menu page). |
| `issue.labels` | Add or remove labels | `shift+l` or `l` | Add or remove labels (menu page). |
| `issue.project` | Move to project | `m` | Move to a project (menu page). |
| `issue.cycle` | Set cycle | `shift+c` | Set cycle (menu page). |
| `issue.due` | Set due date | `d` | Set due date: Today, Tomorrow, In a week, none, or type `fri`, `next fri`, `nov 28`, `28 nov`, `2026-12-01`, `+3d`, `+2w`, `none`. |
| `issue.start` | Set start date | `shift+d` | Set start date; accepts the same typed dates as due date. |
| `issue.estimate` | Set estimate | `e` | Set estimate: presets or type `30m`, `2h`, `1h30m`, `1.5h`, `1d` (8 hours), a bare number (hours), `none`. |
| `issue.points` | Set points | `shift+e` | Set points: scale values or type a number to one decimal, `none` clears. Only offered when estimation is Points or Both. |
| `issue.parent` | Set parent | `shift+p` | Set the parent issue, or no parent. |
| `issue.property` | Set property | `a` | Set a Select or Checkbox custom property (other kinds are edited in the issue panel or table). |
| `issue.relation` | Add relation | `r` | Add a relation (Blocks, Related to, Duplicate of) to another issue. |
| `issue.archive` | Archive | `#` | Archive the selection. More than 5 issues asks for confirmation in the command menu. |
| `issue.delete` | Delete (archives) | `delete` | Same as Archive. There is no hard delete. |
| `issue.duplicate` | Duplicate | `mod+d` | Duplicate the selection (one undo step). |
| `issue.pin` | Pin or unpin | `.` | Pin or unpin the issue. |
| `issue.copyKey` | Copy key | `mod+c` | Copy the issue key to the clipboard. |
| `issue.copyLink` | Copy link | `mod+shift+c` | Copy a link to the issue. |
| `issue.openPage` | Open full page | `mod+enter` | Open the issue as a full page. |
| `list.title.edit` | Rename issue | `f2` | List: rename the focused issue in place. |
| `list.title.cancel` | Cancel rename | `escape` | Cancel the rename. |

### View keys (on a /view page or project page)

| Command id | Name | Keys | What it does |
|---|---|---|---|
| `view.layout.list` | List layout | `v l` | Switch the view to List. |
| `view.layout.board` | Board layout | `v b` | Switch the view to Board. |
| `view.layout.table` | Table layout | `v t` | Switch the view to Table. |
| `view.layout.timeline` | Gantt layout | `v g` | Switch the view to the Timeline (Gantt). |
| `view.layout.calendar` | Calendar layout | `v c` | Switch the view to Calendar. |
| `view.filter.add` | Add filter | `f` | Open the Add filter menu. |
| `view.insights` | Toggle insights | `shift+i` | Show or hide the insights panel (views and project pages). |
| `view.field.up` | Move field up | `alt+arrowup` | In the Display panel, move the focused field up. |
| `view.field.down` | Move field down | `alt+arrowdown` | In the Display panel, move the focused field down. |

### Table only

| Command id | Name | Keys | What it does |
|---|---|---|---|
| `table.left` | Cell to the left | `arrowleft` | Table: move to the cell on the left. |
| `table.right` | Cell to the right | `arrowright` | Table: move to the cell on the right. |
| `table.edit` | Edit cell | `enter` | Table: edit the focused cell. |
| `table.cancel` | Cancel edit or clear selection | `escape` | Table: cancel the cell edit, or clear the selection. |

### Issue detail (panel or page)

| Command id | Name | Keys | What it does |
|---|---|---|---|
| `issue.detail.prev` | Previous issue | `[` | Open the previous issue in the list you came from. |
| `issue.detail.next` | Next issue | `]` | Open the next issue in the list you came from. |
| `issue.detail.close` | Close issue | `escape` | Close the open editor first, then the panel or page. |

### Calendar layout

| Command id | Name | Keys | What it does |
|---|---|---|---|
| `calendar.prev` | Previous week or month | `[` | Previous week or month. |
| `calendar.next` | Next week or month | `]` | Next week or month. |
| `calendar.today` | Go to today | `t` | Jump to today. |
| `calendar.week` | Week view | `w` | Week range. |
| `calendar.month` | Month view | `m` | Month range. |

### Timeline layout

| Command id | Name | Keys | What it does |
|---|---|---|---|
| `timeline.today` | Timeline: go to today | `t` | Scroll to today. |
| `timeline.zoom-in` | Timeline: zoom in | `=` | Zoom in. |
| `timeline.zoom-out` | Timeline: zoom out | `-` | Zoom out. |

### Charts

| Command id | Name | Keys | What it does |
|---|---|---|---|
| `chart.day.prev` | Previous day on the chart | `arrowleft` | With a chart focused, step to the previous day. |
| `chart.day.next` | Next day on the chart | `arrowright` | With a chart focused, step to the next day. |

### Commands with no key (command menu only)

These appear in the command menu (when they apply) and have no shortcut.

| Command id | Name | Keys | What it does |
|---|---|---|---|
| `app.reload` | Reload | (none) | Reload the window. |
| `cycle.move_unfinished` | Move unfinished issues to the next cycle | (none) | Move unfinished issues of the cycle to the next cycle (Cycles page). |
| `cycle.new` | New cycle | (none) | New cycle. |
| `group.add` | Add issue to this group | (none) | Add an issue to this group. |
| `group.collapse` | Collapse or expand group | (none) | Collapse or expand a group. |
| `group.collapse.all` | Collapse or expand all groups | (none) | Collapse or expand all groups. |
| `import.csv` | Import CSV | (none) | Open the CSV import. |
| `issue.archive.confirm` | Archive the selected issues | (none) | The confirm step of Archive for more than 5 issues. |
| `issue.comment.focus` | Write a comment | (none) | Focus the comment box in the issue panel. |
| `issue.description.edit` | Edit description | (none) | Edit the description. |
| `issue.detail.milestone` | Change milestone | (none) | Change the milestone. |
| `issue.relation.add` | Add relation from the panel | (none) | Add a relation from the panel (the key `r` does the same). |
| `issue.subissue.add` | Add sub-issue | (none) | Add a sub-issue. |
| `label.new` | New label | (none) | New label. |
| `milestone.new` | New milestone | (none) | New milestone (project page). |
| `project.archive` | Archive project | (none) | Archive the project (asks first). |
| `project.description.edit` | Edit project description | (none) | Edit the project description. |
| `project.new` | New project | (none) | New project. |
| `project.pin` | Pin or unpin project | (none) | Pin or unpin the project. |
| `project.rename` | Rename project | (none) | Rename the project. |
| `settings.backups` | Backups | (none) | Open Settings, Backups. |
| `settings.calendar` | Calendar | (none) | Open Settings, Calendar. |
| `settings.cycles` | Cycles | (none) | Open Settings, Cycles. |
| `settings.export` | Export | (none) | Open Settings, Export. |
| `settings.general` | General | (none) | Open Settings, General. |
| `settings.import` | Import | (none) | Open Settings, Import. |
| `settings.labels` | Labels | (none) | Open Settings, Labels. |
| `settings.projects` | Projects | (none) | Open Settings, Projects. |
| `settings.properties` | Properties | (none) | Open Settings, Properties. |
| `settings.reminders` | Reminders | (none) | Open Settings, Reminders. |
| `settings.statuses` | Statuses | (none) | Open Settings, Statuses. |
| `settings.templates` | Templates | (none) | Open Settings, Templates. |
| `status.new` | New status | (none) | New status. |
| `template.new` | New template | (none) | New issue template. |
| `theme.cycle` | Switch theme (light, dark, system) | (none) | Switch theme (light, dark, system). |
| `view.delete` | Delete view | (none) | Delete the view. |
| `view.display.open` | Display options… | (none) | Open Display options. |
| `view.duplicate` | Save as new view | (none) | Save as new view. |
| `view.group.open` | Group by… | (none) | Open Group by. |
| `view.new` | New view | (none) | New view. |
| `view.pin` | Pin or unpin view | (none) | Pin or unpin the view. |
| `view.rename` | Rename view | (none) | Rename the view. |
| `view.resetToTemplate` | Reset to template | (none) | Put the view back to its template. |
| `view.saveAsTemplate` | Save as template | (none) | Save the view as a template. |
| `view.sort.open` | Sort by… | (none) | Open Sort by. |

### Keys inside dialogs and the command menu

These are handled by the dialogs themselves, not the command registry, so they are not checked by the test.

| Where | Keys | What it does |
|---|---|---|
| Quick-add dialog | `enter` / `shift+enter` / `mod+enter` | Create and close / create and keep open / create and open the issue. |
| Quick-add dialog | `tab`, `enter` on a chip | Reach each chip; its menu rewrites the token. |
| Command menu | `arrowdown`, `arrowup` | Move through rows (also `ctrl+n`, `ctrl+p`, `ctrl+j`, `ctrl+k`). |
| Command menu | `enter`, `tab`, `backspace` | Run, complete the label, go back from a page. |
| Any dialog | `escape` | Close it. |
| Description editor | Write and Preview buttons | Edit markdown, check the result, Save or Cancel. |

## CLI reference

Binary `taskapp` (build: `cargo build -p taskapp-cli`; or `cargo run -p taskapp-cli -- <args>`). Global options:
`--db <path>` (default: the app's database) and `--json` (JSON instead of a table). `taskapp --help` and
`taskapp <command> --help` print the full list. Every write is one undoable step with source Cli. There are 45
subcommands; keys are issue keys such as `ABC-1`, ids are numbers.

| Command | What it does | Example |
|---|---|---|
| `add <text> [--tz]` | Create an issue from quick-add text. | `taskapp add "Pay rent friday p1"` |
| `issue create <title> [...]` | Create with flags: `--description --priority --status --project --due --estimate --points --created-at --completed-at --property k=v --parent --cycle --label --migration`. | `taskapp issue create "Fix login" --priority 2 --due 2026-12-01` |
| `issue update <key> [...]` | Change fields: `--title --description --priority --status --due --clear-due --points --clear-points`. | `taskapp issue update ABC-1 --status 3` |
| `issue get <key>` | Show one issue. | `taskapp issue get ABC-1` |
| `issue archive <key>` | Archive an issue. | `taskapp issue archive ABC-1` |
| `issue search <text> [--limit 20]` | Full-text search (archived issues are left out). | `taskapp issue search rent` |
| `view query [--def JSON \| --id N]` | Run an ad hoc view definition or a saved view. | `taskapp view query --id 1` |
| `view templates` | List view templates. | `taskapp view templates` |
| `view save-as-template <view_id> [--name]` | Make a template from a view. | `taskapp view save-as-template 1 --name Sprint` |
| `view from-template <template_id> [--name]` | Make a view from a template. | `taskapp view from-template 1` |
| `view reset <view_id>` | Put a view back to its template. | `taskapp view reset 4` |
| `view delete-template <id>` | Delete a template. | `taskapp view delete-template 1` |
| `property list` | List custom properties. | `taskapp property list` |
| `property create <key> <name> <kind> [--option v]` | Kinds: text, number, date, select, multiselect, checkbox, url, notelink. | `taskapp property create area Area select --option web --option api` |
| `property rename <key> <name>` | Rename, keeping values. | `taskapp property rename area Domain` |
| `property change-kind <key> <kind> [--option]` | Change type; snapshots first. | `taskapp property change-kind area text` |
| `property delete <key>` | Delete the property and its values; snapshots first. | `taskapp property delete area` |
| `property set <issue> <key> <value>` | Set a value (JSON; `null` clears). | `taskapp property set ABC-1 area '"web"'` |
| `project list` / `project create <slug> <name> [--icon --color]` | List or create projects. | `taskapp project create web Website` |
| `cycle list` / `cycle create --start --end [--name --color --icon]` | List or create cycles. | `taskapp cycle create --start 2026-12-01 --end 2026-12-14` |
| `label list` / `label create <name> <color> [--icon]` | List or create labels. | `taskapp label create bug red` |
| `comment add <key> <body>` | Comment on an issue. | `taskapp comment add ABC-1 "Looks good"` |
| `relation add <from> <kind> <to>` / `relation remove ...` | Kinds: blocks, related, duplicate. | `taskapp relation add ABC-1 blocks ABC-2` |
| `pin list` / `pin add <kind> <target> [--title]` / `pin remove <id>` | Kinds: issue, project, view, cycle, note, url. | `taskapp pin add issue ABC-1` |
| `template list` / `create` / `update <id>` / `delete <id>` | Issue templates: `create <name> <slug> [--icon --color --body JSON]`. | `taskapp template create Bug bug --body '{"title_prefix":"[Bug] "}'` |
| `template apply <id> [title]` | Create the issue and its sub-issues. | `taskapp template apply 1 "Login fails"` |
| `insights progress (--project N \| --cycle N \| --view N) [--measure points\|count\|time --from --to]` | Per-day scope, started, completed. | `taskapp insights progress --project 1 --measure count` |
| `insights velocity (--project N \| --cycle N \| --view N) [--measure --periods 6]` | Completed per cycle or week. | `taskapp insights velocity --cycle 1` |
| `history <key>` | Change history of an issue. | `taskapp history ABC-1` |
| `undo` / `redo` | Undo or redo the newest step. | `taskapp undo` |
| `backup list --dir <dir>` | List backups in a folder. | `taskapp backup list --dir ~/Backups/cairn` |
| `backup restore <file> --yes` | Replace the database; quit the app first. | `taskapp backup restore snap.db --yes` |
| `import csv <file> [--map COLUMN=FIELD] [--apply]` | Plan, or apply with `--apply`. | `taskapp import csv tasks.csv --map Name=title --apply` |
| `import linear [--apply]` | Plan or apply a Linear import (`LINEAR_API_KEY`). | `taskapp import linear` |
| `setup <name> <key> [--tz]` | First-run setup. | `taskapp setup Cairn ABC --tz Europe/Berlin` |

The CLI has no command for statuses, milestones, settings or unarchiving; use the app or MCP (statuses, milestones).

## MCP tools reference

Server `taskapp-mcp` over stdio (`taskapp-mcp [--db <path>]`). Writes use source Agent, or Migration when a call passes
`migration: true`. Unknown argument names are refused. Only `import_linear_plan` uses the network. 51 tools:

| Tool | What it does (key arguments) |
|---|---|
| `search_issues` | Full-text search. `text`, `limit` (20). |
| `query_view` | Run a view. `def` (a ViewDef) or `view_id`. |
| `get_issue` | One issue with relations, comments, children. `key`. |
| `create_issue` | Create. Fields of NewIssue (`title` required; `due` as `{"Date":"2026-01-10"}`; `properties`; `created_at`, `completed_at`). |
| `update_issue` | Change an issue. `key`, `patch` (null clears a nullable field). |
| `archive_issue` | Archive. `key`. |
| `bulk_update` | Many patches. `items` = list of `{key, patch}`; over 10 snapshots first; an unknown key applies nothing. |
| `add_comment` | `key`, `body`, `created_at`. |
| `add_relation` / `remove_relation` | `from_key`, `to_key`, `kind` (Blocks, Related, Duplicate). |
| `set_points` | `key`, `points` (null clears). |
| `get_history` | Change history. `key`. |
| `list_properties` | List custom properties. |
| `create_property` | `key`, `name`, `kind`, `options`, `project_id`, `position`. |
| `rename_property` | `key`, `new_name`. |
| `change_property_kind` | `key`, `kind`, `options`; snapshots first. |
| `delete_property` | `key`; snapshots first. |
| `set_property` | `key` (issue), `property`, `value` (null clears). |
| `list_statuses` / `create_status` | `name`, `category`, `color`, `icon`. |
| `list_projects` / `create_project` | `slug`, `name`, `icon`, `color`. |
| `update_project` | `id` plus name, description, doc_link, start_date, target_date. |
| `create_milestone` | `project_id`, `name`, `target_date`. |
| `list_cycles` / `create_cycle` | `start_date`, `end_date`, `name`, `icon`, `color`. |
| `list_labels` / `create_label` | `name`, `color`, `icon`. |
| `list_pins` / `pin` / `unpin` | `pin`: `kind` (Issue, Project, View, Cycle, Note, Url), `target`, `title`; `unpin`: `id`. |
| `import_csv_plan` | Dry run. `csv` text, `map`. Returns a plan. |
| `import_linear_plan` | Dry run from Linear using the server's `LINEAR_API_KEY`. |
| `import_apply` | Write exactly the plan returned. `plan`. |
| `insights_progress` | `scope` (`{"Project":id}`, `{"Cycle":id}`, `{"View":id}`), `measure`, `from`, `to`. |
| `insights_velocity` | `scope`, `measure`, `periods`. |
| `list_view_templates` / `save_view_as_template` / `create_view_from_template` / `reset_view_to_template` / `delete_view_template` | View templates: `view_id`, `template_id`, `name`, `id`. |
| `list_templates` / `create_template` / `update_template` / `delete_template` | Issue templates: `name`, `slug`, `icon`, `color`, `body`, `id`. |
| `apply_template` | `id`, `overrides` (NewIssue fields). |
| `undo` / `redo` / `undo_status` | One tool call is one step. |
| `workspace_status` / `workspace_setup` | `workspace_setup`: `name`, `key`, `timezone`. |

## How do I ...

1. **Create an issue fast?** Press `c`, type `Fix login p2 #web @bug tomorrow`, press `enter`.
2. **Create several in a row?** In quick-add press `shift+enter` to keep the dialog open.
3. **Create from a template?** `c`, then type `/bug Login fails`.
4. **Set a due date with the keyboard?** Focus the issue, press `d`, type `fri` (or `+3d`, `nov 28`), press `enter`.
5. **Set a start date?** `shift+d`, same typed dates.
6. **Change status?** Focus the issue, press `s`, pick one.
7. **Set priority?** `p`, pick Urgent, High, Medium, Low or No priority.
8. **Add or remove a label?** `l`, pick "Add bug" or "Remove bug". Several issues at once: select first.
9. **Move several issues to a project?** `x` on each (or `shift+j`), then `m`, pick the project.
10. **Select everything in a view and change its status?** `mod+a`, `s`, pick.
11. **Set an estimate?** `e`, pick or type `1h30m`. Points: `shift+e`, pick or type `5`.
12. **Make an issue a sub-issue?** `shift+p`, pick the parent. Or add it from the parent's Sub-issues box.
13. **Say one issue blocks another?** Focus it, `r`, pick "Blocks ABC-2 ...". See it as an arrow on the timeline.
14. **Rename an issue in a list?** `f2`, type, `enter` (`escape` cancels).
15. **Open an issue in a panel / full page?** `enter` / `mod+enter`. Close with `escape`.
16. **Copy an issue's key?** `mod+c`. Its link: `mod+shift+c`.
17. **Archive and restore?** `#` archives. Restore: undo with `mod+z`, or the archive button on the issue page.
18. **Undo a bulk edit or an import?** `mod+z` (or `u`); after an import, "Undo import".
19. **Switch layout?** `v b` for board, `v l` list, `v t` table, `v g` timeline, `v c` calendar.
20. **Filter a view to one project?** `f`, choose Project, tick the project.
21. **Group a board by priority instead of status?** Group button, choose Priority.
22. **Sort a list by due date?** Sort button, choose Due; turn on Descending to reverse.
23. **Show a custom property on board cards?** Switch the view to Board, open Display, find the property under "Custom
    properties", turn its switch on (on a board this edits the card fields).
24. **Reorder the fields shown?** Display, focus a row, `alt+arrowup` / `alt+arrowdown`, or drag.
25. **Start a view from a template?** Views page (or view menu) > New view, pick the template, "Create view". Make the
    template first with "Save as template" on a view you like.
26. **Make a template the default for new views?** Views page > Templates > "Set as default".
27. **Save a changed copy of a view?** View menu > "Save as new view".
28. **Pin something?** The pin icon on a sidebar row or view header; `.` for an issue. See them on Pinned (`g h`).
29. **Move unfinished issues to the next cycle?** Cycles page > "Move unfinished to next cycle" (or the same command in the command menu).
30. **See a burn-up for a project?** Open the project page, press `shift+i`, choose Chart "Burn-up line" and a Measure.
31. **Turn on points?** Settings > General > Estimation: Points or Both. Pick a Point scale. Then `shift+e`, or `5pt` in quick-add.
32. **Go to a place quickly?** `g m` My issues, `g p` Projects, `g c` Cycles, `g v` Views, `g t` timeline, `g s` Settings.
33. **Find an issue by words?** `mod+k` and type, or `taskapp issue search <text>`.
34. **Reschedule on the timeline?** Switch to Timeline, drag the bar or its handles; Apply the "Also moves" toast if shown.
35. **Schedule an undated issue?** Timeline: drag it from Unscheduled onto a day. Calendar: drag it from "Issues to schedule".
36. **Create a custom property?** Settings > Properties, name, key, kind, "Add property".
37. **Add a milestone?** Project page > Milestones, name and date.
38. **Import tasks from a CSV?** Settings > Import, choose the file, map columns, Dry run, Import. Undo import if wrong.
39. **Back up now?** Settings > Backups, set a folder, "Snapshot now".
40. **Restore a backup?** Quit the app, `taskapp backup restore <file> --yes`.
41. **Change the theme?** Top-bar theme button, or Settings > General > Theme.
42. **See every key?** Press `?`.
43. **Reset what a view shows?** Display > "Reset to default"; or "Reset to template" in the view menu.
44. **Change the workspace key?** Settings > General > Workspace key; old issue keys keep working.
45. **Use Cairn from an agent?** Point an MCP client at `taskapp-mcp --db <path>` (README.md has the config) or call the CLI with `--json`.
46. **Pick an icon?** See Statuses, labels, icons and colours: today only Settings (statuses, labels, projects, cycles,
    templates, properties) has the picker.

## Limits and not built yet

From PROGRESS.md "After this plan" and from reading the code:

- Not built: themes (only light, dark, system); agents as assignees; decision tickets; cost per issue; multi-device
  sync; a phone app; recurring tasks; remind dates and snooze; an inbox view (the page is a stub); a weekly review
  screen; relative dates in filters ("due within 7 days" is a fixed date); changing the time zone without a restart;
  keyboard resize of panels.
- Later, from the owner: ephemeral view filters (today every filter chip is saved with the view); resizable panes by
  dragging (today only the issue and insights panels have a grip, mouse only, the sidebar cannot be resized); compact
  link previews (title, summary, image; they need a network fetch, so none is built); picking icons where the thing is (today the picker is only in Settings); an
  Interface size setting.
- Single user, one Mac, one writer. No signed or notarized build and no licence yet.
- A new database seeds the issue prefix `ZF` until setup runs.
- Not verified against real accounts: Apple Reminders import, Google Calendar sync, GitHub export, a restore from the export.
- The CLI cannot create statuses or milestones, change settings, or unarchive; the MCP server cannot read settings.
