#!/bin/sh
set -eu

REPO="${STATUSLINE_REPO:-Darkwing4/statusline-rs-cc}"
INSTALL_DIR="${STATUSLINE_INSTALL_DIR:-$HOME/.claude/bin}"
SETTINGS="${STATUSLINE_SETTINGS:-$HOME/.claude/settings.json}"
TAG="${STATUSLINE_TAG:-latest}"
BIN="statusline"
SKIP_SETTINGS="${STATUSLINE_SKIP_SETTINGS:-}"

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"

case "$os" in
    msys*|mingw*|cygwin*)
        cat >&2 <<'EOF'
Detected a Windows shell (Git Bash / MSYS / Cygwin).
Run the PowerShell installer instead:

  powershell -NoProfile -ExecutionPolicy Bypass -Command "iwr -useb https://raw.githubusercontent.com/Darkwing4/statusline-rs-cc/main/install.ps1 | iex"
EOF
        exit 1
        ;;
esac

case "$os-$arch" in
    linux-x86_64)           target="x86_64-unknown-linux-gnu" ;;
    linux-aarch64|linux-arm64) target="aarch64-unknown-linux-gnu" ;;
    darwin-x86_64)          target="x86_64-apple-darwin" ;;
    darwin-arm64)           target="aarch64-apple-darwin" ;;
    *) echo "unsupported platform: $os-$arch" >&2; exit 1 ;;
esac

asset="statusline-${target}.tar.gz"

token="${GH_TOKEN:-${GITHUB_TOKEN:-}}"
if [ -z "$token" ] && command -v gh >/dev/null 2>&1; then
    token="$(gh auth token 2>/dev/null || true)"
fi

auth_header=""
if [ -n "$token" ]; then
    auth_header="Authorization: Bearer $token"
fi

if [ "$TAG" = "latest" ]; then
    api_url="https://api.github.com/repos/$REPO/releases/latest"
else
    api_url="https://api.github.com/repos/$REPO/releases/tags/$TAG"
fi

fetch() {
    if [ -n "$auth_header" ]; then
        curl -fsSL -H "$auth_header" "$@"
    else
        curl -fsSL "$@"
    fi
}

echo "fetching release metadata: $api_url"
release_json="$(fetch -H "Accept: application/vnd.github+json" "$api_url")"

asset_id="$(printf '%s' "$release_json" | awk -v name="$asset" '
    /"assets"[[:space:]]*:[[:space:]]*\[/ { in_assets=1 }
    in_assets && /"id"[[:space:]]*:[[:space:]]*[0-9]+/ {
        match($0, /[0-9]+/)
        current_id = substr($0, RSTART, RLENGTH)
    }
    in_assets && /"name"[[:space:]]*:[[:space:]]*"[^"]+"/ {
        n = $0
        sub(/.*"name"[[:space:]]*:[[:space:]]*"/, "", n)
        sub(/".*/, "", n)
        if (n == name) { print current_id; exit }
    }
')"

if [ -z "$asset_id" ]; then
    echo "asset $asset not found in release" >&2
    echo "release URL: $api_url" >&2
    exit 1
fi

asset_url="https://api.github.com/repos/$REPO/releases/assets/$asset_id"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "downloading $asset"
fetch -H "Accept: application/octet-stream" -o "$tmp/$asset" "$asset_url"

tar -xzf "$tmp/$asset" -C "$tmp"

mkdir -p "$INSTALL_DIR"
install -m 0755 "$tmp/$BIN" "$INSTALL_DIR/$BIN"

echo
echo "installed: $INSTALL_DIR/$BIN"

print_snippet() {
    cat <<EOF

add this to $SETTINGS manually:

{
  "statusLine": {
    "type": "command",
    "command": "$INSTALL_DIR/$BIN"
  }
}
EOF
}

if [ -n "$SKIP_SETTINGS" ]; then
    print_snippet
    exit 0
fi

if ! command -v python3 >/dev/null 2>&1; then
    echo "python3 not found, leaving $SETTINGS untouched"
    print_snippet
    exit 0
fi

SETTINGS="$SETTINGS" CMD="$INSTALL_DIR/$BIN" python3 <<'PY'
import json, os, sys
from pathlib import Path

path = Path(os.environ["SETTINGS"])
cmd = os.environ["CMD"]
path.parent.mkdir(parents=True, exist_ok=True)

data = {}
if path.exists() and path.stat().st_size > 0:
    try:
        data = json.loads(path.read_text())
        if not isinstance(data, dict):
            raise ValueError("top-level JSON is not an object")
    except Exception as e:
        print(f"warning: could not parse {path}: {e}", file=sys.stderr)
        print(f"leaving {path} untouched", file=sys.stderr)
        sys.exit(0)

prev = (data.get("statusLine") or {}).get("command")
if prev == cmd:
    print(f"{path}: statusLine already points at {cmd}")
    sys.exit(0)

if path.exists():
    backup = path.with_suffix(path.suffix + ".bak")
    backup.write_text(path.read_text())
    print(f"backup written: {backup}")

data["statusLine"] = {"type": "command", "command": cmd}
path.write_text(json.dumps(data, indent=2) + "\n")
if prev:
    print(f"replaced previous statusLine command: {prev}")
print(f"updated: {path}")
PY

echo
echo "done. statusline will refresh on the next Claude Code event."
