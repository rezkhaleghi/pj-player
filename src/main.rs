use reqwest::Client;
use std::process::Command;
use std::env;
use serde_json::Value;
use std::fs;
use std::io::{self};

#[derive(Debug)]
enum Source {
    WWW,
    YouTube,
    Spotify,
    InternetArchive,
}

#[async_trait::async_trait]
trait SourceHandler {
    async fn search(&self, query: &str) -> Result<Vec<(String, String, Source)>, Box<dyn std::error::Error>>;
    fn download(&self, identifier: &str, title: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn stream(&self, identifier: &str) -> Result<(), Box<dyn std::error::Error>>;
}

#[async_trait::async_trait]
impl SourceHandler for Source {
    async fn search(&self, query: &str) -> Result<Vec<(String, String, Source)>, Box<dyn std::error::Error>> {
        match self {
            Source::WWW => {
                let mut results = Vec::new();
                if let Ok(mut youtube_results) = search_youtube(query).await {
                    results.append(&mut youtube_results);
                }
                if let Ok(mut archive_results) = search_archive(query).await {
                    results.append(&mut archive_results);
                }
                if results.is_empty() {
                    Err("No results found".into())
                } else {
                    Ok(results)
                }
            }
            Source::YouTube => search_youtube(query).await,
            Source::Spotify => Err("Spotify search not implemented yet".into()),
            Source::InternetArchive => search_archive(query).await,
        }
    }

    fn download(&self, identifier: &str, title: &str) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Source::YouTube => download_youtube_audio(identifier, title),
            Source::Spotify => Err("Spotify download not implemented yet".into()),
            Source::InternetArchive => download_archive_audio(identifier, title),
            Source::WWW => Err("WWW download not supported directly".into()),
        }
    }

    fn stream(&self, identifier: &str) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Source::YouTube => stream_audio(identifier),
            _ => Err("Streaming not implemented for this source".into()),
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let args: Vec<String> = env::args().collect();
    let play_flag = args.contains(&"--play".to_string());

    let song_query = get_query_from_terminal();
    println!("Where would you like to search for the song?");
    println!("(Press ENTER to default to WWW)");
    println!("1. YouTube");
    println!("2. Internet Archive");
    println!("3. Spotify");

    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read line");
    let choice: usize = input.trim().parse().unwrap_or(4);

    let source = match choice {
        1 => Source::YouTube,
        2 => Source::InternetArchive,
        3 => Source::Spotify,
        4 => Source::WWW,
        _ => {
            println!("Invalid choice");
            return;
        }
    };

    match source.search(&song_query).await {
        Ok(results) => {
            println!("Found the following results:");
            for (i, (identifier, title, src)) in results.iter().enumerate() {
                println!("{}. {} (ID: {}, Source: {:?})", i + 1, title, identifier, src);
            }

            println!("Enter the number of the result you want to select:");
            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("Failed to read line");
            let choice: usize = input.trim().parse().expect("Invalid input");

            if choice > 0 && choice <= results.len() {
                let (identifier, title, src) = &results[choice - 1];
                if play_flag {
                    println!("Streaming audio...");
                    if let Err(e) = src.stream(identifier) {
                        eprintln!("Error streaming audio: {}", e);
                    }
                } else {
                    println!("Downloading audio...");
                    if let Err(e) = src.download(identifier, title) {
                        eprintln!("Error downloading audio: {}", e);
                    } else {
                        println!("Download complete!");
                    }
                }
            } else {
                println!("Invalid choice");
            }
        }
        Err(e) => {
            eprintln!("Error searching: {}", e);
        }
    }
}

fn stream_audio(video_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let youtube_url = format!("https://www.youtube.com/watch?v={}", video_id);

    // Stream audio directly using yt-dlp and ffplay
    let yt_dlp = Command::new("yt-dlp")
        .args(&[
            "-o", "-",          // Output to stdout
            "-f", "bestaudio",  // Choose the best audio stream
            "--quiet",
            &youtube_url,
        ])
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    let status = Command::new("ffplay")
        .args(&["-nodisp", "-autoexit", "-"])
        .stdin(yt_dlp.stdout.unwrap())
        .status();

    match status {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to stream audio: {}", e).into()),
    }
}

/// Searches for a song on YouTube using the YouTube Data API.
async fn search_youtube(query: &str) -> Result<Vec<(String, String, Source)>, Box<dyn std::error::Error>> {
    let api_key = "AIzaSyD9sc6z8J8I-imV-htavHTb1NP_q3EDcOY";  // Add your API key here
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
                results.push((video_id.to_string(), title.to_string(), Source::YouTube));
            }
        }
    }

    if results.is_empty() {
        Err("No videos found".into())
    } else {
        Ok(results)
    }
}

/// Searches for a song on Archive.org using the advanced search API.
async fn search_archive(query: &str) -> Result<Vec<(String, String, Source)>, Box<dyn std::error::Error>> {
    let url = format!(
        "https://archive.org/advancedsearch.php?q={}&output=json",
        query.replace(" ", "+")
    );

    let client = Client::new();
    let response = client.get(&url).send().await?;
    let json: Value = response.json().await?;

    let mut results = Vec::new();
    if let Some(items) = json["response"]["docs"].as_array() {
        for item in items {
            if let Some(identifier) = item["identifier"].as_str() {
                if let Some(title) = item["title"].as_str() {
                    results.push((identifier.to_string(), title.to_string(), Source::InternetArchive));
                }
            }
        }
    }

    if results.is_empty() {
        Err("No tracks found".into())
    } else {
        Ok(results)
    }
}

/// Downloads the audio of a YouTube video using yt-dlp.
fn download_youtube_audio(video_id: &str, title: &str) -> Result<(), Box<dyn std::error::Error>> {
    let download_path = "./";
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

/// Downloads the audio of a track from Archive.org.
fn download_archive_audio(identifier: &str, title: &str) -> Result<(), Box<dyn std::error::Error>> {
    let download_path = "./";
    fs::create_dir_all(download_path)?; // Ensure the directory exists

    let sanitized_title = title.replace("/", "_").replace("\\", "_");
    let output_path = format!("{}/{} (PJ-PLAYER).mp3", download_path, sanitized_title);
    let download_url = format!("https://archive.org/download/{}/{}", identifier, identifier);

    println!("Downloading audio to: {}", output_path);

    // Download file using `wget` or any preferred download tool
    let status = Command::new("wget")
        .args(&[
            "-O", &output_path,
            &download_url,
        ])
        .status();

    match status {
        Ok(status) if status.success() => println!("Download completed successfully."),
        Ok(status) => println!("wget returned an error: Exit code {}", status),
        Err(err) => println!("Error executing wget: {}", err),
    }

    Ok(())
}

/// Get the search query from the terminal arguments or user input.
fn get_query_from_terminal() -> String {
    // First, try to get the query from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        return args[1].clone(); // Get the first argument as the search query
    }

    // If no arguments, prompt the user for the search query
println!("Please enter the name of the song or artist you'd like to search for (e.g., 'Glorybox by Portishead' or 'Tool Sober'): ");
    let mut query = String::new();
    io::stdin()
        .read_line(&mut query)
        .expect("Failed to read line");
    query.trim().to_string()
}

