#!/usr/bin/env bash
# 在 main 分支上 bump 版本号、提交、打 tag 并推送，触发 GitHub release 工作流。
#
# 用法:
#   ./scripts/release.sh patch          # 1.2.0 -> 1.2.1
#   ./scripts/release.sh minor          # 1.2.0 -> 1.3.0
#   ./scripts/release.sh major          # 1.2.0 -> 2.0.0
#   ./scripts/release.sh 1.2.1          # 指定版本
#
# 也可: pnpm release patch

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PKG_JSON="$ROOT/package.json"
TAURI_CONF="$ROOT/src-tauri/tauri.conf.json"
CARGO_TOML="$ROOT/src-tauri/Cargo.toml"

die() {
  echo "error: $*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "缺少命令: $1"
}

require_cmd git
require_cmd node
require_cmd python3

BUMP="${1:-}"
[[ -n "$BUMP" ]] || die "请指定 bump 类型 (patch|minor|major) 或具体版本号 (如 1.2.1)"

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
[[ "$BRANCH" == "main" ]] || die "当前分支为「${BRANCH}」，请在 main 分支运行"

if [[ -n "$(git status --porcelain)" ]]; then
  die "工作区不干净，请先提交或 stash 本地改动"
fi

git fetch origin main >/dev/null 2>&1 || true
LOCAL="$(git rev-parse HEAD)"
REMOTE="$(git rev-parse origin/main 2>/dev/null || echo "")"
if [[ -n "$REMOTE" && "$LOCAL" != "$REMOTE" ]]; then
  die "本地 main 与 origin/main 不一致，请先 pull 或 push"
fi

CURRENT="$(node -p "require('./package.json').version")"
[[ "$CURRENT" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "当前版本格式无效: ${CURRENT}"

NEW="$(python3 - "$CURRENT" "$BUMP" <<'PY'
import re
import sys

current = sys.argv[1]
bump = sys.argv[2]

def parse(v: str) -> tuple[int, int, int]:
    m = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)", v)
    if not m:
        raise SystemExit(f"invalid semver: {v}")
    return int(m.group(1)), int(m.group(2)), int(m.group(3))

if re.fullmatch(r"\d+\.\d+\.\d+", bump):
    new = bump
else:
    major, minor, patch = parse(current)
    if bump == "patch":
        patch += 1
    elif bump == "minor":
        minor += 1
        patch = 0
    elif bump == "major":
        major += 1
        minor = 0
        patch = 0
    else:
        raise SystemExit(f"unknown bump: {bump}")
    new = f"{major}.{minor}.{patch}"

parse(new)
if tuple(map(int, new.split("."))) <= tuple(map(int, current.split("."))):
    raise SystemExit(f"new version must be greater than current ({current} -> {new})")

print(new)
PY
)" || die "无法计算新版本（bump=${BUMP}, current=${CURRENT}）"

TAG="v${NEW}"
if git rev-parse "$TAG" >/dev/null 2>&1; then
  die "标签 ${TAG} 已存在"
fi

echo "发布版本: ${CURRENT} -> ${NEW} (${TAG})"

node - "$NEW" <<'NODE'
const fs = require("fs");

const version = process.argv[1];
const root = process.cwd();

const pkgPath = `${root}/package.json`;
const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
pkg.version = version;
fs.writeFileSync(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`);

const tauriPath = `${root}/src-tauri/tauri.conf.json`;
const tauri = JSON.parse(fs.readFileSync(tauriPath, "utf8"));
tauri.version = version;
fs.writeFileSync(tauriPath, `${JSON.stringify(tauri, null, 2)}\n`);
NODE

# 仅更新 [package] 段的 version（行首 version =，不匹配依赖里的 { version = ... }）
python3 - "$NEW" "$CARGO_TOML" <<'PY'
import pathlib
import re
import sys

new_version = sys.argv[1]
path = pathlib.Path(sys.argv[2])
text = path.read_text(encoding="utf-8")
updated, count = re.subn(
    r'^(version = ")[^"]+(")\s*$',
    rf'\g<1>{new_version}\2',
    text,
    count=1,
    flags=re.MULTILINE,
)
if count != 1:
    raise SystemExit("failed to update src-tauri/Cargo.toml version")
path.write_text(updated, encoding="utf-8")
PY

CONF_VERSION="$(node -e "console.log(require('./src-tauri/tauri.conf.json').version)")"
CARGO_VERSION="$(python3 - <<PY
import re, pathlib
text = pathlib.Path("src-tauri/Cargo.toml").read_text()
m = re.search(r'^version = "([^"]+)"', text, re.M)
print(m.group(1) if m else "")
PY
)"

[[ "$NEW" == "$CONF_VERSION" && "$NEW" == "$CARGO_VERSION" ]] \
  || die "版本同步失败 (package=${NEW}, tauri=${CONF_VERSION}, cargo=${CARGO_VERSION})"

git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml
git commit -m "$(cat <<EOF
chore(release): ${TAG}

Bump version ${CURRENT} -> ${NEW} to trigger release workflow.
EOF
)"

git tag -a "$TAG" -m "Trove ${TAG}"

echo "推送 main 与 ${TAG} …"
git push origin main
git push origin "$TAG"

echo ""
echo "已推送 ${TAG}，GitHub Actions release 工作流将自动构建并发布。"
echo "查看进度: https://github.com/ZturnLibs/trove/actions"
