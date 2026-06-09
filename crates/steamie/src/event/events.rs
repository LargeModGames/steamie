use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event as CEvent, KeyEventKind};

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

        thread::spawn(move || {
            loop {
                if event::poll(tick_rate).unwrap_or(false) {
                    match event::read() {
                        // On Windows, crossterm emits a KeyEvent for both press and
                        // release; forwarding both moves list selections twice per
                        // keypress. Only act on presses (and OS key-repeats).
                        Ok(CEvent::Key(k)) if k.kind == KeyEventKind::Press => {
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
            }
        });

        Self { rx }
    }
}
