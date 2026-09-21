use std::time::Instant;

use ratatui::{
    layout::Rect,
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

pub struct Loading {
    started_at: Instant,
}

impl Loading {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
        }
    }

    fn bar(&self, width: usize) -> String {
        let unit = "▗▄▄▖";
        let units = width / unit.chars().count();
        let progress = ((self.started_at.elapsed().as_millis() / 120) as usize) % (units + 1);
        unit.repeat(progress)
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let main_block = Block::default()
            .borders(Borders::ALL)
            .title("PJ-Player")
            .style(Style::default().fg(Color::LightGreen));
        frame.render_widget(main_block, area);

        let width = area.width.saturating_sub(4).min(42).max(8);
        let height = area.height.saturating_sub(4).min(3).max(3);
        let loading_area = Rect::new(
            area.x + area.width.saturating_sub(width) / 2,
            area.y + area.height.saturating_sub(height) / 2,
            width,
            height,
        );
        let bar_width = loading_area.width.saturating_sub(2) as usize;

        frame.render_widget(
            Paragraph::new(self.bar(bar_width))
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::White))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Loading")
                        .style(Style::default().fg(Color::LightGreen)),
                ),
            loading_area,
        );
    }
}
