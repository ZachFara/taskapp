---
links:
  - '[[Personal Coding Projects]]'
date: 2026-11-21
time: '02:35'
title: Personal Task App Design
status: In Progress
references: https://claude.ai/artifact/3w6Z9ZYxhzvMtj2oiTnKME
tags:
  - Personal_Coding_Projects
---
> [!note] AI Generated
> Written by Claude (Opus 5.5) in chat on 2026-11-21, from a design session with me. Related: [[Personal Task App Implementation Plan]], [[Personal Task App Design.pdf|PDF version]], [[Personal Task Manager Project Architect Agent]].

# Purpose

The design for my personal, self-hosted Linear replacement: what it does, why, and which choices were mine versus Claude's. The build plan that implements it is [[Personal Task App Implementation Plan]].

---

Status: approved by Zach, revised 2026-11-21. Date: 2026-11-20. Name: not yet chosen.

# TLDR

A personal, single-user task manager that copies most of Linear's features, adds user-defined custom properties at its core, and adds a Gantt chart. It runs as a fast Mac desktop app (Rust + Tauri, Svelte UI) on a local SQLite database that Zach owns. It can optionally export its data to a private GitHub repo as text files for history and backup. On the phone, tasks are captured through an Apple Reminders list that the app imports. Notifications go through a Google Calendar calendar that the app writes events to, so Google's servers send them even when the Mac is off. Agents can read and reshape the data through an MCP server, and while they build it they can see and check the running app through a development harness. Views work like Linear's project views: filtered lists, boards, tables, multi-project timelines, and a calendar view. A Pinned page collects everything Zach pins.

---

# Contents

1. Why
2. Goals, non-goals, anti-goals
3. Constraints
4. Approaches considered
5. Design
   1. Data model
   2. Linear feature baseline
   3. Views, timelines, and calendar
   4. Phone capture through Apple Reminders
   5. Notifications through Google Calendar
   6. Optional GitHub export
   7. Agent access, CLI, and Linear migration
   8. Backups
   9. Pinned items
   10. Development harness
6. Build order
7. Who decided what
8. Left to the builder

---

# 1. Why

Zach uses Linear and likes most of what it does. He also uses Obsidian, where the main appeal is that the data is his and the structure is whatever he wants, and where his agents routinely rearrange things for him. He wants one tool with both: Linear's structure and speed, and Obsidian's ownership and flexibility.

The first thing he wants to do in it that Linear doesn't allow well is define his own custom properties on tasks. He also wants a real Gantt chart and reliable reminders on his Apple devices.

---

# 2. Goals, non-goals, anti-goals

Goals:
- Most of Linear's features for one person (section 5.2).
- Custom properties as a first-class concept: any number of user-defined fields, usable in filters, sorts, grouping, and views.
- An issue-level Gantt chart with dependencies.
- Data owned by Zach: a local database, with optional export to a private GitHub repo.
- Capture from the phone with no custom phone app.
- Notifications that reach the phone even when the Mac is asleep or off.
- Agents can read and change the data, including restructuring properties, safely.
- Minimal and fast.
- A Pinned page for anything Zach wants close at hand.
- A development harness from the start, so the agents building the app can see and check what they build.

Non-goals:
- Teams, workspaces, permissions, invites, multiple users.
- Linear's team-process features: triage rotations, SLAs, most third-party integrations.
- A phone app for viewing or editing tasks. The phone only captures.
- Two-way calendar sync.

Anti-goals (it fails if this happens, even if it technically works):
- It feels slower or clunkier than Linear for everyday use.
- Adding or changing a property requires a code change or migration.
- An agent's bulk change can destroy data with no way back.
- It needs an always-on server just to function.

---

# 3. Constraints

- Single user, running on one Mac only: Zach's personal Mac. No multi-device sync, and this one app instance is the only writer to the Reminders list and the Google calendar.
- Phone is an iPhone, with an Apple Watch possible.
- Data size is small, similar to an Obsidian vault.
- Built mostly by coding agents (Claude Code), so the stack should catch mistakes early.
- Zach likes Rust's strictness.

