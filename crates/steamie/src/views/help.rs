use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::theme::Theme;

pub fn draw(f: &mut Frame, theme: &Theme, area: Rect) {
    let popup = centered_rect(60, 80, area);
    f.render_widget(Clear, popup);

    let keybinds: &[(&str, &str)] = &[
        ("j / ↓", "Move down"),
        ("k / ↑", "Move up"),
        ("g g", "Jump to top"),
        ("G", "Jump to bottom"),
        ("Enter", "Open detail · launch in detail"),
        ("l", "Launch highlighted game"),
        ("L", "Recently-played quick launch"),
        ("Esc / Backspace", "Go back"),
        ("/", "Search library"),
        ("t", "Cycle library type filter"),
        ("r", "Reload current view"),
        ("1", "Switch to Library"),
        ("2", "Switch to Friends"),
        ("3", "Switch to Wishlist"),
        ("4", "Switch to News"),
        ("?", "Toggle this help"),
        ("q / Ctrl+c", "Quit"),
    ];

    let rows: Vec<Line> = keybinds
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(
                    format!("  {:20}", key),
                    Style::default()
                        .fg(theme.highlight)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(*desc, Style::default().fg(theme.fg)),
            ])
        })
        .collect();

    let help = Paragraph::new(rows)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.highlight))
                .title(" Keybinds — press ? to close "),
        )
        .alignment(Alignment::Left);

    f.render_widget(help, popup);
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
