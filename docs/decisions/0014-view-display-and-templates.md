# 0014 Per-view display settings and view templates

Decision (display): a saved view stores what it shows. `columns` is the ordered list of displayed fields for every
layout (built-in fields and any custom property); `card_fields` overrides it on the board. The view also stores
density, whether empty groups show, how sub-issues show (nested, flat, hidden) and whether weekends show. All fields
default when absent, so views stored before this load unchanged. One renderer draws a field the same way in list,
board, table, timeline and calendar.

Decision (templates): a view template is a named view definition in its own table. A new view can start blank, from a
template, or as a copy of the current view; a view remembers its template and can be reset to it; any view can be
saved as a template; one template can be the default for new views. Templates are workspace-wide. Three are seeded.
Agents get the same operations through MCP and the CLI, and all of it can be undone.

Why a separate table and not a flag on views: templates must not appear in the sidebar, in view counts or in view
queries, and deleting a view must never delete what other views were made from.

Consequence: deleting a custom property must scrub it from views and from templates (review finding R1-1).
