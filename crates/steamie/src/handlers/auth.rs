use crate::app::App;
use crate::event::Key;
use crate::protocol::{ProtocolCommand, ProtocolGuardKind, ProtocolStatus};

pub fn handle(app: &mut App, key: Key) {
    match &app.protocol_status {
        ProtocolStatus::AwaitingGuardCode {
            kind: ProtocolGuardKind::EmailCode | ProtocolGuardKind::DeviceCode,
        } => match key {
            Key::Enter => app.submit_guard_code(),
            Key::Backspace => {
                app.protocol_input.pop();
            }
            Key::Esc => {
                let _ = app.protocol_tx.send(ProtocolCommand::Cancel);
                app.protocol_status = ProtocolStatus::Disconnected;
                app.protocol_input.clear();
            }
            Key::Char(c) if !c.is_control() => {
                app.protocol_input.push(c);
            }
            _ => {}
        },
        ProtocolStatus::AwaitingGuardCode {
            kind: ProtocolGuardKind::DeviceConfirmation,
        } => {
            if key == Key::Esc {
                let _ = app.protocol_tx.send(ProtocolCommand::Cancel);
                app.protocol_status = ProtocolStatus::Disconnected;
            }
        }
        ProtocolStatus::AwaitingQrScan { .. } | ProtocolStatus::Connecting if key == Key::Esc => {
            let _ = app.protocol_tx.send(ProtocolCommand::Cancel);
            app.protocol_status = ProtocolStatus::Disconnected;
        }
        _ => {}
    }
}
