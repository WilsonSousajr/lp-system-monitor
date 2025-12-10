use crossterm::event::{self, Event as CEvent, KeyEvent, KeyEventKind};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Input(KeyEvent),
    Tick,
}

pub fn spawn_events(tick_rate: Duration) -> Receiver<Event> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || loop {
        if event::poll(tick_rate).unwrap_or(false) {
            if let Ok(CEvent::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press {
                    let _ = tx.send(Event::Input(key));
                }
            }
        }
        let _ = tx.send(Event::Tick);
    });
    rx
}
