// Hello Friend :)

// const YT_DLP_PATH: &str = "./bin/yt-dlp";
// const WGET_PATH: &str = "./bin/wget";
// const FFMPEG_PATH: &str = "./bin/ffmpeg";
const YT_DLP_PATH: &str = "yt-dlp";
const WGET_PATH: &str = "wget";
const FFMPEG_PATH: &str = "ffplay";

use std::time::{ Duration, Instant };
use std::error::Error;
use std::io;
use std::process::{ Command, Stdio, Child };
use std::fs;
use std::thread;
use std::sync::{ Arc, Mutex };
use std::fs::File;
use std::io::Read;

use ratatui::{ prelude::*, widgets::*, layout::{ Layout, Direction, Constraint } };
use crossterm::{
    event::{ self, Event, KeyCode, KeyModifiers },
    terminal::{ disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen },
    execute,
};

use reqwest::Client;
use serde_json::Value;
#[derive(Debug, Clone, PartialEq)]
enum Source {
    YouTube,
    InternetArchive,
}

#[derive(PartialEq)]
enum Mode {
    Stream,
    Download,
}

// represent the current view of the application
#[derive(PartialEq)]
enum View {
    InitialSelection,
    SearchInput,
    SourceSelection,
    SearchResults,
    Streaming,
    Downloading,
}

// represent a search result
#[derive(Debug, Clone)]
struct SearchResult {
    identifier: String,
    title: String,
    source: Source,
}

// represent the UI state of the application
struct AppUi {
    search_input: String,
    search_results: Vec<SearchResult>,
    selected_result_index: Option<usize>,
    selected_source_index: usize,
    source: Source,
    current_view: View,
    visualization_data: Arc<Mutex<Vec<u8>>>,
    ffplay_process: Option<Child>,
    mode: Option<Mode>,
    download_status: Arc<Mutex<Option<String>>>,
}

impl AppUi {
    fn new() -> Self {
        AppUi {
            search_input: String::new(),
            search_results: Vec::new(),
            selected_result_index: Some(0),
            selected_source_index: 0,
            source: Source::YouTube,
            current_view: View::InitialSelection,
            visualization_data: Arc::new(Mutex::new(vec![0; 10])),
            ffplay_process: None,
            mode: None,
            download_status: Arc::new(Mutex::new(None)),
        }
    }

    // Method to perform a search based on the current source
    async fn search(&mut self) -> Result<(), Box<dyn Error>> {
        self.search_results = match self.source {
            Source::YouTube => search_youtube(&self.search_input).await?,
            Source::InternetArchive => search_archive(&self.search_input).await?,
        };
        self.current_view = View::SearchResults;
        self.selected_result_index = Some(0);
        Ok(())
    }

    fn stop_streaming(&mut self) {
        if let Some(mut process) = self.ffplay_process.take() {
            let _ = process.kill();
        }
    }
}

