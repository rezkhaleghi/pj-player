// download.rs
use std::process::{ Command, Stdio };
use std::sync::{ Arc, Mutex };
use std::fs::{ self, File };
use std::thread;
use std::env;
use std::path::PathBuf;
use serde_json::Value;

// const YT_DLP_PATH: &str = "bin/yt-dlp";
const YT_DLP_PATH: &str = "yt-dlp";

fn get_download_path() -> PathBuf {
    let home_dir = env::var("HOME").expect("Could not find home directory");
    PathBuf::from(home_dir).join("Downloads")
}

pub fn download_youtube_audio(
    video_id: String,
    title: String,
    download_status: Arc<Mutex<Option<String>>>
) {
    let status_message = format!("{} is downloading", title);
    {
        let mut status = download_status.lock().unwrap();
        *status = Some(status_message);
    }

    thread::spawn(move || {
        let download_path = get_download_path();
        if let Err(e) = fs::create_dir_all(&download_path) {
            let mut status_message = download_status.lock().unwrap();
            *status_message = Some(format!("Failed to create directory: {}", e));
            return;
        }

        let sanitized_title = title.replace("/", "_").replace("\\", "_");
        let output_path = download_path.join(format!("{} (PJ-PLAYER).mp3", sanitized_title));

        let status = Command::new(YT_DLP_PATH)
            .args(
                &[
                    "--extract-audio",
                    "--audio-format",
                    "mp3",
                    "-o",
                    output_path.to_str().unwrap(),
                    &format!("https://www.youtube.com/watch?v={}", video_id),
                ]
            )
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        let mut status_message = download_status.lock().unwrap();
        *status_message = match status {
            Ok(status) if status.success() => Some(format!("{} downloaded successfully", title)),
            Ok(status) => Some(format!("yt-dlp returned an error: Exit code {}", status)),
            Err(err) => Some(format!("Error executing yt-dlp: {}", err)),
        };
    });
}

pub fn download_archive_audio(
    identifier: String,
    title: String,
    download_status: Arc<Mutex<Option<String>>>
) {
    let status_message = format!("{} is downloading", title);
    {
        let mut status = download_status.lock().unwrap();
        *status = Some(status_message);
    }

    thread::spawn(move || {
        let download_path = get_download_path();
        if let Err(e) = fs::create_dir_all(&download_path) {
            let mut status_message = download_status.lock().unwrap();
            *status_message = Some(format!("Failed to create directory: {}", e));
            return;
        }

        let sanitized_title = title.replace("/", "_").replace("\\", "_");
        let output_path = download_path.join(format!("{} (PJ-PLAYER).mp3", sanitized_title));
        let client = reqwest::blocking::Client::new();

        // More robust error handling
        match download_archive_file(&client, &identifier, &output_path) {
            Ok(_) => {
                let mut status_message = download_status.lock().unwrap();
                *status_message = Some(format!("{} downloaded successfully", title));
            }
            Err(e) => {
                let mut status_message = download_status.lock().unwrap();
                *status_message = Some(format!("Download failed: {}", e));
            }
        }
    });
}

// Separate function for download logic with better error handling
fn download_archive_file(
    client: &reqwest::blocking::Client,
    identifier: &str,
    output_path: &PathBuf
) -> Result<(), Box<dyn std::error::Error>> {
    let metadata_url = format!("https://archive.org/metadata/{}", identifier);
    let metadata_response = client.get(&metadata_url).send()?;
    let metadata: Value = metadata_response.json()?;

    // More flexible audio format search
    let audio_formats = ["VBR MP3", "MP3", "WAVE", "WAV", "FLAC", "OGG"];

    if let Some(files) = metadata["files"].as_array() {
        for format in &audio_formats {
            if let Some(file) = files.iter().find(|&f| f["format"] == *format) {
                if let Some(download_name) = file["name"].as_str() {
                    let download_url = format!(
                        "https://archive.org/download/{}/{}",
                        identifier,
                        download_name
                    );

                    let mut response = client.get(&download_url).send()?;
                    let mut file = File::create(output_path)?;
                    std::io::copy(&mut response, &mut file)?;

                    return Ok(());
                }
            }
        }
    }

    Err("No suitable audio file found".into())
}
