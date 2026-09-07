#[allow(non_snake_case)]
mod aboutApp;
mod app;
mod download;
mod error;
#[allow(non_snake_case)]
mod offlinePlayer;
mod search;
mod stream;
mod ui;
mod visualizer;

use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{ Duration, Instant };

use crossterm::event::KeyEvent;
use crossterm::{
    event::{ self, Event, KeyCode },
    execute,
    terminal::{ disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen },
};

use ratatui::prelude::*;
use tokio::main;

use app::{ AppUi, Mode, Source, View };
use download::{ download_archive_audio, download_youtube_audio };
use error::AppError;
use stream::{ stream_audio, stream_local_audio };
use ui::render;

#[main]
async fn main() -> Result<(), AppError> {
    enable_raw_mode()?;

    let mut stdout = io::stdout();

    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppUi::new();

    let result = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut AppUi
) -> Result<(), AppError> {
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        app.update_stream_lifecycle()?;
        app.update_playback_position();

        if
            app.current_view == View::OfflineFiles &&
            app.offline_autoplay &&
            app.stream_process.is_none()
        {
            start_offline_playback(app)?;
        }

        terminal.draw(|frame| render(app, frame))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if
                    key.code == KeyCode::Char('c') &&
                    key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    break;
                }

                handle_key_event(app, key).await?;
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    Ok(())
}

async fn handle_key_event(app: &mut AppUi, key: KeyEvent) -> Result<(), AppError> {
    match app.current_view {
        View::ModeSelection => handle_mode_selection(app, key).await,
        View::SearchInput => handle_search_input(app, key).await,
        View::SourceSelection => handle_source_selection(app, key).await,
        View::FolderInput => handle_folder_input(app, key).await,
        View::OfflineFiles => handle_offline_files(app, key).await,
        View::About => handle_about(app, key).await,
        View::SearchResults => handle_search_results(app, key).await,
        View::Streaming => handle_streaming(app, key).await,
        View::Downloading => handle_downloading(app, key).await,
    }
}

async fn handle_mode_selection(app: &mut AppUi, key: KeyEvent) -> Result<(), AppError> {
    match key.code {
        KeyCode::Up => {
            app.selected_result_index = Some(
                app.selected_result_index.unwrap_or(0).saturating_sub(1)
            );
        }
        KeyCode::Down => {
            app.selected_result_index = Some((app.selected_result_index.unwrap_or(0) + 1).min(3));
        }
        KeyCode::Enter | KeyCode::Right =>
            match app.selected_result_index.unwrap_or(0) {
                0 => {
                    app.mode = Some(Mode::Stream);
                    app.source = Source::YouTube;
                    reset_search(app);
                    app.current_view = View::SearchInput;
                }
                1 => {
                    app.mode = Some(Mode::Download);
                    app.source = Source::YouTube;
                    reset_search(app);
                    app.current_view = View::SearchInput;
                }
                2 => {
                    app.mode = Some(Mode::OfflinePlayer);
                    app.folder_input = home_directory();
                    app.offline_root = PathBuf::from(&app.folder_input);
                    app.offline_search_input.clear();
                    app.offline_searching = false;
                    app.current_view = View::FolderInput;
                }
                3 => {
                    app.mode = Some(Mode::AboutApp);
                    app.current_view = View::About;
                }
                _ => {}
            }
        _ => {}
    }

    Ok(())
}

async fn handle_search_input(app: &mut AppUi, key: KeyEvent) -> Result<(), AppError> {
    match key.code {
        KeyCode::Enter | KeyCode::Right => {
            if app.search_input.trim().is_empty() {
                app.notice = Some("Enter a search query first".to_string());
            } else if app.mode == Some(Mode::Download) {
                app.search_results.clear();
                app.selected_result_index = Some(0);
                app.selected_source_index = 0;
                app.current_view = View::SourceSelection;
            } else {
                app.search().await?;
            }
        }

        KeyCode::Char(c) => {
            app.search_input.push(c);
        }

        KeyCode::Backspace => {
            app.search_input.pop();
        }

        KeyCode::Left => {
            app.current_view = View::ModeSelection;
        }

        _ => {}
    }

    Ok(())
}

