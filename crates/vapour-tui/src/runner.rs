use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use vapour_api::SteamApiClient;
use vapour_core::Config;

use crate::app::App;
use crate::event::{Event, Events};
use crate::event::Key;
use crate::handlers;
use crate::io_event::IoEvent;
use crate::network;
use crate::theme::Theme;
use crate::views::root;

pub async fn run(config: Config) -> anyhow::Result<()> {
    let theme = Theme::from_name(&config.ui.theme.clone());
    let tick_rate = config.ui.tick_rate_ms;
    let client = Arc::new(SteamApiClient::new(
        config.api_key.clone(),
        config.steam_id.clone(),
    ));

    // Set up channels
    let (io_tx, io_rx) = std::sync::mpsc::channel::<IoEvent>();

    let app = Arc::new(Mutex::new(App::new(io_tx.clone(), config)));

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

    // Kick off initial library load
    io_tx.send(IoEvent::LoadLibrary)?;

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
            Ok(Event::Key(Key::Char('q'))) | Ok(Event::Key(Key::Ctrl('c'))) => break,
            Ok(Event::Key(key)) => {
                let mut app_lock = app.lock().unwrap();
                handlers::handle_key(&mut app_lock, key);
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
