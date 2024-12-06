use ratatui::{ prelude::*, widgets::*, layout::{ Layout, Direction, Constraint } };
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
            let sources = vec!["1. YouTube", "2. Internet Archive", "3. Free Music Archive"];
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
                .title(format!("Visual (Equalizer {})", app.current_equalizer + 1))
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
