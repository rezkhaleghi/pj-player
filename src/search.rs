use std::error::Error;
use std::process::Command;
use serde_json::Value;
use reqwest::Client;
use crate::app::{ SearchResult, Source };

const YT_DLP_PATH: &str = "yt-dlp";

pub async fn search_youtube(query: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let output = Command::new(YT_DLP_PATH)
        .arg("--default-search")
        .arg("ytsearch")
        .arg(format!("ytsearch15:{}", query))
        .arg("--dump-json")
        .arg("--flat-playlist")
        .arg("--skip-download")
        .arg("--ignore-errors")
        .output()?;

    if !output.status.success() {
        return Err(
            format!(
                "yt-dlp failed with status: {:?}, stderr: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ).into()
        );
    }

    let stdout = String::from_utf8(output.stdout)?;
    let results: Vec<SearchResult> = stdout
        .lines()
        .filter_map(|line| {
            let json: Value = serde_json::from_str(line).ok()?;
            Some(SearchResult {
                identifier: json.get("id")?.as_str()?.to_string(),
                title: json.get("title")?.as_str()?.to_string(),
                source: Source::YouTube,
            })
        })
        .collect();

    Ok(results)
}

pub async fn search_archive(query: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let url = format!(
        "https://archive.org/advancedsearch.php?q={}&output=json&rows=15",
        query.replace(" ", "+")
    );

    let client = Client::new();
    let response = client.get(&url).send().await?;
    let json: Value = response.json().await?;

    let mut results = Vec::new();
    if let Some(items) = json["response"]["docs"].as_array() {
        for item in items {
            if
                let (Some(identifier), Some(title)) = (
                    item["identifier"].as_str(),
                    item["title"].as_str(),
                )
            {
                results.push(SearchResult {
                    identifier: identifier.to_string(),
                    title: title.to_string(),
                    source: Source::InternetArchive,
                });
            }
        }
    }

    Ok(results)
}

pub async fn search_fma(query: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let url = format!(
        "https://freemusicarchive.org/api/trackSearch?q={}&limit=10",
        query.replace(" ", "+")
    );

    let client = Client::new();
    let response = client.get(&url).send().await?;
    let json: Value = response.json().await?;

    let mut results = Vec::new();
    if let Some(dataset) = json["dataset"].as_array() {
        for item in dataset {
            if
                let (Some(track_id), Some(track_title), Some(track_url)) = (
                    item["track_id"].as_u64(),
                    item["track_title"].as_str(),
                    item["track_url"].as_str(),
                )
            {
                results.push(SearchResult {
                    identifier: track_id.to_string(),
                    title: track_title.to_string(),
                    source: Source::InternetArchive, // or Source::FMA if you add a new variant
                });
            }
        }
    }

    Ok(results)
}
