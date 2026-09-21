use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    widgets::*,
};

use std::path::Path;

use crate::aboutApp;
use crate::app::{AppUi, View};
use crate::video::VideoMode;

pub fn render(app: &AppUi, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Header
            Constraint::Length(1), // Second header
            Constraint::Min(10),   // Main content
        ])
        .split(frame.area());

    let light_green_style = Style::default().fg(Color::LightGreen);
    let white_style = Style::default().fg(Color::White);
    let dim_style = Style::default().fg(Color::Gray);

    let header_paragraph = Paragraph::new(
        r#"
▗▄▄▖ ▗▖    ▗▄▄▖ ▗▖    ▗▄▖▗▖  ▗▖▗▄▄▄▖▗▄▄▖
▐▌ ▐▌▐▌    ▐▌ ▐▌▐▌   ▐▌ ▐▌▝▚▞▘ ▐▌   ▐▌ ▐▌
▐▛▀▘ ▐▌    ▐▛▀▘ ▐▌   ▐▛▀▜▌ ▐▌  ▐▛▀▀▘▐▛▀▚▖
▐▌▗▄▄▞▘    ▐▌   ▐▙▄▄▖▐▌ ▐▌ ▐▌  ▐▙▄▄▖▐▌ ▐▌
"#,
    )
    .style(light_green_style)
    .alignment(Alignment::Center);

    frame.render_widget(header_paragraph, chunks[0]);

    let second_header_paragraph = Paragraph::new("Made with 🌿 by Pocket Jack")
        .style(white_style)
        .alignment(Alignment::Center);

    frame.render_widget(second_header_paragraph, chunks[1]);

    if let Some(loading) = &app.loading {
        loading.render(frame, chunks[2]);
        return;
    }

    match app.current_view {
        View::ModeSelection => {
            let modes = [
                "1. STREAM MUSIC",
                "2. DOWNLOAD MUSIC",
                "3. OFFLINE PLAYER",
                "4. ABOUT PJ-PLAYER",
            ];

            let items: Vec<ListItem> = modes
                .iter()
                .enumerate()
                .map(|(index, mode)| {
                    let style = if Some(index) == app.selected_result_index {
                        Style::default().bg(Color::Blue).fg(Color::White)
                    } else {
                        white_style
                    };

                    ListItem::new(*mode).style(style)
                })
                .collect();

            frame.render_widget(
                List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Choose a Mode")
                        .style(light_green_style),
                ),
                chunks[2],
            );
        }

        View::SearchInput => {
            let input_block = Block::default()
                .borders(Borders::ALL)
                .title("Search Music")
                .style(light_green_style);

            let input_text = app
                .notice
                .as_deref()
                .map(|notice| format!("(Search Query): {}\n{}", app.search_input, notice))
                .unwrap_or_else(|| format!("(Search Query): {}", app.search_input));

            let input = Paragraph::new(input_text)
                .style(white_style)
                .block(input_block);

            let search_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(10)])
                .split(chunks[2]);

            frame.render_widget(input, search_chunks[0]);
        }

        View::SourceSelection => {
            let sources = ["1. YouTube", "2. Internet Archive"];

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
                    .style(light_green_style),
            );

            frame.render_widget(list, chunks[2]);
        }

        View::FolderInput => {
            let folder_block = Block::default()
                .borders(Borders::ALL)
                .title("Offline Player - Folder")
                .style(light_green_style);

            let folder_text = app
                .notice
                .as_deref()
                .map(|notice| format!("Folder: {}\n{}", app.folder_input, notice))
                .unwrap_or_else(|| format!("Folder: {}", app.folder_input));

            frame.render_widget(
                Paragraph::new(folder_text)
                    .style(white_style)
                    .block(folder_block),
                chunks[2],
            );
        }

        View::OfflineFiles => {
            let offline_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(1)])
                .split(chunks[2]);

            let search_text = if app.offline_search_task.is_some() {
                format!(
                    "Searching: {}\nPress Esc or Ctrl+C to cancel",
                    app.offline_search_input
                )
            } else if app.offline_search_input.is_empty() {
                "Search by name or anything...".to_string()
            } else {
                format!("Search: {}", app.offline_search_input)
            };

            frame.render_widget(
                Paragraph::new(search_text).style(white_style).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(if app.offline_searching {
                            "Offline Search"
                        } else {
                            "Offline Player Search (press /)"
                        })
                        .style(light_green_style),
                ),
                offline_chunks[0],
            );

            let items: Vec<ListItem> = if app.offline_entries.is_empty() {
                vec![ListItem::new("No matching folders or audio files").style(white_style)]
            } else {
                let selected = app.selected_offline_entry.unwrap_or(0);
                let visible_rows = offline_chunks[1].height.saturating_sub(2).max(1) as usize;
                let start = selected.saturating_sub(visible_rows - 1);
                let end = (start + visible_rows).min(app.offline_entries.len());

                app.offline_entries[start..end]
                    .iter()
                    .enumerate()
                    .map(|(index, path)| {
                        let actual_index = start + index;

                        let style = if Some(actual_index) == app.selected_offline_entry {
                            Style::default().bg(Color::Blue).fg(Color::White)
                        } else {
                            white_style
                        };

                        let marker = if path.is_dir() { "[DIR] " } else { "      " };

                        let display_name = path
                            .strip_prefix(Path::new(&app.folder_input))
                            .unwrap_or(path)
                            .display();

                        ListItem::new(format!("{}{}", marker, display_name)).style(style)
                    })
                    .collect()
            };

            frame.render_widget(
                List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(
                            "Offline Player: {} ({}/{})",
                            app.folder_input,
                            app.selected_offline_entry
                                .map(|index| index + 1)
                                .unwrap_or(0),
                            app.offline_entries.len()
                        ))
                        .style(light_green_style),
                ),
                offline_chunks[1],
            );
        }

        View::About => {
            frame.render_widget(
                Paragraph::new(aboutApp::content())
                    .style(white_style)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("About PJ-Player")
                            .style(light_green_style),
                    )
                    .wrap(Wrap { trim: false }),
                chunks[2],
            );
        }

        View::SearchResults => {
            let mut download_status = app.download_status.lock().unwrap();
            *download_status = None;
            drop(download_status);

            if app.search_results.is_empty() {
                let no_results_item = ListItem::new("NO MUSIC FOUND ==").style(white_style).bold();

                let no_results_list = List::new(vec![no_results_item]).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Search Results")
                        .style(light_green_style),
                );

                frame.render_widget(no_results_list, chunks[2]);
            } else {
                let selected = app.selected_result_index.unwrap_or(0);
                let visible_rows = chunks[2].height.saturating_sub(2).max(1) as usize;
                let start = selected.saturating_sub(visible_rows - 1);
                let end = (start + visible_rows).min(app.search_results.len());

                let results: Vec<ListItem> = app.search_results[start..end]
                    .iter()
                    .enumerate()
                    .map(|(i, result)| {
                        let actual_index = start + i;

                        let style = if Some(actual_index) == app.selected_result_index {
                            Style::default().bg(Color::Blue).fg(Color::White)
                        } else {
                            white_style
                        };

                        let content = Line::from(vec![
                            Span::raw(format!("{}: ", actual_index + 1)),
                            Span::raw(
                                result
                                    .title
                                    .chars()
                                    .take(chunks[2].width.saturating_sub(20) as usize)
                                    .collect::<String>(),
                            ),
                            Span::raw(format!(" ({:?})", result.source)),
                        ]);

                        ListItem::new(content).style(style)
                    })
                    .collect();

                let list = List::new(results).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Search Results")
                        .style(light_green_style),
                );

                frame.render_widget(list, chunks[2]);
            }
        }

        View::Streaming => {
            let streaming_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(5), // Song information
                    Constraint::Min(8),    // Visual / video
                    Constraint::Length(7), // Controls
                ])
                .split(chunks[2]);

            // ---------------------------------------------------------
            // Song information
            // ---------------------------------------------------------

            let song_block = Block::default()
                .borders(Borders::ALL)
                .title(if app.paused {
                    "Now Paused"
                } else {
                    "Now Streaming"
                })
                .style(light_green_style);

            let song_name = if app.mode == Some(crate::app::Mode::OfflinePlayer) {
                app.selected_offline_index
                    .and_then(|index| app.offline_files.get(index))
                    .and_then(|path| path.file_name())
                    .and_then(|name| name.to_str())
                    .unwrap_or("Unknown Song")
            } else if let Some(index) = app.selected_result_index {
                app.search_results
                    .get(index)
                    .map(|result| result.title.as_str())
                    .unwrap_or("Unknown Song")
            } else {
                "Unknown Song"
            };

            let current_time = format_time(app.position);
            let total_time = format_time(app.duration);

            let song_info = Text::from(vec![
                Line::from(Span::styled(song_name, white_style)),
                Line::from(Span::styled(
                    format!("{} / {}", current_time, total_time),
                    light_green_style,
                )),
            ]);

            let song_paragraph = Paragraph::new(song_info)
                .block(song_block)
                .alignment(Alignment::Center);

            frame.render_widget(song_paragraph, streaming_chunks[0]);

            // ---------------------------------------------------------
            // Visualizer / Video
            // ---------------------------------------------------------

            if let Some(video_player) = &app.video_player {
                if let Some(video_mode) = app.video_mode {
                    let video_title = match video_mode {
                        VideoMode::AsciiShading => "Retro Video - ASCII SHADING",
                        VideoMode::MonoBlock => "Retro Video - MONO BLOCK",
                        VideoMode::MonoVideo => "Retro Video - MONO VIDEO",
                        VideoMode::Video => "Retro Video - VIDEO",
                    };

                    let video_block = Block::default()
                        .borders(Borders::ALL)
                        .title(video_title)
                        .style(light_green_style);

                    frame.render_widget(video_block.clone(), streaming_chunks[1]);

                    let video_area = video_block.inner(streaming_chunks[1]);

                    if let Some(video_frame) = video_player.latest_frame() {
                        render_video_frame(frame, video_area, &video_frame, video_mode);
                    } else if let Some(error) = video_player.error() {
                        let error = Paragraph::new(format!("Video error: {error}"))
                            .style(dim_style)
                            .alignment(Alignment::Center);

                        frame.render_widget(error, video_area);
                    } else {
                        let waiting = Paragraph::new("Loading video...")
                            .style(dim_style)
                            .alignment(Alignment::Center);

                        frame.render_widget(waiting, video_area);
                    }
                } else {
                    render_equalizer(app, frame, streaming_chunks[1], light_green_style);
                }
            } else {
                render_equalizer(app, frame, streaming_chunks[1], light_green_style);
            }

            // ---------------------------------------------------------
            // Controls
            // ---------------------------------------------------------

            let help_block = Block::default()
                .borders(Borders::ALL)
                .title("Controls")
                .style(light_green_style);

            let status_text = if app.paused {
                "Paused - Press SPACE to play"
            } else {
                "Playing - Press SPACE to pause"
            };

            let help_text = Text::from(vec![
                Line::from(Span::raw(status_text)),
                Line::from(Span::raw("← / →  Seek 15 seconds")),
                Line::from(Span::raw(
                    if app.mode == Some(crate::app::Mode::OfflinePlayer) {
                        "↑ / ↓  Next / previous song"
                    } else {
                        ""
                    },
                )),
                Line::from(Span::raw(
                    "1-6 Equalizer styles   |   7-0 Video modes   |   ESC Back to search",
                )),
            ]);

            let help_paragraph = Paragraph::new(help_text)
                .style(dim_style)
                .block(help_block)
                .alignment(Alignment::Center);

            frame.render_widget(help_paragraph, streaming_chunks[2]);
        }

        View::Downloading => {
            let download_status = app.download_status.lock().unwrap();

            let status_message = download_status
                .as_deref()
                .unwrap_or("No downloads in progress");

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

fn render_equalizer(app: &AppUi, frame: &mut Frame, area: Rect, light_green_style: Style) {
    let eq_data = app.visualization_data.lock().unwrap();

    let visual_block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Visual (Equalizer {})", app.current_equalizer + 1))
        .style(light_green_style);

    frame.render_widget(visual_block.clone(), area);

    let inner_area = visual_block.inner(area);

    if !eq_data.is_empty() && inner_area.width > 0 && inner_area.height > 0 {
        let max_height = (inner_area.height as usize).min(10);

        let bar_width = ((inner_area.width as usize) / eq_data.len()).max(1);

        let eq_styles = [
            (vec!['|', ' '], Style::default().fg(Color::Green)),
            (vec!['█'], Style::default().fg(Color::Cyan)),
            (vec!['='], Style::default().fg(Color::Yellow)),
            (vec!['▒'], Style::default().fg(Color::Magenta)),
            (vec!['‖'], Style::default().fg(Color::Blue)),
            (vec!['█', ' '], Style::default().fg(Color::Red)),
        ];

        let (chars, style) = &eq_styles[app.current_equalizer];

        for (i, &value) in eq_data.iter().enumerate() {
            let bar_height = (((value as f64) / 10.0) * (max_height as f64)).round() as usize;

            let x = inner_area.x + ((i * bar_width) as u16);

            if x >= inner_area.x + inner_area.width {
                break;
            }

            let available_width = (inner_area.x + inner_area.width - x) as usize;

            let actual_width = bar_width.min(available_width);

            let clamped_height = bar_height.min(inner_area.height as usize);

            let y = inner_area.y + inner_area.height - (clamped_height as u16);

            for j in 0..clamped_height {
                let y_pos = y + (j as u16);

                let character = chars[j % chars.len()];

                let bar = Paragraph::new(character.to_string())
                    .style(*style)
                    .alignment(Alignment::Center);

                frame.render_widget(bar, Rect::new(x, y_pos, actual_width as u16, 1));
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Video rendering
// -----------------------------------------------------------------------------
//
// Mapping:
// 7 -> ASCII Shading
// 8 -> MonoBlock
// 9 -> Mono Video
// 0 -> Video
//
// The renderers operate directly on Ratatui's frame buffer.
// -----------------------------------------------------------------------------

fn render_video_frame(
    frame: &mut Frame,
    area: Rect,
    video_frame: &retrotermplayer::decoder::VideoFrame,
    mode: VideoMode,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    match mode {
        VideoMode::AsciiShading => {
            render_ascii_shading(frame, area, video_frame);
        }

        VideoMode::MonoBlock => {
            render_mono_block(frame, area, video_frame);
        }

        VideoMode::MonoVideo => {
            render_mono_video(frame, area, video_frame);
        }

        VideoMode::Video => {
            render_color_video(frame, area, video_frame);
        }
    }
}

// -----------------------------------------------------------------------------
// 7 - ASCII Shading
// -----------------------------------------------------------------------------

fn render_ascii_shading(
    frame: &mut Frame,
    area: Rect,
    video_frame: &retrotermplayer::decoder::VideoFrame,
) {
    let source_width = video_frame.width as usize;
    let source_height = video_frame.height as usize;

    if source_width == 0 || source_height == 0 {
        return;
    }

    const RAMP: &[u8] = b"@#8&o:,. ";

    let render_width = area.width as usize;
    let render_height = (area.height as usize) * 2;

    for y in 0..area.height as usize {
        let top_y = ((y * 2 * source_height) / render_height).min(source_height - 1);

        let bottom_y = (((y * 2 + 1) * source_height) / render_height).min(source_height - 1);

        for x in 0..render_width {
            let source_x = ((x * source_width) / render_width).min(source_width - 1);

            let top = brightness(pixel(video_frame, source_x, top_y));

            let bottom = brightness(pixel(video_frame, source_x, bottom_y));

            let average = (((top as u16) + (bottom as u16)) / 2) as u8;

            let index = ((average as usize) * (RAMP.len() - 1)) / 255;

            let symbol = (RAMP[index] as char).to_string();

            set_cell(
                frame,
                area.x + (x as u16),
                area.y + (y as u16),
                &symbol,
                Color::White,
                Color::Black,
            );
        }
    }
}

fn render_mono_block(
    frame: &mut Frame,
    area: Rect,
    video_frame: &retrotermplayer::decoder::VideoFrame,
) {
    let source_width = video_frame.width;
    let source_height = video_frame.height;

    if source_width == 0 || source_height == 0 {
        return;
    }

    let render_width = area.width as usize;
    let render_height = (area.height as usize) * 2;

    for y in 0..area.height as usize {
        let top_y = ((y * 2 * source_height) / render_height).min(source_height - 1);

        let bottom_y = (((y * 2 + 1) * source_height) / render_height).min(source_height - 1);

        for x in 0..render_width {
            let source_x = ((x * source_width) / render_width).min(source_width - 1);

            let top = brightness(pixel(video_frame, source_x, top_y));
            let bottom = brightness(pixel(video_frame, source_x, bottom_y));

            let symbol = match (top > 100, bottom > 100) {
                (true, true) => "█",
                (true, false) => "▀",
                (false, true) => "▄",
                (false, false) => " ",
            };

            set_cell(
                frame,
                area.x + (x as u16),
                area.y + (y as u16),
                symbol,
                Color::White,
                Color::Black,
            );
        }
    }
}

fn render_mono_video(
    frame: &mut Frame,
    area: Rect,
    video_frame: &retrotermplayer::decoder::VideoFrame,
) {
    let source_width = video_frame.width;
    let source_height = video_frame.height;

    if source_width == 0 || source_height == 0 {
        return;
    }

    let render_width = area.width as usize;
    let render_height = (area.height as usize) * 2;

    for y in 0..area.height as usize {
        let top_y = ((y * 2 * source_height) / render_height).min(source_height - 1);

        let bottom_y = (((y * 2 + 1) * source_height) / render_height).min(source_height - 1);

        for x in 0..render_width {
            let source_x = ((x * source_width) / render_width).min(source_width - 1);

            let top = brightness(pixel(video_frame, source_x, top_y));
            let bottom = brightness(pixel(video_frame, source_x, bottom_y));

            let top = grayscale_to_ansi(top);
            let bottom = grayscale_to_ansi(bottom);

            set_cell(
                frame,
                area.x + (x as u16),
                area.y + (y as u16),
                "▀",
                Color::Indexed(top),
                Color::Indexed(bottom),
            );
        }
    }
}

fn render_color_video(
    frame: &mut Frame,
    area: Rect,
    video_frame: &retrotermplayer::decoder::VideoFrame,
) {
    let source_width = video_frame.width;
    let source_height = video_frame.height;

    if source_width == 0 || source_height == 0 {
        return;
    }

    let render_width = area.width as usize;
    let render_height = (area.height as usize) * 2;

    for y in 0..area.height as usize {
        let top_y = ((y * 2 * source_height) / render_height).min(source_height - 1);

        let bottom_y = (((y * 2 + 1) * source_height) / render_height).min(source_height - 1);

        for x in 0..render_width {
            let source_x = ((x * source_width) / render_width).min(source_width - 1);

            let top = pixel(video_frame, source_x, top_y);
            let bottom = pixel(video_frame, source_x, bottom_y);

            let top = process_pixel(top, bayer_dither(x, y * 2));
            let bottom = process_pixel(bottom, bayer_dither(x, y * 2 + 1));

            set_cell(
                frame,
                area.x + (x as u16),
                area.y + (y as u16),
                "▀",
                Color::Indexed(rgb_to_ansi256(top.0, top.1, top.2)),
                Color::Indexed(rgb_to_ansi256(bottom.0, bottom.1, bottom.2)),
            );
        }
    }
}

fn process_pixel(pixel: (u8, u8, u8), dither: i16) -> (u8, u8, u8) {
    (
        process_channel(pixel.0, dither),
        process_channel(pixel.1, dither),
        process_channel(pixel.2, dither),
    )
}

fn process_channel(value: u8, dither: i16) -> u8 {
    let value = value as i16;

    let contrasted = ((value - 128) * 106) / 100 + 128;

    (contrasted + dither).clamp(0, 255) as u8
}

fn bayer_dither(x: usize, y: usize) -> i16 {
    const MATRIX: [[i16; 2]; 2] = [[-2, 1], [2, -1]];

    MATRIX[y & 1][x & 1]
}

fn grayscale_to_ansi(value: u8) -> u8 {
    if value < 8 {
        return 16;
    }

    if value > 248 {
        return 231;
    }

    232 + (((((value as u16) - 8) * 23) / 240) as u8)
}

fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
    if r.abs_diff(g) < 10 && g.abs_diff(b) < 10 {
        return grayscale_to_ansi(r);
    }

    let red = (((r as u16) * 5 + 127) / 255) as u8;
    let green = (((g as u16) * 5 + 127) / 255) as u8;
    let blue = (((b as u16) * 5 + 127) / 255) as u8;

    16 + 36 * red + 6 * green + blue
}
// -----------------------------------------------------------------------------
// Pixel helpers
// -----------------------------------------------------------------------------

fn pixel(frame: &retrotermplayer::decoder::VideoFrame, x: usize, y: usize) -> (u8, u8, u8) {
    let width = frame.width as usize;
    let index = (y * width + x) * 3;

    (
        frame.pixels[index],
        frame.pixels[index + 1],
        frame.pixels[index + 2],
    )
}

fn brightness(pixel: (u8, u8, u8)) -> u8 {
    (((pixel.0 as u32) * 299 + (pixel.1 as u32) * 587 + (pixel.2 as u32) * 114) / 1000) as u8
}

fn set_cell(frame: &mut Frame, x: u16, y: u16, symbol: &str, foreground: Color, background: Color) {
    if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
        cell.set_symbol(symbol)
            .set_fg(foreground)
            .set_bg(background);
    }
}

// -----------------------------------------------------------------------------
// Time formatting
// -----------------------------------------------------------------------------

/// Formats a duration in seconds as MM:SS.
///
/// Examples:
///     0      -> 00:00
///     83     -> 01:23
///     843    -> 14:03
fn format_time(seconds: f64) -> String {
    let total_seconds = seconds.max(0.0) as u64;

    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

    format!("{:02}:{:02}", minutes, seconds)
}
