// stream.rs

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::process::{ Command, Stdio };
use std::sync::{ Arc, Mutex };
use std::thread;
use std::time::Duration;

use crate::app::StreamProcess;

const YT_DLP_PATH: &str = "yt-dlp";
const FFMPEG_PATH: &str = "ffplay";

/// Starts streaming a YouTube video.
///
/// The audio pipeline is:
///
///     YouTube
///        ↓
///     yt-dlp
///        ↓ stdout
///     ffplay
///
/// The returned StreamProcess owns both child processes so the
/// application can later pause, resume, or stop the stream.
pub fn stream_audio(
    video_id: &str,
    visualization_data: Arc<Mutex<Vec<u8>>>
) -> Result<StreamProcess, Box<dyn Error>> {
    // Build the YouTube URL from the selected search result.
    let youtube_url = format!("https://www.youtube.com/watch?v={}", video_id);

    // First ask yt-dlp for the title so we can display what is
    // currently being streamed.
    let output = Command::new(YT_DLP_PATH).args(["-s", "--get-title", &youtube_url]).output()?;

    let song_name = String::from_utf8_lossy(&output.stdout).trim().to_string();

    println!("Streaming: {}", song_name);

    // Start yt-dlp and tell it to write the best available audio
    // directly to stdout instead of creating a file.
    let mut yt_dlp = Command::new(YT_DLP_PATH)
        .args(["-o", "-", "-f", "bestaudio", "--quiet", &youtube_url])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    // Move yt-dlp's stdout pipe into ffplay.
    //
    // `take()` is important here because we still need to keep
    // ownership of the yt-dlp Child itself inside StreamProcess.
    let ffplay_stdin = yt_dlp.stdout.take().unwrap();

    // Clone the visualization state so the visualization thread
    // can update it independently of the main application.
    let visualization_data_clone = Arc::clone(&visualization_data);

    // Start ffplay and feed it the audio coming from yt-dlp.
    let ffplay = Command::new(FFMPEG_PATH)
        .args(["-nodisp", "-autoexit", "-loglevel", "quiet", "-"])
        .stdin(ffplay_stdin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let ffplay_id = ffplay.id();

    // This thread currently generates fake visualization data.
    //
    // It checks whether ffplay is still running and updates the
    // visualization approximately every 100ms.
    //
    // We will replace this OS/process polling and random-data
    // approach in a later step.
    thread::spawn(move || {
        let mut file = File::open("/dev/urandom").unwrap();

        while
            Command::new("ps")
                .arg("-p")
                .arg(ffplay_id.to_string())
                .output()
                .unwrap()
                .status.success()
        {
            let mut data = visualization_data_clone.lock().unwrap();

            for value in data.iter_mut() {
                let mut buf = [0u8; 1];

                // Generate a random value for the fake equalizer.
                file.read_exact(&mut buf).unwrap();

                *value = buf[0] % 10;
            }

            // Update the visualization roughly ten times per second.
            thread::sleep(Duration::from_millis(100));
        }
    });

    // StreamProcess now owns both child processes.
    //
    // AppUi does not need to know that there are two processes or
    // how they are stopped.
    Ok(StreamProcess::new(yt_dlp, ffplay))
}
