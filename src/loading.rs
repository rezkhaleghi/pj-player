use std::time::Instant;

use ratatui::{ layout::Rect, prelude::*, widgets::{ Block, Borders, Paragraph } };

pub struct Loading {
    label: String,
    started_at: Instant,
}

impl Loading {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            started_at: Instant::now(),
        }
    }

    pub fn text(&self) -> String {
        const FRAMES: [&str; 4] = ["[=   ]", "[==  ]", "[=== ]", "[====]"];
        let frame = ((self.started_at.elapsed().as_millis() / 180) as usize) % FRAMES.len();
        format!("{} {}", self.label, FRAMES[frame])
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let width = area.width.min(42);
        let height = area.height.min(3);
        let loading_area = Rect::new(
            area.x + area.width.saturating_sub(width) / 2,
            area.y + area.height.saturating_sub(height) / 2,
            width,
            height
        );

        frame.render_widget(
            Paragraph::new(self.text())
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::White))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Loading")
                        .style(Style::default().fg(Color::LightGreen))
                ),
            loading_area
        );
    }
}
