use std::error::Error;
use std::process::{ Command, Stdio, Child };
use std::sync::{ Arc, Mutex };
use std::fs::File;
use std::io::Read;
use std::thread;
use std::time::Duration;
// use std::path::PathBuf;

const YT_DLP_PATH: &str = "~/pj-player/bin/yt-dlp";
const FFMPEG_PATH: &str = "~/pj-player/bin/ffplay";
// const YT_DLP_PATH: &str = "yt-dlp";
// const FFMPEG_PATH: &str = "ffplay";

pub fn stream_audio(
    video_id: &str,
    visualization_data: Arc<Mutex<Vec<u8>>>
) -> Result<Child, Box<dyn Error>> {
    let youtube_url = format!("https://www.youtube.com/watch?v={}", video_id);
    let output = Command::new(YT_DLP_PATH).args(&["-s", "--get-title", &youtube_url]).output()?;
    let song_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    println!("Streaming: {}", song_name);

    let yt_dlp = Command::new(YT_DLP_PATH)
        .args(&["-o", "-", "-f", "bestaudio", "--quiet", &youtube_url])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let ffplay_stdin = yt_dlp.stdout.unwrap();
    let visualization_data_clone = Arc::clone(&visualization_data);
    let ffplay = Command::new(FFMPEG_PATH)
        .args(&["-nodisp", "-autoexit", "-loglevel", "quiet", "-"])
        .stdin(ffplay_stdin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let ffplay_id = ffplay.id();
    thread::spawn(move || {
        let mut file = File::open("/dev/urandom").unwrap();
        while
            Command::new("ps")
                .arg("-p")
                .arg(ffplay_id.to_string())
                .output()
                .unwrap()
                .status.success()
        {
            let mut data = visualization_data_clone.lock().unwrap();
            for v in data.iter_mut() {
                let mut buf = [0u8; 1];
                file.read_exact(&mut buf).unwrap();
                *v = buf[0] % 10;
            }
            thread::sleep(Duration::from_millis(100));
        }
    });
    Ok(ffplay)
}
