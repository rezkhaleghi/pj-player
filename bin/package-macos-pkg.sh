#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="$ROOT_DIR/target/macos-package"
APP_DIR="$BUILD_DIR/pjplayer.app"
STAGE_DIR="$BUILD_DIR/pkg-root"
PKG_PATH="$BUILD_DIR/pjplayer.pkg"
VERSION="${PJ_PLAYER_VERSION:-0.1.0}"
APP_SIGN_IDENTITY="${PJ_PLAYER_APP_SIGN_IDENTITY:-}"
INSTALLER_SIGN_IDENTITY="${PJ_PLAYER_INSTALLER_SIGN_IDENTITY:-}"

"$ROOT_DIR/bin/package-macos.sh"

if [[ -n "$APP_SIGN_IDENTITY" ]]; then
  echo "Signing bundled runtime binaries..."
  codesign --force --options runtime --timestamp --sign "$APP_SIGN_IDENTITY" \
    "$APP_DIR/Contents/Resources/bin/ffmpeg" \
    "$APP_DIR/Contents/Resources/bin/ffplay" \
    "$APP_DIR/Contents/Resources/bin/yt-dlp"

  echo "Signing PJ-Player..."
  codesign --force --options runtime --timestamp --sign "$APP_SIGN_IDENTITY" \
    "$APP_DIR/Contents/MacOS/pjplayer"

  echo "Signing application bundle..."
  codesign --force --deep --options runtime --timestamp --sign "$APP_SIGN_IDENTITY" "$APP_DIR"
  codesign --verify --deep --strict --verbose=2 "$APP_DIR"
else
  echo "APP_SIGN_IDENTITY is not set; creating an unsigned package for local testing."
fi

rm -rf "$STAGE_DIR" "$PKG_PATH"
mkdir -p "$STAGE_DIR/Applications" "$STAGE_DIR/usr/local/lib/pjplayer" "$STAGE_DIR/usr/local/bin"
cp -R "$APP_DIR" "$STAGE_DIR/Applications/pjplayer.app"
cp "$BUILD_DIR/command/pjplayer" "$STAGE_DIR/usr/local/lib/pjplayer/pjplayer"
cp -R "$BUILD_DIR/command/bin/." "$STAGE_DIR/usr/local/lib/pjplayer/bin"
ln -s ../lib/pjplayer/pjplayer "$STAGE_DIR/usr/local/bin/pjplayer"
chmod 755 "$STAGE_DIR/usr/local/lib/pjplayer/pjplayer" "$STAGE_DIR/usr/local/lib/pjplayer/bin"/*

PKGBUILD_ARGS=(
  --root "$STAGE_DIR"
  --identifier com.pocketjack.pjplayer
  --version "$VERSION"
  --install-location /
  --ownership recommended
)

if [[ -n "$INSTALLER_SIGN_IDENTITY" ]]; then
  PKGBUILD_ARGS+=(--sign "$INSTALLER_SIGN_IDENTITY")
fi

pkgbuild "${PKGBUILD_ARGS[@]}" "$PKG_PATH"

if [[ -n "$INSTALLER_SIGN_IDENTITY" ]]; then
  pkgutil --check-signature "$PKG_PATH"
fi

cat <<EOF

Created installer:
  $PKG_PATH

It installs:
  /Applications/pjplayer.app
  /usr/local/bin/pjplayer
  /usr/local/lib/pjplayer/bin/

Test locally with:
  sudo installer -pkg "$PKG_PATH" -target /

For public distribution, notarize the package after signing it.
EOF
