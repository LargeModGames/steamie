use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph, Wrap},
};

use crate::app::App;
use crate::theme::Theme;

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    draw_info_panel(f, app, theme, chunks[0]);
    draw_achievements_panel(f, app, theme, chunks[1]);
}

fn draw_info_panel(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    // --- Game info ---
    let mut lines: Vec<Line> = vec![];

    if let Some(details) = &app.selected_game {
        lines.push(Line::from(Span::styled(
            &details.name,
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::default());

        if let Some(devs) = &details.developers {
            lines.push(Line::from(vec![
                Span::styled("Developer  ", Style::default().fg(theme.muted)),
                Span::styled(devs.join(", "), Style::default().fg(theme.fg)),
            ]));
        }
        if let Some(pubs) = &details.publishers {
            lines.push(Line::from(vec![
                Span::styled("Publisher  ", Style::default().fg(theme.muted)),
                Span::styled(pubs.join(", "), Style::default().fg(theme.fg)),
            ]));
        }
        if let Some(rd) = &details.release_date {
            lines.push(Line::from(vec![
                Span::styled("Released   ", Style::default().fg(theme.muted)),
                Span::styled(&rd.date, Style::default().fg(theme.fg)),
            ]));
        }
        if let Some(mc) = &details.metacritic {
            let color = if mc.score >= 75 {
                theme.online
            } else if mc.score >= 50 {
                theme.ingame
            } else {
                theme.error
            };
            lines.push(Line::from(vec![
                Span::styled("Metacritic ", Style::default().fg(theme.muted)),
                Span::styled(mc.score.to_string(), Style::default().fg(color).add_modifier(Modifier::BOLD)),
            ]));
        }
        if let Some(genres) = &details.genres {
            let g: Vec<&str> = genres.iter().map(|g| g.description.as_str()).collect();
            lines.push(Line::from(vec![
                Span::styled("Genres     ", Style::default().fg(theme.muted)),
                Span::styled(g.join(", "), Style::default().fg(theme.fg)),
            ]));
        }

        lines.push(Line::default());

        if let Some(desc) = &details.short_description {
            for chunk in textwrap_simple(desc, area.width.saturating_sub(4) as usize) {
                lines.push(Line::from(Span::styled(chunk, Style::default().fg(theme.muted))));
            }
        }
    } else {
        lines.push(Line::from(Span::styled("Loading…", Style::default().fg(theme.muted))));
    }

    let info = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" Game Info "),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(info, rows[0]);

    // --- Achievement progress bar ---
    let total = app.achievements.len();
    let unlocked = app.achievements.iter().filter(|a| a.is_unlocked()).count();
    let ratio = if total > 0 { unlocked as f64 / total as f64 } else { 0.0 };
    let label = format!("{}/{} achievements", unlocked, total);

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.border)))
        .gauge_style(Style::default().fg(theme.highlight))
        .ratio(ratio)
        .label(label);
    f.render_widget(gauge, rows[1]);
}

fn draw_achievements_panel(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let items: Vec<ListItem> = app
        .achievements
        .iter()
        .map(|a| {
            let (icon, color) = if a.is_unlocked() {
                ("✓", theme.online)
            } else {
                ("✗", theme.offline)
            };
            let line = Line::from(vec![
                Span::styled(format!("{} ", icon), Style::default().fg(color)),
                Span::styled(a.display_name(), Style::default().fg(theme.fg)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let title = format!(" Achievements ({}) ", app.achievements.len());
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(title),
        )
        .highlight_style(Style::default().bg(theme.highlight).fg(theme.highlight_text))
        .highlight_symbol("> ");

    let mut state = app.achievements_state.clone();
    f.render_stateful_widget(list, area, &mut state);
}

/// Naive word wrap — splits on spaces to fit within `width` chars.
fn textwrap_simple(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_owned()];
    }
    let mut lines = vec![];
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current.clone());
            current = word.to_owned();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
