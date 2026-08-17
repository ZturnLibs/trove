#!/usr/bin/env node
// Tauri universal macOS only lipos the default-run binary. Extra cargo bins
// (trove-cli) must be merged before the bundle step copies them into the .app.
import { existsSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const targetRoot = existsSync(join(process.cwd(), "target"))
  ? join(process.cwd(), "target")
  : join(repoRoot, "src-tauri", "target");

const extraBins = ["trove-cli"];

for (const name of extraBins) {
  const out = join(targetRoot, "universal-apple-darwin", "release", name);
  const arm = join(targetRoot, "aarch64-apple-darwin", "release", name);
  const intel = join(targetRoot, "x86_64-apple-darwin", "release", name);
  if (!existsSync(arm) || !existsSync(intel)) {
    continue;
  }
  if (existsSync(out)) {
    continue;
  }
  const result = spawnSync("lipo", ["-create", arm, intel, "-output", out], {
    stdio: "inherit",
  });
  if ((result.status ?? 1) !== 0) {
    process.exit(result.status ?? 1);
  }
}
