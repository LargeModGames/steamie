use std::collections::HashMap;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem},
};
use steam_cm_protocol::{Persona, PersonaState};
use steamie_api::Game;

use crate::app::App;
use crate::protocol::ProtocolStatus;
use crate::theme::Theme;
use crate::views::library::draw_loading;

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    if matches!(app.protocol_status, ProtocolStatus::LoggedOn { .. }) {
        draw_protocol_friends(f, app, theme, area);
    } else {
        draw_web_api_friends(f, app, theme, area);
    }
}

fn draw_protocol_friends(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    // Shared with the "open chat" handler so the selection index maps to the same friend.
    let friends = app.sorted_protocol_friends();

    let online = friends
        .iter()
        .filter(|p| p.state != PersonaState::Offline && p.state != PersonaState::Invisible)
        .count();

    let items: Vec<ListItem> = friends
        .iter()
        .map(|p| ListItem::new(persona_line(p, theme, &app.games, &app.game_name_cache)))
        .collect();

    let title = format!(" Friends ({} online / {}) ", online, friends.len());
    render_list(
        f,
        items,
        &title,
        theme,
        area,
        &mut app.friends_state.clone(),
    );
}

fn draw_web_api_friends(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
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
        format!(
            " Friends ({} online / {}/{} loaded) ",
            online, loaded, total
        )
    } else {
        format!(" Friends ({} online / {}) ", online, loaded)
    };

    render_list(
        f,
        items,
        &title,
        theme,
        area,
        &mut app.friends_state.clone(),
    );
}

fn persona_line<'a>(
    p: &'a Persona,
    theme: &Theme,
    games: &[Game],
    cache: &HashMap<u32, String>,
) -> Line<'a> {
    let (dot, dot_color) = match &p.state {
        PersonaState::Offline | PersonaState::Invisible => ("○", theme.offline),
        _ if p.game_app_id.is_some() => ("●", theme.ingame),
        _ => ("●", theme.online),
    };

    let state_label = match &p.state {
        PersonaState::Online => "",
        PersonaState::Busy => "Busy",
        PersonaState::Away => "Away",
        PersonaState::Snooze => "Snooze",
        PersonaState::LookingToTrade => "Looking to Trade",
        PersonaState::LookingToPlay => "Looking to Play",
        PersonaState::Invisible => "Invisible",
        PersonaState::Offline => "Offline",
    };

    let mut spans = vec![
        Span::styled(format!("{} ", dot), Style::default().fg(dot_color)),
        Span::styled(p.name.clone(), Style::default().fg(theme.fg)),
    ];

    if let Some(app_id) = p.game_app_id {
        // Steam doesn't send game_name for catalogue games; look it up from
        // the user's library, then the async-fetched cache (for games they
        // don't own), then the persona field (non-Steam games).
        let cache_name = cache
            .get(&app_id)
            .filter(|s| !s.is_empty())
            .map(|s| s.as_str());
        let name = games
            .iter()
            .find(|g| g.appid == app_id)
            .and_then(|g| g.name.as_deref())
            .or(cache_name)
            .or(p.game_name.as_deref())
            .unwrap_or("In-Game");
        spans.push(Span::styled(
            format!("  — {}", name),
            Style::default().fg(theme.ingame),
        ));
    } else if !state_label.is_empty() {
        spans.push(Span::styled(
            format!("  {}", state_label),
            Style::default().fg(theme.muted),
        ));
    }

    Line::from(spans)
}

fn render_list(
    f: &mut Frame,
    items: Vec<ListItem>,
    title: &str,
    theme: &Theme,
    area: Rect,
    state: &mut ratatui::widgets::ListState,
) {
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border_focused))
                .title(title.to_owned()),
        )
        .highlight_style(
            Style::default()
                .bg(theme.highlight)
                .fg(theme.highlight_text)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    f.render_stateful_widget(list, area, state);
}
