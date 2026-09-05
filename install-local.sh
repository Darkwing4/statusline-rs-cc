#!/bin/sh
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

INSTALL_DIR="${STATUSLINE_INSTALL_DIR:-$HOME/.claude/bin}"
CONFIG_PATH="${STATUSLINE_CONFIG_PATH:-$HOME/.claude/statusline/config.ron}"
BIN="statusline"

if [ -f "config/local.ron" ] && [ -z "${STATUSLINE_CONFIG:-}" ]; then
    STATUSLINE_CONFIG="config/local.ron"
    export STATUSLINE_CONFIG
    echo "using config/local.ron"
elif [ -n "${STATUSLINE_CONFIG:-}" ]; then
    echo "using $STATUSLINE_CONFIG"
else
    STATUSLINE_CONFIG="config/default.ron"
    echo "using config/default.ron"
fi

cargo build --release

"target/release/$BIN" --check-config "$STATUSLINE_CONFIG"

mkdir -p "$INSTALL_DIR"
install -m 0755 "target/release/$BIN" "$INSTALL_DIR/$BIN"

echo "installed: $INSTALL_DIR/$BIN"

config_dir="$(dirname "$CONFIG_PATH")"
mkdir -p "$config_dir"
pending_config="$(mktemp "$config_dir/.config.ron.XXXXXX")"
install -m 0644 "$STATUSLINE_CONFIG" "$pending_config"
mv -f "$pending_config" "$CONFIG_PATH"

echo "installed config: $CONFIG_PATH"
