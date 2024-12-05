# PJ-Player

PJ-Player is a Rust-based terminal application (TUI) that allows users to search, DOWNLOAD or STREAM audio tracks directly from the terminal. It supports downloading and streaming from YouTube and Internet Archive.

# DEMO

![Project Demo](/demos/demo.gif)

## Features

- **Search for audio** on YouTube, Internet Archive, and (future) Spotify.
- **Download audio** from YouTube or Internet Archive.
- Support for **command-line arguments** and **interactive search**.

## Requirements

Before running PJ-Player, you need to ensure that the following dependencies are installed on your system:

- **Rust**: This project is written in Rust. Install Rust by following the instructions at [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install).
- **yt-dlp**: A YouTube downloader tool. Install it from [https://github.com/yt-dlp/yt-dlp](https://github.com/yt-dlp/yt-dlp).
- **wget**: A command-line tool to download files from the web. Install it from [https://www.gnu.org/software/wget/](https://www.gnu.org/software/wget/).
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
   - OR Run the install.sh script in bin Directory
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

## Usage

1. **Run the application**:

   ```sh
   pjplayer
   ```

2. **Select DONLOAD / STREAM**:
   ![Project Demo](/demos/1-select-mode.jpeg)

3. **Search For Your Favorite Song/Podcast**:

   ![Project Demo](/demos/2-search.jpeg)

4. **Select One of Search Results**:

   ![Project Demo](/demos/3-select.jpeg)

5. **Stream Or Download The Content**:
- Stream
   ![Project Demo](/demos/4-stream.jpeg)
- Download
   ![Project Demo](/demos/5-dl.jpeg)

## Contributing

Contributions are welcome! Please open an issue or submit a pull request on GitHub.

## Author

Created and maintained by "PocketJack (Rez Khaleghi)"

- GitHub: https://github.com/rezkhaleghi
- Email: rezaxkhaleghi@gmail.com

## License

This project is licensed under the MIT License. See the LICENSE file for details.

## Acknowledgements

- [Rust Programming Language](https://www.rust-lang.org/)