async fn handle_folder_input(app: &mut AppUi, key: KeyEvent) -> Result<(), AppError> {
    match key.code {
        KeyCode::Enter | KeyCode::Right => {
            if let Err(error) = app.load_offline_folder() {
                app.notice = Some(error.to_string());
            }
        }
        KeyCode::Char(c) => app.folder_input.push(c),
        KeyCode::Backspace => {
            app.folder_input.pop();
        }
        KeyCode::Left => {
            app.current_view = View::ModeSelection;
        }
        _ => {}
    }

    Ok(())
}

async fn handle_offline_files(app: &mut AppUi, key: KeyEvent) -> Result<(), AppError> {
    if app.offline_searching {
        match key.code {
            KeyCode::Enter => {
                app.offline_searching = false;
                app.load_offline_directory(&PathBuf::from(&app.folder_input))?;
            }
            KeyCode::Esc => {
                app.offline_searching = false;
                app.offline_search_input.clear();
                app.load_offline_directory(&PathBuf::from(&app.folder_input))?;
            }
            KeyCode::Backspace => {
                app.offline_search_input.pop();
                app.load_offline_directory(&PathBuf::from(&app.folder_input))?;
            }
            KeyCode::Char(character) => {
                app.offline_search_input.push(character);
                app.load_offline_directory(&PathBuf::from(&app.folder_input))?;
            }
            _ => {}
        }
        return Ok(());
    }

    match key.code {
        KeyCode::Up => {
            if let Some(index) = &mut app.selected_offline_entry {
                *index = index.saturating_sub(1);
            }
        }
        KeyCode::Down => {
            if !app.offline_entries.is_empty() {
                let index = app.selected_offline_entry.unwrap_or(0);
                app.selected_offline_entry = Some((index + 1).min(app.offline_entries.len() - 1));
            }
        }
        KeyCode::Char('/') => {
            app.offline_searching = true;
            app.offline_search_input.clear();
        }
        KeyCode::Enter | KeyCode::Right => {
            if let Some(entry_index) = app.selected_offline_entry {
                if let Some(entry) = app.offline_entries.get(entry_index).cloned() {
                    if entry.is_dir() {
                        app.offline_search_input.clear();
                        app.load_offline_directory(&entry)?;
                    } else if
                        let Some(audio_index) = app.offline_files
                            .iter()
                            .position(|file| file == &entry)
                    {
                        app.selected_offline_index = Some(audio_index);
                        app.offline_autoplay = true;
                        start_offline_playback(app)?;
                    }
                }
            }
        }
        KeyCode::Left | KeyCode::Esc => {
            app.offline_autoplay = false;
            let current_folder = PathBuf::from(&app.folder_input);
            if current_folder != app.offline_root {
                if let Some(parent) = current_folder.parent().map(PathBuf::from) {
                    app.offline_search_input.clear();
                    app.load_offline_directory(&parent)?;
                }
            } else {
                app.current_view = View::FolderInput;
            }
        }
        _ => {}
    }

    Ok(())
}

fn home_directory() -> String {
    std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
}

async fn handle_about(app: &mut AppUi, key: KeyEvent) -> Result<(), AppError> {
    if matches!(key.code, KeyCode::Left | KeyCode::Esc) {
        app.current_view = View::ModeSelection;
    }

    Ok(())
}

fn start_offline_playback(app: &mut AppUi) -> Result<(), AppError> {
    let Some(index) = app.selected_offline_index else {
        return Ok(());
    };
    let Some(path) = app.offline_files.get(index) else {
        return Ok(());
    };

    let (stream_process, stream_info) = stream_local_audio(
        path,
        Arc::clone(&app.visualization_data)
    )?;
    app.stream_process = Some(stream_process);
    app.start_playback(stream_info.duration);
    app.current_view = View::Streaming;

    Ok(())
}

fn reset_search(app: &mut AppUi) {
    app.search_input.clear();
    app.search_results.clear();
    app.selected_result_index = Some(0);
    app.notice = None;
}

