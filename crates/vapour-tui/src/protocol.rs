use std::sync::{Arc, Mutex};

use tokio::{
    sync::mpsc,
    time::{Duration, sleep},
};
use vapour_core::{AuthMethod as ConfigAuthMethod, AuthState, Session};
use vapour_protocol::{AuthEvent, AuthMethod, Error as ProtocolError, GuardKind, LoggedOn};

use crate::app::App;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolGuardKind {
    EmailCode,
    DeviceCode,
    DeviceConfirmation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolStatus {
    Disconnected,
    Connecting,
    AwaitingQrScan { qr_url: String },
    AwaitingGuardCode { kind: ProtocolGuardKind },
    LoggedOn { account_name: String },
    Failed(String),
}

impl ProtocolStatus {
    pub fn modal_visible(&self) -> bool {
        matches!(
            self,
            Self::Connecting | Self::AwaitingQrScan { .. } | Self::AwaitingGuardCode { .. }
        )
    }

    pub fn accepts_text_input(&self) -> bool {
        matches!(
            self,
            Self::AwaitingGuardCode {
                kind: ProtocolGuardKind::EmailCode | ProtocolGuardKind::DeviceCode,
            }
        )
    }
}

#[derive(Debug)]
pub enum ProtocolCommand {
    SubmitGuardCode(String),
    Cancel,
}

#[derive(Debug)]
pub struct ProtocolBootstrap {
    pub primary: AuthMethod,
    pub fallback: Option<AuthMethod>,
}

pub fn spawn_protocol_task(
    app: Arc<Mutex<App>>,
    mut session: Session,
    bootstrap: ProtocolBootstrap,
    mut command_rx: mpsc::UnboundedReceiver<ProtocolCommand>,
) {
    tokio::spawn(async move {
        let result = run_protocol_task(&app, &mut session, bootstrap, &mut command_rx).await;
        if let Err(error) = result {
            set_status(&app, ProtocolStatus::Failed(error.to_string()));
        }
    });
}

async fn run_protocol_task(
    app: &Arc<Mutex<App>>,
    session: &mut Session,
    bootstrap: ProtocolBootstrap,
    command_rx: &mut mpsc::UnboundedReceiver<ProtocolCommand>,
) -> anyhow::Result<()> {
    set_status(app, ProtocolStatus::Connecting);

    let logged_on = match drive_auth(app, session, bootstrap.primary.clone(), command_rx).await {
        Ok(logged_on) => logged_on,
        Err(error) => {
            if matches!(bootstrap.primary, AuthMethod::RefreshToken(_)) {
                session.clear_auth()?;
                if let Some(fallback) = bootstrap.fallback {
                    set_status(app, ProtocolStatus::Connecting);
                    drive_auth(app, session, fallback, command_rx).await?
                } else {
                    return Err(error.into());
                }
            } else {
                return Err(error.into());
            }
        }
    };

    session.save_auth(AuthState {
        account_name: logged_on.account_name.clone(),
        refresh_token: logged_on.refresh_token.clone(),
    })?;
    set_status(
        app,
        ProtocolStatus::LoggedOn {
            account_name: logged_on.account_name.clone(),
        },
    );

    session.protocol_client.run().await?;
    Ok(())
}

async fn drive_auth(
    app: &Arc<Mutex<App>>,
    session: &mut Session,
    method: AuthMethod,
    command_rx: &mut mpsc::UnboundedReceiver<ProtocolCommand>,
) -> Result<LoggedOn, ProtocolError> {
    loop {
        let mut events = session.protocol_client.begin_auth(method.clone()).await?;

        while let Some(event) = events.recv().await {
            match event {
                AuthEvent::QrChallenge(qr_url) => {
                    set_status(app, ProtocolStatus::AwaitingQrScan { qr_url });
                }
                AuthEvent::GuardRequired(kind) => {
                    let guard_kind = map_guard_kind(kind.clone());
                    set_status(
                        app,
                        ProtocolStatus::AwaitingGuardCode {
                            kind: guard_kind.clone(),
                        },
                    );

                    if matches!(
                        guard_kind,
                        ProtocolGuardKind::EmailCode | ProtocolGuardKind::DeviceCode
                    ) {
                        match command_rx.recv().await {
                            Some(ProtocolCommand::SubmitGuardCode(code)) => {
                                session.protocol_client.submit_guard_code(code)?;
                                set_status(app, ProtocolStatus::Connecting);
                            }
                            Some(ProtocolCommand::Cancel) => {
                                return Err(ProtocolError::Authentication(
                                    "authentication cancelled".to_owned(),
                                ));
                            }
                            None => {
                                return Err(ProtocolError::Authentication(
                                    "authentication command channel closed".to_owned(),
                                ));
                            }
                        }
                    }
                }
                AuthEvent::Success(logged_on) => return Ok(logged_on),
                AuthEvent::Failure(error) => {
                    if should_retry_auth(&method, &error) {
                        set_status(app, ProtocolStatus::Connecting);
                        sleep(Duration::from_secs(1)).await;
                        break;
                    }
                    return Err(error);
                }
            }
        }

        if should_retry_auth(&method, &ProtocolError::Closed) {
            set_status(app, ProtocolStatus::Connecting);
            sleep(Duration::from_secs(1)).await;
            continue;
        }

        return Err(ProtocolError::Closed);
    }
}

fn set_status(app: &Arc<Mutex<App>>, status: ProtocolStatus) {
    let mut app = app.lock().unwrap();
    app.protocol_status = status;
    if !app.protocol_status.accepts_text_input() {
        app.protocol_input.clear();
    }
}

fn map_guard_kind(kind: GuardKind) -> ProtocolGuardKind {
    match kind {
        GuardKind::EmailCode => ProtocolGuardKind::EmailCode,
        GuardKind::DeviceCode => ProtocolGuardKind::DeviceCode,
        GuardKind::DeviceConfirmation => ProtocolGuardKind::DeviceConfirmation,
    }
}

pub fn build_bootstrap(session: &Session, credentials: Option<(String, String)>) -> ProtocolBootstrap {
    let fallback = match session.preferred_auth_method() {
        ConfigAuthMethod::Qr => Some(AuthMethod::Qr),
        ConfigAuthMethod::Credentials => credentials
            .map(|(account, password)| AuthMethod::Credentials { account, password }),
    };

    if let Some(stored_auth) = session.stored_auth().cloned() {
        ProtocolBootstrap {
            primary: AuthMethod::RefreshToken(stored_auth.refresh_token),
            fallback,
        }
    } else {
        ProtocolBootstrap {
            primary: fallback.unwrap_or(AuthMethod::Qr),
            fallback: None,
        }
    }
}

fn should_retry_auth(method: &AuthMethod, error: &ProtocolError) -> bool {
    matches!(method, AuthMethod::Qr | AuthMethod::Credentials { .. }) && is_closed_error(error)
}

fn is_closed_error(error: &ProtocolError) -> bool {
    match error {
        ProtocolError::Closed => true,
        ProtocolError::Transport(message) => message.contains("closed"),
        ProtocolError::WebSocket(error) => error.to_string().contains("closed"),
        _ => false,
    }
}
