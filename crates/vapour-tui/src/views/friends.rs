use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
};

use crate::app::App;
use crate::theme::Theme;
use crate::views::library::draw_loading;

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    if app.loading.friends && app.friends.is_empty() {
        return draw_loading(f, theme, area, "Friends");
    }
    let items: Vec<ListItem> = app
        .friends
        .iter()
        .map(|p| {
            let (dot, color) = if p.is_in_game() {
                ("●", theme.ingame)
            } else if p.personastate > 0 {
                ("●", theme.online)
            } else {
                ("○", theme.offline)
            };

            let mut spans = vec![
                Span::styled(format!("{} ", dot), Style::default().fg(color)),
                Span::styled(&p.personaname, Style::default().fg(theme.fg)),
            ];

            if let Some(game) = &p.gameextrainfo {
                spans.push(Span::styled(
                    format!("  — {}", game),
                    Style::default().fg(theme.ingame),
                ));
            } else if p.personastate > 0 {
                spans.push(Span::styled(
                    format!("  {}", p.persona_state_label()),
                    Style::default().fg(theme.muted),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let online = app.friends.iter().filter(|p| p.personastate > 0).count();
    let total = app.friend_ids.len();
    let loaded = app.friends.len();
    let title = if loaded < total && total > 0 {
        format!(" Friends ({} online / {}/{} loaded) ", online, loaded, total)
    } else {
        format!(" Friends ({} online / {}) ", online, loaded)
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

    let mut state = app.friends_state.clone();
    f.render_stateful_widget(list, area, &mut state);
}
