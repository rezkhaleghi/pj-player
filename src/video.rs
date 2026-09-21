use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use retrotermplayer::{
    decoder::{DecoderProfile, FfmpegDecoder, VideoFrame},
    source::{VideoQuality, VideoSource, YouTubeSource},
};

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
            Self::AsciiShading | Self::MonoBlock => DecoderProfile::RETRO,
            Self::MonoVideo | Self::Video => DecoderProfile::VIDEO,
        }
    }

    pub fn quality(self) -> VideoQuality {
        match self {
            Self::AsciiShading | Self::MonoBlock | Self::MonoVideo => VideoQuality::Low,
            Self::Video => VideoQuality::Medium,
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
    Seek(f64),
    Stop,
}

pub struct VideoPlayer {
    frame: Arc<Mutex<Option<VideoFrame>>>,
    error: Arc<Mutex<Option<String>>>,
    command_sender: Sender<VideoCommand>,
    thread: Option<JoinHandle<()>>,
}

impl VideoPlayer {
    pub fn start(video_url: String, mode: VideoMode, position: f64) -> Result<Self, String> {
        if !position.is_finite() || position < 0.0 {
            return Err("Video position must be finite and non-negative.".to_string());
        }

        let frame = Arc::new(Mutex::new(None));
        let frame_store = Arc::clone(&frame);

        let error = Arc::new(Mutex::new(None));
        let error_store = Arc::clone(&error);

        let (command_sender, command_receiver) = mpsc::channel();

        let thread = thread::Builder::new()
            .name("pj-player-video".to_string())
            .spawn(move || {
                run_video_thread(
                    video_url,
                    mode,
                    position,
                    frame_store,
                    error_store,
                    command_receiver,
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

        self.command_sender
            .send(VideoCommand::Seek(position))
            .map_err(|error| format!("Could not seek video: {error}"))
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
) {
    let profile = mode.profile();
    let quality = mode.quality();

    let fps = profile.fps.max(1);
    let frame_duration = Duration::from_secs_f64(1.0 / (fps as f64));

    let mut decoder = match create_decoder(&video_url, profile, quality, position) {
        Ok(decoder) => decoder,
        Err(error) => {
            set_error(&error_store, error);
            return;
        }
    };

    let mut paused = false;
    let mut next_frame_deadline = Instant::now();

    loop {
        while let Ok(command) = command_receiver.try_recv() {
            match command {
                VideoCommand::Pause => {
                    paused = true;
                }

                VideoCommand::Resume => {
                    paused = false;
                    next_frame_deadline = Instant::now();
                }

                VideoCommand::Seek(position) => {
                    match create_decoder(&video_url, profile, quality, position) {
                        Ok(new_decoder) => {
                            decoder = new_decoder;

                            if let Ok(mut frame) = frame_store.lock() {
                                *frame = None;
                            }

                            clear_error(&error_store);

                            next_frame_deadline = Instant::now();
                        }

                        Err(error) => {
                            set_error(&error_store, error);
                        }
                    }
                }

                VideoCommand::Stop => {
                    return;
                }
            }
        }

        if paused {
            match command_receiver.recv_timeout(Duration::from_millis(50)) {
                Ok(VideoCommand::Pause) => {}

                Ok(VideoCommand::Resume) => {
                    paused = false;
                    next_frame_deadline = Instant::now();
                }

                Ok(VideoCommand::Seek(position)) => {
                    match create_decoder(&video_url, profile, quality, position) {
                        Ok(new_decoder) => {
                            decoder = new_decoder;

                            if let Ok(mut frame) = frame_store.lock() {
                                *frame = None;
                            }

                            clear_error(&error_store);

                            next_frame_deadline = Instant::now();
                        }

                        Err(error) => {
                            set_error(&error_store, error);
                        }
                    }
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

        /*
         * Do not decode faster than the actual playback rate.
         *
         * FFmpeg generates 15 frames/sec, but stdout can still be consumed
         * much faster than real time. Without this pacing, the decoder reaches
         * EOF after only a few seconds of wall-clock time.
         */
        let now = Instant::now();

        if now < next_frame_deadline {
            let wait = next_frame_deadline - now;

            match command_receiver.recv_timeout(wait) {
                Ok(VideoCommand::Pause) => {
                    paused = true;
                }

                Ok(VideoCommand::Resume) => {
                    paused = false;
                    next_frame_deadline = Instant::now();
                }

                Ok(VideoCommand::Seek(position)) => {
                    match create_decoder(&video_url, profile, quality, position) {
                        Ok(new_decoder) => {
                            decoder = new_decoder;

                            if let Ok(mut frame) = frame_store.lock() {
                                *frame = None;
                            }

                            clear_error(&error_store);

                            next_frame_deadline = Instant::now();
                        }

                        Err(error) => {
                            set_error(&error_store, error);
                        }
                    }
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

        match decoder.next_frame() {
            Ok(Some(next_frame)) => {
                clear_error(&error_store);

                if let Ok(mut frame) = frame_store.lock() {
                    *frame = Some(next_frame);
                }

                next_frame_deadline += frame_duration;

                /*
                 * If rendering/decoding fell significantly behind, don't try
                 * to replay old deadlines forever.
                 */
                let now = Instant::now();

                if next_frame_deadline < now {
                    next_frame_deadline = now;
                }
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

fn create_decoder(
    video_url: &str,
    profile: DecoderProfile,
    quality: VideoQuality,
    position: f64,
) -> Result<FfmpegDecoder, String> {
    let source = VideoSource::YouTube(YouTubeSource {
        url: video_url.to_string(),
    });

    FfmpegDecoder::new_at_position(source, profile, quality, position)
        .map_err(|error| error.to_string())
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
