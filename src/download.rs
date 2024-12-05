use std::process::{ Command, Stdio };
use std::sync::{ Arc, Mutex };
use std::fs;
use std::thread;

const YT_DLP_PATH: &str = "yt-dlp";
const WGET_PATH: &str = "wget";

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
        let download_path = "./";
        fs::create_dir_all(download_path).unwrap();

        let sanitized_title = title.replace("/", "_").replace("\\", "_");
        let output_path = format!("{}/{} (PJ-PLAYER).mp3", download_path, sanitized_title);

        let status = Command::new(YT_DLP_PATH)
            .args(
                &[
                    "--extract-audio",
                    "--audio-format",
                    "mp3",
                    "-o",
                    &output_path,
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
        let download_path = "./";
        fs::create_dir_all(download_path).unwrap();

        let sanitized_title = title.replace("/", "_").replace("\\", "_");
        let output_path = format!("{}/{} (PJ-PLAYER).mp3", download_path, sanitized_title);
        let download_url = format!("https://archive.org/download/{}/{}", identifier, identifier);

        let status = Command::new(WGET_PATH)
            .args(&["-O", &output_path, &download_url])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        let mut status_message = download_status.lock().unwrap();
        *status_message = match status {
            Ok(status) if status.success() => Some(format!("{} downloaded successfully", title)),
            Ok(status) => Some(format!("wget returned an error: Exit code {}", status)),
            Err(err) => Some(format!("Error executing wget: {}", err)),
        };
    });
}
