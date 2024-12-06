mod app;
mod search;
mod stream;
mod download;
mod ui;

use std::error::Error;
use std::io;
use std::time::{ Duration, Instant };
use std::sync::Arc;
use crossterm::event::KeyEvent;
use crossterm::{
    event::{ self, Event, KeyCode },
    execute,
    terminal::{ disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen },
};
use ratatui::prelude::*;
use tokio::main;

use app::{ AppUi, Mode, Source, View };
use stream::stream_audio;
use download::{ download_youtube_audio, download_archive_audio, download_fma_audio };
use ui::render;

#[main]
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
                if
                    key.code == KeyCode::Esc ||
                    (key.code == KeyCode::Char('c') &&
                        key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL))
                {
                    break;
                }
                handle_key_event(&mut app, key).await?;
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

async fn handle_key_event(app: &mut AppUi, key: KeyEvent) -> Result<(), Box<dyn Error>> {
    match app.current_view {
        View::InitialSelection => handle_initial_selection(app, key).await,
        View::SearchInput => handle_search_input(app, key).await,
        View::SourceSelection => handle_source_selection(app, key).await,
        View::SearchResults => handle_search_results(app, key).await,
        View::Streaming => handle_streaming(app, key).await,
        View::Downloading => handle_downloading(app, key).await,
    }
}

async fn handle_initial_selection(app: &mut AppUi, key: KeyEvent) -> Result<(), Box<dyn Error>> {
    match key.code {
        KeyCode::Up => {
            app.selected_result_index = Some(
                app.selected_result_index.unwrap_or(0).saturating_sub(1)
            );
        }
        KeyCode::Down => {
            app.selected_result_index = Some((app.selected_result_index.unwrap_or(0) + 1).min(1));
        }
        KeyCode::Enter => {
            match app.selected_result_index {
                Some(0) => {
                    app.mode = Some(Mode::Stream);
                    app.source = Source::YouTube;
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
    Ok(())
}

async fn handle_search_input(app: &mut AppUi, key: KeyEvent) -> Result<(), Box<dyn Error>> {
    match key.code {
        KeyCode::Enter => {
            if app.mode == Some(Mode::Stream) {
                app.search().await?;
            } else {
                app.current_view = View::SourceSelection;
                // app.source = Source::YouTube;
                // app.search().await?;
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
                // app.source = Source::YouTube;
                // app.search().await?;
            }
        }
        _ => {}
    }
    Ok(())
}

async fn handle_source_selection(app: &mut AppUi, key: KeyEvent) -> Result<(), Box<dyn Error>> {
    match key.code {
        KeyCode::Up => {
            app.selected_source_index = app.selected_source_index.saturating_sub(1);
        }
        KeyCode::Down => {
            app.selected_source_index = (app.selected_source_index + 1).min(2); // Update the max index to 2
        }
        KeyCode::Enter => {
            app.source = match app.selected_source_index {
                0 => Source::YouTube,
                1 => Source::InternetArchive,
                2 => Source::FreeMusicArchive,
                _ => Source::YouTube,
            };
            app.search().await?;
        }
        KeyCode::Left => {
            app.current_view = View::SearchInput;
        }
        KeyCode::Right => {
            app.source = match app.selected_source_index {
                0 => Source::YouTube,
                1 => Source::InternetArchive,
                2 => Source::FreeMusicArchive,
                _ => Source::YouTube,
            };
            app.search().await?;
        }
        _ => {}
    }
    Ok(())
}

async fn handle_search_results(app: &mut AppUi, key: KeyEvent) -> Result<(), Box<dyn Error>> {
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
                        let visualization_data = Arc::clone(&app.visualization_data);
                        let ffplay_process = stream_audio(&identifier, visualization_data)?;
                        app.ffplay_process = Some(ffplay_process);
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
                            Source::FreeMusicArchive => {
                                download_fma_audio(
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
            match app.mode {
                Some(Mode::Stream) => {
                    app.current_view = View::SearchInput;
                }
                Some(Mode::Download) => {
                    app.current_view = View::SourceSelection;
                }
                _ => {}
            }
        }
        KeyCode::Right => {
            if let Some(index) = app.selected_result_index {
                let selected = &app.search_results[index];
                match app.mode {
                    Some(Mode::Stream) => {
                        app.current_view = View::Streaming;
                        let identifier = selected.identifier.clone();
                        let visualization_data = Arc::clone(&app.visualization_data);
                        let ffplay_process = stream_audio(&identifier, visualization_data)?;
                        app.ffplay_process = Some(ffplay_process);
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
                            Source::FreeMusicArchive => {
                                download_fma_audio(
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
    Ok(())
}

async fn handle_streaming(app: &mut AppUi, key: KeyEvent) -> Result<(), Box<dyn Error>> {
    if key.code == KeyCode::Esc {
        app.stop_streaming();
        app.current_view = View::SearchResults;
    } else if key.code == KeyCode::Left {
        app.stop_streaming();
        app.current_view = View::SearchResults;
    }
    Ok(())
}

async fn handle_downloading(app: &mut AppUi, key: KeyEvent) -> Result<(), Box<dyn Error>> {
    if key.code == KeyCode::Left {
        app.current_view = View::SearchResults;
        let mut download_status = app.download_status.lock().unwrap();
        *download_status = None;
    } else if key.code == KeyCode::Esc {
        app.current_view = View::SearchResults;
        let mut download_status = app.download_status.lock().unwrap();
        *download_status = None;
    }
    Ok(())
}
