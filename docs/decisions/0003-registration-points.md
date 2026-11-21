# 0003 Registration points

Context: parallel tasks must never edit the same list.

Decision: T0 owns every module declaration (`lib.rs`), manifest, route table, Tauri command list and migration
`0001_init.sql`. Each task fills files that T0 already declared. Later additions follow one file per entry
(migrations are `NNNN_name.sql`, numbered by the orchestrator) or a self-registration call. Tests derive counts
(statuses, tools, commands) from the files or registries, never from a literal. When one shared file causes a conflict
twice, add a registration point before more tasks touch it.

Consequences: an implementer that needs a shared file changed reports the exact lines; the orchestrator applies them.
