use std::error::Error;
use std::io::{ Read, Write };
use std::process::{ Child, Command, Stdio };
use std::sync::{ atomic::{ AtomicBool, Ordering }, Arc, Mutex };
use std::thread::{ self, JoinHandle };
use std::time::Instant;

use crate::search::{ search_archive, search_youtube };

const FFMPEG_PATH: &str = "ffmpeg";
const FFPLAY_PATH: &str = "ffplay";

const AUDIO_SAMPLE_RATE: &str = "44100";
const AUDIO_CHANNELS: &str = "2";
const AUDIO_FORMAT: &str = "s16le";

const VISUALIZATION_BAR_COUNT: usize = 10;

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

/// Owns the complete audio playback pipeline.
///
/// direct URL
///     │
///     ▼
///   ffmpeg
///     │ raw PCM
///     ├──────────► Rust audio analyzer
///     │
///     ▼
///   ffplay
///     │
///     ▼
///  speakers
///
/// Seeking restarts the complete pipeline at the requested position.
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
    ) -> Result<Self, Box<dyn Error>> {
        let visualizer_running = Arc::new(AtomicBool::new(true));

        reset_visualization(&visualization_data);

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
    ) -> Result<(Child, Child, JoinHandle<()>), Box<dyn Error>> {
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

        let mut ffmpeg_stdout = ffmpeg.stdout.take().ok_or("Failed to access ffmpeg stdout")?;

        let mut ffplay_stdin = ffplay.stdin.take().ok_or("Failed to access ffplay stdin")?;

        let audio_thread = thread::spawn(move || {
            let mut buffer = vec![0u8; 16 * 1024];

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

                update_visualization(&buffer[..bytes_read], &visualization_data);
            }

            reset_visualization(&visualization_data);
        });

        Ok((ffmpeg, ffplay, audio_thread))
    }

    /// Checks whether ffplay has exited without blocking.
    ///
    /// ffmpeg can finish producing audio before ffplay finishes
    /// playing it, so playback lifecycle is determined by ffplay.
    pub fn try_wait(&mut self) -> Result<bool, Box<dyn Error>> {
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

    pub fn pause(&self) -> Result<(), Box<dyn Error>> {
        pause_process(self.ffmpeg.id())?;
        pause_process(self.ffplay.id())?;

        Ok(())
    }

    pub fn resume(&self) -> Result<(), Box<dyn Error>> {
        resume_process(self.ffmpeg.id())?;
        resume_process(self.ffplay.id())?;

        Ok(())
    }

    /// Restarts both ffmpeg and ffplay at the requested position.
    pub fn seek(&mut self, position: f64) -> Result<(), Box<dyn Error>> {
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

fn pause_process(pid: u32) -> Result<(), Box<dyn Error>> {
    let status = Command::new("kill").args(["-s", "STOP", &pid.to_string()]).status()?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Failed to pause process {}", pid).into())
    }
}

fn resume_process(pid: u32) -> Result<(), Box<dyn Error>> {
    let status = Command::new("kill").args(["-s", "CONT", &pid.to_string()]).status()?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Failed to resume process {}", pid).into())
    }
}

/// Converts raw PCM samples into visualization levels.
///
/// The PCM stream is divided into ten small windows.
/// Each window gets an RMS amplitude value and is converted
/// into a 0-10 level for the terminal equalizer.
fn update_visualization(audio_data: &[u8], visualization_data: &Arc<Mutex<Vec<u8>>>) {
    // Stereo s16le audio uses 2 bytes per sample
    // and 2 samples (left + right) per audio frame.
    const BYTES_PER_SAMPLE: usize = 2;
    const CHANNELS: usize = 2;
    const BYTES_PER_FRAME: usize = BYTES_PER_SAMPLE * CHANNELS;

    if audio_data.len() < BYTES_PER_FRAME {
        return;
    }

    let frame_count = audio_data.len() / BYTES_PER_FRAME;

    if frame_count == 0 {
        return;
    }

    let frames_per_bar = frame_count.div_ceil(VISUALIZATION_BAR_COUNT);

    let mut levels = vec![0u8; VISUALIZATION_BAR_COUNT];

    for (bar, level) in levels.iter_mut().enumerate() {
        let start_frame = bar * frames_per_bar;
        let end_frame = ((bar + 1) * frames_per_bar).min(frame_count);

        if start_frame >= end_frame {
            continue;
        }

        let start_byte = start_frame * BYTES_PER_FRAME;
        let end_byte = end_frame * BYTES_PER_FRAME;

        let mut sum = 0.0f64;
        let mut count = 0usize;

        for frame in audio_data[start_byte..end_byte].chunks_exact(BYTES_PER_FRAME) {
            let left = i16::from_le_bytes([frame[0], frame[1]]);
            let right = i16::from_le_bytes([frame[2], frame[3]]);

            let left = (left as f64) / (i16::MAX as f64);
            let right = (right as f64) / (i16::MAX as f64);

            // Average the two stereo channels into one amplitude value.
            let sample = (left + right) / 2.0;

            sum += sample * sample;
            count += 1;
        }

        if count == 0 {
            continue;
        }

        let rms = (sum / (count as f64)).sqrt();

        // Boost quieter audio while keeping the result in 0..10.
        let value = (rms * 20.0).round().clamp(0.0, 10.0);

        *level = value as u8;
    }

    if let Ok(mut data) = visualization_data.lock() {
        *data = levels;
    }
}
fn reset_visualization(visualization_data: &Arc<Mutex<Vec<u8>>>) {
    if let Ok(mut data) = visualization_data.lock() {
        data.fill(0);
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
    pub playback_base_position: f64,
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
            playback_base_position: 0.0,
        }
    }

    pub async fn search(&mut self) -> Result<(), Box<dyn Error>> {
        self.search_results = match self.source {
            Source::YouTube => { search_youtube(&self.search_input).await? }
            Source::InternetArchive => { search_archive(&self.search_input).await? }
        };

        self.current_view = View::SearchResults;

        self.selected_result_index = if self.search_results.is_empty() { None } else { Some(0) };

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

        reset_visualization(&self.visualization_data);

        self.paused = false;
        self.position = 0.0;
        self.duration = 0.0;
        self.playback_started_at = None;
        self.playback_base_position = 0.0;
    }

    pub fn toggle_pause(&mut self) -> Result<(), Box<dyn Error>> {
        if self.stream_process.is_none() {
            return Err("No stream process running".into());
        }

        if self.paused {
            self.stream_process.as_ref().ok_or("No stream process running")?.resume()?;

            self.paused = false;
            self.playback_base_position = self.position;
            self.playback_started_at = Some(Instant::now());
        } else {
            self.update_playback_position();

            self.stream_process.as_ref().ok_or("No stream process running")?.pause()?;

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
            return Err("No stream process running".into());
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
