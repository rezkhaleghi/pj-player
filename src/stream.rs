use std::error::Error;
use std::process::Command;
use std::sync::{ Arc, Mutex };

use crate::app::StreamProcess;

const YT_DLP_PATH: &str = "yt-dlp";

pub struct StreamInfo {
    pub title: String,
    pub duration: f64,
}

/// Resolves a YouTube video into a direct audio URL and starts
/// the StreamProcess playback pipeline.
///
/// yt-dlp:
///
///     YouTube → direct audio URL
///
/// StreamProcess:
///
///     direct URL → ffmpeg → raw PCM → ffplay
///                              │
///                              └→ Rust visualization
pub fn stream_audio(
    video_id: &str,
    visualization_data: Arc<Mutex<Vec<u8>>>
) -> Result<(StreamProcess, StreamInfo), Box<dyn Error>> {
    let youtube_url = format!("https://www.youtube.com/watch?v={}", video_id);

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

    let stream_process = StreamProcess::start(direct_url, 0.0, visualization_data)?;

    Ok((
        stream_process,
        StreamInfo {
            title,
            duration,
        },
    ))
}
