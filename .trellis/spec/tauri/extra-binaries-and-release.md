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
