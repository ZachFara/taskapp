# 0015 Optional points and progress charts

Decision (points): an issue has optional `points` (0 to 1000, one decimal), separate from the time estimate. The
`estimation` setting chooses none, points, time or both; `point_scale` picks the suggested values. With points off
they are hidden everywhere and quick add leaves `3pt` in the title.

Decision (charts): progress is rebuilt from the change history, not stored. For each local day of the workspace zone,
`insights_progress` gives scope, started and completed for a project, a cycle or a view, measured in points, issue
count or time. Canceled issues leave scope. A cycle counts an issue only while it is in the cycle and ignores work
finished before the cycle began. `insights_velocity` gives net completed per cycle, or per week without cycles.

Where it shows, each optional: an insights panel on any view (burn-up line, stacked area or bars; measure and range
remembered per view), a compact chart on the project page, a burn-up with an ideal line on the current cycle,
sparklines on past cycles and, off by default, in the sidebar. Every chart has a text summary and a table view.

Why rebuild from history: the history already exists for undo, a stored series would drift from it, and at 2,000
issues and about a year of changes the command takes about 0.15 s in a debug build.
