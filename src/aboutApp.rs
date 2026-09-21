use ratatui::prelude::*;

pub fn content() -> Text<'static> {
    Text::from(vec![
        Line::from(Span::styled(
            "A terminal music player that somehow works",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "(despite being written in our beloved Rust🦀)",
            Style::default().fg(Color::DarkGray).italic(),
        )),
        Line::from(vec![
            Span::raw("Powered by: "),
            Span::styled(
                "Rust + Ratatui + ffmpeg + ffplay + yt-dlp",
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from("Listen on YouTube • Download • Offline Player (with search) • Visualizer"),
        Line::from(""),
        Line::from(vec![
            Span::raw("Repo: "),
            Span::styled(
                "https://github.com/rezkhaleghi/pj-player",
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("Author: "),
            Span::styled(
                "PocketJack (Reza Khaleghi) (📧rezaxkhaleghi@gmail.com)",
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("Support w/ Doge → "),
            Span::styled(
                "D8ArwStSd9rNmeKX2bUPGsbr9t8E68Ztfo",
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(vec![
            Span::raw("Support w/ Tron → "),
            Span::styled(
                "TA8T2vysRaRVh1dQX6RzMLqSQzXCMtUJwS",
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press ESC or ← to escape this beautiful mess",
            Style::default().fg(Color::DarkGray),
        )),
    ])
}
