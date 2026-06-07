use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Row, Table, TableState},
};

use crate::app::App;
use crate::theme::Theme;
use crate::views::library::draw_loading;

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    if app.loading.wishlist && app.wishlist.is_empty() {
        return draw_loading(f, theme, area, "Wishlist");
    }
    let rows: Vec<Row> = app
        .wishlist
        .iter()
        .enumerate()
        .map(|(i, item)| {
            Row::new(vec![
                format!("{}", i + 1),
                item.name.clone(),
                if item.added > 0 {
                    format_timestamp(item.added)
                } else {
                    "—".to_owned()
                },
            ])
            .style(Style::default().fg(theme.fg))
        })
        .collect();

    let widths = [
        Constraint::Length(4),
        Constraint::Min(20),
        Constraint::Length(12),
    ];

    let header = Row::new(["#", "Name", "Added"]).style(
        Style::default()
            .fg(theme.muted)
            .add_modifier(Modifier::BOLD),
    );

    let title = format!(" Wishlist ({}) ", app.wishlist.len());

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border_focused))
                .title(title),
        )
        .row_highlight_style(
            Style::default()
                .bg(theme.highlight)
                .fg(theme.highlight_text)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = TableState::default();
    state.select(app.wishlist_state.selected());
    f.render_stateful_widget(table, area, &mut state);
}

fn format_timestamp(ts: u64) -> String {
    let years_since_1970 = ts / 31_557_600;
    format!("{}", 1970 + years_since_1970)
}
