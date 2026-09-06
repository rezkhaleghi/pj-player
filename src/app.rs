// app.rs

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

/// Owns all processes involved in an active audio stream.
///
/// AppUi should not need to know that streaming uses both yt-dlp
/// and ffplay. This struct is responsible for managing those
/// processes and exposing only the operations the application needs.
pub struct StreamProcess {
    yt_dlp: Child,
    ffplay: Child,
}

impl StreamProcess {
    /// Creates a new StreamProcess from the yt-dlp and ffplay
    /// child processes.
    ///
    /// The processes are kept private so process management stays
    /// inside this type instead of being spread across AppUi.
    pub fn new(yt_dlp: Child, ffplay: Child) -> Self {
        Self {
            yt_dlp,
            ffplay,
        }
    }

    /// Returns the ffplay process ID.
    ///
    /// This is currently needed for sending pause/resume signals
    /// to ffplay on macOS/Linux.
    pub fn ffplay_id(&self) -> u32 {
        self.ffplay.id()
    }

    /// Stops the entire audio stream.
    ///
    /// Both processes are killed and then waited on so we don't
    /// leave child processes running after the stream is stopped.
    pub fn stop(mut self) {
        // Stop ffplay first because it is the process consuming
        // the audio produced by yt-dlp.
        let _ = self.ffplay.kill();

        // Stop yt-dlp as well so it doesn't continue downloading
        // audio after ffplay has exited.
        let _ = self.yt_dlp.kill();

        // Reap both child processes and avoid leaving zombies.
        let _ = self.ffplay.wait();
        let _ = self.yt_dlp.wait();
    }

    /// Pauses ffplay by sending SIGSTOP.
    ///
    /// SIGSTOP is currently used because ffplay is an external
    /// process. This implementation is Unix-specific and will
    /// be replaced later if we want a more portable approach.
    pub fn pause(&self) -> Result<(), Box<dyn Error>> {
        let status = Command::new("kill")
            .args(["-s", "STOP", &self.ffplay.id().to_string()])
            .status()?;

        if status.success() {
            Ok(())
        } else {
            Err("Failed to pause ffplay".into())
        }
    }

    /// Resumes ffplay by sending SIGCONT.
    pub fn resume(&self) -> Result<(), Box<dyn Error>> {
        let status = Command::new("kill")
            .args(["-s", "CONT", &self.ffplay.id().to_string()])
            .status()?;

        if status.success() {
            Ok(())
        } else {
            Err("Failed to resume ffplay".into())
        }
    }
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

        // Keep the selected index valid when search returns no results.
        self.selected_result_index = if self.search_results.is_empty() { None } else { Some(0) };

        Ok(())
    }

    /// Stops the currently active stream.
    ///
    /// AppUi only tells StreamProcess to stop; it does not need
    /// to know how yt-dlp and ffplay are managed internally.
    pub fn stop_streaming(&mut self) {
        if let Some(stream_process) = self.stream_process.take() {
            stream_process.stop();
        }

        // Reset pause state after the stream has been stopped.
        self.paused = false;
    }

    /// Toggles the paused state of the current stream.
    ///
    /// The actual process manipulation is delegated to StreamProcess.
    pub fn toggle_pause(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(stream_process) = &self.stream_process else {
            return Err("No ffplay process running".into());
        };

        if self.paused {
            stream_process.resume()?;
            self.paused = false;
        } else {
            stream_process.pause()?;
            self.paused = true;
        }

        Ok(())
    }
}

impl Drop for AppUi {
    fn drop(&mut self) {
        // Make sure an active stream is stopped when the application
        // exits, even if AppUi is dropped from another code path.
        self.stop_streaming();
    }
}
