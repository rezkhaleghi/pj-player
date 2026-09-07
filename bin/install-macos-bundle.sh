#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_DIR="$ROOT_DIR/target/macos-package/PJ-Player.app"
COMMAND_DIR="$ROOT_DIR/target/macos-package/command"
INSTALL_ROOT="$HOME/.local/share/pj-player"
INSTALL_DIR="$HOME/.local/bin"

if [[ ! -d "$APP_DIR" || ! -x "$COMMAND_DIR/pj-player" ]]; then
  "$ROOT_DIR/bin/package-macos.sh"
fi

mkdir -p "$HOME/Applications" "$INSTALL_DIR"
rm -rf "$HOME/Applications/PJ-Player.app"
cp -R "$APP_DIR" "$HOME/Applications/PJ-Player.app"
rm -rf "$INSTALL_ROOT"
mkdir -p "$INSTALL_ROOT"
cp -R "$COMMAND_DIR/bin" "$INSTALL_ROOT/bin"
cp "$COMMAND_DIR/pj-player" "$INSTALL_ROOT/pj-player"
chmod 755 "$INSTALL_ROOT/pj-player" "$INSTALL_ROOT/bin"/*
ln -sfn "$INSTALL_ROOT/pj-player" "$INSTALL_DIR/pj-player"

cat <<EOF
Installed PJ-Player to:
  $HOME/Applications/PJ-Player.app
  $INSTALL_DIR/pj-player

Add this directory to PATH if needed:
  export PATH="$INSTALL_DIR:\$PATH"

Then run:
  pj-player
EOF
