use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};

use crate::app::App;
use crate::theme::Theme;

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let popup = centered_rect(60, 60, area);
    f.render_widget(Clear, popup);

    let entries = app.quick_launch_entries();

    if entries.is_empty() {
        let empty = Paragraph::new("  No recently-played games yet.")
            .block(block(theme))
            .style(Style::default().fg(theme.muted));
        f.render_widget(empty, popup);
        return;
    }

    let items: Vec<ListItem> = entries
        .iter()
        .map(|(appid, name)| {
            ListItem::new(Line::from(vec![
                Span::styled(name.clone(), Style::default().fg(theme.fg)),
                Span::styled(format!("  ({appid})"), Style::default().fg(theme.muted)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(block(theme))
        .highlight_style(
            Style::default()
                .bg(theme.highlight)
                .fg(theme.highlight_text)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = app.quick_launch_state;
    f.render_stateful_widget(list, popup, &mut state);
}

fn block(theme: &Theme) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.highlight))
        .title(" Quick Launch — Enter launch · Esc close ")
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
