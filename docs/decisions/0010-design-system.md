# 0010 Design system: direction B "Survey map"

Context: three static directions were compared on real screens: A "Night trail", B "Survey map", C "Stone ledger".
The owner chose B.

Decision: the product look is B. A deep-green frame (sidebar and top bar) carries a pale rounded sheet; group headers
are legend bands whose tone deepens with the status category (like map elevation); contour lines are the one piece of
art (empty states, detail panel); status is a cairn, priority is contour rings; one signal orange marks the current
row and urgency. Type is Bricolage Grotesque, self-hosted, tabular figures on, no monospace.

Why B over A and C: A is dark-first and cool, which makes the light theme an afterthought and the group headers
indistinguishable from rows. C relies on ledger rules and numerals, which reads dense and flat in a list-heavy app. B
keeps rows quiet, gives every group a clear start without lines or boxes, and works in both themes from the same roles.

Themes are deferred. Light and dark ship now. Every colour a component uses is a role token in
`ui/src/styles/tokens.css`; a theme is one more block of those values (docs/design-system.md). Nothing about a user
theme picker or custom palettes is decided.

Consequences: `scripts/check-tokens.sh` keeps raw values out of components. The 12 label colours have per-theme
values, 3:1 against the sheet and band steps 1 and 2; on band steps 3 and 4 (light) they fall to about 2.6 and 2.1, so
bands use the stone ink, not label colours.
