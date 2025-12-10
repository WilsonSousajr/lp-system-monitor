use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Terminal events.
#[derive(Clone, Copy, Debug)]
pub enum Event {
    /// Terminal tick.
    Tick,
    /// Key press.
    Input(KeyEvent),
}

/// Helper function to spawn the event loop.
///
/// Returns a receiver that will receive events from the event loop.
pub fn spawn_events(tick_rate: Duration) -> mpsc::Receiver<Event> {
    let (tx, rx) = mpsc::channel();
    
    // Input thread
    let event_tx = tx.clone();
    thread::spawn(move || {
        loop {
            // Poll for events
            if event::poll(tick_rate).unwrap() {
                if let CrosstermEvent::Key(key) = event::read().unwrap() {
                    event_tx.send(Event::Input(key)).unwrap();
                }
            }
            // Send tick event
            event_tx.send(Event::Tick).unwrap();
        }
    });

    rx
}
