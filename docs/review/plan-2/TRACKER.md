# Review findings tracker (plan 2). One line per finding. Update the state when a fixer reports.
States: ASSIGNED (who) / FIXED (PR) / DEFERRED (why, where recorded) / UNASSIGNED (needs the sweep).

## Important (27)
| id | what | owner | state |
|---|---|---|---|
| R1-1 | deleted property stays in view templates | FX1 | FIXED (#118) |
| R1-6 | no test: undo refuses uncovered entity | FX1 | FIXED (#118) |
| R1-7 | no test: bulk rollback on failed issue update | FX1 | FIXED (#118) |
| R2-1 | export/rebuild drops icons, colours, templates, view templates, import_log | FX1 | FIXED (#118) |
| R2-2 | re-import after undo says already imported | FX1 | FIXED (#118) |
| R2-3 | import hex colour rewritten on open, undo conflicts | FX1 | FIXED (#118) |
| R2-4 | mock redo of created issue; mis-aimed contract cases | FX1 (verify; fixed in #114) | FIXED (#118) |
| R2-5 | MCP ignores unknown argument names | FX1 | FIXED (#118) |
| R2-6 | MCP instructions wrong about Linear | FX1 | FIXED (#118) |
| R3-1 | description editor opens unfocused | FX3 | FIXED (#116) |
| R3-3 | no recents in command menu | FX2 | FIXED (#117) |
| R3-4 | shortcut tooltips missing | FX2 shell/sidebar FIXED (#117); FX4 pages FIXED (#115); FX3 views/detail FIXED (#116); GroupHeader +/collapse and view Tabs tooltips (src/ui) -> sweep |
| R3-7 | mouse actions without a command | FX2 commands FIXED (#117); listeners FX3 (#116), FX4 (#115) | FIXED |
| R3-9 | selection has no aria-selected; cmd-click opens panel | FX3 | FIXED (#116) |
| R3-10 | test gap: open-issue guard in move/escape | FX2 | FIXED (#117) |
| R4-01 | 1024 + side panel breaks layout | FX3 | PARTLY (#116: columns drop, no clip); panel overlay under 1100px needs Shell.svelte -> sweep |
| R4-02 | table overflows at 1024 | FX3 | FIXED (#116) |
| R4-03 | raw key names shown | FX2 | FIXED (#117) |
| R4-04 | cheat sheet taller than the window | FX2 | FIXED (#117) |
| R4-05 | side panel order differs from full page | FX3 | FIXED (#116) |
| R4-06 | settings: double and inconsistent headers | FX4 | FIXED (#115) |
| R4-07 | settings rail height / scrolls | FX4 | FIXED (#115) |
| R4-08 | properties settings bare buttons | FX4 | FIXED (#115) |
| R4-09 | quick add ISO date chip, art collides, priority chip inverted | FX3 | FIXED (#116). ROOT CAUSE was detail/format.ts: priority names AND the detail picker's saved numbers were inverted -> sweep audits priority mapping everywhere |
| R5-1 | quick add stores points when estimation is off | FX1 | FIXED (#118) |
| R5-2 | Points offered/rendered when estimation is off | FX3 | FIXED (#116) |
| R5-3 | points validation unbounded | FX1 | FIXED (#118) |

## Minor (38)
| id | what | owner | state |
|---|---|---|---|
| R1-2 | settings_set bypasses validation | FX1 (cheap minors) | FIXED (FX6) |
| R1-3 | last status of a category can be deleted | FX1 | FIXED (FX6) |
| R1-4 | key lookup accepts odd forms | FX1 | FIXED (FX6) |
| R1-5 | stale redo after a non-step write | FX1 | FIXED (FX6) |
| R1-8 | no test: property-key quote guard | FX1 | FIXED (#118: guard unreachable, asserted at create) |
| R1-9 | no test: delete status into itself | FX1 | FIXED (#118) |
| R2-7 | import-created statuses have no history | FX1 | DEFERRED: FK keeps the status; mock matches; recorded in decision 0012 |
| R2-8 | CSV edge cases | FX1 | FIXED (FX6) |
| R2-9 | MCP pin source and missing targets | FX1 | FIXED (FX6) |
| R2-10 | tool text and errors | FX1 | FIXED (FX6) |
| R2-11 | import step label is the command name | FX1 (step labels with counts) | FIXED (#118) |
| R2-12 | personal strings in tracked files (spec doc, PROGRESS.md ZF-1, home paths in .briefs) | orchestrator, records PR | ASSIGNED (briefs redacted in docs/plan-2-records; PROGRESS.md and spec doc to do) |
| R2-13 | test gaps from break tests (safe_name, mock re-apply, real-db guard hang) | FX1 | FIXED (#118) |
| R2-contract | 28 UI-reachable commands without a contract case | FX1 does 15; FX6 does 6 | FIXED (FX6) for the 6 reachable (blocks_*, properties_reorder, properties_set_scope); 8 are not served by the devserver |
| R3-2 | shortcuts fire behind an open menu | FX2 | FIXED (#117) |
| R3-5 | estimate page ignores a bare number | FX2 | FIXED (#117) |
| R3-6 | design gallery in the menu | FX2 | FIXED (#117; already dev-only, test added) |
| R3-8 | setup first field not focused | FX4 | FIXED (#115) |
| R3-9 detail | two Escs within 60 ms clear nothing (part of R3-9 text) | FX3 | ASSIGNED (R3-07/12/18/19 in R3.md are screenshot names, not findings) |
| R4-10 | table header case, baseline, focus ring | FX3 | DEFERRED -> sweep |
| R4-11 | timeline today flag overlap, hatched bars | FX3 | DEFERRED -> sweep |
| R4-12 | calendar dead area, chip truncation | FX3 | DEFERRED -> sweep |
| R4-13 | board empty right side, orphan icon | FX3 | DEFERRED -> sweep |
| R4-14 | views page actions, "Gantt" naming | FX4 | FIXED (#115) |
| R4-15 | projects page sparse | FX4 | FIXED (#115) |
| R4-16 | cycles header glyph, heavy button | FX4 | FIXED (#115) |
| R4-17 | "1 issues"; status swatches | FX4 | grammar FIXED (#115); swatches DEFERRED -> sweep decides |
| R4-18 | project page three header layers, grey empty boxes | FX3 | FIXED (#116) |
| R4-19 | inbox placeholder in nav | FX2 | FIXED (#117) |
| R4-20 | display panel hugs the edge; group popover poor | FX3 | DEFERRED -> sweep |
| R4-21 | toast bell, noise, spelled-out key | FX2 | FIXED (#117); ui/Toast.svelte info tone still draws a bell -> sweep |
| R4-22 | icon inconsistencies | FX2 (sidebar, menu) FIXED (#117); issue page/detail project icon -> sweep |
| R4-23 | issue page textarea grip, washed button, blank bands | FX3 | DEFERRED -> sweep |
| R4-24 | setup layout imbalance, theme control | FX4 | DEFERRED -> sweep |
| R4-25 | blank fields without placeholders, pill styles | FX4 | FIXED (#115) |
| R5-4..R5-7 | `3 pt`/`3pts`/`.5pt`; reversed range error; Move button at 1024; "Set points" in menu when off | FX1 (parse, range), FX4 (#115 button), FX2 (#117 menu) | ASSIGNED / FIXED |

## Not from the review, but open (must not be lost)
| what | owner | state |
|---|---|---|
| Step labels with counts in the toast ("Set status on 2 issues") | FX1 backend FIXED (#118); FX2 shows verbatim (#117) | FIXED once both land |
| Time zone change in setup needs a restart | - | DEFERRED (noted in General settings by #115; After this plan) |
| Remove-filter and view "More" menu items as commands | FX3 | FIXED (#116: chip remove is a focusable button with tooltip; More items via view commands) |
| views_save raises no undo toast (FX2 ruling) | owner may overrule | NOTED for the owner |
| Linear import never run live; packaged Mac app never launched | owner steps | qa-checklist |
| R5 coverage gaps: DST by hand, MCP points tools, chart table vs backend, keyboard day stepping, sparklines | sweep / final walk | UNASSIGNED |
| Git push of export, reminders, gcal untouched by review | owner steps | qa-checklist |

## Sweep (FX5), after FX1 and FX3 report
Everything above marked UNASSIGNED, DEFERRED -> sweep, or PARTLY, plus whatever FX1 and FX3 report as deferred.

| R2-5 limitation: scalar MCP tool arguments still accept unknown names | FX6 | FIXED (FX6) |
