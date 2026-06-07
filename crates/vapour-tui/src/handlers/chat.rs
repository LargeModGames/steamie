use crate::app::App;
use crate::event::Key;
use crate::routes::ActiveBlock;

use super::library::switch_tab;

pub fn handle(app: &mut App, key: Key) {
    match app.active_block() {
        ActiveBlock::ChatComposer => handle_composer(app, key),
        // ActiveBlock::Chat — conversation list focused.
        _ => handle_list(app, key),
    }
}

/// Conversation list: navigate conversations, Enter opens one, number keys switch tabs.
fn handle_list(app: &mut App, key: Key) {
    match key {
        Key::Char('j') | Key::Down => {
            app.pending_g = false;
            let len = app.conversations.len();
            App::scroll_down(&mut app.chat_list_state, len);
        }
        Key::Char('k') | Key::Up => {
            app.pending_g = false;
            App::scroll_up(&mut app.chat_list_state);
        }
        Key::Char('g') => {
            if app.pending_g {
                app.pending_g = false;
                App::scroll_top(&mut app.chat_list_state);
            } else {
                app.pending_g = true;
            }
        }
        Key::Char('G') => {
            app.pending_g = false;
            let len = app.conversations.len();
            App::scroll_bottom(&mut app.chat_list_state, len);
        }
        Key::Enter => {
            app.pending_g = false;
            if let Some(sel) = app.chat_list_state.selected()
                && let Some(&steamid) = app.conversation_order().get(sel)
            {
                app.open_conversation(steamid);
            }
        }
        Key::Char('1') => switch_tab(app, 0),
        Key::Char('2') => switch_tab(app, 1),
        Key::Char('3') => switch_tab(app, 2),
        Key::Char('4') => switch_tab(app, 3),
        Key::Char('5') => switch_tab(app, 4),
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

/// Message composer: type to edit, Enter sends, Esc returns to the list, Up/Down scroll history.
fn handle_composer(app: &mut App, key: Key) {
    match key {
        Key::Enter => app.send_chat_message(),
        Key::Esc => {
            let route = app.navigation_stack.last_mut().expect("never empty");
            route.active_block = ActiveBlock::Chat;
        }
        Key::Backspace => {
            app.chat_input.pop();
        }
        Key::Char(c) => {
            app.chat_input.push(c);
            app.notify_typing();
        }
        Key::Up => {
            app.chat_scroll_back = app.chat_scroll_back.saturating_add(1);
        }
        Key::Down => {
            app.chat_scroll_back = app.chat_scroll_back.saturating_sub(1);
        }
        _ => {}
    }
}
