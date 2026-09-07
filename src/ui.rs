use ratatui::{ layout::{ Constraint, Direction, Layout }, prelude::*, widgets::* };

use crate::aboutApp;
use crate::app::{ AppUi, View };

pub fn render(app: &AppUi, frame: &mut Frame) {
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
    let dim_style = Style::default().fg(Color::Gray);

    let header_paragraph = Paragraph::new(
        r#"
▗▄▄▖ ▗▖    ▗▄▄▖ ▗▖    ▗▄▖▗▖  ▗▖▗▄▄▄▖▗▄▄▖ 
▐▌ ▐▌▐▌    ▐▌ ▐▌▐▌   ▐▌ ▐▌▝▚▞▘ ▐▌   ▐▌ ▐▌
▐▛▀▘ ▐▌    ▐▛▀▘ ▐▌   ▐▛▀▜▌ ▐▌  ▐▛▀▀▘▐▛▀▚▖
▐▌▗▄▄▞▘    ▐▌   ▐▙▄▄▖▐▌ ▐▌ ▐▌  ▐▙▄▄▖▐▌ ▐▌
"#
    )
        .style(light_green_style)
        .alignment(Alignment::Center);

    frame.render_widget(header_paragraph, chunks[0]);

    let second_header_paragraph = Paragraph::new("Made with 🌿 by Pocket Jack")
        .style(white_style)
        .alignment(Alignment::Center);

    frame.render_widget(second_header_paragraph, chunks[1]);

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
                        .style(light_green_style)
                ),
                chunks[2]
            );
        }

        View::SearchInput => {
            let input_block = Block::default()
                .borders(Borders::ALL)
                .title("Search Music")
                .style(light_green_style);

            let input_text = app.notice
                .as_deref()
                .map(|notice| format!("(Search Query): {}\n{}", app.search_input, notice))
                .unwrap_or_else(|| format!("(Search Query): {}", app.search_input));

            let input = Paragraph::new(input_text).style(white_style).block(input_block);

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
                    .style(light_green_style)
            );

            frame.render_widget(list, chunks[2]);
        }

        View::FolderInput => {
            let folder_block = Block::default()
                .borders(Borders::ALL)
                .title("Offline Player - Folder")
                .style(light_green_style);
            let folder_text = app.notice
                .as_deref()
                .map(|notice| format!("Folder: {}\n{}", app.folder_input, notice))
                .unwrap_or_else(|| format!("Folder: {}", app.folder_input));
            frame.render_widget(
                Paragraph::new(folder_text).style(white_style).block(folder_block),
                chunks[2]
            );
        }

        View::OfflineFiles => {
            let items: Vec<ListItem> = if app.offline_entries.is_empty() {
                vec![ListItem::new("No matching folders or audio files").style(white_style)]
            } else {
                let selected = app.selected_offline_entry.unwrap_or(0);
                let visible_rows = chunks[2].height.saturating_sub(2).max(1) as usize;
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
                        ListItem::new(
                            format!(
                                "{}{}",
                                marker,
                                path.file_name().unwrap_or_default().to_string_lossy()
                            )
                        ).style(style)
                    })
                    .collect()
            };

            frame.render_widget(
                List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(
                            if app.offline_searching {
                                format!("Offline Search: {}", app.offline_search_input)
                            } else {
                                format!(
                                    "Offline Player: {} ({}/{})",
                                    app.folder_input,
                                    app.selected_offline_entry.unwrap_or(0) + 1,
                                    app.offline_entries.len()
                                )
                            }
                        )
                        .style(light_green_style)
                ),
                chunks[2]
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
                            .style(light_green_style)
                    )
                    .wrap(Wrap { trim: false }),
                chunks[2]
            );
        }

        View::SearchResults => {
            // Clear any previous download status when returning
            // to the search results screen.
            let mut download_status = app.download_status.lock().unwrap();
            *download_status = None;
            drop(download_status);

            if app.search_results.is_empty() {
                let no_results_item = ListItem::new("NO MUSIC FOUND ==").style(white_style).bold();

                let no_results_list = List::new(vec![no_results_item]).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Search Results")
                        .style(light_green_style)
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
                    Constraint::Length(5), // Song information
                    Constraint::Min(8), // Equalizer
                    Constraint::Length(7), // Controls
                ])
                .split(chunks[2]);

            // ---------------------------------------------------------
            // Song information
            // ---------------------------------------------------------

            let song_block = Block::default()
                .borders(Borders::ALL)
                .title(if app.paused { "Now Paused" } else { "Now Streaming" })
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

            let song_info = Text::from(
                vec![
                    Line::from(Span::styled(song_name, white_style)),
                    Line::from(
                        Span::styled(
                            format!("{} / {}", current_time, total_time),
                            light_green_style
                        )
                    )
                ]
            );

            let song_paragraph = Paragraph::new(song_info)
                .block(song_block)
                .alignment(Alignment::Center);

            frame.render_widget(song_paragraph, streaming_chunks[0]);

            // ---------------------------------------------------------
            // Equalizer
            // ---------------------------------------------------------

            let eq_area = streaming_chunks[1];

            let eq_data = app.visualization_data.lock().unwrap();

            let visual_block = Block::default()
                .borders(Borders::ALL)
                .title(format!("Visual (Equalizer {})", app.current_equalizer + 1))
                .style(light_green_style);

            frame.render_widget(visual_block.clone(), eq_area);

            let inner_area = visual_block.inner(eq_area);

            // Don't attempt to divide by the number of values if
            // the visualization data happens to be empty.
            if !eq_data.is_empty() && inner_area.width > 0 && inner_area.height > 0 {
                let max_height = (inner_area.height as usize).min(10);

                let bar_width = ((inner_area.width as usize) / eq_data.len()).max(1);

                let eq_styles = [
                    // Style 1
                    (vec!['|', ' '], Style::default().fg(Color::Green)),
                    // Style 2
                    (vec!['█'], Style::default().fg(Color::Cyan)),
                    // Style 3
                    (vec!['='], Style::default().fg(Color::Yellow)),
                    // Style 4
                    (vec!['▒'], Style::default().fg(Color::Magenta)),
                    // Style 5
                    (vec!['‖'], Style::default().fg(Color::Blue)),
                    // Style 6
                    (vec!['█', ' '], Style::default().fg(Color::Red)),
                ];

                let (chars, style) = &eq_styles[app.current_equalizer];

                for (i, &value) in eq_data.iter().enumerate() {
                    let bar_height = (
                        ((value as f64) / 10.0) *
                        (max_height as f64)
                    ).round() as usize;

                    let x = inner_area.x + ((i * bar_width) as u16);

                    // Stop rendering once the next bar would be outside
                    // the available width.
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

            let help_text = Text::from(
                vec![
                    Line::from(Span::raw(status_text)),
                    Line::from(Span::raw("← / →  Seek 15 seconds")),
                    Line::from(
                        Span::raw(
                            if app.mode == Some(crate::app::Mode::OfflinePlayer) {
                                "↑ / ↓  Next / previous song"
                            } else {
                                ""
                            }
                        )
                    ),
                    Line::from(Span::raw("1-6 Equalizer style   |   ESC Back to search"))
                ]
            );

            let help_paragraph = Paragraph::new(help_text)
                .style(dim_style)
                .block(help_block)
                .alignment(Alignment::Center);

            frame.render_widget(help_paragraph, streaming_chunks[2]);
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
