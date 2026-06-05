use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
};

use crate::app::App;
use crate::theme::Theme;
use crate::views::library::draw_loading;

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    if app.loading.news && app.news_feed.is_empty() {
        return draw_loading(f, theme, area, "News");
    }
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // --- News list ---
    let items: Vec<ListItem> = app
        .news_feed
        .iter()
        .map(|n| {
            let line = Line::from(vec![
                Span::styled(&n.title, Style::default().fg(theme.fg)),
                Span::styled(
                    format!("  {}", &n.feedlabel),
                    Style::default().fg(theme.muted),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let title = format!(" News ({}) ", app.news_feed.len());
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

    let mut state = app.news_state;
    f.render_stateful_widget(list, chunks[0], &mut state);

    // --- Preview pane ---
    let preview_text = app
        .news_state
        .selected()
        .and_then(|i| app.news_feed.get(i))
        .map(|n| {
            let mut lines: Vec<Line> = vec![
                Line::from(Span::styled(
                    &n.title,
                    Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(&n.feedlabel, Style::default().fg(theme.muted))),
                Line::default(),
            ];
            if let Some(contents) = &n.contents {
                // Strip basic HTML tags for preview
                let plain = strip_html(contents);
                lines.push(Line::from(Span::styled(
                    plain,
                    Style::default().fg(theme.fg),
                )));
            }
            lines
        })
        .unwrap_or_default();

    let preview = Paragraph::new(preview_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" Preview "),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(preview, chunks[1]);
}

fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}
