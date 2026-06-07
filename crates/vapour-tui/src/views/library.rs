use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
};

use crate::app::App;
use crate::theme::Theme;

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    if app.loading.library && app.games.is_empty() {
        return draw_loading(f, theme, area, "Library");
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if app.is_searching { 3 } else { 0 }),
            Constraint::Min(0),
        ])
        .split(area);

    if app.is_searching {
        let search = Paragraph::new(format!("/{}", app.search_input))
            .block(Block::default().borders(Borders::ALL).title("Search"))
            .style(Style::default().fg(theme.fg));
        f.render_widget(search, chunks[0]);
    }

    let content_area = if app.is_searching { chunks[1] } else { area };

    let games = app.visible_games();
    let items: Vec<ListItem> = games
        .iter()
        .map(|g| {
            let hours = g.playtime_hours();
            let playtime = if hours < 1.0 {
                format!("{} min", g.playtime_forever)
            } else {
                format!("{:.1} hrs", hours)
            };
            let line = Line::from(vec![
                Span::styled(app.game_display_name(g), Style::default().fg(theme.fg)),
                Span::styled(format!("  {}", playtime), Style::default().fg(theme.muted)),
            ]);
            ListItem::new(line)
        })
        .collect();

    use crate::app::AppTypeFilter;
    let type_tag = if app.app_type_filter == AppTypeFilter::All {
        String::new()
    } else {
        format!(" [{}]", app.app_type_filter.label())
    };
    let title = if app.is_searching || app.app_type_filter != AppTypeFilter::All {
        format!(
            " Library ({}/{}){} ",
            games.len(),
            app.games.len(),
            type_tag
        )
    } else {
        format!(" Library ({}) ", app.games.len())
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border_focused))
                .title(title),
        )
        .highlight_style(
            Style::default()
                .bg(theme.highlight)
                .fg(theme.highlight_text)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = app.library_state;
    f.render_stateful_widget(list, content_area, &mut state);
}

pub fn draw_loading(f: &mut Frame, theme: &Theme, area: Rect, label: &str) {
    let msg = Paragraph::new(format!("  Loading {}…  press r to retry", label))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border)),
        )
        .style(Style::default().fg(theme.muted));
    f.render_widget(msg, area);
}
