use serde_json::Value;
use std::env;
use std::fs::{self, File};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::error::AppError;

// const YT_DLP_PATH: &str = "bin/yt-dlp";
const YT_DLP_PATH: &str = "yt-dlp";

fn get_download_path() -> Result<PathBuf, AppError> {
    let home_dir = env::var("HOME")
        .map_err(|_| AppError::Message("Could not find home directory".to_string()))?;

    Ok(PathBuf::from(home_dir).join("Downloads"))
}

pub fn download_youtube_audio(
    video_id: String,
    title: String,
    download_status: Arc<Mutex<Option<String>>>,
) {
    let status_message = format!("{} is downloading", title);

    if let Ok(mut status) = download_status.lock() {
        *status = Some(status_message);
    }

    thread::spawn(move || {
        let download_path = match get_download_path() {
            Ok(path) => path,
            Err(error) => {
                if let Ok(mut status_message) = download_status.lock() {
                    *status_message = Some(format!("Download failed: {}", error));
                }

                return;
            }
        };

        if let Err(error) = fs::create_dir_all(&download_path) {
            if let Ok(mut status_message) = download_status.lock() {
                *status_message = Some(format!("Failed to create directory: {}", error));
            }

            return;
        }

        let sanitized_title = title.replace("/", "_").replace("\\", "_");

        let output_path = download_path.join(format!("{} (PJ-PLAYER).mp3", sanitized_title));

        let output_path = match output_path.to_str() {
            Some(path) => path,
            None => {
                if let Ok(mut status_message) = download_status.lock() {
                    *status_message = Some("Download failed: invalid output path".to_string());
                }

                return;
            }
        };

        let youtube_url = format!("https://www.youtube.com/watch?v={}", video_id);

        let status = Command::new(YT_DLP_PATH)
            .args([
                "--extract-audio",
                "--audio-format",
                "mp3",
                "-o",
                output_path,
                &youtube_url,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        let result = match status {
            Ok(status) if status.success() => Ok(format!("{} downloaded successfully", title)),

            Ok(status) => Err(AppError::Process {
                command: YT_DLP_PATH.to_string(),
                status: status.code(),
                stderr: String::new(),
            }),

            Err(error) => Err(AppError::Io(error)),
        };

        if let Ok(mut status_message) = download_status.lock() {
            *status_message = Some(match result {
                Ok(message) => message,
                Err(error) => format!("Download failed: {}", error),
            });
        }
    });
}

pub fn download_archive_audio(
    identifier: String,
    title: String,
    download_status: Arc<Mutex<Option<String>>>,
) {
    let status_message = format!("{} is downloading", title);

    if let Ok(mut status) = download_status.lock() {
        *status = Some(status_message);
    }

    thread::spawn(move || {
        let download_path = match get_download_path() {
            Ok(path) => path,
            Err(error) => {
                if let Ok(mut status_message) = download_status.lock() {
                    *status_message = Some(format!("Download failed: {}", error));
                }

                return;
            }
        };

        if let Err(error) = fs::create_dir_all(&download_path) {
            if let Ok(mut status_message) = download_status.lock() {
                *status_message = Some(format!("Failed to create directory: {}", error));
            }

            return;
        }

        let sanitized_title = title.replace("/", "_").replace("\\", "_");

        let output_path = download_path.join(format!("{} (PJ-PLAYER).mp3", sanitized_title));

        let client = reqwest::blocking::Client::new();

        match download_archive_file(&client, &identifier, &output_path) {
            Ok(_) => {
                if let Ok(mut status_message) = download_status.lock() {
                    *status_message = Some(format!("{} downloaded successfully", title));
                }
            }

            Err(error) => {
                if let Ok(mut status_message) = download_status.lock() {
                    *status_message = Some(format!("Download failed: {}", error));
                }
            }
        }
    });
}

fn download_archive_file(
    client: &reqwest::blocking::Client,
    identifier: &str,
    output_path: &PathBuf,
) -> Result<(), AppError> {
    let metadata_url = format!("https://archive.org/metadata/{}", identifier);

    let metadata_response = client.get(&metadata_url).send()?;

    let metadata: Value = metadata_response.json()?;

    // More flexible audio format search.
    let audio_formats = ["VBR MP3", "MP3", "WAVE", "WAV", "FLAC", "OGG"];

    if let Some(files) = metadata["files"].as_array() {
        for format in &audio_formats {
            if let Some(file) = files.iter().find(|file| file["format"] == *format) {
                if let Some(download_name) = file["name"].as_str() {
                    let download_url = format!(
                        "https://archive.org/download/{}/{}",
                        identifier, download_name
                    );

                    let mut response = client.get(&download_url).send()?;

                    let mut file = File::create(output_path)?;

                    std::io::copy(&mut response, &mut file)?;

                    return Ok(());
                }
            }
        }
    }

    Err(AppError::Message(
        "No suitable audio file found".to_string(),
    ))
}
