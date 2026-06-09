use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use steamie_core::{AuthMethod, Config, Session};

use crate::app::App;
use crate::event::Key;
use crate::event::{Event, Events};
use crate::handlers;
use crate::io_event::IoEvent;
use crate::network;
use crate::protocol;
use crate::theme::Theme;
use crate::views::root;

pub async fn run(config: Config) -> anyhow::Result<()> {
    let theme = Theme::from_name(&config.ui.theme.clone());
    let tick_rate = config.ui.tick_rate_ms;
    let session = Session::new(config.clone())?;
    let credential_prompt = if matches!(session.preferred_auth_method(), AuthMethod::Credentials) {
        Some(prompt_for_credentials(config.auth.account_name.clone())?)
    } else {
        None
    };
    let protocol_bootstrap = protocol::build_bootstrap(&session, credential_prompt);
    let client = Arc::new(session.api_client.clone());

    // Set up channels
    let (io_tx, io_rx) = std::sync::mpsc::channel::<IoEvent>();
    let (protocol_tx, protocol_rx) = tokio::sync::mpsc::unbounded_channel();

    let app = Arc::new(Mutex::new(App::new(io_tx.clone(), protocol_tx, config)));

    // Spawn network dispatch task.
    // spawn_blocking keeps the sync recv off the async executor thread pool.
    let app_net = Arc::clone(&app);
    let client_net = Arc::clone(&client);
    let rt = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        while let Ok(event) = io_rx.recv() {
            let app_clone = Arc::clone(&app_net);
            let client_clone = Arc::clone(&client_net);
            rt.spawn(async move {
                network::handle_io(app_clone, client_clone, event).await;
            });
        }
    });

    protocol::spawn_protocol_task(Arc::clone(&app), session, protocol_bootstrap, protocol_rx);

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let events = Events::new(tick_rate);
    // Block for up to 50ms waiting for an event rather than spinning.
    // This lets tokio tasks run freely instead of fighting the mutex.
    let frame_timeout = Duration::from_millis(50);

    loop {
        {
            let app_lock = app.lock().unwrap();
            terminal.draw(|f| root::draw(f, &app_lock, &theme))?;
        }

        match events.rx.recv_timeout(frame_timeout) {
            Ok(Event::Key(key)) => {
                let mut app_lock = app.lock().unwrap();
                if app_lock.protocol_modal_active() {
                    handlers::handle_key(&mut app_lock, key);
                    continue;
                }

                // In text-input modes (search, chat composer) every printable key must reach the
                // handler — only Ctrl+C still quits — so typing 'q' types a 'q' instead of exiting.
                if app_lock.is_text_input_active() {
                    if key == Key::Ctrl('c') {
                        break;
                    }
                    handlers::handle_key(&mut app_lock, key);
                    continue;
                }

                // A key-capturing overlay (quick-launch) gets every key, so 'q' closes it rather
                // than quitting the app.
                if app_lock.modal_overlay_active() {
                    if key == Key::Ctrl('c') {
                        break;
                    }
                    handlers::handle_key(&mut app_lock, key);
                    continue;
                }

                match key {
                    Key::Char('q') | Key::Ctrl('c') => break,
                    other => handlers::handle_key(&mut app_lock, other),
                }
            }
            Ok(Event::Resize) | Ok(Event::Tick) | Err(_) => {}
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn prompt_for_credentials(account_name_hint: Option<String>) -> anyhow::Result<(String, String)> {
    let account = match account_name_hint {
        Some(account_name) => account_name,
        None => {
            print!("Steam account name: ");
            io::stdout().flush()?;

            let mut account_name = String::new();
            io::stdin().read_line(&mut account_name)?;
            account_name.trim().to_owned()
        }
    };

    let password = rpassword::prompt_password("Steam password: ")?;
    Ok((account, password))
}
