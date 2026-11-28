# Using Cairn

How to use the app, written from the code. If you are an agent asked "how do I do X in Cairn", read the section that
matches and answer from it. Every key, setting, CLI command and MCP tool named here exists in the code; where a
feature is only partly built, the text says so. The keyboard tables were generated from the command
registry (`ui/src/lib/commands/defs/**`); `ui/src/lib/commands/guide.test.ts` keeps them in sync.

Contents: [What it is](#what-cairn-is) - [First run](#first-run) - [The screen](#the-screen) - [Issues](#issues) -
[Selection and bulk edit](#selection-and-bulk-edit) - [Undo and redo](#undo-and-redo) - [Views](#views) -
[Layouts](#the-five-layouts) - [Projects, milestones, cycles](#projects-milestones-cycles) - [Events](#events) - [Decisions](#decisions) -
[Points and insights](#points-and-insights) - [Custom properties](#custom-properties) -
[Statuses, labels, icons, colours](#statuses-labels-icons-and-colours) - [Templates](#issue-templates) - [Images](#images) -
[Import, export, backups](#import-export-and-backups) - [Restore](#restore) - [Reminders and Google Calendar](#reminders-and-google-calendar) -
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
The same screen has a second choice, "Restore from an export": see [Restore](#restore).

## The screen

- **Sidebar (left).** Top: My issues. Then four collapsible sections with counts: Pinned, Views, Projects (with "All
  projects"), Cycles (with "All cycles"). Then Events, Decisions and Dashboard (Settings > Features can hide each of these). Bottom: Settings. Drag a pin or a view to reorder it. Right-click a view, folder,
  project or cycle and choose Pin or Unpin. The Projects section shows only "All projects" until you right-click its
  heading and choose "Show projects in the sidebar" (kept on this device); pinned projects always show under Pinned. With "Sidebar sparklines" on (Settings > General) each project
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
| `+slug` | Project by slug. An unknown slug goes back into the title; nothing is created. | `+website` |
| `#name` | Label by name. An unknown label goes back into the title. Repeat for several. `@name` also works as a label. | `#bug #urgent` |
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

Quick-add has no token for status, parent, cycle or milestone. Example: `Pay rent friday p1 +home #bills ~30m 3pt`.

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

### Checklists in descriptions

A line `- [ ] item` in a description is a checklist item; `- [x] item` is ticked. Click the box in the description to tick it
or untick it; the tick rewrites that one line and is one undo step. In the editor's Preview you tick the draft, and Save keeps
it. Nested items count, and lines inside a code block do not. A list row, table row or board card shows `done/total` when the
issue has items (the Checklist field, on by default). `taskapp issue get` reports `checklist: {done, total}`.

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
- **Relations.** Kinds: blocks, related, duplicate, follows, caused by, part of, plus your own. Each reads one way from
  the first issue and the other way from the second (blocks / blocked by). Add from the panel's Relations section, or
  with `r`. Manage kinds in Settings > Relations: add one with its two labels, rename it, delete it once no relation uses
  it. Built-in kinds cannot be changed. A view can filter on "Relation kind" (issues with a relation of that kind, on
  either end).
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
MCP call is one step. After a change a toast offers Undo. Toasts stack (at most three) and each leaves six seconds after it appeared. The
toast "Nothing to undo" shows when the stack is empty.
Undo refuses when something changed since. Inside a text field `mod+z` is the field's own undo. CLI: `taskapp undo`,
`taskapp redo`; MCP: `undo`, `redo`, `undo_status`. An import is one step ("Undo import" button after it).

## Views

A saved view is a definition: filter, sort, group, layout, displayed fields, and display options. Edits to a view are
saved automatically (after a short pause), except temporary sorting changes. Filter changes show an "Undo" toast.

- **Make one.** Views page > New view, or the view menu > "New view..." (choose Blank, a copy of the current view, or
  a template), or "Save as new view" (copies the current one).
- **Folders.** Every sidebar section (Pinned, Views, Projects, Cycles) can hold folders, and folders can sit inside
  folders, up to three deep. Make one from the section's "..." menu, by right-clicking empty space in the section, from
  a folder's menu ("New folder inside"), or from the command menu ("New folder", for the page you are on); name it in
  the dialog. Click a folder to collapse it; which folders are collapsed is remembered on this device only. A folder
  only holds items of its own section. A folder can be pinned (right-click > Pin): Pinned then shows it as a folder row
  with its contents, always current, and unpinning leaves the folder as it was. Rename or delete from the folder's right-click menu; deleting moves what is in
  it up one level, it never deletes the contents. Three ways to move something, each one undo step:
  drag the row (press, move a few pixels, release; a ghost follows the pointer). Drop on a folder to put it inside
  (hold over a collapsed folder and it opens); drop between rows to reorder, where the line shows the place; drop in
  the "Move to top level" strip at the end of the section to take it out. Esc cancels the drag. Or right-click the row
  and choose "Move to folder" (the folders are listed by depth, plus "Top level" and "New folder..."). Or open a view
  or project and use the command menu's "Move to folder...". The Views, Projects and Cycles pages show the same
  folders, nested and indented.
- **Layouts.** Tabs in the header, or `v l` list, `v b` board, `v t` table, `v g` timeline, `v c` calendar.
- **Filter.** "Add filter" (`f`) lists Status, Status category, Project, Label, Priority, Due and every custom property.
  Each filter is a chip: click it to change the operator and values; the x removes it. Operators: is, is not, is any
  of, is none of, contains, before, up to, after, from, is empty, is set. Chips combine with AND. A filter that
  uses OR or NOT (made by CLI or MCP) shows as one "Advanced filter" chip you can only remove. Filters are saved
  with the view (see [Limits](#limits-and-not-built-yet)). The full field list a view definition accepts: Title,
  Status, StatusCategory, Priority, Project, Milestone, Cycle, Label, Parent, StartDate, DueDate, Estimate, Points,
  CreatedAt, UpdatedAt, CompletedAt, Archived, Key, SubIssueProgress, Checklist, RelationCount, CommentCount, and any
  custom property.
- **Relative dates.** On a date field (Start, Due, Created, Updated, Completed, a date property) the chip's first
  line picks "Exact date" or a relative day: today, 7 or 14 or 30 days ago, in 7, 14 or 30 days. Relative days count
  from today in the workspace time zone and move on their own. "Before today" lists overdue dates; "up to in 7 days"
  includes overdue dates too. A view definition writes the same value as `today`, `today+7` or `today-14`.
- **Quick filter.** The filter icon in the header (`shift+f`) opens a bar of temporary filters: a text box that matches
  titles, and chips for Status, Priority, Project, Label and Due. They narrow the view while the bar is open and are not
  saved with it; the bar says "Temporary". "Save to view" keeps them as part of the saved filter. Esc, the x, or
  leaving the view clears them.
- **Group.** "Group" button (not on the calendar): choose any of Status, Status category, Priority, Project, Cycle,
  Due, Title, Points (if on) or a custom property. Group headers collapse; the list adds an issue to a group.
- **Sort.** Choose a field and direction with Sort, or click a sortable list or table heading for ascending, descending,
  then the saved order. Sorting lasts until you leave the view; it survives layout changes and live updates but does not
  change the saved definition. Grouped rows stay within their groups and nested issues stay with their parents.
- **Save to view.** When the sort or the quick filter differs from the saved view, the header shows "Save to view" and
  "Reset to saved". "Save to view" writes both into the saved view in one step (undo reverts it); "Reset to saved" drops
  them. Groups and the filter chips were always saved as you change them and are unaffected.
- **Display** (button, or the command "Display options..."). Switch fields on or off and drag (or use the arrow
  buttons, or `alt+arrowup` / `alt+arrowdown` on a focused row) to reorder: Key, Priority, Status, Labels, Project,
  Milestone, Cycle, Parent, Start, Due, Estimate, Points, Sub-issues, Relations, Comments, Created, Updated, and your
  custom properties (listed first under "Custom properties"). "Reset to default" restores the layout's default fields.
  Options: Compact rows (list, table, board), Show empty groups (list, table), Show archived, Sub-issues Nested, Flat or
  Hidden (list), Show weekends (timeline, calendar). On a board the field list means the card fields.
  Wrap (list, table; off by default) lets long titles, several labels and long values run onto more lines instead of
  being cut off; a wrapped list shows every row, without the windowing that keeps long plain lists fast.
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
  appears. Drag an issue from the Unscheduled band onto a day. Milestones show as a diamond with its name on the
  project's row, at its target date; undated milestones are not drawn. Click a diamond to open the project at its
  Milestones. There is also an all-projects timeline (`g t`).
- **Calendar.** Week or Month (tabs, or `w` and `m`), previous and next (`[` and `]`), today (`t`). Issues appear on
  their due date. Drag an issue from the "Issues to schedule" side list onto a day to set its due date, or onto a time
  slot to create a one-hour work block. Drag items to move or resize them.

## Project views

A saved view can list projects instead of issues. It opens at `/view/<id>` like any view and has the same five layouts,
filter chips, Group, Sort and Display.

- **Make one.** Views page > New view > choose Projects; the command menu "New project view"; right-click the
  Views heading's "..." menu in the sidebar > New project view. On the Projects page, "Save as view" keeps its current
  filters, sort and grouping as a project view (a table). A project view shows in the sidebar among the other views
  (marked with a small project icon) and sits in the same folders; the Views page lists both, with a filter All views /
  Issue views / Project views. From outside: `taskapp view save NAME --def JSON --entity project`.
- **Fields.** Name, Status, Status category, Priority, Labels, Start, Target, Created, Updated (any project write moves it), Progress (percent of issues done),
  Progress chart (off by default; Display > Hidden: the percent plus grey scope and coloured completed lines over time,
  when Progress charts is on), Issues (count) and custom properties that apply to projects. Issue views can also filter by "Project status".
- **Layouts.** List: status, priority, name, then the fields, in bands when grouped. Table: the columns chosen in
  Display. Board: a column per project status (or priority, or a select property, from Group); dragging a card to
  another column sets that field, one undo step. Timeline: a bar from start to target date per project, milestones as
  diamonds and a today line; drag a bar or its ends to change the dates. Every project has a row: a project with only a
  start date runs from it to today (or its latest dated issue) with a faded end, one with only a target date gets a week
  before it, and a project with no dates shows "Set dates", which opens its project page. Calendar: a month grid with a chip on each
  project's start and target day. Clicking a row, card, bar label or chip opens the project.
- **Display.** Which fields show, compact rows, empty groups and "Show archived" (archived projects are hidden by default).
- **Not available.** Templates and Insights are for issue views; a project's created and updated times are not fields yet.

## Projects, milestones, cycles

- **Projects.** Projects page > New project (name, slug). Settings > Projects also adds and deletes. A project page
  shows a header (name, description, "Open doc" when a doc link is set, progress, target), milestones, a layout tab
  strip (with its own filter, group, sort and display controls) and the project's issues; `shift+i` toggles the
  insights panel. Rename, pin and archive are in the header. Archiving asks first, keeps the issues, and can be
  restored. Fields: slug, name, description, doc link, start date, target date, icon, colour. A project can also scope
  a custom property.
- **Project status, priority, labels and properties.** A project has a status of its own (Settings > Project statuses:
  Planned, In progress, Paused, Done, Cancelled; add, rename, recolour, reorder, delete moves the projects to another
  status), a priority (the same scale as issues), labels (the same labels) and any custom property set to apply to
  projects. A "Project details" plate on the project page edits all of them; each edit is one undo step. In the command
  menu on a project page: "Set project status", "Set project priority", "Add or remove project labels" and "Set project
  property" (Select and Checkbox only). The Projects page shows Status, Priority, Labels and a column per project
  property, and groups by status or priority, sorts, and filters by status, priority or label. The sidebar shows each
  project's status glyph. CLI: `project statuses`, `project set <slug> --status --priority --label --property`, plus `--name --description --icon --color --start --target` (`taskapp icons` and `taskapp colors` list the choices); MCP:
  `list_project_statuses`, and `update_project` takes `status_id`, `priority`, `label_ids`, `properties`. Export writes
  `project_statuses.yaml` and the fields into each project file.
- **Milestones.** On a project page, "Milestones": add a name and a date. An issue is assigned a milestone in the
  issue panel (after choosing the project) or with the "Change milestone" command. Milestones draw as markers on the timeline.
- **Cycles.** Fixed, numbered, non-overlapping date ranges. Cycles page > New cycle (optional name, start, end), or
  Settings > Cycles. The current cycle shows a burn-up chart, past cycles a spark line and the page a "Completed per
  cycle" bar chart, when Progress charts is on. "Move unfinished to next cycle" (button, or the command) moves unfinished issues
  to the next cycle. Set an issue's cycle with `shift+c`.
- **Cycle retrospective.** Click a cycle on the Cycles page to open its retrospective above its issues: a one-sentence
  goal (set at planning), Goal met yes or no, a 1 to 5 rating, notes, and the cycle's own numbers (throughput, say/do and
  scope change, AI vs me, flow efficiency, blocked work). In the command menu: "Set cycle goal...", "Was the cycle goal
  met...", "Rate the cycle..." (they act on the open cycle, else the running one) and "Open cycle retrospective...". On the
  Dashboard, a cycle chosen in Range shows a Retrospective button. For AI: `taskapp cycle set NAME --goal ... --goal-met
  yes|no --rating N --notes ...`, and `taskapp metrics --cycle NAME` for the numbers.
- **Pins.** Pin issues, projects, views and cycles (CLI also notes and URLs). The Pinned page lists them; `g h`.

## Events

An event is a dated thing (a meeting, a launch, a deadline day) that many issues can be linked to, and one issue can
belong to many events. Times are wall-clock times with no zone.

- **Events page** (sidebar "Events", `g e`): upcoming events grouped by day, with past ones behind a "Past" toggle.
  "New event" (or the command) asks for title, date, optional end date, all day or a from and to time, place and notes.
  Right-click an event for Open, Edit, Delete and Link an issue; the command menu runs the same actions on the event
  you right-clicked last.
- **An event** (click it, `/event/<id>`): details, Edit, Delete (undoable), and the linked issues with "Link an issue"
  (search by text) and a remove button on each.
- **On an issue**: the Events section lists its events, with "Add to event" and a remove button on each.
- **Calendar layout**: events show as their own chip (calendar icon, accent edge); click one to open it. They are not
  dragged from the calendar and do not follow the view's filter.
- Undo covers create, edit, delete, link and unlink. The export writes one `events/<id>.yaml` per event and a rebuild
  restores it with its links.
- CLI: `event list`, `event create`, `event link`. MCP: `list_events`, `create_event`, `link_event`.
- Google Calendar: when Calendar sync is on, each event is also written to your Cairn calendar. A timed event keeps its
  start and end (no end means one hour), an all-day event stays all-day (over several days if it has an end date).
  Editing or deleting (or archiving) an event updates or removes it there. Cairn events have no reminder, so none is
  written; the popup is only for tasks.
- Not built: recurrence, reminders, and searching events.

## Decisions

A decision is a question with options that someone is waiting on you to answer: an agent raises one instead of
guessing, or you note one for yourself. It has a question, markdown context, options (each with a trade-off), an
optional recommendation, and a mode: **blocking** (whoever raised it waits for your answer) or **assume** (it carries on
with its recommendation until you answer). Decisions are Cairn's own; nothing outside Cairn is called.

- **Decisions page** (sidebar "Decisions", `g q`): one card per open decision, blocking ones first with a red edge. Click
  an option to answer; the optional note box is sent with the answer. "Or answer in your own words" sends free text when no
  option fits. "Withdraw" closes a question that no longer applies. The sidebar shows how many are open (red when one is
  blocking). The page refreshes when the window gains focus and every minute. The "Answered" tab keeps the closed ones.
- The command menu has "Answer decision with the recommendation" and "Withdraw decision" for the card you last
  clicked (the first open one before any click).
- **On an issue**: open decisions that name the issue are listed in a Decisions section.
- Undo covers create, answer, withdraw, link and unlink. The export writes `decisions.yaml` and a rebuild restores it.
- CLI (add `--json` for scripts):
  `taskapp decision create "Which database?" --option a="Postgres: heavier" --option b="SQLite: simpler" --recommend b --mode blocking --issue ABC-1`,
  `decision list [--status open|answered|withdrawn|all]`, `decision get <id>`,
  `decision answer <id> (--option KEY | --text TEXT) [--note ...]`, `decision withdraw <id>`. Agents can also call the
  `decisions_*` operations through `taskapp api call`.
- Reserved for integrations: `source` names who raised a decision (`cli`, `ui`, `agent`, or any name you pass with
  `--source`, such as `agent:builder` or later `curia`), and `external_id` is the raiser's own id, unique per source, so a
  later integration can mirror a decision and not raise it twice. Nothing reads them yet.

## Dashboard

The Dashboard (sidebar, or `g b`) shows how work is going over a range. Pick the range at the top: Today, This week,
This cycle, any other cycle, Month, Quarter, Year, or Custom with two dates. The chart steps by day up to 31 days, by
week up to about six months, and by month beyond. Limit it to a project or a label, and switch Count or Points.
Cards: Summary (completed, created, in progress now, lead time and cycle time as median and 85th percentile;
a note appears when fewer than 5 issues finished), Burn-up (scope against done), Throughput (completed per day, week
or month; points when Points is on), Created vs completed (issues, always counts), Velocity (the last five cycles, with
their average as a dashed line), AI vs me (created and completed split into you in the app, AI and tools, imported,
and other), By label (the top five labels by completed, the rest as Other), Cycle time (one dot per finished issue,
by the day it finished against its days; the lines mark the median and the 85th percentile, and "85% finish within N
days"; click a dot to open the issue), Lead time (created to finished), and Work in progress (now, and per period with
Little's Law: average cycle time = WIP / throughput), Cycle commitment (pick a cycle in Range: say/do and scope change),
Blocked work (issues waiting on an unfinished blocker, approximate blocked time) and Flow efficiency (active time over
cycle time, and which Waiting statuses held work longest). The Cards button shows, hides and
reorders cards, kept on this device. Every choice is also in the command menu ("Dashboard range", "Dashboard: show or
hide cards"). Settings > Features > Dashboard turns the whole page off; it is on by default.

### Stale issues

An issue is stale when it has sat in its status longer than that status's nudge days (Settings > Statuses). The clock
starts at the issue's last status change; an issue with no status history (an import, for example) uses its last
update. A status with no nudge, or 0 days, never makes issues stale, so nothing lights up until you set nudges, and
imported Linear issues stay quiet unless their status has one. Stale rows in the table carry a quiet "stale" tag. The
Stale card on the Dashboard counts them by status and lists the oldest first; click a key to open the issue. The CLI
prints the same list: `taskapp stale`.

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
- **Progress charts** switch (Settings > Features): burn-ups on project and cycle pages, sparklines on past cycles.
- CLI: `insights progress` and `insights velocity`; MCP: `insights_progress`, `insights_velocity`.

### Metrics
Delivery numbers for a range of days or one cycle, rebuilt from the change history (nothing is stored). The
registry command is `metrics_for` (query: scope Range or Cycle, bucket Day, Week or Month, optional filter, measure
`count` or `points`); the CLI is `taskapp metrics`. Per bucket: issues created, completed (and their points), scope and
done so far, and work in progress. Over the whole range: lead time (created to done) and cycle time (first move into a
Started status to done) as p50 and p85 calendar days in the workspace time zone, work in progress now, completed per
label and per source (who made and who finished the issue: Ui, Cli, Agent, Migration), and the points and count of
the last six finished cycles. Only Done counts as finished; archived and canceled issues are left out. Issues without
a Started move (including Linear imports, which have no history) have a lead time but no cycle time. `thin` is true
when fewer than five issues finished in the range: read the numbers as anecdotes. With a cycle, only its members are
counted; velocity always looks across cycles.

### Forecast
When will the open issues be done? The registry command is `forecast_for` (query: optional filter, optional cycle);
the CLI is `taskapp forecast [--project NAME | --cycle NAME]`. It replays the last 56 days of completions for the
filtered issues, then simulates the open issues (a cycle's members, when a cycle is given) 1000 times, each day
finishing as many as a randomly drawn past day did. The answer is the day by which half (`p50`, "likely by") and 85 in
100 (`p85`) of the runs are done. Fewer than 10 finishes in the window gives `enough: false` and no dates, and
nothing open gives no dates either. The same data always gives the same answer. The Dashboard has a Forecast card (for
its project and label filters, or the cycle when the range is a cycle), and each project page shows one line:
"At the current pace: likely by Jan 12, 85% by Jan 20."

## Custom properties

Your own fields on issues, on projects, or on both (or, for issues, just one project's). Kinds: Text, Number, Date, Select, MultiSelect, Checkbox, Url,
NoteLink. Create in Settings > Properties (name, key like `effort`, kind, applies to (Issues, Projects or Both), scope, and options for Select kinds), or
CLI `property create`, or MCP `create_property`. Set a value in the issue panel, in a table cell, with `a` (Select and
Checkbox only), or CLI `property set`. Show a property in views through the Display panel; filter, group and sort on
it. Changing a property's kind or deleting it takes a database snapshot first (and deleting removes every issue's
value).

## Choosing which fields issues have

Settings > Fields lists the built-in fields: priority, project, milestone, cycle, labels, start, due, estimate, points
and parent. Each has a switch, and the arrows (or dragging a row) set the order the issue panel lists them in. A field
that is off is hidden from the issue panel, quick-add tokens (the token stays in the title), filter, group and sort
choices, the Display panel, table columns, list and board fields, its issue commands and their rows in the shortcut
sheet. Issues keep their values, so switching a field back on brings them back. Status and title are always on. Points
are also off whenever Estimation (General) has no points. The list is the `issue_fields` setting.

## Statuses, labels, icons and colours

**New issues start in** (top of Settings > Statuses) picks the status an issue gets when none is given: quick add,
templates and imports. Unset, or if that status is deleted, new issues start in the first status.

- **Statuses** (Settings > Statuses): grouped by category; add a name with an icon and colour; delete asks which status to
  move that status's issues to. Each status shows its own icon and colour (from the same picker) wherever its glyph appears;
  without one it shows the default circle for its category: backlog dashed, unstarted empty, started half-filled, completed
  filled with a check, canceled with a cross.
  Statuses always list by category (Backlog, Unstarted, Started, Completed, Canceled), then by their position inside it,
  in every picker, group header and column.
- **Status definitions** (same page): each status row also holds a short definition, a flow kind and an optional nudge.
  The kind says how time in the status counts for flow efficiency: Active (work is happening), Waiting (blocked on someone
  or something) or Idle (not being worked). New and existing Started statuses are Active, all others Idle. "Nudge after N
  days" is stored for stale-issue nudges (0 or empty means never). CLI: `taskapp status list` shows the fields,
  `taskapp status update NAME --definition "..." --kind active|waiting|idle --nudge-days N`.
- **Priority** shows as three bars of rising height: none faint, low one bar, medium two, high three, urgent three in red.
- **Labels** (Settings > Labels): name, icon, colour. Add to issues with `l`, the panel, or `#name` in quick-add.
- **Colours** are a fixed palette of 12 keys: gray, red, orange, amber, yellow, green, teal, cyan, blue, indigo, purple,
  pink. A `#rrggbb` value sent by CLI or MCP maps to the nearest key.
- **Icons**: `icon:<id>` from the built-in set or `emoji:<emoji>` (any 1 to 8 non-ASCII characters). Today the
  icon and colour picker exists only in Settings: the "icon and colour" button next to the name field when you add a
  status, label, project, cycle, template or property. Click the icon of a view, project or cycle in the sidebar, of a
  view or project in its page header, or pick "Change icon" in the command menu on a view or project page: the same
  picker opens and saves. CLI and MCP can set icons when creating projects, labels, cycles, statuses and templates.

## Hover actions and right-click menus

- Point at a sidebar row (view, project, cycle), a view on the Views page, or an issue row in a list, and a single "…"
  button appears at its right edge. Tab to a row to show it from the keyboard. Clicking the rest of the row opens it as
  before. The "…" opens the row's actions menu.
- Right-click a sidebar row or a Views page row, or press Shift+F10 or the menu key on it, for the same actions as a
  menu. Views: rename, duplicate, pin or unpin, change icon, delete (delete asks first and keeps the issues).
  Projects and cycles: rename, pin or unpin, change icon, delete (the same confirm as Settings).
- Issue rows in a list: open, set priority, pin or unpin, delete (archives, as the command is named). Set status with
  the status glyph. Right-click still opens the issue menu.
- Projects page, Cycles page, Settings > Labels and Settings > Templates rows: rename, change icon, delete (and pin for
  projects and cycles). Each action is a registry command; the command menu does not list them because they need a row.
- Every row's icon opens the icon and colour picker; its choice is saved and shown after a reload.

## Issue templates

Settings > Templates. A template has a name, a slug (used as `/slug` in quick-add), icon, colour, a title prefix, default
fields (status, priority, project, labels, estimate, points, properties) and sub-issues (each with its own title and
fields). Creating from a template makes the issue and its sub-issues as one undo step. Use it with `/slug ...` in
quick-add, `taskapp template apply <id> [title]`, or MCP `apply_template`.

## Images

Paste an image (a screenshot, a copied picture) into a description or a comment box, or drop an image file on it: it is
stored and `![name](attachment:3)` is written at the cursor. **Attach image** under the box (also in the command menu while
you are in a box) opens a file picker for the same thing. Pasted text works as before. PNG, JPEG, GIF and WebP up to
10 MB; the file's own bytes decide the type, so SVG and anything else is refused with a message and nothing is inserted.
The same picture added twice is stored once. Click a picture to see it larger. Pictures from the web are never shown
(they would tell the site you opened the issue); only `attachment:` images are.

Files live in an `attachments` folder next to the database. A GitHub export copies the pictures that some description
or comment still uses into `attachments/` (listed in `attachments.yaml`), and a restore, rebuild or pull brings them back.
Backups: the nightly and snapshot copies also copy new pictures into `attachments/` inside the backup folder (one copy
per picture, never pruned); a database restore from a backup needs that folder copied back next to the database.
Adding a picture is not an undo step (the text edit that uses it is). A picture nothing refers to any more stays on disk;
a cleanup command is not built yet.

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
  against a real repository. The export carries everything but secrets, machine-local settings and the raw change
  history, including each issue's work blocks. It also writes `local.yaml`: this machine's export folder, remote
  (a password or token inside the address is removed) and backup folder, which a restore offers back but never forces.
- **Two machines** (same remote on both): before it writes, an export fetches the remote. If another machine pushed
  first, nothing is overwritten; Settings > Export shows the message and "Pull and apply" fast-forwards the folder and
  applies the incoming issues in one undoable step, "Sync from remote". The newer edit (by updated time) wins per
  issue; conflicts are listed under "Last pull from the remote". New issues, comments, relations and work blocks are
  added. Statuses, labels, projects, settings and deletions are not carried over yet (docs/decisions/0017). If both
  machines have unpushed commits the export stops with a message; merge them in the export folder with git.
- **Restore from an export**: in the app, see [Restore](#restore). From the terminal,
  `taskapp restore --from <export folder> --db <new database path>` builds a fresh database from the files and refuses
  to overwrite an existing one; quit the app first and point `--db` at a path that does not exist.
- **Backups** (Settings > Backups): set a folder, "Snapshot now" writes a copy of the database to `snapshots/`; a nightly
  copy goes to `nightly/`. The newest 14 nightly and 30 snapshot files are kept. Before risky changes (property type
  change or delete, a bulk update over 10 items) a snapshot is taken. To restore: quit the app, then
  `taskapp backup restore <file> --yes`. `taskapp backup list --dir <folder>` lists backups.

## Restore

Loads a whole workspace back from an export, from inside the app. Two ways in: on the first-run screen choose
"Restore from an export", or in a workspace that already has data open the command menu and run "Restore from
export...". Type the export folder, or the Git address of the repository it was pushed to (`https://...`,
`ssh://...` or `git@host:path`). An address is cloned with your own `git` and your own sign-in into the app's data
folder; no password or token is ever stored. A folder with a link in it is refused.

The command asks first: it replaces everything in the workspace, and it saves a backup of the current database
(in the backup folder, `snapshots/`) before it replaces anything. A broken export changes nothing. Afterwards the export
settings point at the restored folder (and the address it came from), so the next export continues the same history.
"Also restore the export's remote and backup folder" applies `local.yaml`; leave it off to keep this machine's own.

Comes back: issues with their descriptions, comments, labels, parents, relations (and custom relation kinds), work
blocks, projects and milestones, cycles, events and their issue links, statuses, views and view templates, issue
templates, pins, custom properties, the import log, and the settings the export carries (workspace name, key, time
zone, theme, estimation and the like). Issue numbers stay; row ids inside the database are new.

Never comes back: the change history behind undo (it names rows by internal id, so it could not be replayed safely;
the old history is cleared on restore), the Google and Reminders sign-ins and tokens (connect them again in Settings), and anything in a setting named like a
token, secret, path or remote, except what `local.yaml` offers. A restore that you did not ask for can be undone by
restoring the backup it wrote (`taskapp backup restore`).

## Reminders and Google Calendar

- **Apple Reminders capture** (Settings > Reminders): press Authorize (macOS asks for access to Reminders), then pick
  the "Inbox list". Reminders you add to that list (phone, Siri, share sheet) are imported as issues: the title is
  parsed with the quick-add grammar, notes become the description, a due date becomes the due date (as described in docs/DESIGN.md). Needs the Swift
  helper (`scripts/build-helper.sh`) and your Apple account's Reminders. Not verified against a real list.
- **Google Calendar** (Settings > Calendar): "Connect Google Calendar", choose a calendar (use a dedicated one),
  turn on "Enable notifications". Sync is one way: the app writes events and overwrites manual edits to its own events.
  Needs your Google Cloud OAuth client and a Google login (setup steps in docs/gcal.md). Not verified against a real account.
- The legend bar shows the sync state; a sync error appears there. If Reminders access is off (a fresh build), it says
  "Reminders access is off: press Authorize in Settings > Reminders".
- **Sync now**: the command menu has "Sync now" (runs every sync step in the app). From the terminal,
  `taskapp sync now [--helper PATH]` runs the Reminders import and the Google Calendar update once against the
  database, because the CLI cannot reach the running app. The helper defaults to
  `/Applications/taskapp.app/Contents/MacOS/reminders-helper`. Both steps do nothing when their setting is off.

## Settings

Settings is a list of sections: General, Features, Statuses, Project statuses, Labels, Projects, Cycles, Fields, Properties, Relations, Templates, Import, Reminders,
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
| Sidebar sparklines | on or off | A small progress line per project. |
| Interface size | 100%, 110%, 115%, 125% | Scales the whole app on this device (not saved in the workspace). Default 110%. Applies at once. |
| Animation speed | Off, Fast, Normal, Slow | Per device, like interface size. Default Normal; Off by default when the system asks for reduced motion. Applies at once. |
| Show tooltips | On, Off | Per device. Off by default: no name or shortcut boxes appear on hover. Shortcuts stay listed in the command menu and the shortcut sheet. Applies at once. |

### Features

Settings > Features lists every optional feature with a switch, all on by default: Dashboard, Decisions, Cycles,
Events, Progress charts (burn-ups and sparklines on projects and cycles), Stale nudges, Reminders capture, Google
Calendar, and the coming Process Inbox, Next actions and Focus mode. Switching one off hides its sidebar entries, its
page (the address shows "is switched off"), its commands in the command menu, its Settings page where it has one
(Cycles, Reminders, Calendar) and its dashboard card. Your data stays and comes back when you switch it on. Each switch
applies at once, has its own command ("Turn Cycles on or off"), and is stored as a setting (`dashboard`,
`insights_charts`, `feature_<name>`; `taskapp api call settings_get`). Show tooltips stays in General: it is per
device and off by default, so it is a preference, not a workspace feature.

From the terminal: `taskapp features` lists them; `taskapp features set cycles off` (names: dashboard, decisions,
cycles, events, charts, stale, reminders, gcal, process_inbox, next_actions, focus).

Sidebar and issue panel widths are remembered per device. Drag an edge to resize; double-click it to reset.

Other stored settings (not all with a screen): backup folder, Reminders list, Google Calendar choice and switch, export
folder, remote and switch, the default view template.

### Motion

Dialogs and the command menu scale up and rise over a fading backdrop, and the command menu's rows and pages slide
in. Menus, popovers, tooltips, toasts and the issue panel animate out as well as in. The issue panel slides from the
right and dims the list; changing page rises the sheet's content (the issue page slides in); sections, sidebar groups
and chevrons ease open and shut; the layout tab underline slides; a row flashes when its status or priority changes
and when quick add creates it. Nothing animates when the app first opens. Settings > General > Animation
speed sets how long this takes (Off, Fast, Normal, Slow). Keys are never delayed by it: a key typed during an animation
lands where it should. The command menu has "Faster animations" and "Slower animations" too.

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
| `sidebar.toggle` | Show or hide sidebar | `mod+\` | Collapse or show the sidebar. |
| `pane.sidebar.wider` | Make sidebar wider | `alt+arrowright` | Widen the sidebar by 16 px. |
| `pane.sidebar.narrower` | Make sidebar narrower | `alt+arrowleft` | Narrow the sidebar by 16 px. |
| `pane.panel.wider` | Make issue panel wider | `alt+shift+arrowleft` | Widen the issue panel by 16 px. |
| `pane.panel.narrower` | Make issue panel narrower | `alt+shift+arrowright` | Narrow the issue panel by 16 px. |
| `size.up` | Increase interface size | `mod+=` | Next interface size step (100, 110, 115, 125%). |
| `size.down` | Decrease interface size | `mod+-` | Previous interface size step. |
| `size.reset` | Reset interface size | `mod+0` | Back to 110%. |
| `help.shortcuts` | Keyboard shortcuts | `?` | Open the keyboard shortcuts sheet (grouped, with a filter box). |
| `issue.create` | New issue | `c` | Open the quick-add dialog. |
| `history.undo` | Undo | `mod+z` or `u` | Undo the newest step. Inside a text field mod+z stays the field's own undo. |
| `history.redo` | Redo | `mod+shift+z` | Redo the step you undid. |
| `settings.save` | Save settings | `mod+s` | Save the open settings page. Works inside text fields. |
| `nav.my` | Go to My issues | `g m` | Go to My issues. |
| `nav.pinned` | Go to Pinned | `g h` | Go to Pinned. |
| `nav.projects` | Go to Projects | `g p` | Go to the project list. |
| `nav.cycles` | Go to Cycles | `g c` | Go to Cycles. |
| `nav.events` | Go to Events | `g e` | Go to Events. |
| `nav.dashboard` | Go to Dashboard | `g b` | Go to the Dashboard (when it is switched on in Settings). |
| `nav.decisions` | Go to Decisions | `g q` | Go to Decisions. |
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
| `issue.relation` | Add relation | `r` | Add a relation of any kind to another issue. |
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
| `view.quickFilter` | Show or hide quick filter | `shift+f` | Temporary filters that are not saved with the view. |
| `view.saveToView` | Save to view | none | Saves the temporary sort and quick filter into the view. |
| `view.resetToSaved` | Reset to saved | none | Drops the temporary sort and quick filter. |
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
| `image.attach` | Attach image | (none) | Choose an image file for the description or comment box you used last. |
| `app.reload` | Reload | (none) | Reload the window. |
| `pane.sidebar.reset` | Reset sidebar width | (none) | Back to the default sidebar width. |
| `pane.panel.reset` | Reset issue panel width | (none) | Back to the default issue panel width. |
| `motion.faster` | Faster animations | (none) | One animation speed step faster (Normal, then Fast, then Off). |
| `motion.slower` | Slower animations | (none) | One animation speed step slower (Normal, then Slow). |
| `tooltips.toggle` | Show or hide tooltips | (none) | Switch the Show tooltips setting. |
| `feature.<name>.toggle` | Turn Cycles on or off (one per feature) | (none) | Switch that feature on Settings > Features. |
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
| `settings.restore` | Restore from export... | (none) | Replace this workspace with an export folder or Git repository, after a backup. |
| `settings.export.pull` | Pull and apply from remote | (none) | On Settings, Export: pull the remote export and apply it. |
| `settings.general` | General | (none) | Open Settings, General. |
| `settings.import` | Import | (none) | Open Settings, Import. |
| `settings.labels` | Labels | (none) | Open Settings, Labels. |
| `settings.projects` | Projects | (none) | Open Settings, Projects. |
| `settings.properties` | Properties | (none) | Open Settings, Properties. |
| `settings.relations` | Relations | (none) | Open Settings, Relations. |
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
| `view.newProject` | New project view | (none) | New view that lists projects. |
| `projects.saveAsView` | Save projects as view | (none) | On the Projects page: save its filters, sort and grouping as a project view. |
| `view.pin` | Pin or unpin view | (none) | Pin or unpin the view. |
| `folder.pin` | Pin or unpin folder | (none) | Pin or unpin the folder; a pinned folder lists its contents under Pinned. |
| `sidebar.showProjects` | Show projects in the sidebar | (none) | List every project under Projects, or only "All projects". |
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
`taskapp <command> --help` print the full list. Keys are issue keys such as `ABC-1`, ids are numbers. Writes that
record history normally use source Cli; some core operations use their own Ui or System source, and some do not
record undoable history. `--migration` explicitly selects Migration for issue creation.

Issue creation accepts status, project, cycle and label IDs or exact case-insensitive names; projects also accept
slugs. Issue updates accept status and project IDs or names. Ambiguous names are rejected; use an ID.

`taskapp guide` shows a short orientation. For operations without a dedicated subcommand, `taskapp api list`
lists the core-backed app commands, `taskapp api describe <name>` shows argument names and Rust types, and
`taskapp api call <name> --args '<JSON object>'` runs one. Use `--json` for machine-readable results and
runtime errors. Optional arguments may be omitted; unexpected arguments are rejected. IDs are numbers. For
complex objects, fetch an existing object first and modify its JSON; field definitions are also in
`ui/src/lib/bindings.ts`. For example:

```sh
taskapp --json api call labels_list
taskapp --json api call labels_update --args '{"label":{"id":1,"name":"Bug","color":"red","color_hex":"#ef4444","icon":"icon:tag"}}'
taskapp --json api call issues_restore --args '{"id":1}'
```

Changing or deleting a property takes a database snapshot first, including through `api`; bulk issue updates
of more than 10 IDs do too. A snapshot failure prevents the operation. API history step labels contain the
operation name, not the argument JSON. The API covers the app's core-backed commands, not shell-only Google
Calendar sign-in/listing, Reminders listing, immediate sync or export actions. Those remain in the app.

| Command | What it does | Example |
|---|---|---|
| `add <text> [--tz]` | Create an issue from quick-add text. | `taskapp add "Pay rent friday p1"` |
| `issue create <title> [...]` | Create with flags: `--description --priority --status --project --due --estimate --points --created-at --completed-at --property k=v --parent --cycle --label --migration`. | `taskapp issue create "Fix login" --priority 2 --due 2026-12-01` |
| `issue update <key> [...]` | Change fields: `--title --description --priority --status --due --clear-due --points --clear-points`. | `taskapp issue update ABC-1 --status 3` |
| `issue list [...]` | Issues by name filters, most urgent first, with status, priority, project and label names: `--project --status --not-status --priority --label` (repeat or comma-separate). | `taskapp issue list --project Cairn --not-status Done,Canceled,Someday` |
| `issue get <key>` | Show one issue, with `status`, `project`, `labels` and `priority_name` as names next to the ids. | `taskapp issue get ABC-1` |
| `issue archive <key>` | Archive an issue. | `taskapp issue archive ABC-1` |
| `event list [--from --to]` | Events overlapping a date range. | `taskapp event list --from 2026-12-01` |
| `event create <title> --start T [--end T --all-day --place --notes]` | Create an event (`T` is `YYYY-MM-DDTHH:MM` or a date). | `taskapp event create Demo --start 2026-12-10T09:30` |
| `event link <id> <key>` | Link an event to an issue. | `taskapp event link 1 ABC-1` |
| `decision create <question> [--option KEY="label: trade-off" --recommend KEY --mode blocking\|assume --context --source --external-id --issue KEY]` | Raise a decision for the owner. | `taskapp decision create "Ship now?" --option y="Yes: fast" --option n="No: safe" --recommend n` |
| `decision list [--status open]` / `decision get <id>` | Read decisions (`open`, `answered`, `withdrawn`, `all`). The table shows options as `a) Label; b) Label`; `get` and `--json` give the trade-offs. | `taskapp decision list --status all` |
| `decision answer <id> (--option KEY \| --text TEXT) [--note]` / `decision withdraw <id>` | Answer or close an open decision. | `taskapp decision answer 1 --option n` |
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
| `project set <slug> [--name --description --icon --color --start --target --status --priority --label --property]` | Change a project's name, look (`--icon` from `icons`, `--color` from `colors`), dates (`YYYY-MM-DD`) or other fields. | `taskapp project set web --icon folder --color teal --start 2026-12-01` |
| `icons` / `colors` | List the icon names and palette colors `project set` accepts. | `taskapp colors` |
| `cycle list` / `cycle create --start --end [--name --color --icon]` | List or create cycles. | `taskapp cycle create --start 2026-12-01 --end 2026-12-14` |
| `label list` / `label create <name> <color> [--icon]` | List or create labels. | `taskapp label create bug red` |
| `comment add <key> <body>` | Comment on an issue. | `taskapp comment add ABC-1 "Looks good"` |
| `relation add <from> <kind> <to>` / `relation remove ...` | Kind: an id or name (blocks, related, duplicate, follows, caused_by, part_of, or a custom one). | `taskapp relation add ABC-1 blocks ABC-2` |
| `pin list` / `pin add <kind> <target> [--title]` / `pin remove <id>` | Kinds: issue, project, view, cycle, note, url. | `taskapp pin add issue ABC-1` |
| `attachment path <id>` | Absolute path of a pasted image's file. | `taskapp attachment path 3` |
| `attachment list <key>` | Images the issue's description points at with `(attachment:N)`: id, path, mime, bytes. | `taskapp attachment list ABC-1` |
| `template list` / `create` / `update <id>` / `delete <id>` | Issue templates: `create <name> <slug> [--icon --color --body JSON]`. | `taskapp template create Bug bug --body '{"title_prefix":"[Bug] "}'` |
| `template apply <id> [title]` | Create the issue and its sub-issues. | `taskapp template apply 1 "Login fails"` |
| `insights progress (--project N \| --cycle N \| --view N) [--measure points\|count\|time --from --to]` | Per-day scope, started, completed. | `taskapp insights progress --project 1 --measure count` |
| `insights velocity (--project N \| --cycle N \| --view N) [--measure --periods 6]` | Completed per cycle or week. | `taskapp insights velocity --cycle 1` |
| `metrics [--range today\|week\|month\|quarter\|year\|FROM..TO \| --cycle NAME] [--project NAME --label NAME --points --json]` | Throughput, lead and cycle time, WIP, by label and source, velocity. Ranges end today; buckets are days up to 31 days, weeks up to about six months, then months. | `taskapp metrics --range month --label bug` |
| `forecast [--project NAME \| --cycle NAME]` | When the open issues are likely done at the recent pace (p50 and p85 dates). | `taskapp forecast --project Website` |
| `history <key>` | Change history of an issue. | `taskapp history ABC-1` |
| `undo` / `redo` | Undo or redo the newest step. | `taskapp undo` |
| `backup list --dir <dir>` | List backups in a folder. | `taskapp backup list --dir ~/Backups/cairn` |
| `backup restore <file> --yes` | Replace the database; quit the app first. | `taskapp backup restore snap.db --yes` |
| `import csv <file> [--map COLUMN=FIELD] [--apply]` | Plan, or apply with `--apply`. | `taskapp import csv tasks.csv --map Name=title --apply` |
| `import linear [--apply]` | Plan or apply a Linear import (`LINEAR_API_KEY`). | `taskapp import linear` |
| `setup <name> <key> [--tz]` | First-run setup. | `taskapp setup Cairn ABC --tz Europe/Berlin` |

## MCP tools reference

Server `taskapp-mcp` over stdio (`taskapp-mcp [--db <path>]`). Writes use source Agent, or Migration when a call passes
`migration: true`. Unknown argument names are refused. Only `import_linear_plan` uses the network. 54 tools:

| Tool | What it does (key arguments) |
|---|---|
| `search_issues` | Full-text search. `text`, `limit` (20). |
| `query_view` | Run a view. `def` (a ViewDef) or `view_id`. |
| `get_issue` | One issue with relations, comments, children. `key`. |
| `create_issue` | Create. Fields of NewIssue (`title` required; `due` as `{"Date":"2026-01-10"}`; `properties`; `created_at`, `completed_at`). |
| `update_issue` | Change an issue. `key`, `patch` (null clears a nullable field). |
| `archive_issue` | Archive. `key`. |
| `list_events` | Events overlapping a range. `from`, `to` (dates, optional). |
| `create_event` | Create an event. `title`, `starts_at` (`2026-12-10T09:30:00`), `ends_at`, `all_day`, `place`, `notes`, `icon`, `color`. |
| `link_event` | Link an event to an issue. `id` (event), `key` (issue). |
| `bulk_update` | Many patches. `items` = list of `{key, patch}`; over 10 snapshots first; an unknown key applies nothing. |
| `add_comment` | `key`, `body`, `created_at`. |
| `add_relation` / `remove_relation` | `from_key`, `to_key`, `kind` (an id or name, as in the CLI). |
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
- Not verified against real accounts: Apple Reminders import, Google Calendar sync, GitHub export, a restore from a real export (proven only on throwaway data and a local bare repo).
- The CLI cannot create statuses or milestones, change settings, or unarchive; the MCP server cannot read settings.