---

# 4. Approaches considered

Base:
- Fork Plane. Closest to Linear's features, but large and heavy to run and modify, and as far as we know custom properties are in its paid editions. Rejected.
- Fork Vikunja. Small and hackable, but closer to Todoist than Linear, so most Linear features would be built anyway, on a schema not designed for custom properties. Rejected.
- Build fresh, single-user, with custom properties in the foundation. Chosen. Dropping multi-user removes most of what makes the existing apps heavy.

Stack:
- Rust backend with a TypeScript (Svelte) UI. Chosen. Rust's compiler catches agent mistakes in the data and logic layer, and the UI can use mature JS libraries for Gantt, command palette, and drag-and-drop.
- All Rust (Leptos). One language, but the UI pieces Zach wants would need wrapping or writing from scratch. Rejected.
- All TypeScript. Fastest to start but loses the strictness Zach wants. Rejected.

Storage:
- Postgres on a server. Strong for agent SQL access but requires running a server. Rejected for now.
- Text files as the source of truth with SQLite as a rebuildable index, synced through git. Good for merging, but Zach wants the database to be primary. Rejected.
- Local SQLite database as the source of truth, with optional export to GitHub. Chosen.

Phone capture:
- An iOS Shortcut writing files into the GitHub repo. Zach didn't like it. Rejected.
- An Apple Reminders list imported by the Mac app. Chosen.
- Alternatives kept in reserve: an iCloud Drive inbox folder, or Google Tasks.

