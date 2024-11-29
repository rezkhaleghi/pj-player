# PJ-Player

PJ-Player is a Rust-based application that allows users to search, DOWNLOAD or STREAM audio tracks directly from the terminal. Users can search for music from various sources like YouTube, Internet Archive, and more. The program supports streaming music instantly using tools like yt-dlp and ffplay (and cava for Audio Visualizer), as well as downloading audio using external tools like yt-dlp and wget. Whether you want to stream your favorite songs or download them for offline listening, PJ-Player makes it easy to access and enjoy music from multiple platforms through their respective APIs.

## Features

- **Search for audio** on YouTube, Internet Archive, and (future) Spotify.
- **Download audio** from YouTube or Internet Archive.
- Support for **command-line arguments** and **interactive search**.
- Customizable search query input via terminal.

## Requirements

Before running PJ-Player, you need to ensure that the following dependencies are installed on your system:

- **Rust**: This project is written in Rust. Install Rust by following the instructions at [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install).
- **yt-dlp**: A YouTube downloader tool. Install it from [https://github.com/yt-dlp/yt-dlp](https://github.com/yt-dlp/yt-dlp).
- **wget**: A command-line tool to download files from the web. Install it from [https://www.gnu.org/software/wget/](https://www.gnu.org/software/wget/).
- **dotenv**: The application uses environment variables for certain configurations (like YouTube API key). Install the necessary `.env` file and configure API keys.

## Installation

To use [`PJ-Player`]

1. **Install Rust** (if not already installed):

   - Follow the instructions on the official [Rust installation page](https://www.rust-lang.org/tools/install).
   - On most systems, you can install it via the following command:
     ```sh
     curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
     ```

2. **Clone the Repository**:
   Clone this repository to your local machine:

   ```sh
   git clone https://github.com/rezkhaleghi/pj-player.git
   cd pj-player
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

1. **Run the application and search for your favorite music**:

   For Download:

   ```sh
   pjplayer "Portishead Glorybox"
   ```

   For Stream:

   ```sh
   pjplayer "Portishead Glorybox" --play
   ```

2. **Select the source where you want to search for the song**:

   ```sh
   Where would you like to search for the song?
   (Press ENTER to default to WWW)
   1. YouTube
   2. Internet Archive
   ```

3. **Select the result number you wish to Download / Stream from the search results list:**:

   After selecting the source, you will see the following results:

   ```sh
   Found the following results:
   1. Portishead - Glory Box (ID: 4qQyUi4zfDs, Source: YouTube)
   2. Portishead - Glory Box - Remastered (ID: yAKX51r7erw, Source: YouTube)
   3. Portishead Glory Box Live At Roseland NY ( Best Audio) (ID: MnMTK8EdsOc, Source: YouTube)
   4. Portishead - Glory Box (lyrics) (ID: g2lhOPjLEfk, Source: YouTube)
   5. Portishead - Glory box (Roseland NYC) (HQ) (ID: SLrkE6T_m5Y, Source: YouTube)
   6. Glory Box (Live) (ID: JBfAtRvW1Ao, Source: YouTube)
   7. Portishead - Glory Box (Lyrics) [Tiktok Song] (ID: 2XTLhm6EcFw, Source: YouTube)
   8. Portishead Glory Box Live (ID: C3LK5ELvZwI, Source: YouTube)
   9. Portishead - Glory Box (HD Version) (ID: 6ylDDs3mdJE, Source: YouTube)
   10. Portishead - glory box (ID: yF-GvT8Clnk, Source: YouTube)
   ```

4. **Stream Or wait for Download**:

   After selecting the result, you will be prompted with two options:

   - To wait for the download to finish:

   If you choose to download the file first, you will see the message:

   ```sh
   Download started...
   Download completed successfully.
   ```

   - To stream immediately:

   If you choose to stream the song immediately, you will see the message and then shows the Audio Visualizer (cava) :

   ```sh
   <YOU FAVORITE SONG> IS STREAMING...
   ```

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
