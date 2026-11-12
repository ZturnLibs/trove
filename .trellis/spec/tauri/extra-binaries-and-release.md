# Extra Cargo binaries and release builds

## Scenario: ship `trove-cli` beside the Tauri app

### 1. Scope / Trigger

- Trigger: any new `src-tauri/src/bin/*.rs` (or extra `[[bin]]`) in the same package as the Tauri app.
- v1.4.0 release failed until `default-run` and universal lipo were in place.

### 2. Signatures

- Package: `src-tauri/Cargo.toml`
  - `name = "trove"`
  - `default-run = "trove"` (required once a second bin exists)
  - implicit bins: `src/main.rs` → `trove`, `src/bin/trove-cli.rs` → `trove-cli`
- Bundle hook: `src-tauri/tauri.conf.json` → `build.beforeBundleCommand`
  - value: `node scripts/lipo-extra-bins.mjs`
- Helper: `scripts/lipo-extra-bins.mjs` lipos extra bins into `target/universal-apple-darwin/release/`
- Release: run `./scripts/release.sh` **on `main` only**, working tree clean and equal to `origin/main`. Do not bump version on a feature branch.

### 3. Contracts

- Tauri `build` / `bundle` must resolve the **app** binary as `trove`.
- Universal macOS (`--target universal-apple-darwin`) only lipos `default-run`. Extra bins must already exist at `target/universal-apple-darwin/release/<bin>` before the `.app` copy step.
- `beforeBundleCommand` cwd in CI is the **repository root**, not `src-tauri`. Invoke `node scripts/lipo-extra-bins.mjs`, not `node ../scripts/...`.
- The helper no-ops when per-arch extra bins are missing (Windows / non-universal macOS).

### 4. Validation & Error Matrix

| Condition | Error |
| --- | --- |
| Extra bin exists, no `default-run` | `failed to find main binary, make sure you have a package > default-run` |
| Universal bundle, extra bin not lipo'd | `Failed to copy binary from ".../universal-apple-darwin/release/trove-cli": does not exist` |
| `beforeBundleCommand` path assumes `src-tauri` cwd | `Cannot find module '.../scripts/lipo-extra-bins.mjs'` (resolved one directory too high) |
| `release.sh` on a non-main branch | script exits: current branch is not `main` |

### 5. Good / Base / Bad Cases

- Good: `default-run = "trove"` + lipo helper + `beforeBundleCommand: node scripts/lipo-extra-bins.mjs`
- Base: single binary package needs neither `default-run` nor lipo
- Bad: add `src/bin/trove-cli.rs` and only run `cargo test --lib` (CI lib tests pass; `tauri build` / release workflow fail)

### 6. Tests Required

- `cargo metadata --no-deps --format-version 1` → package `default_run` is `"trove"`
- Release workflow `publish` jobs: macOS universal **and** Windows both green
- Assets on the GitHub Release: `Trove_*_universal.dmg`, `Trove_universal.app.tar.gz` + `.sig`, Windows exe/msi + `.sig`, `latest.json` with `darwin-aarch64`, `darwin-x86_64`, `windows-x86_64`

### 7. Wrong vs Correct

#### Wrong

```toml
[package]
name = "trove"
# no default-run
```

```json
"beforeBundleCommand": "node ../scripts/lipo-extra-bins.mjs"
```

#### Correct

```toml
[package]
name = "trove"
default-run = "trove"
```

```json
"beforeBundleCommand": "node scripts/lipo-extra-bins.mjs"
```

Keep extra bin names in `scripts/lipo-extra-bins.mjs` (`extraBins`) in sync with `src/bin/*.rs`.


### macOS runner 镜像 EOL 与标签选择（2026-11-12 定案，v2.6.2 实证）

| 标签 | 状态 | 症状 |
| --- | --- | --- |
| macos-13 (Intel) | **已关停**（2025-09-19 官方公告） | 任务排队 **36h+ 无 runner** → 24h 队列超时自动取消 |
| macos-14 (arm64) | **已退役**（2026-11-02 官方公告） | 启动即取消（容量约束） |
| **macos-15 (arm64)** | ✅ 现役，universal 交叉编译标准池 | v2.6.0/v2.5.1 时代 macos-14 同模式连续成功 |

- 规则：release matrix 的 macOS 一律用 **macos-15**；升级周期关注
  `actions/runner-images` 的 EOL 公告（每镜像约 18 个月生命周期）。
- 诊断特征：job 长期 queued、`started_at: null`、~24h 整被自动取消 =
  标签已退役，换标签重跑即可；不要反复 rerun 空耗（每次 rerun 会取消
  上一 attempt 的排队任务）。
- 已知竞态（独立问题）：三矩阵并发 create-release 偶发
  `already_exists` 422 → `gh run rerun --failed` 重跑失败 job 即可。

### 已知抖动：三矩阵并发 create-release（2026-11-03 实证，v2.5.1）

- 症状：release workflow 中**最后到达**的 publish job 失败，日志
  `##[error]Validation Failed: {"code":"already_exists","field":"tag_name"}`——
  另一矩阵 job 已抢先创建 Release。
- 处置：`gh run rerun <run-id> --failed` 重跑失败 job 即可；Release 已存在时
  tauri-action 走上传路径，重跑后三平台资产齐套（v2.5.1 实证）。
- 根因：tauri-action 每矩阵 job 各自 create-release，无互斥；上游已知竞态，
  暂不做 workflow 加固（重跑成本低）。
