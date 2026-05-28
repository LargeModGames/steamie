use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event as CEvent};

use super::key::Key;

pub enum Event {
    Key(Key),
    Tick,
    Resize,
}

pub struct Events {
    pub rx: mpsc::Receiver<Event>,
}

impl Events {
    pub fn new(tick_rate_ms: u64) -> Self {
        let (tx, rx) = mpsc::channel();
        let tick_rate = Duration::from_millis(tick_rate_ms);

        thread::spawn(move || loop {
            if event::poll(tick_rate).unwrap_or(false) {
                match event::read() {
                    Ok(CEvent::Key(k)) => {
                        let _ = tx.send(Event::Key(Key::from(k)));
                    }
                    Ok(CEvent::Resize(_, _)) => {
                        let _ = tx.send(Event::Resize);
                    }
                    _ => {}
                }
            } else {
                let _ = tx.send(Event::Tick);
            }
        });

        Self { rx }
    }

}
