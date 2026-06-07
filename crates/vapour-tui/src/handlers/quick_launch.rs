use crate::app::App;
use crate::event::Key;
use crate::io_event::IoEvent;

/// Keys for the recently-played quick-launch overlay.
pub fn handle(app: &mut App, key: Key) {
    match key {
        Key::Char('j') | Key::Down => {
            let len = app.recently_played_appids.len();
            App::scroll_down(&mut app.quick_launch_state, len);
        }
        Key::Char('k') | Key::Up => {
            App::scroll_up(&mut app.quick_launch_state);
        }
        Key::Enter => {
            if let Some(appid) = app.selected_quick_launch_appid() {
                app.dispatch(IoEvent::LaunchGame(appid));
            }
            app.close_quick_launch();
        }
        Key::Esc | Key::Char('q') | Key::Char('L') => app.close_quick_launch(),
        _ => {}
    }
}
