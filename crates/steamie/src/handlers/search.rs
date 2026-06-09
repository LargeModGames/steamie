use crate::app::App;
use crate::event::Key;
use crate::routes::ActiveBlock;

pub fn handle(app: &mut App, key: Key) {
    match key {
        Key::Esc => {
            app.is_searching = false;
            app.search_input.clear();
            app.update_search();
            let route = app.navigation_stack.last_mut().expect("never empty");
            route.active_block = ActiveBlock::Library;
        }
        Key::Enter => {
            app.is_searching = false;
            let route = app.navigation_stack.last_mut().expect("never empty");
            route.active_block = ActiveBlock::Library;
        }
        Key::Backspace => {
            app.search_input.pop();
            app.update_search();
        }
        Key::Char(c) => {
            app.search_input.push(c);
            app.update_search();
        }
        _ => {}
    }
}
