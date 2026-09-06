use std::error::Error;
use std::process::{ Child, Command };
use std::sync::{ atomic::{ AtomicBool, Ordering }, Arc, Mutex };
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
    visualizer_running: Arc<AtomicBool>,
}

impl StreamProcess {
    pub fn start(
        direct_url: String,
        position: f64,
        visualizer_running: Arc<AtomicBool>
    ) -> Result<Self, Box<dyn Error>> {
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
            visualizer_running,
        })
    }

    pub fn ffplay_id(&self) -> u32 {
        self.ffplay.id()
    }

    /// Checks whether ffplay has exited without blocking.
    pub fn try_wait(&mut self) -> Result<bool, Box<dyn Error>> {
        Ok(self.ffplay.try_wait()?.is_some())
    }

    pub fn stop(mut self) {
        self.visualizer_running.store(false, Ordering::Relaxed);

        let _ = self.ffplay.kill();
        let _ = self.ffplay.wait();
    }

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

    pub duration: f64,
    pub position: f64,
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

    pub fn start_playback(&mut self, duration: f64) {
        self.duration = duration;
        self.position = 0.0;
        self.playback_started_at = Some(Instant::now());
        self.paused = false;
    }

    pub fn update_playback_position(&mut self) {
        if self.current_view != View::Streaming || self.paused {
            return;
        }

        let Some(started_at) = self.playback_started_at else {
            return;
        };

        self.position = started_at.elapsed().as_secs_f64();

        if self.duration > 0.0 {
            self.position = self.position.min(self.duration);
        }
    }

    /// Detects when ffplay exits naturally.
    ///
    /// When playback finishes, clean up the stream state and return
    /// to the search results view.
    pub fn update_stream_lifecycle(&mut self) -> Result<(), Box<dyn Error>> {
        let finished = if let Some(stream_process) = &mut self.stream_process {
            stream_process.try_wait()?
        } else {
            false
        };

        if finished {
            self.stop_streaming();
            self.current_view = View::SearchResults;
        }

        Ok(())
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
            self.update_playback_position();

            self.stream_process.as_ref().ok_or("No ffplay process running")?.pause()?;

            self.paused = true;
            self.playback_started_at = None;
        }

        Ok(())
    }

    pub fn seek_forward(&mut self) -> Result<(), Box<dyn Error>> {
        self.seek_by(15.0)
    }

    pub fn seek_backward(&mut self) -> Result<(), Box<dyn Error>> {
        self.seek_by(-15.0)
    }

    fn seek_by(&mut self, amount: f64) -> Result<(), Box<dyn Error>> {
        self.update_playback_position();

        let mut new_position = self.position + amount;
        new_position = new_position.max(0.0);

        if self.duration > 0.0 {
            new_position = new_position.min(self.duration);
        }

        let Some(stream_process) = &mut self.stream_process else {
            return Err("No ffplay process running".into());
        };

        stream_process.seek(new_position)?;

        self.position = new_position;

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
