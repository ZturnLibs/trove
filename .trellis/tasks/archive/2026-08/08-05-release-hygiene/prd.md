# 版本号与备份恢复校验（release-hygiene）

## Goal

将应用版本与迭代同步（bump 至 1.2.0，用户已确认），并为备份恢复增加「可读性 + 可迁移性」校验，避免损坏/旧版本备份覆盖当前数据后应用不可用。

## 背景（勘察确认）

- 版本现状：`package.json:3`、`src-tauri/Cargo.toml`（trove、trove_lib 两个 crate）、`src-tauri/tauri.conf.json "version"` 均为 `0.0.1`；release CI（release.yml check-tag）要求 git tag 与 package.json/tauri.conf 版本一致，当前 tag `v0.0.1`。
- 目标：`1.2.0`（用户确认，与文档 v1.1/v1.2 迭代标签对齐）。
- 备份恢复（backup.rs:170-185）：`restore()` 先做 pre-restore 快照，然后 `Connection::open(备份路径)` → `Backup::new` 覆盖写入。**无读可性校验**：损坏文件可能到 `Backup::run_to_completion` 才失败，且失败时目标库已被部分覆盖。
- `Database::migrate` 幂等（有 `migrate_is_idempotent` 测试）；恢复后的库不会自动迁移到当前 schema（迁移仅在 `Database::open` 时执行）。

## Requirements

- R1 将版本 bump 至 `1.2.0`：`package.json`、`src-tauri/Cargo.toml`（两个 crate）、`src-tauri/tauri.conf.json`，四处一致。
- R2 `BackupService.restore()` 在覆盖目标库**之前**校验备份文件：
  - 能用 SQLite 打开；
  - `PRAGMA quick_check` 通过（损坏即拒绝）；
  - 校验失败返回明确错误，且不触碰目标库（保持 pre-restore 快照前的数据）。
- R3 恢复完成后对目标库执行 `self.db.migrate(None)`，将旧版本备份的 schema 提升到当前版本（幂等，安全）。
- R4 为 R2/R3 补 Rust 单测：损坏备份被拒绝、恢复旧版本备份后可迁移到最新 schema。

## Acceptance Criteria

- [ ] AC1 四处版本号一致为 `1.2.0`；`cargo build`/`pnpm build` 通过。
- [ ] AC2 损坏的备份文件 restore 被拒绝，返回可读错误，目标库数据未被覆盖。
- [ ] AC3 恢复旧版本备份后 schema 自动迁移到当前版本，应用可用。
- [ ] AC4 `cargo test`、`pnpm typecheck` 通过。

## Notes

- 中复杂度任务：PRD + 简要 design。改动为 Rust（backup.rs + 测试）与版本文件。
- 不触碰 `tauri.conf.json` 的 productName/bundle 配置，仅 version。
