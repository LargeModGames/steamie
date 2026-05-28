use crate::app::App;
use crate::event::Key;
use crate::routes::ActiveBlock;

use super::library::switch_tab;

pub fn handle(app: &mut App, key: Key) {
    match key {
        Key::Char('j') | Key::Down => {
            app.pending_g = false;
            let len = app.friends.len();
            App::scroll_down(&mut app.friends_state, len);
        }
        Key::Char('k') | Key::Up => {
            app.pending_g = false;
            App::scroll_up(&mut app.friends_state);
        }
        Key::Char('g') => {
            if app.pending_g {
                app.pending_g = false;
                App::scroll_top(&mut app.friends_state);
            } else {
                app.pending_g = true;
            }
        }
        Key::Char('G') => {
            app.pending_g = false;
            let len = app.friends.len();
            App::scroll_bottom(&mut app.friends_state, len);
        }
        Key::Char('r') => {
            app.pending_g = false;
            app.dispatch(crate::io_event::IoEvent::LoadFriendIds);
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
