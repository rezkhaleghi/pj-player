#!/usr/bin/env bash
set -euo pipefail

APP_NAME="pjplayer.app"
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="$ROOT_DIR/target/macos-package"
APP_DIR="$BUILD_DIR/$APP_NAME"
CONTENTS_DIR="$APP_DIR/Contents"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
BIN_DIR="$RESOURCES_DIR/bin"
ARCH="$(uname -m)"

case "$ARCH" in
  arm64) FFMPEG_URL="https://evermeet.cx/ffmpeg/getrelease/zip" ;;
  x86_64) FFMPEG_URL="https://evermeet.cx/ffmpeg/getrelease/zip" ;;
  *) echo "Unsupported macOS architecture: $ARCH" >&2; exit 1 ;;
esac

YT_DLP_URL="https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"

rm -rf "$APP_DIR"
mkdir -p "$CONTENTS_DIR/MacOS" "$BIN_DIR"

echo "Building PJ-Player for $ARCH..."
cargo build --release --manifest-path "$ROOT_DIR/Cargo.toml"

cp "$ROOT_DIR/target/release/pjplayer" "$CONTENTS_DIR/MacOS/pjplayer"
chmod 755 "$CONTENTS_DIR/MacOS/pjplayer"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
if [[ "$ARCH" == "arm64" ]] &&
  command -v ffmpeg >/dev/null &&
  command -v ffplay >/dev/null &&
  file "$(command -v ffmpeg)" "$(command -v ffplay)" | grep -q "arm64"; then
  echo "Using native ffmpeg and ffplay from PATH..."
  cp "$(command -v ffmpeg)" "$BIN_DIR/ffmpeg"
  cp "$(command -v ffplay)" "$BIN_DIR/ffplay"
else
  echo "Downloading ffmpeg and ffplay for $ARCH..."
  curl --fail --location "$FFMPEG_URL" --output "$tmp_dir/ffmpeg.zip"
  unzip -q "$tmp_dir/ffmpeg.zip" -d "$tmp_dir/ffmpeg"
  cp "$tmp_dir/ffmpeg/ffmpeg" "$BIN_DIR/ffmpeg"
  curl --fail --location "https://evermeet.cx/ffmpeg/getrelease/ffplay/zip" --output "$tmp_dir/ffplay.zip"
  unzip -q "$tmp_dir/ffplay.zip" -d "$tmp_dir/ffplay"
  cp "$tmp_dir/ffplay/ffplay" "$BIN_DIR/ffplay"
fi

if [[ "$ARCH" == "arm64" ]] && ! file "$BIN_DIR/ffmpeg" "$BIN_DIR/ffplay" | grep -q "arm64"; then
  echo "Downloaded ffmpeg/ffplay are not arm64 binaries. Refusing to create an Apple Silicon package." >&2
  echo "Use an arm64 or universal ffmpeg/ffplay distribution, or install Rosetta separately." >&2
  exit 1
fi

echo "Downloading standalone yt-dlp..."
curl --fail --location "$YT_DLP_URL" --output "$BIN_DIR/yt-dlp"
chmod 755 "$BIN_DIR/ffmpeg" "$BIN_DIR/ffplay" "$BIN_DIR/yt-dlp"

cat > "$CONTENTS_DIR/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>pj-player-launcher</string>
  <key>CFBundleIdentifier</key><string>com.pocketjack.pjplayer</string>
  <key>CFBundleName</key><string>PJ-Player</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>0.1.0</string>
</dict>
</plist>
PLIST

cat > "$CONTENTS_DIR/MacOS/pj-player-launcher" <<'LAUNCHER'
#!/usr/bin/env bash
set -euo pipefail
APP_DIR="$(cd "$(dirname "$0")/.." && pwd)"
osascript -e "tell application \"Terminal\" to do script \"exec '$APP_DIR/MacOS/pjplayer'\""
LAUNCHER
chmod 755 "$CONTENTS_DIR/MacOS/pj-player-launcher"

rm -rf "$BUILD_DIR/command"
mkdir -p "$BUILD_DIR/command"
cp "$CONTENTS_DIR/MacOS/pjplayer" "$BUILD_DIR/command/pjplayer"
cp -R "$BIN_DIR" "$BUILD_DIR/command/bin"
chmod 755 "$BUILD_DIR/command/pjplayer"

cat <<EOF

Created:
  $APP_DIR
  $BUILD_DIR/command

Open the app with:
  open "$APP_DIR"

Run from a terminal with:
  "$BUILD_DIR/command/pjplayer"

For distribution, sign and notarize the app, then package it in a DMG or zip.
EOF
