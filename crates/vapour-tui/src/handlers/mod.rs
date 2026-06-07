mod auth;
mod chat;
mod friends;
mod game_detail;
mod library;
mod news;
mod search;
mod wishlist;

use crate::app::App;
use crate::event::Key;
use crate::routes::ActiveBlock;

/// Route a key event to the correct handler based on the active block.
pub fn handle_key(app: &mut App, key: Key) {
    if app.protocol_modal_active() {
        auth::handle(app, key);
        return;
    }

    // Escape dismisses error or help from any block
    if app.error.is_some() {
        if key == Key::Esc {
            app.clear_error();
        }
        return;
    }

    if matches!(app.active_block(), ActiveBlock::Help) {
        if matches!(key, Key::Char('?') | Key::Esc | Key::Char('q')) {
            let route = app.navigation_stack.last_mut().expect("never empty");
            route.active_block = ActiveBlock::Library; // return to whatever makes sense
        }
        return;
    }

    match app.active_block().clone() {
        ActiveBlock::Search => search::handle(app, key),
        ActiveBlock::Library => library::handle(app, key),
        ActiveBlock::GameDetail => game_detail::handle(app, key),
        ActiveBlock::Friends => friends::handle(app, key),
        ActiveBlock::Wishlist => wishlist::handle(app, key),
        ActiveBlock::News => news::handle(app, key),
        ActiveBlock::Chat | ActiveBlock::ChatComposer => chat::handle(app, key),
        ActiveBlock::Help | ActiveBlock::Error => {}
    }
}
