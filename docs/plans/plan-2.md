# Plan 2: Cairn. Generic product, UI upgrade, command menu, keyboard

Status: building. The owner waived the spec-and-plan approval step on 2026-11-22 ("just start working on all the
plan 2 stuff in parallel") and asked that nothing be moved to plan 3. Merging `plan-2` to main still needs the owner.

## Decisions from the design round (2026-11-22)
- Name: **Cairn**, for now. Crate, binary, bundle id and data directory names stay `taskapp` until the owner decides.
- Visual direction: **B, "Survey map"** (chosen over A "Night trail" and C "Stone ledger"). Record: docs/decisions/0010-design-system.md.
- A comprehensive themes system is future work. Tokens are role-based so a theme is a new block of values.
- Nothing is cut. The import screen and the template editor stay in plan 2.
- Open with the owner: licence. README is written without a licence section.

## Goals, in priority order
1. A product anyone can use: no trace of the owner; first-run setup (workspace name, 2 to 5 letter key, default ABC, system time zone); neutral example data.
2. A complete UI upgrade on the design system: real headers, legend-band group headers, styled controls, designed empty states, rebuilt project page.
3. A command menu that reaches every action (Cmd+K, Cmd+P), contextual, nested, shortcuts shown.
4. Vim-style keys with visible hints, one registry, a cheat sheet, tooltips.
5. Multi-select and bulk actions as one undo step.
6. Harness fidelity: the mock equals the real backend, proven by contract tests.

## Waves and file ownership
A task edits only its own files. Shared registration points take added lines only.

### Wave F (done, backend, merged to plan-2)
| ID | Task | Model |
|---|---|---|
| F1 | Undo and redo (steps on the `changes` table) | Sonnet |
| F2 | Issue templates, `/slug` quick-add | Haiku |
| F3 | Icons and palette colours on entities | Haiku |
| F4 | CSV and Linear import, dry run then apply | Sonnet |

### Wave 0 (foundations, parallel)
| ID | Task | Owns | Model |
|---|---|---|---|
| P1 | Design system: tokens, fonts, primitives, icons, `/design` gallery, token check, decision record | `ui/src/styles/**`, `ui/src/ui/**`, `scripts/check-tokens.sh`, `docs/design-system.md` | Sonnet |
| P2 | Command and key registry, key engine, selection store, fuzzy match | `ui/src/lib/commands/**`, `ui/src/lib/selection.svelte.ts` | Sonnet |
| P3 | Generic backend, CLI, MCP, docs, README, first-run setup commands | `crates/**` (not the seed), `docs/**`, `README.md` | Haiku |
| P4 | Mock equals backend, contract tests, free ports, neutral fixtures | `ui/src/lib/api.mock.ts`, `ui/src/lib/contract/**`, `ui/harness/**`, devserver seed | Sonnet |
| P5 | Undo covers every entity kind | `crates/core/src/undo.rs`, `history.rs`, migration 0006 | Sonnet |

### Scaffold (orchestrator, after P1 and P2 land)
Stub files and route lines for every new screen, so wave 1 tasks replace a stub and never add a shared line.

### Wave 1 (screens and commands, all parallel, after the scaffold)
| ID | Task | Owns | Model |
|---|---|---|---|
| U1 | App root, shell, sidebar, theme switch, toast host | `app/App.svelte`, `components/Shell.svelte`, `Sidebar.svelte` | Sonnet |
| U2 | View page header: layout switcher, filter chips, view editor | `app/ViewPage.svelte`, `components/ViewEditor.svelte` | Sonnet |
| U3 | List view: legend bands, selection, density | `components/views/ListView.svelte` | Sonnet |
| U4 | Board view | `components/views/BoardView.svelte` | Haiku |
| U5 | Table view | `components/views/TableView.svelte` | Haiku |
| U6 | Timeline view | `components/views/GanttView.svelte`, `lib/gantt.ts` | Sonnet |
| U7 | Calendar view | `components/views/CalendarView.svelte` | Haiku |
| U8 | Issue detail panel | `components/IssueDetail.svelte`, `components/detail/**` | Sonnet |
| U9 | Project page rebuild | `components/ProjectPage.svelte`, `components/project/**` | Sonnet |
| U10 | Cycles and Pinned pages | `components/CyclesPage.svelte`, `PinnedPage.svelte` | Haiku |
| U11 | Settings for statuses, labels, projects, cycles, properties, with the icon and colour picker | those five settings files, `components/pickers/**` | Sonnet |
| U12 | Settings frame, General, Reminders, Calendar, Export, Backups | those files | Haiku |
| U13 | Command menu UI: nested pages, contextual actions, recents | `components/CommandPalette.svelte` | Sonnet |
| U14 | Cheat sheet overlay and `CommandButton` (tooltip with shortcut) | `components/CheatSheet.svelte`, `CommandButton.svelte` | Haiku |
| U15 | Commands: movement and g-jumps | `lib/commands/defs/nav/**`, `defs/move/**` | Haiku |
| U16 | Commands: issue actions and bulk edit as one undo step | `lib/commands/defs/issue/**`, bulk backend command | Sonnet |
| U17 | Quick add restyle, `/template` | `components/QuickAdd.svelte` | Haiku |
| U18 | First-run setup screen | `components/Setup.svelte` | Haiku |
| U19 | Import screen | `components/import/**` | Haiku |
| U20 | Template editor | `components/settings/Templates.svelte` | Haiku |
| U21 | Undo and redo: commands, toast | `lib/commands/defs/history/**`, `components/UndoToast.svelte` | Haiku |

### Added by the owner on 2026-11-22
| ID | Task | Model |
|---|---|---|
| W1 | Display options per view (which fields and custom properties show in each layout), custom properties in every layout, view templates that new views start from | Sonnet |
| X1 | Optional points (estimation setting, point scale, quick add `3pt`) and progress charts: burn-up, stacked area, velocity bars and sparklines in an optional insights panel on views, the project page and cycles | Sonnet x2 |

### Finish
One review (4 reviewers by area, plus a visual review of every screen against the design system), one fix wave, a
live click-through in Chrome on the real-mode harness, records, one PR to main. Stop for the owner.

## Quality bar for every UI task
Harness captures in light and dark at 1440x900 and 1024x768, mock and real mode, zero console errors, opened and
checked by eye, with one line on how the screen follows the design system. Tokens only; primitives only; keys only
through the registry.

## Review focus
1. No shortcut fires in a text field; the opening key is never typed.
2. Every mouse action is a command; the registry walk test passes.
3. A bulk edit is one undo step; undo refuses nothing.
4. Mock and devserver agree on every contract case.
5. No personal string anywhere outside the history docs.
6. No hard-coded colour or size outside `ui/src/styles/`.

## After this plan
Themes system; agents as assignees; decision tickets; cost per issue; multi-device sync; phone app; recurring tasks;
remind dates and snooze; inbox view; weekly review screen; Curia integration (keep MCP and CLI stable and documented);
renaming crates, bundle id and data directory to Cairn.
