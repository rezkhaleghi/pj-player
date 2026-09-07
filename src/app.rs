use crate::error::AppError;
use crate::offlinePlayer::{ load_audio_files, load_entries };
use std::io::{ Read, Write };
use std::path::PathBuf;
use std::process::{ Child, Command, Stdio };
use std::sync::{ atomic::{ AtomicBool, Ordering }, Arc, Mutex };
use std::thread::{ self, JoinHandle };
use std::time::Instant;

use crate::search::{ search_archive, search_youtube };
use crate::visualizer::{ Visualizer, VISUALIZATION_BAR_COUNT };

const FFMPEG_PATH: &str = "ffmpeg";
const FFPLAY_PATH: &str = "ffplay";

const AUDIO_SAMPLE_RATE: &str = "44100";
const AUDIO_CHANNELS: &str = "2";
const AUDIO_FORMAT: &str = "s16le";

#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    YouTube,
    InternetArchive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Stream,
    Download,
    OfflinePlayer,
    AboutApp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    ModeSelection,
    SearchInput,
    SearchResults,
    SourceSelection,
    FolderInput,
    OfflineFiles,
    About,
    Streaming,
    Downloading,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub identifier: String,
    pub title: String,
    pub source: Source,
}

/// Owns the complete audio playback pipeline.
///
/// direct URL
///     │
///     ▼
///   ffmpeg
///     │ raw PCM
///     ├──────────► Rust visualizer
///     │
///     ▼
///   ffplay
///     │
///     ▼
///  speakers
///
/// Seeking restarts both ffmpeg and ffplay at the requested position.
pub struct StreamProcess {
    ffmpeg: Child,
    ffplay: Child,
    direct_url: String,

    visualization_data: Arc<Mutex<Vec<u8>>>,
    visualizer_running: Arc<AtomicBool>,
    audio_thread: Option<JoinHandle<()>>,
}

impl StreamProcess {
    pub fn start(
        direct_url: String,
        position: f64,
        visualization_data: Arc<Mutex<Vec<u8>>>
    ) -> Result<Self, AppError> {
        reset_visualization(&visualization_data);

        let visualizer_running = Arc::new(AtomicBool::new(true));

        let (ffmpeg, ffplay, audio_thread) = Self::spawn_pipeline(
            &direct_url,
            position,
            Arc::clone(&visualization_data),
            Arc::clone(&visualizer_running)
        )?;

        Ok(Self {
            ffmpeg,
            ffplay,
            direct_url,
            visualization_data,
            visualizer_running,
            audio_thread: Some(audio_thread),
        })
    }

