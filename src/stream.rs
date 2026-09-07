use std::path::Path;
use std::process::Command;
use std::sync::{Arc, Mutex};

use crate::app::StreamProcess;
use crate::error::AppError;

const YT_DLP_PATH: &str = "yt-dlp";

pub struct StreamInfo {
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
    visualization_data: Arc<Mutex<Vec<u8>>>,
) -> Result<(StreamProcess, StreamInfo), AppError> {
    let youtube_url = format!("https://www.youtube.com/watch?v={}", video_id);

    let metadata_output = Command::new(YT_DLP_PATH)
        .args([
            "--skip-download",
            "--print",
            "%(title)s\n%(duration)s",
            &youtube_url,
        ])
        .output()?;

    if !metadata_output.status.success() {
        return Err(AppError::Process {
            command: YT_DLP_PATH.to_string(),
            status: metadata_output.status.code(),
            stderr: String::from_utf8_lossy(&metadata_output.stderr).to_string(),
        });
    }

    let metadata = String::from_utf8_lossy(&metadata_output.stdout);

    let mut lines = metadata.lines();

    let _title = lines.next().unwrap_or("Unknown Song").trim();

    let duration_text = lines.next().unwrap_or("0").trim();

    let duration = duration_text.parse::<f64>().unwrap_or(0.0);

    let url_output = Command::new(YT_DLP_PATH)
        .args(["-f", "bestaudio", "--get-url", &youtube_url])
        .output()?;

    if !url_output.status.success() {
        return Err(AppError::Process {
            command: YT_DLP_PATH.to_string(),
            status: url_output.status.code(),
            stderr: String::from_utf8_lossy(&url_output.stderr).to_string(),
        });
    }

    let direct_url = String::from_utf8_lossy(&url_output.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();

    if direct_url.is_empty() {
        return Err(AppError::Message(
            "yt-dlp returned an empty audio URL".to_string(),
        ));
    }

    let stream_process = StreamProcess::start(direct_url, 0.0, visualization_data)?;

    Ok((stream_process, StreamInfo { duration }))
}

pub fn stream_local_audio(
    path: &Path,
    visualization_data: Arc<Mutex<Vec<u8>>>,
) -> Result<(StreamProcess, StreamInfo), AppError> {
    let duration_output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()?;

    let duration = if duration_output.status.success() {
        String::from_utf8_lossy(&duration_output.stdout)
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0)
    } else {
        0.0
    };

    let stream_process =
        StreamProcess::start(path.to_string_lossy().into_owned(), 0.0, visualization_data)?;

    Ok((stream_process, StreamInfo { duration }))
}
