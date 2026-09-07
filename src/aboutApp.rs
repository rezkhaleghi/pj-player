use ratatui::prelude::*;

pub fn content() -> Text<'static> {
    Text::from(vec![
        Line::from(Span::styled(
            "PJ-Player",
            Style::default().fg(Color::LightGreen),
        )),
        Line::from("A terminal music player for streaming, downloads, and local audio."),
        Line::from(""),
        Line::from("Features: YouTube search, Internet Archive downloads, offline folders."),
        Line::from("Repository: https://github.com/rezkhaleghi/pj-player"),
        Line::from("Author: PocketJack (Rez Khaleghi)"),
        Line::from("Built with Rust, Ratatui, ffmpeg, ffplay, and yt-dlp."),
        Line::from(""),
        Line::from("Press ESC or LEFT to return."),
    ])
}
