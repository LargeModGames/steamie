use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
};

use crate::app::App;
use crate::routes::RouteId;
use crate::theme::Theme;

use super::{friends, game_detail, library, news, wishlist};

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
    }

    draw_status_bar(f, app, theme, chunks[2]);

    // Overlays last so they appear on top
    if app.error.is_some() {
        super::error::draw(f, app, theme, area);
    }
    if matches!(app.active_block(), crate::routes::ActiveBlock::Help) {
        super::help::draw(f, theme, area);
    }
    // Game detail loads over existing content, so keep a small overlay for that only
    if app.loading.game_detail && app.selected_game.is_none() {
        super::loading::draw(f, theme, area);
    }
}

fn draw_tabs(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let tab_titles: Vec<Line> = ["1 Library", "2 Friends", "3 Wishlist", "4 News"]
        .iter()
        .map(|t| Line::from(*t))
        .collect();

    let selected = match app.current_route().id {
        RouteId::Library | RouteId::GameDetail => 0,
        RouteId::Friends => 1,
        RouteId::Wishlist => 2,
        RouteId::News => 3,
    };

    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title(" vapour "))
        .select(selected)
        .style(Style::default().fg(theme.tab_inactive))
        .highlight_style(Style::default().fg(theme.tab_active).add_modifier(Modifier::BOLD));

    f.render_widget(tabs, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let hints = if app.is_searching {
        "  Enter confirm  Esc cancel  (type to filter)"
    } else {
        "  ? help  / search  r reload  q quit"
    };

    let spans = vec![Span::styled(hints, Style::default().fg(theme.muted))];
    let bar = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(theme.status_bar_bg));
    f.render_widget(bar, area);
}
