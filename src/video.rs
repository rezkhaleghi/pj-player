// use std::sync::{ mpsc::{ self, Receiver, Sender }, Arc, Mutex };
// use std::thread::{ self, JoinHandle };
// use std::time::Duration;

// use retrotermplayer::{
//     decoder::{ DecoderProfile, FfmpegDecoder, VideoFrame },
//     source::{ VideoQuality, VideoSource },
// };

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum VideoMode {
//     RetroAscii,
//     RetroColor,
//     RetroVideo,
// }

// impl VideoMode {
//     fn profile(self) -> DecoderProfile {
//         match self {
//             Self::RetroAscii | Self::RetroColor => DecoderProfile::RETRO,
//             Self::RetroVideo => DecoderProfile::VIDEO,
//         }
//     }
// }

// enum VideoCommand {
//     Pause,
//     Resume,
//     Seek(f64),
//     Stop,
// }

// pub struct VideoPlayer {
//     frame: Arc<Mutex<Option<VideoFrame>>>,
//     command_sender: Sender<VideoCommand>,
//     thread: Option<JoinHandle<()>>,
// }

// impl VideoPlayer {
//     pub fn start(video_url: String, mode: VideoMode, position: f64) -> Result<Self, String> {
//         if !position.is_finite() || position < 0.0 {
//             return Err("Video position must be finite and non-negative.".to_string());
//         }

//         let frame = Arc::new(Mutex::new(None));
//         let frame_store = Arc::clone(&frame);

//         let (command_sender, command_receiver) = mpsc::channel();

//         let thread = thread::Builder
//             ::new()
//             .name("pj-player-video".to_string())
//             .spawn(move || {
//                 run_video_thread(video_url, mode, position, frame_store, command_receiver);
//             })
//             .map_err(|error| format!("Could not start video thread: {error}"))?;

//         Ok(Self {
//             frame,
//             command_sender,
//             thread: Some(thread),
//         })
//     }

//     pub fn latest_frame(&self) -> Option<VideoFrame> {
//         self.frame.lock().ok()?.clone()
//     }

//     pub fn pause(&self) -> Result<(), String> {
//         self.command_sender
//             .send(VideoCommand::Pause)
//             .map_err(|error| format!("Could not pause video: {error}"))
//     }

//     pub fn resume(&self) -> Result<(), String> {
//         self.command_sender
//             .send(VideoCommand::Resume)
//             .map_err(|error| format!("Could not resume video: {error}"))
//     }

//     pub fn seek(&self, position: f64) -> Result<(), String> {
//         if !position.is_finite() || position < 0.0 {
//             return Err("Video position must be finite and non-negative.".to_string());
//         }

//         self.command_sender
//             .send(VideoCommand::Seek(position))
//             .map_err(|error| format!("Could not seek video: {error}"))
//     }

//     pub fn stop(mut self) {
//         let _ = self.command_sender.send(VideoCommand::Stop);

//         if let Some(thread) = self.thread.take() {
//             let _ = thread.join();
//         }

//         if let Ok(mut frame) = self.frame.lock() {
//             *frame = None;
//         }
//     }
// }

// impl Drop for VideoPlayer {
//     fn drop(&mut self) {
//         let _ = self.command_sender.send(VideoCommand::Stop);

//         if let Some(thread) = self.thread.take() {
//             let _ = thread.join();
//         }
//     }
// }

// fn run_video_thread(
//     video_url: String,
//     mode: VideoMode,
//     position: f64,
//     frame_store: Arc<Mutex<Option<VideoFrame>>>,
//     command_receiver: Receiver<VideoCommand>
// ) {
//     let profile = mode.profile();

//     let source = VideoSource::Direct(video_url);

//     let mut decoder = match
//         FfmpegDecoder::new_at_position(source, profile, VideoQuality::Low, position)
//     {
//         Ok(decoder) => decoder,
//         Err(_) => {
//             return;
//         }
//     };

//     let mut paused = false;

//     loop {
//         while let Ok(command) = command_receiver.try_recv() {
//             match command {
//                 VideoCommand::Pause => {
//                     paused = true;
//                 }

//                 VideoCommand::Resume => {
//                     paused = false;
//                 }

//                 VideoCommand::Seek(position) => {
//                     match
//                         FfmpegDecoder::new_at_position(
//                             VideoSource::Direct(match &decoder_source_placeholder() {
//                                 Some(_) => unreachable!(),
//                                 None => String::new(),
//                             }),
//                             profile,
//                             VideoQuality::Low,
//                             position
//                         )
//                     {
//                         Ok(_) => {}
//                         Err(_) => {}
//                     }
//                 }

//                 VideoCommand::Stop => {
//                     return;
//                 }
//             }
//         }

//         if paused {
//             match command_receiver.recv_timeout(Duration::from_millis(50)) {
//                 Ok(VideoCommand::Pause) => {}
//                 Ok(VideoCommand::Resume) => {
//                     paused = false;
//                 }
//                 Ok(VideoCommand::Seek(position)) => {
//                     let _ = position;
//                 }
//                 Ok(VideoCommand::Stop) => {
//                     return;
//                 }
//                 Err(mpsc::RecvTimeoutError::Timeout) => {}
//                 Err(mpsc::RecvTimeoutError::Disconnected) => {
//                     return;
//                 }
//             }

//             continue;
//         }

//         match decoder.next_frame() {
//             Ok(Some(next_frame)) => {
//                 if let Ok(mut frame) = frame_store.lock() {
//                     *frame = Some(next_frame);
//                 }
//             }

//             Ok(None) => {
//                 return;
//             }

//             Err(_) => {
//                 return;
//             }
//         }
//     }
// }

// fn decoder_source_placeholder() -> Option<()> {
//     None
// }
