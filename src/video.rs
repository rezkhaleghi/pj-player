use std::sync::{
    atomic::{ AtomicBool, AtomicU64, Ordering },
    mpsc::{ self, Receiver, Sender },
    Arc,
    Mutex,
};
use std::thread::{ self, JoinHandle };
use std::time::Duration;

use retrotermplayer::{
    decoder::{ DecoderProfile, FfmpegDecoder, VideoFrame },
    source::{ VideoQuality, VideoSource, YouTubeSource },
};

const PLAYBACK_POSITION_SCALE: f64 = 1_000_000.0;

pub struct PlaybackClock {
    position_micros: AtomicU64,
    playing: AtomicBool,
}

impl PlaybackClock {
    pub fn new() -> Self {
        Self {
            position_micros: AtomicU64::new(0),
            playing: AtomicBool::new(false),
        }
    }

    pub fn set_position(&self, position: f64) {
        let position = if position.is_finite() { position.max(0.0) } else { 0.0 };

        let micros = (position * PLAYBACK_POSITION_SCALE).round() as u64;

        self.position_micros.store(micros, Ordering::Relaxed);
    }

    pub fn position(&self) -> f64 {
        (self.position_micros.load(Ordering::Relaxed) as f64) / PLAYBACK_POSITION_SCALE
    }

    pub fn set_playing(&self, playing: bool) {
        self.playing.store(playing, Ordering::Relaxed);
    }

    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoMode {
    AsciiShading,
    MonoBlock,
    MonoVideo,
    Video,
}

impl VideoMode {
    pub fn profile(self) -> DecoderProfile {
        match self {
            Self::AsciiShading | Self::MonoBlock =>
                DecoderProfile {
                    width: 120,
                    height: 72,
                    fps: 15,
                },

            Self::MonoVideo | Self::Video =>
                DecoderProfile {
                    width: 128,
                    height: 72,
                    fps: 15,
                },
        }
    }

    pub fn quality(self) -> VideoQuality {
        match self {
            Self::AsciiShading | Self::MonoBlock => VideoQuality::Medium,

            Self::MonoVideo | Self::Video => VideoQuality::High,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::AsciiShading => "Retro Video - ASCII SHADING",
            Self::MonoBlock => "Retro Video - MONO BLOCK",
            Self::MonoVideo => "Retro Video - MONO VIDEO",
            Self::Video => "Retro Video - VIDEO",
        }
    }
}

enum VideoCommand {
    Pause,
    Resume,
    Seek {
        position: f64,
        response: Sender<Result<(), String>>,
    },
    Stop,
}

pub struct VideoPlayer {
    frame: Arc<Mutex<Option<VideoFrame>>>,
    error: Arc<Mutex<Option<String>>>,
    command_sender: Sender<VideoCommand>,
    thread: Option<JoinHandle<()>>,
}

impl VideoPlayer {
    pub fn start(
        video_url: String,
        mode: VideoMode,
        position: f64,
        playback_clock: Arc<PlaybackClock>
    ) -> Result<Self, String> {
        if !position.is_finite() || position < 0.0 {
            return Err("Video position must be finite and non-negative.".to_string());
        }

        let frame = Arc::new(Mutex::new(None));
        let frame_store = Arc::clone(&frame);

        let error = Arc::new(Mutex::new(None));
        let error_store = Arc::clone(&error);

        let (command_sender, command_receiver) = mpsc::channel();

        let thread = thread::Builder
            ::new()
            .name("pj-player-video".to_string())
            .spawn(move || {
                run_video_thread(
                    video_url,
                    mode,
                    position,
                    frame_store,
                    error_store,
                    command_receiver,
                    playback_clock
                );
            })
            .map_err(|error| format!("Could not start video thread: {error}"))?;

        Ok(Self {
            frame,
            error,
            command_sender,
            thread: Some(thread),
        })
    }

    pub fn latest_frame(&self) -> Option<VideoFrame> {
        self.frame.lock().ok()?.clone()
    }

    pub fn error(&self) -> Option<String> {
        self.error.lock().ok()?.clone()
    }

    pub fn pause(&self) -> Result<(), String> {
        self.command_sender
            .send(VideoCommand::Pause)
            .map_err(|error| format!("Could not pause video: {error}"))
    }

