use reqwest::Client;
use std::process::Command;
use std::env;
use serde_json::Value;
use std::fs;
use std::io::{self, Write};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let song_query = "Shape of You Ed Sheeran";

    // Try YouTube
    match search_youtube(song_query).await {
        Ok(results) => {
            println!("Found the following videos on YouTube:");
            for (i, (video_id, title)) in results.iter().enumerate() {
                println!("{}. {} (ID: {})", i + 1, title, video_id);
            }

            println!("Enter the number of the video you want to download:");
            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("Failed to read line");
            let choice: usize = input.trim().parse().expect("Invalid input");

            if choice > 0 && choice <= results.len() {
                let (video_id, title) = &results[choice - 1];
                println!("Downloading audio...");
                if let Err(e) = download_youtube_audio(video_id, title) {
                    eprintln!("Error downloading audio: {}", e);
                } else {
                    println!("Download complete!");
                }
            } else {
                println!("Invalid choice");
            }
        }
        Err(e) => {
            eprintln!("Error searching on YouTube: {}", e);
        }
    }
}

/// Searches for a song on YouTube using the YouTube Data API.
async fn search_youtube(query: &str) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let api_key = env::var("YOUTUBE_API_KEY").expect("Missing YouTube API Key");
    let url = format!(
        "https://www.googleapis.com/youtube/v3/search?part=snippet&q={}&type=video&maxResults=10&key={}",
        query, api_key
    );

    let client = Client::new();
    let response = client.get(&url).send().await?;
    let json: Value = response.json().await?;

    let mut results = Vec::new();
    if let Some(items) = json["items"].as_array() {
        for item in items {
            if let (Some(video_id), Some(title)) = (
                item["id"]["videoId"].as_str(),
                item["snippet"]["title"].as_str(),
            ) {
                results.push((video_id.to_string(), title.to_string()));
            }
        }
    }

    if results.is_empty() {
        Err("No videos found".into())
    } else {
        Ok(results)
    }
}

/// Downloads the audio of a YouTube video using yt-dlp.
fn download_youtube_audio(video_id: &str, title: &str) -> Result<(), Box<dyn std::error::Error>> {
    let download_path = "./downloads";
    fs::create_dir_all(download_path)?; // Ensure the directory exists

    let sanitized_title = title.replace("/", "_").replace("\\", "_");
    let output_path = format!("{}/{} (PJ-PLAYER).mp3", download_path, sanitized_title);
    println!("Downloading audio to: {}", output_path);

    // Using yt-dlp for download
    let status = Command::new("yt-dlp")
        .args(&[
            "--extract-audio",
            "--audio-format", "mp3",
            "-o", &output_path,
            &format!("https://www.youtube.com/watch?v={}", video_id),
        ])
        .status();

    match status {
        Ok(status) if status.success() => println!("Download completed successfully."),
        Ok(status) => println!("yt-dlp returned an error: Exit code {}", status),
        Err(err) => println!("Error executing yt-dlp: {}", err),
    }

    Ok(())
}