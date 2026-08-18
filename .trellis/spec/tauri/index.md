# Tauri / Rust Guidelines

Contracts for the `src-tauri` crate. Read the listed files before changing the matching area.

## Pre-Development Checklist

- [ ] Adding `src/bin/*` or a second Cargo binary → [extra-binaries-and-release.md](./extra-binaries-and-release.md)
- [ ] Changing `WorkbenchAction` / `ActionOutcome` / IPC `workbench_action_dispatch` → [workbench-action.md](./workbench-action.md)
- [ ] Changing task CSV import/export/undo or migration `0018` → [csv-tasks.md](./csv-tasks.md)
- [ ] Bumping app version or cutting a GitHub Release → [extra-binaries-and-release.md](./extra-binaries-and-release.md)

## Quality Check

- [ ] `cargo test --lib` in `src-tauri`
- [ ] Schema assertion in `infrastructure/db/mod.rs` matches the latest migration number
- [ ] `tauri build` (or CI `release` workflow) still finds the **app** binary after extra bins exist
- [ ] Database tests keep the `tempdir` alive for the whole `Database` lifetime

## Guidelines Index

| Guide | Description |
| --- | --- |
| [Extra binaries and release](./extra-binaries-and-release.md) | `default-run`, universal lipo, `beforeBundleCommand` cwd |
| [Workbench action](./workbench-action.md) | Serde tag, confirmation gate, IPC |
| [CSV tasks](./csv-tasks.md) | Preview/import/undo contracts |
