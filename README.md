# PJ-Player

PJ-Player is a Rust-based terminal application (TUI) that allows users to stream, download, and play local audio directly from the terminal.

### End-user macOS instructions

Download the release corresponding to your Mac's architecture:

- **Apple Silicon**: M1, M2, M3, M4, and newer Macs use the `arm64` release.
- **Intel**: older Intel Macs use the `x86_64` release.

After downloading:

1. Move `pjplayer.app` to `/Applications`.
2. Double-click it to start PJ-Player in Terminal.
3. If macOS blocks the unsigned app, right-click it, choose **Open**, and confirm. Alternatively, run:

   ```sh
   xattr -dr com.apple.quarantine /Applications/pjplayer.app
   ```

To also use PJ-Player with the `pjplayer` terminal command, run:

```sh
mkdir -p "$HOME/.local/bin"

cat > "$HOME/.local/bin/pjplayer" <<'EOF'
#!/bin/sh
if [ -x "/Applications/pjplayer.app/Contents/MacOS/pjplayer" ]; then
   APP_BIN="/Applications/pjplayer.app/Contents/MacOS/pjplayer"
elif [ -x "$HOME/Applications/pjplayer.app/Contents/MacOS/pjplayer" ]; then
   APP_BIN="$HOME/Applications/pjplayer.app/Contents/MacOS/pjplayer"
else
   echo "pjplayer.app was not found. Move it to /Applications or ~/Applications." >&2
   exit 1
fi

exec "$APP_BIN" "$@"
EOF

chmod +x "$HOME/.local/bin/pjplayer"
echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.zshrc"
source "$HOME/.zshrc"
```

The user can then start PJ-Player from any Terminal window:

```sh
pjplayer
```

The app must remain at `/Applications/pjplayer.app` or `~/Applications/pjplayer.app` for this terminal command to work. A signed `.pkg` release sets up both the clickable app and terminal command automatically.

# DEMO

![Project Demo](/demos/demo.gif)

## Features

- **Search for audio** on YouTube or Internet Archive.
- **Stream audio** on YouTube.
- **Download audio** from YouTube or Internet Archive.
- **Offline Player** mode for selecting a folder and playing its audio files in order.
- **About** screen with project and repository information.

## Requirements

Before running PJ-Player, you need to ensure that the following dependencies are installed on your system:

- **Rust**: This project is written in Rust. Install Rust by following the instructions at [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install).
- **yt-dlp**: A YouTube downloader tool. Install it from [https://github.com/yt-dlp/yt-dlp](https://github.com/yt-dlp/yt-dlp).
- **ffmpeg**: A complete, cross-platform solution to record, convert and stream audio and video. Install it from [https://ffmpeg.org/](https://ffmpeg.org/).

## Installation

To use [`PJ-Player`]

1. **Clone the Repository**:
   Clone this repository to your local machine:

   ```sh
   git clone https://github.com/rezkhaleghi/pj-player.git
   cd pj-player
   ```

2. **Install Dependencies** (if not already installed):

   - Install Dependencies Manually 
   - OR Run the install.sh (for macos: install-macos.sh) script in bin Directory
     (Assuming your in the /pj-player Directory)

   ```sh
   ./bin/install.sh
   ```

3. **Build the Project**:
   Build the pjgrep application with the following command:

   ```sh
   cargo build --release
   ```

4. **Install the Binary**:
   Optionally, you can copy the binary to a directory in your `$PATH` (e.g., `/usr/local/bin` or `~/bin`) for easy access:

   ```sh
   sudo cp target/release/pjplayer /usr/local/bin/pjplayer
   ```

## Self-contained macOS package

End users do not need Rust, Cargo, yt-dlp, ffmpeg, or ffplay when using a packaged release. The release bundle contains the compiled app and standalone runtime tools.

These two commands are for the **release builder**, because the first command compiles Rust and downloads the runtime tools:

```sh
./bin/package-macos.sh
```

This creates `target/macos-package/pjplayer.app` and a terminal distribution under `target/macos-package/command`. To install both the clickable app and the `pjplayer` terminal command for the current user:

```sh
./bin/install-macos-bundle.sh
```

For end users, distribute the generated `pjplayer.app` in a signed zip or DMG. They download it, move it to Applications, and double-click it. To provide the terminal command too, distribute the generated `command` directory with an installer package; the end user should not need to run `package-macos.sh`.

For public distribution, sign and notarize the app with an Apple Developer certificate. Build separate arm64 and x86_64 releases, or provide a universal build, because ffmpeg and ffplay must match the user's Mac architecture.

The `ARCHITECTURE` file in `bin/` and in `target/macos-package/` identifies the binaries in that directory as `arm64` or `intel`. The macOS packaging script creates or refreshes the package marker automatically.

The current `evermeet.cx` download used by the packaging script provides Intel ffmpeg/ffplay binaries. The script therefore refuses to create an arm64 package until arm64 or universal ffmpeg/ffplay binaries are supplied. This avoids giving Apple Silicon users a package that unexpectedly requires Rosetta.

### One-click installer package

To create a `.pkg` that installs both the clickable app and the `pjplayer` terminal command:

```sh
./bin/package-macos-pkg.sh
```

Without signing identities this creates an unsigned package for local testing. For a public release, first set your Developer ID identities:

```sh
export PJ_PLAYER_APP_SIGN_IDENTITY="Developer ID Application: Your Name (TEAMID)"
export PJ_PLAYER_INSTALLER_SIGN_IDENTITY="3rd Party Mac Developer Installer: Your Name (TEAMID)"
./bin/package-macos-pkg.sh
```

The package installs `pjplayer.app` into `/Applications` and `pjplayer` into `/usr/local/bin`. Configure an app-specific `notarytool` profile, then notarize it with:

```sh
export PJ_PLAYER_NOTARY_PROFILE="pjplayer-notary"
./bin/notarize-macos-pkg.sh target/macos-package/pjplayer.pkg
```

Users download the notarized `pjplayer.pkg`, double-click it, and can then either open PJ-Player from Applications or run `pjplayer` in Terminal. The installer asks for an administrator password because it writes to `/Applications` and `/usr/local`.

## Usage

1. **Run the application**:

   ```sh
   pjplayer
   ```

2. **Select a mode**:
   ![Project Demo](/demos/1-select-mode.jpeg)

3. **For Stream or Download, search for music**:

   ![Project Demo](/demos/2-search.jpeg)

4. **Select One of Search Results**:

   ![Project Demo](/demos/3-select.jpeg)

5. **Stream Or Download The Content**:
- Stream
   ![Project Demo](/demos/4-stream.jpeg)
- Download
   ![Project Demo](/demos/5-dl.jpeg)

For Offline Player, choose **Offline Player** from the startup menu, enter a folder path, and select a track. Supported files are `mp3`, `m4a`, `wav`, `flac`, `ogg`, `aac`, and `opus`; playback advances through the folder automatically.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request on GitHub.

## Author

Created and maintained by "PocketJack (Rez Khaleghi)"

- GitHub: https://github.com/rezkhaleghi
- Email: rezaxkhaleghi@gmail.com

## Support

If you enjoy this app and would like to support my work:
- Patreon : https://patreon.com/PocketJack
Your support helps me continue developing free and open-source stuff.



## License

This project is licensed under the MIT License. See the LICENSE file for details.

## Acknowledgements

- [Rust Programming Language](https://www.rust-lang.org/)
