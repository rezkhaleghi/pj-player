// stream.rs

use std::error::Error;
use std::process::Command;
use std::sync::{ atomic::{ AtomicBool, Ordering }, Arc, Mutex };
use std::thread;
use std::time::Duration;

use crate::app::StreamProcess;

const YT_DLP_PATH: &str = "yt-dlp";

pub struct StreamInfo {
    pub title: String,
    pub duration: f64,
    pub direct_url: String,
}

/// Starts a YouTube audio stream.
///
/// The pipeline is:
///
///     YouTube
///        ↓
///     yt-dlp metadata + direct URL
///        ↓
///     ffplay
///
/// ffplay receives the direct media URL itself, which allows us
/// to restart it with `-ss` when the user seeks.
pub fn stream_audio(
    video_id: &str,
    visualization_data: Arc<Mutex<Vec<u8>>>
) -> Result<(StreamProcess, StreamInfo), Box<dyn Error>> {
    let youtube_url = format!("https://www.youtube.com/watch?v={}", video_id);

    // Get the title and duration from yt-dlp.
    //
    // We request both values in one metadata call so we don't
    // need another process just to determine the song duration.
    let metadata_output = Command::new(YT_DLP_PATH)
        .args(["--skip-download", "--print", "%(title)s\n%(duration)s", &youtube_url])
        .output()?;

    if !metadata_output.status.success() {
        let error = String::from_utf8_lossy(&metadata_output.stderr);

        return Err(format!("yt-dlp failed to get video metadata: {}", error.trim()).into());
    }

    let metadata = String::from_utf8_lossy(&metadata_output.stdout);

    let mut lines = metadata.lines();

    let title = lines.next().unwrap_or("Unknown Song").trim().to_string();

    let duration_text = lines.next().unwrap_or("0").trim();

    let duration = duration_text.parse::<f64>().unwrap_or(0.0);

    // Ask yt-dlp for the actual media URL.
    let url_output = Command::new(YT_DLP_PATH)
        .args(["-f", "bestaudio", "--get-url", &youtube_url])
        .output()?;

    if !url_output.status.success() {
        let error = String::from_utf8_lossy(&url_output.stderr);

        return Err(format!("yt-dlp failed to get audio URL: {}", error.trim()).into());
    }

    let direct_url = String::from_utf8_lossy(&url_output.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();

    if direct_url.is_empty() {
        return Err("yt-dlp returned an empty audio URL".into());
    }

    println!("Streaming: {}", title);

    // This flag controls the lifetime of the visualizer worker.
    //
    // The StreamProcess owns the flag, so stopping the stream
    // automatically tells the visualizer thread to exit.
    let visualizer_running = Arc::new(AtomicBool::new(true));

    // Start ffplay directly from the media URL.
    let stream_process = StreamProcess::start(
        direct_url.clone(),
        0.0,
        Arc::clone(&visualizer_running)
    )?;

    // Keep the existing visualizer temporarily.
    //
    // This is still fake visualization data and will be replaced
    // in the visualization step later.
    let visualization_data_clone = Arc::clone(&visualization_data);
    let visualizer_running_clone = Arc::clone(&visualizer_running);

    thread::spawn(move || {
        while visualizer_running_clone.load(Ordering::Relaxed) {
            let Ok(mut data) = visualization_data_clone.lock() else {
                return;
            };

            for value in data.iter_mut() {
                *value = (*value + 1) % 10;
            }

            drop(data);

            thread::sleep(Duration::from_millis(100));
        }
    });

    Ok((
        stream_process,
        StreamInfo {
            title,
            duration,
            direct_url,
        },
    ))
}
