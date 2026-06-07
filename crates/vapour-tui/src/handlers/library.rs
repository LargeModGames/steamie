use crate::app::App;
use crate::event::Key;
use crate::io_event::IoEvent;
use crate::routes::{ActiveBlock, Route};

pub fn handle(app: &mut App, key: Key) {
    match key {
        Key::Char('j') | Key::Down => {
            app.pending_g = false;
            let len = app.filtered_games.len();
            App::scroll_down(&mut app.library_state, len);
        }
        Key::Char('k') | Key::Up => {
            app.pending_g = false;
            App::scroll_up(&mut app.library_state);
        }
        Key::Char('g') => {
            if app.pending_g {
                app.pending_g = false;
                App::scroll_top(&mut app.library_state);
            } else {
                app.pending_g = true;
            }
        }
        Key::Char('G') => {
            app.pending_g = false;
            let len = app.filtered_games.len();
            App::scroll_bottom(&mut app.library_state, len);
        }
        Key::Enter => {
            app.pending_g = false;
            if let Some(sel) = app.library_state.selected()
                && let Some(&game_idx) = app.filtered_games.get(sel)
            {
                let appid = app.games[game_idx].appid;
                app.selected_game = None;
                app.achievements.clear();
                app.push_route(Route::game_detail());
                app.dispatch(IoEvent::LoadGameDetail(appid));
                app.dispatch(IoEvent::LoadAchievements(appid));
            }
        }
        Key::Char('/') => {
            app.pending_g = false;
            app.is_searching = true;
            app.search_input.clear();
            let route = app.navigation_stack.last_mut().expect("never empty");
            route.active_block = ActiveBlock::Search;
        }
        Key::Char('r') => {
            app.pending_g = false;
            app.dispatch(IoEvent::LoadLibrary);
        }
        Key::Char('t') => {
            app.pending_g = false;
            app.app_type_filter = app.app_type_filter.cycle();
            app.update_search();
        }
        Key::Char('1') => switch_tab(app, 0),
        Key::Char('2') => switch_tab(app, 1),
        Key::Char('3') => switch_tab(app, 2),
        Key::Char('4') => switch_tab(app, 3),
        Key::Char('?') => {
            app.pending_g = false;
            let route = app.navigation_stack.last_mut().expect("never empty");
            route.active_block = ActiveBlock::Help;
        }
        _ => {
            app.pending_g = false;
        }
    }
}

pub fn switch_tab(app: &mut App, tab: u8) {
    use crate::protocol::ProtocolStatus;

    let route = match tab {
        0 => Route::library(),
        1 => Route::friends(),
        2 => Route::wishlist(),
        3 => Route::news(),
        _ => return,
    };
    let event = route.load_event();
    app.navigation_stack = vec![route];
    if let Some(ev) = event {
        // Skip Web API friend loading when the protocol connection is active.
        if matches!(ev, IoEvent::LoadFriendIds)
            && matches!(app.protocol_status, ProtocolStatus::LoggedOn { .. })
        {
            return;
        }
        app.dispatch(ev);
    }
}
