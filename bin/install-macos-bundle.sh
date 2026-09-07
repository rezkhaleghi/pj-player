#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_DIR="$ROOT_DIR/target/macos-package/pjplayer.app"
COMMAND_DIR="$ROOT_DIR/target/macos-package/command"
INSTALL_ROOT="$HOME/.local/share/pjplayer"
INSTALL_DIR="$HOME/.local/bin"

if [[ ! -d "$APP_DIR" || ! -x "$COMMAND_DIR/pjplayer" ]]; then
  "$ROOT_DIR/bin/package-macos.sh"
fi

mkdir -p "$HOME/Applications" "$INSTALL_DIR"
rm -rf "$HOME/Applications/pjplayer.app"
cp -R "$APP_DIR" "$HOME/Applications/pjplayer.app"
rm -rf "$INSTALL_ROOT"
mkdir -p "$INSTALL_ROOT"
cp -R "$COMMAND_DIR/bin" "$INSTALL_ROOT/bin"
cp "$COMMAND_DIR/pjplayer" "$INSTALL_ROOT/pjplayer"
chmod 755 "$INSTALL_ROOT/pjplayer" "$INSTALL_ROOT/bin"/*
ln -sfn "$INSTALL_ROOT/pjplayer" "$INSTALL_DIR/pjplayer"

cat <<EOF
Installed PJ-Player to:
  $HOME/Applications/pjplayer.app
  $INSTALL_DIR/pjplayer

Add this directory to PATH if needed:
  export PATH="$INSTALL_DIR:\$PATH"

Then run:
  pjplayer
EOF
