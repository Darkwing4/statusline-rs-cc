#!/bin/sh
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

INSTALL_DIR="${STATUSLINE_INSTALL_DIR:-$HOME/.claude/bin}"
BIN="statusline"

if [ -f "config/local.ron" ] && [ -z "${STATUSLINE_CONFIG:-}" ]; then
    STATUSLINE_CONFIG="config/local.ron"
    export STATUSLINE_CONFIG
    echo "using config/local.ron"
elif [ -n "${STATUSLINE_CONFIG:-}" ]; then
    echo "using $STATUSLINE_CONFIG"
else
    echo "using config/default.ron"
fi

cargo build --release

mkdir -p "$INSTALL_DIR"
install -m 0755 "target/release/$BIN" "$INSTALL_DIR/$BIN"

echo "installed: $INSTALL_DIR/$BIN"
