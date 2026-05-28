use crate::app::App;
use crate::event::Key;
use crate::routes::ActiveBlock;

pub fn handle(app: &mut App, key: Key) {
    match key {
        Key::Char('j') | Key::Down => {
            let len = app.achievements.len();
            App::scroll_down(&mut app.achievements_state, len);
        }
        Key::Char('k') | Key::Up => {
            App::scroll_up(&mut app.achievements_state);
        }
        Key::Char('g') => {
            if app.pending_g {
                app.pending_g = false;
                App::scroll_top(&mut app.achievements_state);
            } else {
                app.pending_g = true;
            }
        }
        Key::Char('G') => {
            app.pending_g = false;
            let len = app.achievements.len();
            App::scroll_bottom(&mut app.achievements_state, len);
        }
        Key::Esc | Key::Backspace => {
            app.pending_g = false;
            app.pop_route();
        }
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