// search on YouTube using yt-dlp
async fn search_youtube(query: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    // Construct the yt-dlp command
    let output = Command::new(YT_DLP_PATH)
        .arg("--default-search")
        .arg("ytsearch") // Specify YouTube search
        .arg(format!("ytsearch15:{}", query)) // Fetch top 15 results
        .arg("--dump-json") // Output results in JSON
        .arg("--flat-playlist") // Skip detailed metadata
        .arg("--skip-download") // Skip downloading anything
        .arg("--ignore-errors") // Avoid delays from errors
        .output()?; // Execute the command

    // Ensure the command was successful
    if !output.status.success() {
        return Err(
            format!(
                "yt-dlp failed with status: {:?}, stderr: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ).into()
        );
    }

    // Parse the JSON output
    let stdout = String::from_utf8(output.stdout)?;
    let results: Vec<SearchResult> = stdout
        .lines() // Parse each JSON line as a result
        .filter_map(|line| {
            // Parse the line as JSON
            let json: Value = serde_json::from_str(line).ok()?;

            // Extract relevant fields
            Some(SearchResult {
                identifier: json.get("id")?.as_str()?.to_string(),
                title: json.get("title")?.as_str()?.to_string(),
                source: Source::YouTube,
            })
        })
        .collect();

    Ok(results)
}

// search audio on archive.org
async fn search_archive(query: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let url = format!(
        "https://archive.org/advancedsearch.php?q={}&output=json&rows=15",
        query.replace(" ", "+")
    );

    // Send the request and parse the response
    let client = Client::new();
    let response = client.get(&url).send().await?;
    let json: Value = response.json().await?;

    // Extract search results from the response
    let mut results = Vec::new();
    if let Some(items) = json["response"]["docs"].as_array() {
        for item in items {
            if
                let (Some(identifier), Some(title)) = (
                    item["identifier"].as_str(),
                    item["title"].as_str(),
                )
            {
                results.push(SearchResult {
                    identifier: identifier.to_string(),
                    title: title.to_string(),
                    source: Source::InternetArchive,
                });
            }
        }
    }

    Ok(results)
}

// stream audio using yt-dlp and ffplay

fn stream_audio(
    video_id: &str,
    visualization_data: Arc<Mutex<Vec<u8>>>
) -> Result<Child, Box<dyn Error>> {
    let youtube_url = format!("https://www.youtube.com/watch?v={}", video_id);
    // Get the title of the video
    let output = Command::new(YT_DLP_PATH).args(&["-s", "--get-title", &youtube_url]).output()?;
    let song_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    println!("Streaming: {}", song_name);
    // Start yt-dlp to stream audio
    let yt_dlp = Command::new(YT_DLP_PATH)
        .args(&["-o", "-", "-f", "bestaudio", "--quiet", &youtube_url])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    // Start ffplay to play the audio
    let ffplay_stdin = yt_dlp.stdout.unwrap();
    let visualization_data_clone = Arc::clone(&visualization_data);
    let ffplay = Command::new(FFMPEG_PATH)
        .args(&["-nodisp", "-autoexit", "-loglevel", "quiet", "-"])
        .stdin(ffplay_stdin)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    // Spawn a thread to update visualization data
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
// download YouTube audio using yt-dlp
fn download_youtube_audio(
    video_id: String,
    title: String,
    download_status: Arc<Mutex<Option<String>>>
) {
    // Update download status
    let status_message = format!("{} is downloading", title);
    {
        let mut status = download_status.lock().unwrap();
        *status = Some(status_message);
    }

    // Spawn a thread to perform the download
    thread::spawn(move || {
        let download_path = "./";
        fs::create_dir_all(download_path).unwrap();

        // Sanitize the title for the file name
        let sanitized_title = title.replace("/", "_").replace("\\", "_");
        let output_path = format!("{}/{} (PJ-PLAYER).mp3", download_path, sanitized_title);

        // Execute yt-dlp to download the audio
        let status = Command::new(YT_DLP_PATH)
            .args(
                &[
                    "--extract-audio",
                    "--audio-format",
                    "mp3",
                    "-o",
                    &output_path,
                    &format!("https://www.youtube.com/watch?v={}", video_id),
                ]
            )
            .stdout(Stdio::null()) // Suppress standard output
            .stderr(Stdio::null()) // Suppress standard error
            .status();

        // Update download status based on the result
        let mut status_message = download_status.lock().unwrap();
        *status_message = match status {
            Ok(status) if status.success() => Some(format!("{} downloaded successfully", title)),
            Ok(status) => Some(format!("yt-dlp returned an error: Exit code {}", status)),
            Err(err) => Some(format!("Error executing yt-dlp: {}", err)),
        };
    });
}

// download audio using wget from archive.org
fn download_archive_audio(
    identifier: String,
    title: String,
    download_status: Arc<Mutex<Option<String>>>
) {
    // Update download status
    let status_message = format!("{} is downloading", title);
    {
        let mut status = download_status.lock().unwrap();
        *status = Some(status_message);
    }

    // Spawn a thread to perform the download
    thread::spawn(move || {
        let download_path = "./";
        fs::create_dir_all(download_path).unwrap();

        // Sanitize the title for the file name
        let sanitized_title = title.replace("/", "_").replace("\\", "_");
        let output_path = format!("{}/{} (PJ-PLAYER).mp3", download_path, sanitized_title);
        let download_url = format!("https://archive.org/download/{}/{}", identifier, identifier);

        // Execute wget to download the audio
        let status = Command::new(WGET_PATH)
            .args(&["-O", &output_path, &download_url])
            .stdout(Stdio::null()) // Suppress standard output
            .stderr(Stdio::null()) // Suppress standard error
            .status();

        // Update download status based on the result
        let mut status_message = download_status.lock().unwrap();
        *status_message = match status {
            Ok(status) if status.success() => Some(format!("{} downloaded successfully", title)),
            Ok(status) => Some(format!("wget returned an error: Exit code {}", status)),
            Err(err) => Some(format!("Error executing wget: {}", err)),
        };
    });
}

//  render ui
fn render(app: &AppUi, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Header
            Constraint::Length(1), // Second header
            Constraint::Min(10), // Main content
        ])
        .split(frame.area());

    let light_green_style = Style::default().fg(Color::LightGreen);
    let white_style = Style::default().fg(Color::White);

    let header_paragraph = Paragraph::new(
        r#"
▗▄▄▖ ▗▖    ▗▄▄▖ ▗▖    ▗▄▖▗▖  ▗▖▗▄▄▄▖▗▄▄▖ 
▐▌ ▐▌▐▌    ▐▌ ▐▌▐▌   ▐▌ ▐▌▝▚▞▘ ▐▌   ▐▌ ▐▌
▐▛▀▘ ▐▌    ▐▛▀▘ ▐▌   ▐▛▀▜▌ ▐▌  ▐▛▀▀▘▐▛▀▚▖
▐▌▗▄▄▞▘    ▐▌   ▐▙▄▄▖▐▌ ▐▌ ▐▌  ▐▙▄▄▖▐▌ ▐▌
"#
    )
        .style(light_green_style)
        .alignment(Alignment::Center); // Center the text
    frame.render_widget(header_paragraph, chunks[0]);

    let second_header_paragraph = Paragraph::new("Made with 🌿 by Pocket Jack")
        .style(white_style)
        .alignment(Alignment::Center); // Center the text
    frame.render_widget(second_header_paragraph, chunks[1]);

    match app.current_view {
        View::InitialSelection => {
            let buttons = vec!["1. STREAM", "2. DOWNLOAD"];
            let items: Vec<ListItem> = buttons
                .iter()
                .enumerate()
                .map(|(i, &button)| {
                    let style = if Some(i) == app.selected_result_index {
                        Style::default().bg(Color::Blue).fg(Color::White)
                    } else {
                        white_style
                    };
                    ListItem::new(button).style(style)
                })
                .collect();

            let list = List::new(items).block(
                Block::default().borders(Borders::ALL).title("Select Mode").style(light_green_style)
            );

            frame.render_widget(list, chunks[2]);
        }
        View::SearchInput => {
            let input_block = Block::default()
                .borders(Borders::ALL)
                .title("Search Music")
                .style(light_green_style);

            let input_text = format!("(Search Query): {}", app.search_input);

            let input = Paragraph::new(input_text).style(white_style).block(input_block);

            // Adjust constraints to make the search input section smaller
            let search_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Smaller height for search input
                    Constraint::Min(10),
                ])
                .split(chunks[2]);

            frame.render_widget(input, search_chunks[0]);
        }
        View::SourceSelection => {
            let sources = vec!["1. YouTube", "2. Internet Archive"];
            let items: Vec<ListItem> = sources
                .iter()
                .enumerate()
                .map(|(i, &source)| {
                    let style = if i == app.selected_source_index {
                        Style::default().bg(Color::Blue).fg(Color::White)
                    } else {
                        white_style
                    };
                    ListItem::new(source).style(style)
                })
                .collect();

            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select Source")
                    .style(light_green_style)
            );

            frame.render_widget(list, chunks[2]);
        }
        View::SearchResults => {
            // Clear the download status message when switching to SearchResults view
            {
                let mut download_status = app.download_status.lock().unwrap();
                *download_status = None;
            }

            if app.search_results.is_empty() {
                let no_results_item = ListItem::new("NO MUSIC FOUND =(").style(white_style).bold();
                let no_results_list = List::new(vec![no_results_item]).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Search Results")
                        .style(light_green_style)
                );
                frame.render_widget(no_results_list, chunks[2]);
            } else {
                let results: Vec<ListItem> = app.search_results
                    .iter()
                    .enumerate()
                    .map(|(i, result)| {
                        let style = if Some(i) == app.selected_result_index {
                            Style::default().bg(Color::Blue).fg(Color::White)
                        } else {
                            white_style
                        };

                        let content = Line::from(
                            vec![
                                Span::raw(format!("{}: ", i + 1)),
                                Span::raw(&result.title),
                                Span::raw(format!(" ({:?})", result.source))
                            ]
                        );
                        ListItem::new(content).style(style)
                    })
                    .collect();

                let list = List::new(results).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Search Results")
                        .style(light_green_style)
                );

                frame.render_widget(list, chunks[2]);
            }
        }
        View::Streaming => {
            let streaming_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(14), // Adjust the percentage to make the blocks closer
                    Constraint::Percentage(50), //
                ])
                .split(chunks[2]);

            let song_block = Block::default()
                .borders(Borders::ALL)
                .title("Now Streaming")
                .style(light_green_style);

            // Assuming `app.search_results` contains the current streaming song
            let song_name = if let Some(index) = app.selected_result_index {
                &app.search_results[index].title
            } else {
                "Unknown Song"
            };

            let song_info = Paragraph::new(song_name)
                .style(white_style)
                .block(song_block)
                .alignment(Alignment::Center); // Center the text horizontally

            frame.render_widget(song_info, streaming_chunks[0]);

            let eq_area = streaming_chunks[1];
            let eq_data = app.visualization_data.lock().unwrap();

            let max_height = (eq_area.height as usize).min(10); // Shorter height
            let bar_width = (eq_area.width as usize) / eq_data.len(); // Adjust bar width calculation
            let visual_block = Block::default()
                .borders(Borders::ALL)
                .title("Visual")
                .style(Style::default().fg(Color::LightGreen));

            frame.render_widget(visual_block.clone(), eq_area);

            let inner_area = visual_block.inner(eq_area);

            for (i, &value) in eq_data.iter().enumerate() {
                let bar_height = (((value as f64) / 10.0) * (max_height as f64)).round() as usize;
                let x = inner_area.x + ((i * bar_width) as u16);
                let y = inner_area.y + inner_area.height - (bar_height as u16);

                for j in 0..bar_height {
                    let y_pos = y + (j as u16);
                    let char = if j % 2 == 0 { '|' } else { ' ' };
                    let bar = Paragraph::new(char.to_string())
                        .style(Style::default().fg(Color::Green))
                        .alignment(Alignment::Center);
                    frame.render_widget(bar, Rect::new(x, y_pos, bar_width as u16, 1));
                }
            }
        }
        View::Downloading => {
            let download_status = app.download_status.lock().unwrap();
            let status_message = download_status.as_deref().unwrap_or("No downloads in progress");

            let download_block = Block::default()
                .borders(Borders::ALL)
                .title("Downloads")
                .style(light_green_style);

            let download_paragraph = Paragraph::new(status_message)
                .style(white_style)
                .block(download_block)
                .alignment(Alignment::Center);

            frame.render_widget(download_paragraph, chunks[2]);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppUi::new();
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| render(&app, frame))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match app.current_view {
                    View::InitialSelection => {
                        match key.code {
                            KeyCode::Up => {
                                app.selected_result_index = Some(
                                    app.selected_result_index.unwrap_or(0).saturating_sub(1)
                                );
                            }
                            KeyCode::Down => {
                                app.selected_result_index = Some(
                                    (app.selected_result_index.unwrap_or(0) + 1).min(1)
                                );
                            }
                            KeyCode::Enter => {
                                match app.selected_result_index {
                                    Some(0) => {
                                        app.mode = Some(Mode::Stream);
                                        app.source = Source::YouTube; // Set source to YouTube for streaming
                                        app.current_view = View::SearchInput;
                                    }
                                    Some(1) => {
                                        app.mode = Some(Mode::Download);
                                        app.current_view = View::SearchInput;
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                    View::SearchInput => {
                        match key.code {
                            KeyCode::Enter => {
                                if app.mode == Some(Mode::Stream) {
                                    app.search().await?;
                                } else {
                                    app.current_view = View::SourceSelection;
                                }
                            }
                            KeyCode::Char(c) => {
                                app.search_input.push(c);
                            }
                            KeyCode::Backspace => {
                                app.search_input.pop();
                            }
                            KeyCode::Left => {
                                app.current_view = View::InitialSelection;
                            }
                            KeyCode::Right => {
                                if app.mode == Some(Mode::Stream) {
                                    app.search().await?;
                                } else {
                                    app.current_view = View::SourceSelection;
                                }
                            }
                            _ => {}
                        }
                    }
                    View::SourceSelection => {
                        match key.code {
                            KeyCode::Up => {
                                app.selected_source_index =
                                    app.selected_source_index.saturating_sub(1);
                            }
                            KeyCode::Down => {
                                app.selected_source_index = (app.selected_source_index + 1).min(1);
                            }
                            KeyCode::Enter => {
                                app.source = match app.selected_source_index {
                                    0 => Source::YouTube,
                                    _ => Source::InternetArchive,
                                };
                                app.search().await?;
                            }
                            KeyCode::Left => {
                                app.current_view = View::SearchInput;
                            }
                            KeyCode::Right => {
                                app.source = match app.selected_source_index {
                                    0 => Source::YouTube,
                                    _ => Source::InternetArchive,
                                };
                                app.search().await?;
                            }
                            _ => {}
                        }
                    }
                    View::SearchResults => {
                        match key.code {
                            KeyCode::Up => {
                                if let Some(idx) = &mut app.selected_result_index {
                                    if *idx == 0 {
                                        app.current_view = View::SearchInput;
                                    } else {
                                        *idx = idx.saturating_sub(1);
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if let Some(mut idx) = app.selected_result_index {
                                    idx = (idx + 1).min(app.search_results.len() - 1);
                                    app.selected_result_index = Some(idx);
                                }
                            }
                            KeyCode::Enter => {
                                if let Some(index) = app.selected_result_index {
                                    let selected = &app.search_results[index];
                                    match app.mode {
                                        Some(Mode::Stream) => {
                                            app.current_view = View::Streaming;
                                            let identifier = selected.identifier.clone();
                                            let visualization_data = Arc::clone(
                                                &app.visualization_data
                                            );
                                            let ffplay_process = stream_audio(
                                                &identifier,
                                                visualization_data
                                            )?;
                                            app.ffplay_process = Some(ffplay_process);
                                        }
                                        Some(Mode::Download) => {
                                            app.current_view = View::Downloading; // Switch to Downloading view
                                            match app.source {
                                                Source::YouTube => {
                                                    download_youtube_audio(
                                                        selected.identifier.clone(),
                                                        selected.title.clone(),
                                                        Arc::clone(&app.download_status)
                                                    );
                                                }
                                                Source::InternetArchive => {
                                                    download_archive_audio(
                                                        selected.identifier.clone(),
                                                        selected.title.clone(),
                                                        Arc::clone(&app.download_status)
                                                    );
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            KeyCode::Left => {
                                app.current_view = View::SourceSelection;
                            }
                            KeyCode::Right => {
                                if let Some(index) = app.selected_result_index {
                                    let selected = &app.search_results[index];
                                    match app.mode {
                                        Some(Mode::Stream) => {
                                            app.current_view = View::Streaming;
                                            let identifier = selected.identifier.clone();
                                            let visualization_data = Arc::clone(
                                                &app.visualization_data
                                            );
                                            let ffplay_process = stream_audio(
                                                &identifier,
                                                visualization_data
                                            )?;
                                            app.ffplay_process = Some(ffplay_process);
                                        }
                                        Some(Mode::Download) => {
                                            app.current_view = View::Downloading; // Switch to Downloading view
                                            match app.source {
                                                Source::YouTube => {
                                                    download_youtube_audio(
                                                        selected.identifier.clone(),
                                                        selected.title.clone(),
                                                        Arc::clone(&app.download_status)
                                                    );
                                                }
                                                Source::InternetArchive => {
                                                    download_archive_audio(
                                                        selected.identifier.clone(),
                                                        selected.title.clone(),
                                                        Arc::clone(&app.download_status)
                                                    );
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    View::Streaming => {
                        if key.code == KeyCode::Esc {
                            app.stop_streaming();
                            app.current_view = View::SearchResults;
                        } else if key.code == KeyCode::Left {
                            app.stop_streaming();
                            app.current_view = View::SearchResults;
                        }
                    }
                    View::Downloading => {
                        if key.code == KeyCode::Left {
                            app.current_view = View::SearchResults;
                            let mut download_status = app.download_status.lock().unwrap();
                            *download_status = None; // Clear the download status message
                        } else if key.code == KeyCode::Esc {
                            app.current_view = View::SearchResults;
                            let mut download_status = app.download_status.lock().unwrap();
                            *download_status = None; // Clear the download status message
                        }
                    }
                }

                if
                    key.code == KeyCode::Esc ||
                    (key.code == KeyCode::Char('c') &&
                        key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    app.stop_streaming();
                    break;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