Notifications:
- ntfy push from a server, or a scheduled GitHub Action. Both need infrastructure or lag.
- Events on a Google Calendar calendar. Chosen (Zach's idea). Google sends the notifications.

---

# 5. Design

## 5.1 Data model

SQLite, accessed from Rust with compile-time-checked queries (for example SQLx).

Issues have a small set of fixed columns, because the app treats these specially:
- ID with a project-style key (for example `ZF-123`), title, description (markdown)
- status, priority
- project, parent issue (sub-issues), cycle
- start date, due date (date or date-time), estimate
- created, updated, completed timestamps

Everything else lives in a `properties` JSON column. A separate property definitions table describes each property: key, display name, type (text, number, date, select, multi-select, checkbox, URL, link to an Obsidian note), select options, and display order. Properties can be global or scoped to a project.

Rule: a field becomes a fixed column only when the app needs to treat it specially (dates for the Gantt and notifications, status for boards). Everything else stays a property.

Projects have an optional description and an optional link to an Obsidian note. Most of the time Zach will just link the note, so the project page shows the link prominently and the description stays out of the way when empty.

Other tables: projects, milestones, cycles, labels, relations (blocks / blocked by / related / duplicate), saved views, pins, an import log, and a change history table recording every change to issues and properties (old value, new value, source: UI, agent, Reminders import).

## 5.2 Linear feature baseline

Included:
- Issues with IDs, custom statuses (a customizable workflow), priorities, labels, estimates, sub-issues
- Projects with milestones and progress
- Cycles
- Relations: blocks, blocked by, related, duplicate
- Saved views with filters, sorting, grouping, and display options, working like Linear's project views, scoped to one project, several, or all
- Command palette and keyboard-first navigation for every common action
- Quick-add with natural-language parsing (`renew lease by fri p1 #ohio-move`)
- Search
- Markdown descriptions and comments
- Archive and restore
- A Pinned page (section 5.9)

Excluded: teams, permissions, triage, SLAs, notifications inbox for other users, most integrations.

## 5.3 Views, timelines, and calendar

Layouts for any saved view: list, board (kanban), table (spreadsheet-like editing of properties), timeline (Gantt), and calendar. Any property can be a filter, sort, group, or column.

Calendar view (the events view): day, week, and month layouts (month is Claude's call) showing work blocks, timed due dates, and date-only due dates as all-day items. In day and week, issues are dragged onto time slots to create work blocks. In the first version it shows only the app's own events, so it needs write access to one calendar only. Showing Zach's other Google calendars on the grid is a later feature, because it needs broader read access.

Gantt:
- Bars from start date to due date; tasks with only an estimate and one date are drawn using the estimate.
- Estimates are durations (hours or days), not Linear-style points (Claude's call: for one person, points add a conversion step with no benefit).
- Tasks with an estimate but no dates appear in an "unscheduled" lane at the side, and can be dragged onto the timeline.
- Dependency arrows from blocking relations.
- Drag to reschedule, with dependent tasks shifting.
- Roll-up bars for projects and parent issues, milestones as markers.
- Zoom from day to quarter.
- Critical path highlight.
- Use an existing Gantt library rather than building one.

Multi-project timelines: a timeline view over any filter that spans projects groups issues by project, each project with its own roll-up bar. A separate projects mode, like Linear's roadmap, shows one bar per project from its start date to its target date, with milestones as markers, and dragging a project bar changes its dates.

## 5.4 Phone capture through Apple Reminders

- In settings, Zach picks which Apple Reminders list is the inbox (default suggestion: "Inbox").
- On the phone, he adds reminders to that list in any normal way: Siri, widget, Watch, share sheet, the Reminders app.
- The Mac app reads that list through Apple's Reminders framework (EventKit), using a small Swift helper or Rust bindings, since Tauri can't call it directly.
- Each reminder becomes an issue: the title is parsed with the quick-add parser, the notes become the description, and a Reminders due date becomes the due date.
- After a successful import, the reminder is deleted from the list.
- Import runs on app start and whenever the list changes while the app is running.

## 5.5 Notifications through Google Calendar

- In settings, Zach connects his personal Google account (Claude's call: a work account's admin can block unapproved apps) and picks which calendar the app writes to (a dedicated calendar is recommended).
- Sync is one-way. The app owns these events and stores the issue ID on each one, so it can update or delete them.
- Issue with a due date and time: a 15-minute event at that time with a popup notification.
- Issues with a due date and no time: one all-day digest event per day ("Due today: 3 tasks") listing each task, with a notification at 8:00 that day. Used even when only one task is due.
- Issues with no due date: nothing on the calendar.
- Work blocks: dragging an issue onto a time slot creates a calendar event for it.
- Completing, deleting, or rescheduling an issue updates or removes its events, and digests rebuild when their tasks change.
- Event text includes the task title and key details, so the notification is useful on its own. Links point to the desktop app, which only helps on the Mac.
- Events are only created or updated while the app runs.
- Google sign-in uses Zach's own Google Cloud OAuth client, published as an unverified production app rather than left in "Testing" mode, because Testing-mode sign-ins expire after 7 days and would silently stop the sync. This means a one-time "unverified app" warning when connecting.
- Zach should use the Google Calendar app on his phone for these notifications, since Apple's Calendar app doesn't reliably honor Google's event notifications.

## 5.6 Optional GitHub export

- Off by default. When turned on, Zach sets a private repo.
- The app exports one text file per issue (markdown with frontmatter, including properties), plus files for projects, property definitions, and saved views, and commits on a schedule and after bulk changes.
- This gives readable history, diffs of agent edits, and an offsite copy.
- The app can rebuild a database from an export, as a restore path.

## 5.7 Agent access, CLI, and Linear migration

- An MCP server exposes the data to agents: query issues, create and update issues, and manage property definitions.
- Before any bulk change through the MCP server, the app takes a database snapshot.
- Every agent change is recorded in the change history with source "agent."

Later agent features (not in the first version): agents as assignees with session logs attached, decision tickets (an agent files a "needs your call" item with options), cost per issue from cost-lens, and agents drafting docs linked to issues.

Command-line interface: the app also ships a CLI with the same operations as the MCP server, for scripts and for agents that prefer shell commands.

Linear migration: there is no built-in Linear importer. Zach will have an agent move his Linear data, reading through Linear's MCP and writing through this app's MCP server or CLI. So the MCP server and CLI must be able to create every kind of record: projects, milestones, cycles, labels, statuses, property definitions, issues with sub-issues, relations, and comments, and must accept original created and completed timestamps. The original Linear ID is kept as a property on each issue. The migration only reads from Linear and never changes anything there.

## 5.8 Backups

- Nightly SQLite backup to a local folder, keeping recent copies.
- The GitHub export, when on, is the offsite copy.
- Snapshots before agent bulk changes.
- A restore test is part of the first release.

## 5.9 Pinned items

- Zach can pin issues, projects, saved views, cycles, Obsidian notes, and web links.
- Pins appear on a Pinned page and in a Pinned section at the top of the sidebar, in an order he can rearrange.
- A pin to something that was deleted disappears on its own. A pin to an archived issue stays and shows it as archived.

## 5.10 Development harness

Built first, so every agent working on the app can see what it is building:

- The UI runs in a normal browser, outside the desktop app, in two modes: with fake data, and against the real core through a local development-only server.
- A screenshot script loads any screen, optionally types keys or clicks, and saves a screenshot, the page's text structure (for agents that read text better than images), and any console errors. Agents run it after each UI change.
- A seed command fills a separate development database with realistic sample data. The harness never touches the real database.

---

# 6. Build order

0. Development harness, alongside the project skeleton.
1. Core: issues, fixed fields, custom properties and definitions, projects, list and table views, quick-add, command palette, change history, backups.
2. Linear baseline: statuses, labels, relations, sub-issues, cycles, milestones, board view, saved views, search.
3. MCP server and CLI, complete enough for the Linear migration (moved early so Zach can switch over and use the app while the rest is built).
4. Timelines and the calendar view.
5. Apple Reminders import.
6. Google Calendar sync.
7. GitHub export.
8. Later agent features.

---

# 7. Who decided what

Decided by Zach:
- Single user to start, no teams.
- Most of Linear's features, plus a Gantt chart.
- Custom properties are the first priority.
- Build fresh rather than fork, simpler and less clunky.
- Rust, for its strictness.
- A local database as the primary store, optionally synced to GitHub.
- Phone capture with something lightweight, not a phone app; Apple Reminders, with a configurable list.
- Notifications through Google Calendar: short events for timed due dates, an all-day event for date-only due dates, one all-day event listing multiple tasks, work blocks as events, a configurable calendar, one-way sync.
- Agents should be able to rearrange the data.
- One Mac only, his personal Mac.
- Linear data moves over through an agent using Linear's MCP and this app's MCP server or CLI, not a built-in importer.
- A development harness early, so agents can see the app while building it.
- Views with filters like Linear's project views, multi-project timelines, list and kanban views, custom statuses, and an events (calendar) view.
- A place for all pinned items.
- Project descriptions are optional; usually a project just links to an Obsidian document.

Calls made by Claude, open to change:
- Svelte for the UI, inside Tauri.
- SQLite rather than Postgres.
- Fixed columns versus properties split (5.1).
- 15-minute events for timed due dates; 8:00 digest notifications; digest used even for one task.
- GitHub export as one text file per issue, off by default.
- Snapshots before agent bulk changes, and a change history table.
- Build order (section 6), including moving the MCP server and CLI to step 3.
- Keeping the original Linear ID as a property.
- A calendar view with day, week, and month layouts, showing only the app's own events at first.
- Pinnable kinds (issues, projects, views, cycles, notes, links) and how pins to deleted or archived things behave.
- The harness design: browser with fake data or a local dev server, a screenshot script that also dumps page text and console errors, and a separate seeded dev database.
- Estimates as durations, with an unscheduled lane in the Gantt.
- A personal Google account, with the OAuth app published rather than in Testing mode.

---

# 8. Left to the builder

- Choice of Gantt, command palette, and drag-and-drop libraries.
- Whether the MCP server is part of the app or a separate binary sharing the database.
- EventKit integration approach (Swift helper or Rust bindings).
- Quick-add grammar details beyond the examples.
- Visual design, following Linear's general feel.
- Export file format details.
