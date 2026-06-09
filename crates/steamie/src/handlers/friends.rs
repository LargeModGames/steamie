use vapour_protocol::{PersonaState, RunCommand};

use crate::app::App;
use crate::event::Key;
use crate::protocol::ProtocolStatus;
use crate::routes::ActiveBlock;

use super::library::switch_tab;

pub fn handle(app: &mut App, key: Key) {
    let protocol_active = matches!(app.protocol_status, ProtocolStatus::LoggedOn { .. });

    match key {
        Key::Char('j') | Key::Down => {
            app.pending_g = false;
            let len = friends_len(app, protocol_active);
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
            let len = friends_len(app, protocol_active);
            App::scroll_bottom(&mut app.friends_state, len);
        }
        Key::Char('r') => {
            app.pending_g = false;
            if !protocol_active {
                app.dispatch(crate::io_event::IoEvent::LoadFriendIds);
            }
        }
        Key::Char('s') if protocol_active => {
            app.pending_g = false;
            cycle_persona_state(app);
        }
        Key::Enter if protocol_active => {
            app.pending_g = false;
            open_chat_with_selected_friend(app);
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

fn friends_len(app: &App, protocol_active: bool) -> usize {
    if protocol_active {
        app.protocol_friends.len()
    } else {
        app.friends.len()
    }
}

/// Open a chat with the currently-selected protocol friend. Uses the same display order as the
/// friends view so the selection index maps to the right SteamID.
fn open_chat_with_selected_friend(app: &mut App) {
    let Some(sel) = app.friends_state.selected() else {
        return;
    };
    let Some(steamid) = app.sorted_protocol_friends().get(sel).map(|p| p.steamid) else {
        return;
    };
    app.open_conversation(steamid);
}

/// Cycle: Online → Away → Invisible → Online.
fn cycle_persona_state(app: &mut App) {
    let Some(tx) = &app.friend_cmd_tx else { return };
    let next = match app.own_persona_state {
        PersonaState::Online => PersonaState::Away,
        PersonaState::Away => PersonaState::Invisible,
        _ => PersonaState::Online,
    };
    app.own_persona_state = next.clone();
    let _ = tx.send(RunCommand::SetPersonaState(next));
}