    fn spawn_pipeline(
        direct_url: &str,
        position: f64,
        visualization_data: Arc<Mutex<Vec<u8>>>,
        visualizer_running: Arc<AtomicBool>
    ) -> Result<(Child, Child, JoinHandle<()>), AppError> {
        let position = position.to_string();

        let mut ffmpeg = Command::new(FFMPEG_PATH)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-ss",
                &position,
                "-i",
                direct_url,
                "-vn",
                "-f",
                AUDIO_FORMAT,
                "-ar",
                AUDIO_SAMPLE_RATE,
                "-ac",
                AUDIO_CHANNELS,
                "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut ffplay = match
            Command::new(FFPLAY_PATH)
                .args([
                    "-nodisp",
                    "-autoexit",
                    "-loglevel",
                    "warning",
                    "-f",
                    AUDIO_FORMAT,
                    "-ar",
                    AUDIO_SAMPLE_RATE,
                    "-ch_layout",
                    "stereo",
                    "-i",
                    "pipe:0",
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
        {
            Ok(process) => process,
            Err(error) => {
                let _ = ffmpeg.kill();
                let _ = ffmpeg.wait();

                return Err(error.into());
            }
        };

        let mut ffmpeg_stdout = ffmpeg.stdout
            .take()
            .ok_or_else(|| AppError::Message("Failed to access ffmpeg stdout".to_string()))?;

        let mut ffplay_stdin = ffplay.stdin
            .take()
            .ok_or_else(|| AppError::Message("Failed to access ffplay stdin".to_string()))?;

        let audio_thread = thread::spawn(move || {
            let mut buffer = vec![0u8; 16 * 1024];
            let mut visualizer = Visualizer::new();

            while visualizer_running.load(Ordering::Relaxed) {
                let bytes_read = match ffmpeg_stdout.read(&mut buffer) {
                    Ok(0) => {
                        break;
                    }
                    Ok(bytes_read) => bytes_read,
                    Err(_) => {
                        break;
                    }
                };

                if ffplay_stdin.write_all(&buffer[..bytes_read]).is_err() {
                    break;
                }

                visualizer.process(&buffer[..bytes_read], &visualization_data);
            }

            visualizer.reset(&visualization_data);
        });

        Ok((ffmpeg, ffplay, audio_thread))
    }

    /// Checks whether ffplay has exited without blocking.
    ///
    /// ffmpeg can finish producing audio before ffplay finishes
    /// playing it, so playback lifecycle is determined by ffplay.
    pub fn try_wait(&mut self) -> Result<bool, AppError> {
        Ok(self.ffplay.try_wait()?.is_some())
    }

    pub fn stop(mut self) {
        self.visualizer_running.store(false, Ordering::Relaxed);

        let _ = self.ffmpeg.kill();
        let _ = self.ffplay.kill();

        let _ = self.ffmpeg.wait();
        let _ = self.ffplay.wait();

        if let Some(thread) = self.audio_thread.take() {
            let _ = thread.join();
        }

        reset_visualization(&self.visualization_data);
    }

    pub fn pause(&self) -> Result<(), AppError> {
        pause_process(self.ffmpeg.id())?;
        pause_process(self.ffplay.id())?;

        Ok(())
    }

    pub fn resume(&self) -> Result<(), AppError> {
        resume_process(self.ffmpeg.id())?;
        resume_process(self.ffplay.id())?;

        Ok(())
    }

    /// Restarts both ffmpeg and ffplay at the requested position.
    pub fn seek(&mut self, position: f64) -> Result<(), AppError> {
        self.visualizer_running.store(false, Ordering::Relaxed);

        let _ = self.ffmpeg.kill();
        let _ = self.ffplay.kill();

        let _ = self.ffmpeg.wait();
        let _ = self.ffplay.wait();

        if let Some(thread) = self.audio_thread.take() {
            let _ = thread.join();
        }

        reset_visualization(&self.visualization_data);

        self.visualizer_running.store(true, Ordering::Relaxed);

        let (ffmpeg, ffplay, audio_thread) = match
            Self::spawn_pipeline(
                &self.direct_url,
                position,
                Arc::clone(&self.visualization_data),
                Arc::clone(&self.visualizer_running)
            )
        {
            Ok(result) => result,
            Err(error) => {
                self.visualizer_running.store(false, Ordering::Relaxed);

                return Err(error);
            }
        };

        self.ffmpeg = ffmpeg;
        self.ffplay = ffplay;
        self.audio_thread = Some(audio_thread);

        Ok(())
    }
}

fn pause_process(pid: u32) -> Result<(), AppError> {
    let status = Command::new("kill").args(["-s", "STOP", &pid.to_string()]).status()?;

    if status.success() {
        Ok(())
    } else {
        Err(AppError::Process {
            command: format!("kill -s STOP {}", pid),
            status: status.code(),
            stderr: String::new(),
        })
    }
}

fn resume_process(pid: u32) -> Result<(), AppError> {
    let status = Command::new("kill").args(["-s", "CONT", &pid.to_string()]).status()?;

    if status.success() {
        Ok(())
    } else {
        Err(AppError::Process {
            command: format!("kill -s CONT {}", pid),
            status: status.code(),
            stderr: String::new(),
        })
    }
}

fn reset_visualization(visualization_data: &Arc<Mutex<Vec<u8>>>) {
    if let Ok(mut data) = visualization_data.lock() {
        if data.len() != VISUALIZATION_BAR_COUNT {
            *data = vec![0; VISUALIZATION_BAR_COUNT];
        } else {
            data.fill(0);
        }
    }
}

pub struct AppUi {
    pub search_input: String,
    pub folder_input: String,
    pub offline_files: Vec<PathBuf>,
    pub offline_entries: Vec<PathBuf>,
    pub offline_search_input: String,
    pub offline_searching: bool,
    pub offline_root: PathBuf,
    pub search_results: Vec<SearchResult>,
    pub selected_result_index: Option<usize>,
    pub selected_source_index: usize,
    pub selected_offline_index: Option<usize>,
    pub selected_offline_entry: Option<usize>,
    pub source: Source,
    pub current_view: View,

    pub visualization_data: Arc<Mutex<Vec<u8>>>,
    pub stream_process: Option<StreamProcess>,

    pub mode: Option<Mode>,
    pub offline_autoplay: bool,
    pub notice: Option<String>,

    pub current_equalizer: usize,
    pub download_status: Arc<Mutex<Option<String>>>,

    pub paused: bool,

    pub duration: f64,
    pub position: f64,
    pub playback_started_at: Option<Instant>,
    pub playback_base_position: f64,
}

impl AppUi {
    pub fn new() -> Self {
        AppUi {
            search_input: String::new(),
            folder_input: String::from("."),
            offline_files: Vec::new(),
            offline_entries: Vec::new(),
            offline_search_input: String::new(),
            offline_searching: false,
            offline_root: PathBuf::from("."),
            search_results: Vec::new(),
            selected_result_index: Some(0),
            selected_source_index: 0,
            selected_offline_index: None,
            selected_offline_entry: None,
            source: Source::YouTube,
            current_view: View::ModeSelection,

            visualization_data: Arc::new(Mutex::new(vec![0; VISUALIZATION_BAR_COUNT])),
            stream_process: None,

            current_equalizer: 0,
            mode: None,
            offline_autoplay: false,
            notice: None,

            download_status: Arc::new(Mutex::new(None)),

            paused: false,

            duration: 0.0,
            position: 0.0,
            playback_started_at: None,
            playback_base_position: 0.0,
        }
    }

    pub async fn search(&mut self) -> Result<(), AppError> {
        self.search_results = match self.source {
            Source::YouTube => search_youtube(&self.search_input).await?,
            Source::InternetArchive => search_archive(&self.search_input).await?,
        };

        self.current_view = View::SearchResults;

        self.selected_result_index = if self.search_results.is_empty() { None } else { Some(0) };

        Ok(())
    }

    pub fn load_offline_folder(&mut self) -> Result<(), AppError> {
        let folder = PathBuf::from(self.folder_input.trim());
        self.offline_root = folder.clone();
        self.load_offline_directory(&folder)?;
        self.current_view = View::OfflineFiles;
        self.notice = None;

        Ok(())
    }

    pub fn load_offline_directory(&mut self, folder: &PathBuf) -> Result<(), AppError> {
        self.folder_input = folder.to_string_lossy().into_owned();
        self.offline_files = load_audio_files(folder)?;
        self.offline_entries = load_entries(folder, &self.offline_search_input)?;
        self.selected_offline_entry = self.offline_entries.first().map(|_| 0);
        self.selected_offline_index = self.offline_files.first().map(|_| 0);

        Ok(())
    }

    pub fn start_playback(&mut self, duration: f64) {
        self.duration = duration;
        self.position = 0.0;
        self.playback_base_position = 0.0;
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

        self.position = self.playback_base_position + started_at.elapsed().as_secs_f64();

        if self.duration > 0.0 {
            self.position = self.position.min(self.duration);
        }
    }

    /// Detects when ffplay exits naturally.
    pub fn update_stream_lifecycle(&mut self) -> Result<(), AppError> {
        let finished = if let Some(stream_process) = &mut self.stream_process {
            stream_process.try_wait()?
        } else {
            false
        };

        if finished {
            self.stop_streaming();
            if self.mode == Some(Mode::OfflinePlayer) {
                let next_index = self.selected_offline_index.and_then(|index|
                    (index + 1 < self.offline_files.len()).then_some(index + 1)
                );
                self.selected_offline_index = next_index;
                self.selected_offline_entry = self.selected_offline_index.and_then(|index| {
                    self.offline_entries
                        .iter()
                        .position(|entry| entry == &self.offline_files[index])
                });
                self.current_view = View::OfflineFiles;
            } else {
                self.current_view = View::SearchResults;
            }
        }

        Ok(())
    }

    pub fn stop_streaming(&mut self) {
        if let Some(stream_process) = self.stream_process.take() {
            stream_process.stop();
        }

        reset_visualization(&self.visualization_data);

        self.paused = false;
        self.position = 0.0;
        self.duration = 0.0;
        self.playback_started_at = None;
        self.playback_base_position = 0.0;
    }

    pub fn toggle_pause(&mut self) -> Result<(), AppError> {
        if self.stream_process.is_none() {
            return Err(AppError::Message("No stream process running".to_string()));
        }

        if self.paused {
            self.stream_process
                .as_ref()
                .ok_or_else(|| AppError::Message("No stream process running".to_string()))?
                .resume()?;

            self.paused = false;
            self.playback_base_position = self.position;
            self.playback_started_at = Some(Instant::now());
        } else {
            self.update_playback_position();

            self.stream_process
                .as_ref()
                .ok_or_else(|| AppError::Message("No stream process running".to_string()))?
                .pause()?;

            self.paused = true;
            self.playback_started_at = None;
        }

        Ok(())
    }

    pub fn seek_forward(&mut self) -> Result<(), AppError> {
        self.seek_by(15.0)
    }

    pub fn seek_backward(&mut self) -> Result<(), AppError> {
        self.seek_by(-15.0)
    }

    fn seek_by(&mut self, amount: f64) -> Result<(), AppError> {
        self.update_playback_position();

        let mut new_position = self.position + amount;

        new_position = new_position.max(0.0);

        if self.duration > 0.0 {
            new_position = new_position.min(self.duration);
        }

        let Some(stream_process) = &mut self.stream_process else {
            return Err(AppError::Message("No stream process running".to_string()));
        };

        stream_process.seek(new_position)?;

        self.position = new_position;

        if self.paused {
            stream_process.pause()?;

            self.playback_base_position = new_position;
            self.playback_started_at = None;
        } else {
            self.playback_base_position = new_position;
            self.playback_started_at = Some(Instant::now());
        }

        Ok(())
    }
}

impl Drop for AppUi {
    fn drop(&mut self) {
        self.stop_streaming();
    }
}
