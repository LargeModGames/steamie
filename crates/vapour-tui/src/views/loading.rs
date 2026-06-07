use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::theme::Theme;

pub fn draw(f: &mut Frame, theme: &Theme, area: Rect) {
    let popup = centered_fixed(24, 3, area);
    f.render_widget(Clear, popup);

    let spinner = Paragraph::new(Line::from("  Loading…"))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border_focused)),
        )
        .alignment(Alignment::Left)
        .style(Style::default().fg(theme.muted));

    f.render_widget(spinner, popup);
}

fn centered_fixed(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(r.width),
        height: height.min(r.height),
    }
}
