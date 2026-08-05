# 技术设计：版本号与备份恢复校验

## 1. 版本 bump 至 1.2.0

| 文件 | 变更 |
|---|---|
| `package.json` | `"version": "1.2.0"` |
| `src-tauri/Cargo.toml` | `version = "1.2.0"`（`trove` 与 `trove_lib` 两个 `[package]`） |
| `src-tauri/tauri.conf.json` | `"version": "1.2.0"` |

- 保持四处一致；release CI 的 check-tag 即可放行 `v1.2.0` 标签。
- 不改 productName/bundle 其它字段。

## 2. 备份恢复校验（backup.rs `restore`）

```rust
pub fn restore(&self, file_name: &str) -> Result<(), DomainError> {
    let path = self.resolve_backup_path(file_name)?;
    // 先校验可读性（不触碰目标库）
    {
        let check = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(internal)?;
        let quick_check: String = check
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(internal)?;
        if quick_check != "ok" {
            return Err(DomainError::Validation(format!(
                "备份文件校验失败（{quick_check}），已取消恢复，数据未改动"
            )));
        }
    }
    // 安全快照当前库
    let _ = self.create_inner("pre-restore");

    let src = Connection::open(path).map_err(internal)?;
    let mut dst = self.db.connect().map_err(internal)?;
    {
        let backup = Backup::new(&src, &mut dst).map_err(internal)?;
        backup
            .run_to_completion(100, Duration::from_millis(25), None)
            .map_err(internal)?;
    }
    // 将恢复后的库迁移到当前 schema（幂等）
    self.db.migrate(None)?;
    self.set_error(None);
    Ok(())
}
```

要点：
- `SQLITE_OPEN_READ_ONLY` + `PRAGMA quick_check`（默认仅检查前 1/1000 页，开销小）在覆盖前拦截损坏文件。
- pre-restore 快照保持在校验之后、覆盖之前，避免无谓快照。
- `migrate(None)` 幂等：旧版本备份恢复后 schema 自动提升；`open_with_backup_dir` 的迁移前快照路径不需 backup_dir（`None`）即可提升。

## 3. 测试

- `restore_rejects_corrupt_backup`：写一个随机字节文件到备份目录，`restore` 返回 `Validation` 错误且当前库内容不变。
- `restore_old_schema_backup_then_migrates`：构造低 schema 版本备份 → restore → 断言 `schema_version == 8`（当前最新）。
- 沿用 backup.rs 既有测试基建（tempdir、svc、TaskService）。
