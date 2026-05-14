#!/bin/sh
set -eu

REPO="${STATUSLINE_REPO:-Darkwing4/statusline-rs-cc}"
INSTALL_DIR="${STATUSLINE_INSTALL_DIR:-$HOME/.claude/bin}"
TAG="${STATUSLINE_TAG:-latest}"
BIN="statusline"

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"

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
echo
echo "add this to ~/.claude/settings.json:"
echo
cat <<EOF
{
  "statusLine": {
    "type": "command",
    "command": "$INSTALL_DIR/$BIN"
  }
}
EOF
