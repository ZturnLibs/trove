#!/usr/bin/env bash
# 验证 GitHub Releases updater endpoint 对匿名客户端可达，且 manifest 结构正确。
set -euo pipefail

ENDPOINT="${1:-https://github.com/ZturnLibs/trove/releases/latest/download/latest.json}"
SIMULATE_VERSION="${2:-1.2.1}"

echo "endpoint: ${ENDPOINT}"
echo "simulate client version: ${SIMULATE_VERSION}"

fetch_json() {
  if JSON="$(curl -fsSL --http1.1 --connect-timeout 30 "${ENDPOINT}" 2>/dev/null)"; then
    printf '%s' "${JSON}"
    return 0
  fi
  python3 - "${ENDPOINT}" <<'PY'
import json, sys
from urllib.request import Request, urlopen
url = sys.argv[1]
with urlopen(Request(url, headers={"User-Agent": "Trove-Updater-Verify/1.0"}), timeout=30) as resp:
    print(resp.read().decode())
PY
}

JSON="$(fetch_json)"
VERSION="$(node -e "const d=JSON.parse(process.argv[1]); if(!d.version) process.exit(1); console.log(d.version)" "${JSON}")"
PLATFORMS="$(node -e "const d=JSON.parse(process.argv[1]); console.log(Object.keys(d.platforms||{}).join(', '))" "${JSON}")"

echo "latest version: ${VERSION}"
echo "platforms: ${PLATFORMS}"

node -e "
const current = process.argv[1];
const latest = process.argv[2];
const parse = v => v.replace(/^v/, '').split('.').map(Number);
const c = parse(current);
const l = parse(latest);
const newer = l[0] > c[0] || (l[0] === c[0] && l[1] > c[1]) || (l[0] === c[0] && l[1] === c[1] && l[2] > c[2]);
if (!newer) { console.error('no update offered for', current, '->', latest); process.exit(1); }
console.log('update available:', current, '->', latest);
" "${SIMULATE_VERSION}" "${VERSION}"

# macOS / Windows bundle URL smoke check
node -e "
const d = JSON.parse(process.argv[1]);
const keys = ['darwin-aarch64', 'windows-x86_64'];
for (const k of keys) {
  const p = d.platforms?.[k];
  if (!p?.url) { console.log('skip', k); continue; }
  console.log(k + ':', p.url);
}
" "${JSON}"

echo "OK: updater endpoint verified"
