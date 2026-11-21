# 0001 Toolchain

Context: Rust stable (cargo 1.99), Tauri 2, SQLx 0.8, Svelte 5 + Vite + TypeScript, Swift 6.3 helper, node 26 locally.

Decision: Cargo workspace, edition 2024, resolver 3. Every dependency is declared once in the root
`[workspace.dependencies]` with an exact `=x.y.z` pin and referenced with `workspace = true`; T0 therefore never edits
manifests for dependencies. Lints live in `[workspace.lints]`: clippy `all` and `pedantic` denied, plus
`cognitive_complexity`, `too_many_lines`, `unwrap_used`, `allow_attributes` and `allow_attributes_without_reason`
(so `#[allow]` is itself an error, which is the no-suppressions rule). Limits in clippy.toml: cognitive complexity 15,
function length 60 lines, 6 arguments. `unsafe_code` is forbidden; `unused` is denied (dead code fails the build).
The package manager is pnpm 10, run as `npx -y pnpm@10` until installed. Rustfmt defaults.

Consequences: pins need a deliberate bump PR. Limits are invented defaults (alternatives: complexity 25, 100 lines);
they change only by owner decision. Pins were resolved on 2026-11-21: tauri 2.12.7, sqlx 0.8.6, tauri-specta 2.0.0-rc.29,
rmcp 3.11.0, reqwest 0.12.28.
