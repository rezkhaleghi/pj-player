use std::error::Error;
use std::process::{ Child, Command };
use std::sync::{ Arc, Mutex };

use crate::search::{ search_archive, search_youtube };

#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    YouTube,
    InternetArchive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Stream,
    Download,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    SearchInput,
    SearchResults,
    InitialSelection,
    SourceSelection,
    Streaming,
    Downloading,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub identifier: String,
    pub title: String,
    pub source: Source,
}

pub struct StreamProcess {
    pub yt_dlp: Child,
    pub ffplay: Child,
}

pub struct AppUi {
    pub search_input: String,
    pub search_results: Vec<SearchResult>,
    pub selected_result_index: Option<usize>,
    pub selected_source_index: usize,
    pub source: Source,
    pub current_view: View,
    pub visualization_data: Arc<Mutex<Vec<u8>>>,
    pub stream_process: Option<StreamProcess>,
    pub mode: Option<Mode>,
    pub current_equalizer: usize,
    pub download_status: Arc<Mutex<Option<String>>>,
    pub paused: bool,
}

impl AppUi {
    pub fn new() -> Self {
        AppUi {
            search_input: String::new(),
            search_results: Vec::new(),
            selected_result_index: Some(0),
            selected_source_index: 0,
            source: Source::YouTube,
            current_view: View::SearchInput,
            visualization_data: Arc::new(Mutex::new(vec![0; 10])),
            stream_process: None,
            current_equalizer: 0,
            mode: None,
            download_status: Arc::new(Mutex::new(None)),
            paused: false,
        }
    }

    pub async fn search(&mut self) -> Result<(), Box<dyn Error>> {
        self.search_results = match self.source {
            Source::YouTube => search_youtube(&self.search_input).await?,
            Source::InternetArchive => search_archive(&self.search_input).await?,
        };

        self.current_view = View::SearchResults;

        self.selected_result_index = if self.search_results.is_empty() { None } else { Some(0) };

        Ok(())
    }

    pub fn stop_streaming(&mut self) {
        if let Some(mut stream_process) = self.stream_process.take() {
            let _ = stream_process.ffplay.kill();
            let _ = stream_process.yt_dlp.kill();

            let _ = stream_process.ffplay.wait();
            let _ = stream_process.yt_dlp.wait();
        }

        self.paused = false;
    }

    pub fn toggle_pause(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(stream_process) = &self.stream_process {
            let pid = stream_process.ffplay.id();

            let signal = if self.paused { "CONT" } else { "STOP" };

            let status = Command::new("kill").args(["-s", signal, &pid.to_string()]).status()?;

            if status.success() {
                self.paused = !self.paused;
                Ok(())
            } else {
                Err(format!("Failed to send {} signal to ffplay", signal).into())
            }
        } else {
            Err("No ffplay process running".into())
        }
    }
}

impl Drop for AppUi {
    fn drop(&mut self) {
        self.stop_streaming();
    }
}
