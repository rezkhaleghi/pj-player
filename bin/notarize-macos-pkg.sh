#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PKG_PATH="${1:-$ROOT_DIR/target/macos-package/PJ-Player.pkg}"
NOTARY_PROFILE="${PJ_PLAYER_NOTARY_PROFILE:-}"

if [[ -z "$NOTARY_PROFILE" ]]; then
  echo "Set PJ_PLAYER_NOTARY_PROFILE to an app-specific notarytool keychain profile." >&2
  exit 1
fi

xcrun notarytool submit "$PKG_PATH" \
  --keychain-profile "$NOTARY_PROFILE" \
  --wait

xcrun stapler staple "$PKG_PATH"
xcrun stapler validate "$PKG_PATH"
spctl --assess --type install --verbose=4 "$PKG_PATH"
