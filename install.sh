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
checksum="$asset.sha256"

token="${GH_TOKEN:-${GITHUB_TOKEN:-}}"
if [ -z "$token" ] && command -v gh >/dev/null 2>&1; then
    token="$(gh auth token 2>/dev/null || true)"
fi

auth_header=""
if [ -n "$token" ]; then
    auth_header="Authorization: Bearer $token"
fi

if [ "$TAG" = "latest" ]; then
    release_url="https://github.com/$REPO/releases/latest/download"
    api_url="https://api.github.com/repos/$REPO/releases/latest"
    api_endpoint="repos/$REPO/releases/latest"
else
    release_url="https://github.com/$REPO/releases/download/$TAG"
    api_url="https://api.github.com/repos/$REPO/releases/tags/$TAG"
    api_endpoint="repos/$REPO/releases/tags/$TAG"
fi

fetch() {
    if [ -n "$auth_header" ]; then
        curl -fsSL -H "$auth_header" "$@"
    else
        curl -fsSL "$@"
    fi
}

find_asset_id() {
    requested_asset="$1"

    if command -v python3 >/dev/null 2>&1; then
        printf '%s' "$release_json" | python3 -c '
import json
import sys

name = sys.argv[1]
matches = [
    str(asset["id"])
    for asset in json.load(sys.stdin).get("assets", [])
    if asset.get("name") == name and isinstance(asset.get("id"), int)
]
if len(matches) != 1:
    sys.exit(1)
print(matches[0])
' "$requested_asset"
    elif command -v jq >/dev/null 2>&1; then
        printf '%s' "$release_json" | jq -er --arg name "$requested_asset" '
            [.assets[] | select(.name == $name) | .id]
            | if length == 1 then .[0] else error("asset not found") end
        '
    elif command -v gh >/dev/null 2>&1; then
        GH_TOKEN="$token" gh api "$api_endpoint" --jq '.assets[] | [.name, .id] | @tsv' |
            awk -F '	' -v name="$requested_asset" '
                $1 == name {
                    id = $2
                    count++
                }
                END {
                    if (count != 1) {
                        exit 1
                    }
                    print id
                }
            '
    else
        echo "python3, jq, or gh is required for authenticated release downloads" >&2
        return 1
    fi
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "downloading $asset"
if fetch -o "$tmp/$asset" "$release_url/$asset"; then
    echo "downloading $checksum"
    fetch -o "$tmp/$checksum" "$release_url/$checksum"
else
    if [ -z "$token" ]; then
        echo "asset $asset not found in release" >&2
        echo "release URL: $release_url" >&2
        exit 1
    fi

    echo "fetching release metadata: $api_url"
    release_json="$(fetch -H "Accept: application/vnd.github+json" "$api_url")"

    if ! asset_id="$(find_asset_id "$asset")"; then
        echo "asset $asset not found in release" >&2
        echo "release URL: $api_url" >&2
        exit 1
    fi

    if ! checksum_id="$(find_asset_id "$checksum")"; then
        echo "asset $checksum not found in release" >&2
        echo "release URL: $api_url" >&2
        exit 1
    fi

    echo "downloading $asset"
    fetch -H "Accept: application/octet-stream" -o "$tmp/$asset" \
        "https://api.github.com/repos/$REPO/releases/assets/$asset_id"

    echo "downloading $checksum"
    fetch -H "Accept: application/octet-stream" -o "$tmp/$checksum" \
        "https://api.github.com/repos/$REPO/releases/assets/$checksum_id"
fi

if ! expected_hash="$(awk -v name="$asset" '
    NR == 1 {
        hash = $1
        file = $2
        if (file == "*" name) {
            file = name
        }
    }
    END {
        if (NR != 1 || NF != 2 || length(hash) != 64 || hash ~ /[^0-9A-Fa-f]/ || file != name) {
            exit 1
        }
        print tolower(hash)
    }
' "$tmp/$checksum")"; then
    echo "invalid checksum file: $checksum" >&2
    exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
    actual_hash="$(sha256sum "$tmp/$asset")"
elif command -v shasum >/dev/null 2>&1; then
    actual_hash="$(shasum -a 256 "$tmp/$asset")"
else
    echo "sha256sum or shasum is required to verify $asset" >&2
    exit 1
fi

actual_hash="${actual_hash%% *}"

if [ "$actual_hash" != "$expected_hash" ]; then
    echo "checksum mismatch for $asset" >&2
    exit 1
fi

echo "verified $asset"

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