    pub fn resume(&self) -> Result<(), String> {
        self.command_sender
            .send(VideoCommand::Resume)
            .map_err(|error| format!("Could not resume video: {error}"))
    }

    pub fn seek(&self, position: f64) -> Result<(), String> {
        if !position.is_finite() || position < 0.0 {
            return Err("Video position must be finite and non-negative.".to_string());
        }

        let (response_sender, response_receiver) = mpsc::channel();

        self.command_sender
            .send(VideoCommand::Seek {
                position,
                response: response_sender,
            })
            .map_err(|error| format!("Could not seek video: {error}"))?;

        response_receiver.recv().map_err(|error| format!("Video seek did not complete: {error}"))?
    }

    pub fn stop(mut self) {
        let _ = self.command_sender.send(VideoCommand::Stop);

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }

        if let Ok(mut frame) = self.frame.lock() {
            *frame = None;
        }
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        let _ = self.command_sender.send(VideoCommand::Stop);

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run_video_thread(
    video_url: String,
    mode: VideoMode,
    position: f64,
    frame_store: Arc<Mutex<Option<VideoFrame>>>,
    error_store: Arc<Mutex<Option<String>>>,
    command_receiver: Receiver<VideoCommand>,
    playback_clock: Arc<PlaybackClock>
) {
    let profile = mode.profile();
    let quality = mode.quality();

    let fps = profile.fps.max(1);
    let frame_seconds = 1.0 / (fps as f64);

    let mut decoder = match create_decoder(&video_url, profile, quality, position) {
        Ok(decoder) => decoder,

        Err(error) => {
            set_error(&error_store, error);
            return;
        }
    };

    // Reusable buffer for FFmpeg decoded frame data.
    let mut frame_buffer = Vec::new();

    let mut paused = false;
    let mut video_position = position;

    loop {
        if let Some(command) = receive_latest_command(&command_receiver) {
            match command {
                VideoCommand::Pause => {
                    paused = true;
                }

                VideoCommand::Resume => {
                    paused = false;
                }

                VideoCommand::Seek { position, response } => {
                    let result = seek_decoder(
                        &video_url,
                        profile,
                        quality,
                        position,
                        &mut decoder,
                        &frame_store,
                        &error_store
                    );

                    if result.is_ok() {
                        video_position = position;
                        paused = false;
                    }

                    let _ = response.send(result);
                }

                VideoCommand::Stop => {
                    return;
                }
            }
        }

        if paused || !playback_clock.is_playing() {
            match command_receiver.recv_timeout(Duration::from_millis(20)) {
                Ok(VideoCommand::Pause) => {
                    paused = true;
                }

                Ok(VideoCommand::Resume) => {
                    paused = false;
                }

                Ok(VideoCommand::Seek { position, response }) => {
                    let result = seek_decoder(
                        &video_url,
                        profile,
                        quality,
                        position,
                        &mut decoder,
                        &frame_store,
                        &error_store
                    );

                    if result.is_ok() {
                        video_position = position;
                        paused = false;
                    }

                    let _ = response.send(result);
                }

                Ok(VideoCommand::Stop) => {
                    return;
                }

                Err(mpsc::RecvTimeoutError::Timeout) => {}

                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return;
                }
            }

            continue;
        }

        let target_position = playback_clock.position();

        /*
         * The application playback clock is the video clock.
         *
         * FFmpeg decoding can start later than audio playback, especially
         * after a seek. When that happens, discard stale decoded frames until
         * the video catches up to the position the audio is currently playing.
         * This prevents decoder startup latency from becoming permanent A/V
         * drift after every seek.
         */
        if video_position + frame_seconds * 0.5 < target_position {
            match decoder.next_frame(&mut frame_buffer) {
                Ok(Some(_)) => {
                    video_position += frame_seconds;
                    continue;
                }

                Ok(None) => {
                    return;
                }

                Err(error) => {
                    set_error(&error_store, error);
                    return;
                }
            }
        }

        /*
         * If decoding has moved slightly ahead of the shared clock, wait for
         * the clock rather than displaying a frame too early.
         */
        if video_position > target_position + frame_seconds * 0.5 {
            let ahead = video_position - target_position;

            let wait = Duration::from_secs_f64(ahead.min(0.02));

            thread::sleep(wait.max(Duration::from_millis(1)));

            continue;
        }

        match decoder.next_frame(&mut frame_buffer) {
            Ok(Some(next_frame)) => {
                clear_error(&error_store);

                if let Ok(mut frame) = frame_store.lock() {
                    *frame = Some(next_frame);
                }

                video_position += frame_seconds;
            }

            Ok(None) => {
                return;
            }

            Err(error) => {
                set_error(&error_store, error);
                return;
            }
        }
    }
}

fn receive_latest_command(receiver: &Receiver<VideoCommand>) -> Option<VideoCommand> {
    let first = receiver.try_recv().ok()?;

    /*
     * If several commands arrived while the decoder was busy, prefer the
     * newest seek position.
     *
     * This is particularly important when the user presses ← / → repeatedly.
     */
    let mut latest = first;

    while let Ok(next) = receiver.try_recv() {
        latest = merge_commands(latest, next);
    }

    Some(latest)
}

fn merge_commands(current: VideoCommand, next: VideoCommand) -> VideoCommand {
    match next {
        VideoCommand::Seek { position, response } => {
            /*
             * If the previous command was also a seek, its caller must still
             * receive a response. Since only the newest seek will actually be
             * performed, report cancellation to the older caller.
             */
            if let VideoCommand::Seek { response: old_response, .. } = current {
                let _ = old_response.send(Err("Video seek superseded.".to_string()));
            }

            VideoCommand::Seek { position, response }
        }

        VideoCommand::Stop => {
            /*
             * Stop always wins.
             */
            if let VideoCommand::Seek { response, .. } = current {
                let _ = response.send(Err("Video stopped.".to_string()));
            }

            VideoCommand::Stop
        }

        VideoCommand::Pause => {
            /*
             * Pause supersedes an older resume/pause state, but preserve a
             * pending seek because the seek still needs to complete.
             */
            match current {
                VideoCommand::Seek { .. } => current,
                _ => VideoCommand::Pause,
            }
        }

        VideoCommand::Resume =>
            match current {
                VideoCommand::Seek { .. } => current,
                _ => VideoCommand::Resume,
            }
    }
}

fn seek_decoder(
    video_url: &str,
    profile: DecoderProfile,
    quality: VideoQuality,
    position: f64,
    decoder: &mut FfmpegDecoder,
    frame_store: &Arc<Mutex<Option<VideoFrame>>>,
    error_store: &Arc<Mutex<Option<String>>>
) -> Result<(), String> {
    let mut new_decoder = create_decoder(video_url, profile, quality, position)?;

    /*
     * Decode the first frame immediately.
     *
     * This means a successful seek does not merely mean that the new FFmpeg
     * process was spawned. It means we already have a frame from the new
     * playback position ready to display.
     */
    let mut frame_buffer = Vec::new();

    let first_frame = match new_decoder.next_frame(&mut frame_buffer) {
        Ok(Some(frame)) => frame,

        Ok(None) => {
            return Err("Video ended while seeking.".to_string());
        }

        Err(error) => {
            return Err(error);
        }
    };

    *decoder = new_decoder;

    if let Ok(mut frame) = frame_store.lock() {
        *frame = Some(first_frame);
    }

    clear_error(error_store);

    Ok(())
}

fn create_decoder(
    video_url: &str,
    profile: DecoderProfile,
    quality: VideoQuality,
    position: f64
) -> Result<FfmpegDecoder, String> {
    let source = VideoSource::YouTube(YouTubeSource {
        url: video_url.to_string(),
    });

    FfmpegDecoder::new_at_position(source, profile, quality, position).map_err(|error|
        error.to_string()
    )
}

fn set_error(error_store: &Arc<Mutex<Option<String>>>, error: String) {
    if let Ok(mut stored_error) = error_store.lock() {
        *stored_error = Some(error);
    }
}

fn clear_error(error_store: &Arc<Mutex<Option<String>>>) {
    if let Ok(mut stored_error) = error_store.lock() {
        *stored_error = None;
    }
}
