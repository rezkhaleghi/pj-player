// app.rs

use std::error::Error;
use std::process::{ Child, Command };
use std::sync::{ Arc, Mutex };
use std::time::Instant;

use crate::search::{ search_archive, search_youtube };

const FFMPEG_PATH: &str = "ffplay";

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

/// Owns the ffplay process for the current stream.
///
/// yt-dlp is no longer kept alive during playback. Instead, yt-dlp
/// gives us a temporary direct media URL and ffplay plays that URL.
///
/// Keeping the URL here allows us to restart ffplay when seeking.
pub struct StreamProcess {
    ffplay: Child,
    direct_url: String,
}

impl StreamProcess {
    /// Starts ffplay using a direct media URL.
    ///
    /// `position` is the position in seconds from which playback
    /// should begin.
    pub fn start(direct_url: String, position: f64) -> Result<Self, Box<dyn Error>> {
        let ffplay = Command::new(FFMPEG_PATH)
            .args([
                "-nodisp",
                "-autoexit",
                "-loglevel",
                "quiet",
                "-ss",
                &position.to_string(),
                &direct_url,
            ])
            .spawn()?;

        Ok(Self {
            ffplay,
            direct_url,
        })
    }

    /// Returns the ffplay process ID.
    pub fn ffplay_id(&self) -> u32 {
        self.ffplay.id()
    }

    /// Stops ffplay and waits for it to exit.
    pub fn stop(mut self) {
        let _ = self.ffplay.kill();
        let _ = self.ffplay.wait();
    }

    /// Pauses ffplay using SIGSTOP.
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

    /// Resumes ffplay using SIGCONT.
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

    /// Restarts ffplay from a new position.
    ///
    /// Seeking is implemented by restarting ffplay with a new
    /// `-ss` value because ffplay does not give our TUI a convenient
    /// seeking API in the current setup.
    pub fn seek(&mut self, position: f64) -> Result<(), Box<dyn Error>> {
        let _ = self.ffplay.kill();
        let _ = self.ffplay.wait();

        self.ffplay = Command::new(FFMPEG_PATH)
            .args([
                "-nodisp",
                "-autoexit",
                "-loglevel",
                "quiet",
                "-ss",
                &position.to_string(),
                &self.direct_url,
            ])
            .spawn()?;

        Ok(())
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

    /// Total duration of the currently playing song in seconds.
    pub duration: f64,

    /// Current playback position in seconds.
    pub position: f64,

    /// When playback started or resumed.
    ///
    /// We use this to calculate the current position without
    /// repeatedly querying ffplay.
    pub playback_started_at: Option<Instant>,
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

            duration: 0.0,
            position: 0.0,
            playback_started_at: None,
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

    /// Starts tracking playback time.
    pub fn start_playback(&mut self, duration: f64) {
        self.duration = duration;
        self.position = 0.0;
        self.playback_started_at = Some(Instant::now());
        self.paused = false;
    }

    /// Updates the current playback position.
    ///
    /// This is called by the main event loop before rendering.
    pub fn update_playback_position(&mut self) {
        if self.current_view != View::Streaming || self.paused {
            return;
        }

        let Some(started_at) = self.playback_started_at else {
            return;
        };

        self.position = started_at.elapsed().as_secs_f64();

        // Never display a position beyond the known duration.
        if self.duration > 0.0 {
            self.position = self.position.min(self.duration);
        }
    }

    pub fn stop_streaming(&mut self) {
        if let Some(stream_process) = self.stream_process.take() {
            stream_process.stop();
        }

        self.paused = false;
        self.position = 0.0;
        self.duration = 0.0;
        self.playback_started_at = None;
    }

    pub fn toggle_pause(&mut self) -> Result<(), Box<dyn Error>> {
        if self.stream_process.is_none() {
            return Err("No ffplay process running".into());
        }

        if self.paused {
            self.stream_process.as_ref().ok_or("No ffplay process running")?.resume()?;

            self.paused = false;

            self.playback_started_at = Some(
                Instant::now() - std::time::Duration::from_secs_f64(self.position)
            );
        } else {
            // Update the position before pausing.
            self.update_playback_position();

            self.stream_process.as_ref().ok_or("No ffplay process running")?.pause()?;

            self.paused = true;
            self.playback_started_at = None;
        }

        Ok(())
    }

    /// Seeks forward by 15 seconds.
    pub fn seek_forward(&mut self) -> Result<(), Box<dyn Error>> {
        self.seek_by(15.0)
    }

    /// Seeks backward by 15 seconds.
    pub fn seek_backward(&mut self) -> Result<(), Box<dyn Error>> {
        self.seek_by(-15.0)
    }

    /// Seeks relative to the current playback position.
    fn seek_by(&mut self, amount: f64) -> Result<(), Box<dyn Error>> {
        self.update_playback_position();

        let mut new_position = self.position + amount;

        // Never seek before the beginning.
        new_position = new_position.max(0.0);

        // Never seek beyond the end of the song.
        if self.duration > 0.0 {
            new_position = new_position.min(self.duration);
        }

        let Some(stream_process) = &mut self.stream_process else {
            return Err("No ffplay process running".into());
        };

        stream_process.seek(new_position)?;

        self.position = new_position;

        // If we were paused, keep the newly restarted ffplay paused.
        if self.paused {
            stream_process.pause()?;
            self.playback_started_at = None;
        } else {
            self.playback_started_at = Some(
                Instant::now() - std::time::Duration::from_secs_f64(new_position)
            );
        }

        Ok(())
    }
}

impl Drop for AppUi {
    fn drop(&mut self) {
        self.stop_streaming();
    }
}
