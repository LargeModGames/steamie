use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use crate::app::App;
use crate::theme::Theme;

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let msg = match &app.error {
        Some(e) => e.as_str(),
        None => return,
    };

    let popup = centered_rect(60, 30, area);
    f.render_widget(Clear, popup);

    let lines = vec![
        Line::from(Span::styled(
            "Error",
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        )),
        Line::default(),
        Line::from(Span::styled(msg, Style::default().fg(theme.fg))),
        Line::default(),
        Line::from(Span::styled(
            "Press Esc to dismiss",
            Style::default().fg(theme.muted),
        )),
    ];

    let popup_widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.error)),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(popup_widget, popup);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vert[1])[1]
}