async fn handle_source_selection(app: &mut AppUi, key: KeyEvent) -> Result<(), AppError> {
    match key.code {
        KeyCode::Up => {
            app.selected_source_index = app.selected_source_index.saturating_sub(1);
        }

        KeyCode::Down => {
            app.selected_source_index = (app.selected_source_index + 1).min(1);
        }

        KeyCode::Enter | KeyCode::Right => {
            app.source = match app.selected_source_index {
                0 => Source::YouTube,
                1 => Source::InternetArchive,
                _ => Source::YouTube,
            };

            app.search().await?;
        }

        KeyCode::Left => {
            app.current_view = View::SearchInput;
        }

        _ => {}
    }

    Ok(())
}

async fn handle_search_results(app: &mut AppUi, key: KeyEvent) -> Result<(), AppError> {
    match key.code {
        KeyCode::Up => {
            if let Some(idx) = &mut app.selected_result_index {
                if *idx == 0 {
                    app.current_view = if app.mode == Some(Mode::Download) {
                        View::SourceSelection
                    } else {
                        View::SearchInput
                    };
                } else {
                    *idx = idx.saturating_sub(1);
                }
            }
        }

        KeyCode::Down => {
            if app.search_results.is_empty() {
                app.selected_result_index = None;
            } else {
                let current_index = app.selected_result_index.unwrap_or(0);
                let next_index = (current_index + 1).min(app.search_results.len() - 1);

                app.selected_result_index = Some(next_index);
            }
        }

        KeyCode::Enter | KeyCode::Right => {
            let Some(index) = app.selected_result_index else {
                return Ok(());
            };

            let Some(selected) = app.search_results.get(index) else {
                return Ok(());
            };

            let identifier = selected.identifier.clone();

            match app.mode {
                Some(Mode::Stream) => {
                    app.current_view = View::Streaming;

                    let visualization_data = Arc::clone(&app.visualization_data);

                    let (stream_process, stream_info) = stream_audio(
                        &identifier,
                        visualization_data
                    )?;

                    app.stream_process = Some(stream_process);

                    app.start_playback(stream_info.duration);
                }

                Some(Mode::Download) => {
                    app.current_view = View::Downloading;

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

        KeyCode::Left =>
            match app.mode {
                Some(Mode::Stream) => {
                    app.current_view = View::ModeSelection;
                }

                Some(Mode::Download) => {
                    app.current_view = View::SourceSelection;
                }

                _ => {}
            }

        _ => {}
    }

    Ok(())
}

async fn handle_streaming(app: &mut AppUi, key: KeyEvent) -> Result<(), AppError> {
    match key.code {
        KeyCode::Esc => {
            app.stop_streaming();
            app.current_view = if app.mode == Some(Mode::OfflinePlayer) {
                app.offline_autoplay = false;
                View::OfflineFiles
            } else {
                View::SearchResults
            };
        }

        KeyCode::Char(' ') => {
            app.toggle_pause()?;
        }

        KeyCode::Left => {
            app.seek_backward()?;
        }

        KeyCode::Right => {
            app.seek_forward()?;
        }

        KeyCode::Up if app.mode == Some(Mode::OfflinePlayer) => {
            change_offline_track(app, 1)?;
        }

        KeyCode::Down if app.mode == Some(Mode::OfflinePlayer) => {
            change_offline_track(app, -1)?;
        }

        KeyCode::Char(c) if c.is_ascii_digit() => {
            let digit = c.to_digit(10).unwrap_or(0) as usize;

            if (1..=6).contains(&digit) {
                app.current_equalizer = digit - 1;
            }
        }

        _ => {}
    }

    Ok(())
}

fn change_offline_track(app: &mut AppUi, amount: isize) -> Result<(), AppError> {
    let Some(current_index) = app.selected_offline_index else {
        return Ok(());
    };

    let next_index = (current_index as isize) + amount;
    if !(0..app.offline_files.len() as isize).contains(&next_index) {
        return Ok(());
    }

    app.stop_streaming();
    app.selected_offline_index = Some(next_index as usize);
    app.selected_offline_entry = app.offline_entries
        .iter()
        .position(|entry| entry == &app.offline_files[next_index as usize]);
    app.offline_autoplay = true;
    start_offline_playback(app)
}

async fn handle_downloading(app: &mut AppUi, key: KeyEvent) -> Result<(), AppError> {
    if key.code == KeyCode::Left || key.code == KeyCode::Esc {
        app.current_view = View::SearchResults;

        if let Ok(mut download_status) = app.download_status.lock() {
            *download_status = None;
        }
    }

    Ok(())
}
