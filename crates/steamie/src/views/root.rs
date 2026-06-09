use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
};
use vapour_protocol::PersonaState;

use crate::app::App;
use crate::protocol::{ProtocolGuardKind, ProtocolStatus};
use crate::routes::RouteId;
use crate::theme::Theme;

use super::{chat, friends, game_detail, library, news, wishlist};

pub fn draw(f: &mut Frame, app: &App, theme: &Theme) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tab bar
            Constraint::Min(0),    // content
            Constraint::Length(1), // status bar
        ])
        .split(area);

    draw_tabs(f, app, theme, chunks[0]);

    match app.current_route().id {
        RouteId::Library | RouteId::GameDetail => {
            if app.current_route().id == RouteId::GameDetail {
                game_detail::draw(f, app, theme, chunks[1]);
            } else {
                library::draw(f, app, theme, chunks[1]);
            }
        }
        RouteId::Friends => friends::draw(f, app, theme, chunks[1]),
        RouteId::Wishlist => wishlist::draw(f, app, theme, chunks[1]),
        RouteId::News => news::draw(f, app, theme, chunks[1]),
        RouteId::Chat => chat::draw(f, app, theme, chunks[1]),
    }

    draw_status_bar(f, app, theme, chunks[2]);

    // Overlays last so they appear on top
    if app.error.is_some() {
        super::error::draw(f, app, theme, area);
    }
    if matches!(app.active_block(), crate::routes::ActiveBlock::Help) {
        super::help::draw(f, theme, area);
    }
    if matches!(app.active_block(), crate::routes::ActiveBlock::QuickLaunch) {
        super::quick_launch::draw(f, app, theme, area);
    }
    // Game detail loads over existing content, so keep a small overlay for that only
    if app.loading.game_detail && app.selected_game.is_none() {
        super::loading::draw(f, theme, area);
    }
    if app.protocol_modal_active() {
        super::auth::draw(f, app, theme, area);
    }
}

fn draw_tabs(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let unread = app.total_unread();
    let chat_title = if unread > 0 {
        format!("5 Chat ({unread})")
    } else {
        "5 Chat".to_owned()
    };
    let tab_titles: Vec<Line> = [
        "1 Library".to_owned(),
        "2 Friends".to_owned(),
        "3 Wishlist".to_owned(),
        "4 News".to_owned(),
        chat_title,
    ]
    .into_iter()
    .map(Line::from)
    .collect();

    let selected = match app.current_route().id {
        RouteId::Library | RouteId::GameDetail => 0,
        RouteId::Friends => 1,
        RouteId::Wishlist => 2,
        RouteId::News => 3,
        RouteId::Chat => 4,
    };

    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title(" steamie "))
        .select(selected)
        .style(Style::default().fg(theme.tab_inactive))
        .highlight_style(
            Style::default()
                .fg(theme.tab_active)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let protocol = match &app.protocol_status {
        ProtocolStatus::Disconnected => "○ Web API only".to_owned(),
        ProtocolStatus::Connecting => "◌ Connecting to Steam".to_owned(),
        ProtocolStatus::AwaitingQrScan { .. } => "◎ Scan Steam QR".to_owned(),
        ProtocolStatus::AwaitingGuardCode { kind } => match kind {
            ProtocolGuardKind::EmailCode => "◎ Awaiting email code".to_owned(),
            ProtocolGuardKind::DeviceCode => "◎ Awaiting Steam Guard code".to_owned(),
            ProtocolGuardKind::DeviceConfirmation => "◎ Approve in Steam app".to_owned(),
        },
        ProtocolStatus::LoggedOn { account_name } => {
            let state_label = match app.own_persona_state {
                PersonaState::Online => "",
                PersonaState::Away => " [Away]",
                PersonaState::Invisible => " [Invisible]",
                PersonaState::Busy => " [Busy]",
                _ => "",
            };
            format!("● Connected as {account_name}{state_label}")
        }
        ProtocolStatus::Failed(message) => format!("○ Web API only ({message})"),
    };
    let hints = if app.protocol_modal_active() {
        if app.protocol_status.accepts_text_input() {
            "  Enter submit  Esc cancel"
        } else {
            "  Esc cancel"
        }
    } else if app.is_searching {
        "  Enter confirm  Esc cancel  (type to filter)"
    } else if matches!(app.current_route().id, RouteId::Chat) {
        if matches!(app.active_block(), crate::routes::ActiveBlock::ChatComposer) {
            "  Enter send  Esc back  ↑↓ history"
        } else {
            "  Enter open  j/k move  1-5 tabs  q quit"
        }
    } else if matches!(app.current_route().id, RouteId::Friends)
        && matches!(app.protocol_status, ProtocolStatus::LoggedOn { .. })
    {
        "  Enter chat  s status  ? help  r reload  q quit"
    } else if matches!(app.current_route().id, RouteId::GameDetail) {
        "  Enter/l launch  Esc back  ? help  q quit"
    } else if matches!(app.current_route().id, RouteId::Library) {
        "  l launch  L recent  t type  / search  ? help  q quit"
    } else {
        "  ? help  / search  r reload  q quit"
    };

    // A fresh launch message takes over the hint slot briefly.
    let spans = if let Some(msg) = app.launch_status_message() {
        vec![
            Span::styled(protocol, Style::default().fg(theme.fg)),
            Span::raw("  "),
            Span::styled(
                msg.to_owned(),
                Style::default()
                    .fg(theme.tab_active)
                    .add_modifier(Modifier::BOLD),
            ),
        ]
    } else {
        vec![
            Span::styled(protocol, Style::default().fg(theme.fg)),
            Span::raw("  "),
            Span::styled(hints, Style::default().fg(theme.muted)),
        ]
    };
    let bar = Paragraph::new(Line::from(spans)).style(Style::default().bg(theme.status_bar_bg));
    f.render_widget(bar, area);
}
