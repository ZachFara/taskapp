# 0011 One registry for commands, keys, menus and hints

Context: plan 2 asks for a command menu that reaches every action, vim-style keys with visible hints, a cheat sheet and
tooltips. Parallel tasks must not edit one shared list.

Decision: every user action is a command in its own file under `ui/src/lib/commands/defs/`, registered when imported.
A command has a label, an icon, a group, a scope, and either a shortcut or a stated reason for having none. The command
menu, the key engine, the right-click menu, tooltips, the legend bar and the cheat sheet all read this registry. No
component binds a key; a screen listens for `command:<id>` events for the actions it owns.

Rules that came out of the build:
- Plain keys never fire in a text field, a menu or a dialog, and the key that opens a field is never typed into it.
- Digits are counts (`5 j`), so layouts switch with `v` + letter, not 1 to 5.
- `l` is "next column" on the board and "labels" in list, table and issue detail; `shift+l` is always labels.
- Esc closes the top overlay, then clears the selection, and never leaves the app.
- A test fails when two commands that can be active together share a key or a sequence prefix.

Why: one source keeps the menu, the keys and the hints from drifting apart, and one file per command removed the merge
conflicts that shared lists caused in plan 1.

Consequence: per-component tests did not catch broken flows between commands and screens (a key opening the wrong
page, Esc leaving the app). `ui/harness/walk.test.ts` drives the real app in a browser and runs in CI for that reason.
