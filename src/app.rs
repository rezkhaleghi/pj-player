use crate::error::AppError;
use crate::loading::Loading;
use crate::offlinePlayer::{load_audio_files, load_entries, load_entries_with_cancel};
use crate::search::{search_archive, search_youtube_blocking};
use crate::stream::{stream_audio, StreamInfo};
use crate::video::{VideoMode, VideoPlayer};
use crate::visualizer::{Visualizer, VISUALIZATION_BAR_COUNT};

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Instant;

const FFMPEG_PATH: &str = "ffmpeg";
const FFPLAY_PATH: &str = "ffplay";

pub fn bundled_command(name: &str) -> Command {
    if let Ok(executable) = std::env::current_exe() {
        let candidates = [
            executable.parent().map(|path| path.join("bin").join(name)),
            executable
                .parent()
                .and_then(|path| path.parent())
                .map(|path| path.join("Resources").join("bin").join(name)),
        ];

        for bundled_path in candidates.into_iter().flatten() {
            if bundled_path.is_file() {
                return Command::new(bundled_path);
            }
        }
    }

    Command::new(name)
}

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
        visualization_data: Arc<Mutex<Vec<u8>>>,
    ) -> Result<Self, AppError> {
        reset_visualization(&visualization_data);

        let visualizer_running = Arc::new(AtomicBool::new(true));

        let (ffmpeg, ffplay, audio_thread) = Self::spawn_pipeline(
            &direct_url,
            position,
            Arc::clone(&visualization_data),
            Arc::clone(&visualizer_running),
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
        visualizer_running: Arc<AtomicBool>,
    ) -> Result<(Child, Child, JoinHandle<()>), AppError> {
        let position = position.to_string();

        let mut ffmpeg = bundled_command(FFMPEG_PATH)
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

        let mut ffplay = match bundled_command(FFPLAY_PATH)
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

        let mut ffmpeg_stdout = ffmpeg
            .stdout
            .take()
            .ok_or_else(|| AppError::Message("Failed to access ffmpeg stdout".to_string()))?;

        let mut ffplay_stdin = ffplay
            .stdin
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

    pub fn direct_url(&self) -> &str {
        &self.direct_url
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

        let (ffmpeg, ffplay, audio_thread) = match Self::spawn_pipeline(
            &self.direct_url,
            position,
            Arc::clone(&self.visualization_data),
            Arc::clone(&self.visualizer_running),
        ) {
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
    let status = Command::new("kill")
        .args(["-s", "STOP", &pid.to_string()])
        .status()?;

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
    let status = Command::new("kill")
        .args(["-s", "CONT", &pid.to_string()])
        .status()?;

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
    pub offline_search_task: Option<tokio::task::JoinHandle<Result<Vec<PathBuf>, AppError>>>,
    pub offline_search_cancel: Option<Arc<AtomicBool>>,
    pub search_task: Option<tokio::task::JoinHandle<Result<Vec<SearchResult>, AppError>>>,
    pub stream_task: Option<tokio::task::JoinHandle<Result<(StreamProcess, StreamInfo), AppError>>>,
    pub loading: Option<Loading>,
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

    pub current_video_id: Option<String>,
    pub video_player: Option<VideoPlayer>,
    pub video_mode: Option<VideoMode>,
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
            offline_search_task: None,
            offline_search_cancel: None,
            search_task: None,
            stream_task: None,
            loading: None,
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

            current_video_id: None,
            video_player: None,
            video_mode: None,
        }
    }

    pub fn start_search(&mut self) {
        if let Some(task) = self.search_task.take() {
            task.abort();
        }

        let query = self.search_input.clone();
        let source = self.source.clone();

        self.loading = Some(Loading::new());

        self.search_task = Some(match source {
            Source::YouTube => tokio::task::spawn_blocking(move || search_youtube_blocking(&query)),
            Source::InternetArchive => tokio::spawn(async move { search_archive(&query).await }),
        });
    }

    pub async fn update_search(&mut self) -> Result<(), AppError> {
        let Some(task) = self.search_task.as_ref() else {
            return Ok(());
        };

        if !task.is_finished() {
            return Ok(());
        }

        let task = self.search_task.take().unwrap();

        match task
            .await
            .map_err(|error| AppError::Message(error.to_string()))?
        {
            Ok(results) => {
                self.search_results = results;

                self.selected_result_index = if self.search_results.is_empty() {
                    None
                } else {
                    Some(0)
                };

                self.current_view = View::SearchResults;
                self.loading = None;
            }

            Err(error) => {
                self.loading = None;
                self.notice = Some(error.to_string());
            }
        }

        Ok(())
    }

    pub fn start_stream(&mut self, identifier: String) {
        if let Some(task) = self.stream_task.take() {
            task.abort();
        }

        // Keep the YouTube video ID available for the optional video player.
        self.current_video_id = Some(identifier.clone());

        let visualization_data = Arc::clone(&self.visualization_data);

        self.loading = Some(Loading::new());

        self.stream_task = Some(tokio::task::spawn_blocking(move || {
            stream_audio(&identifier, visualization_data)
        }));
    }

    pub async fn update_stream_task(&mut self) -> Result<(), AppError> {
        let Some(task) = self.stream_task.as_ref() else {
            return Ok(());
        };

        if !task.is_finished() {
            return Ok(());
        }

        let task = self.stream_task.take().unwrap();

        match task
            .await
            .map_err(|error| AppError::Message(error.to_string()))?
        {
            Ok((stream_process, stream_info)) => {
                self.stream_process = Some(stream_process);
                self.start_playback(stream_info.duration);
                self.current_view = View::Streaming;
                self.loading = None;
            }

            Err(error) => {
                self.current_video_id = None;
                self.loading = None;
                self.notice = Some(error.to_string());
                self.current_view = View::SearchResults;
            }
        }

        Ok(())
    }

    pub fn start_video(&mut self, mode: VideoMode) -> Result<(), AppError> {
        let Some(video_id) = self.current_video_id.clone() else {
            return Err(AppError::Message(
                "No YouTube video is currently playing.".to_string(),
            ));
        };

        let video_url = format!("https://www.youtube.com/watch?v={video_id}");

        self.stop_video();

        let video_player =
            VideoPlayer::start(video_url, mode, self.position).map_err(AppError::Message)?;

        self.video_player = Some(video_player);
        self.video_mode = Some(mode);

        Ok(())
    }

    pub fn stop_video(&mut self) {
        if let Some(video_player) = self.video_player.take() {
            video_player.stop();
        }

        self.video_mode = None;
    }

    pub fn toggle_video(&mut self, mode: VideoMode) -> Result<(), AppError> {
        if self.video_mode == Some(mode) && self.video_player.is_some() {
            self.stop_video();
            return Ok(());
        }

        self.start_video(mode)
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
        if let Some(task) = self.offline_search_task.take() {
            task.abort();
        }

        if let Some(cancel) = self.offline_search_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }

        self.folder_input = folder.to_string_lossy().into_owned();

        self.offline_entries = load_entries(folder, &self.offline_search_input)?;

        self.offline_files = if self.offline_search_input.is_empty() {
            load_audio_files(folder)?
        } else {
            self.offline_entries.clone()
        };

        self.selected_offline_entry = self.offline_entries.first().map(|_| 0);
        self.selected_offline_index = self.offline_files.first().map(|_| 0);

        Ok(())
    }

    pub fn start_offline_search(&mut self) {
        if let Some(task) = self.offline_search_task.take() {
            task.abort();
        }

        if let Some(cancel) = self.offline_search_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }

        if self.offline_search_input.is_empty() {
            let folder = PathBuf::from(&self.folder_input);
            let _ = self.load_offline_directory(&folder);
            return;
        }

        let folder = PathBuf::from(&self.folder_input);
        let query = self.offline_search_input.clone();

        let cancel = Arc::new(AtomicBool::new(false));
        let task_cancel = Arc::clone(&cancel);

        self.offline_search_cancel = Some(cancel);

        self.offline_search_task = Some(tokio::task::spawn_blocking(move || {
            load_entries_with_cancel(&folder, &query, Some(&task_cancel))
        }));
    }

    pub fn cancel_offline_search(&mut self) {
        if let Some(cancel) = self.offline_search_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }

        if let Some(task) = self.offline_search_task.take() {
            task.abort();
        }
    }

    pub async fn update_offline_search(&mut self) -> Result<(), AppError> {
        let Some(task) = self.offline_search_task.as_ref() else {
            return Ok(());
        };

        if !task.is_finished() {
            return Ok(());
        }

        let task = self.offline_search_task.take().unwrap();

        self.offline_search_cancel = None;

        self.offline_entries = task
            .await
            .map_err(|error| AppError::Message(error.to_string()))??;

        self.offline_files = self.offline_entries.clone();

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
                let next_index = self
                    .selected_offline_index
                    .and_then(|index| (index + 1 < self.offline_files.len()).then_some(index + 1));

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
        self.stop_video();

        if let Some(stream_process) = self.stream_process.take() {
            stream_process.stop();
        }

        reset_visualization(&self.visualization_data);

        self.paused = false;
        self.position = 0.0;
        self.duration = 0.0;
        self.current_video_id = None;
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

            self.video_player
                .as_ref()
                .map(|video_player| video_player.resume());

            self.paused = false;
            self.playback_base_position = self.position;
            self.playback_started_at = Some(Instant::now());
        } else {
            self.update_playback_position();

            self.stream_process
                .as_ref()
                .ok_or_else(|| AppError::Message("No stream process running".to_string()))?
                .pause()?;

            self.video_player
                .as_ref()
                .map(|video_player| video_player.pause());

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
            self.notice = Some("No stream process running.".to_string());
            return Ok(());
        };

        // Audio seek must succeed before changing our playback clock.
        if let Err(error) = stream_process.seek(new_position) {
            self.notice = Some(error.to_string());
            return Ok(());
        }

        // Video is optional. A video seek failure must never kill the player.
        if let Some(video_player) = &self.video_player {
            if let Err(error) = video_player.seek(new_position) {
                self.notice = Some(error);
            }
        }

        self.position = new_position;

        if self.paused {
            if let Err(error) = stream_process.pause() {
                self.notice = Some(error.to_string());
                return Ok(());
            }

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
