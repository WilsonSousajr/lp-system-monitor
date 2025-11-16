use std::error::Error;
use std::time::Duration;

mod app;
mod event;
mod sys;
mod ui;

use app::App;
use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use event::{spawn_events, Event as AppEvent};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

fn main() -> Result<(), Box<dyn Error>> {
    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // App and event loop
    let tick_rate = Duration::from_millis(1000);
    let mut app = App::new(tick_rate);
    let rx = spawn_events(tick_rate);

    // Initial refresh so first draw has data
    app.on_tick();

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        match rx.recv() {
            Ok(AppEvent::Tick) => app.on_tick(),
            Ok(AppEvent::Input(key)) => {
                // Also catch Ctrl-C quickly here
                if matches!(
                    key,
                    KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: m,
                        ..
                    } if m.contains(KeyModifiers::CONTROL)
                ) {
                    app.request_quit();
                } else {
                    app.on_key(key);
                }
            }
            Err(_) => break, // sender dropped; exit
        }

        if app.should_quit() {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    // Safety: use backend's writer
    let mut out = io::stdout();
    execute!(out, LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
mod sys;
mod ui;

use crate::sys::App;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    loop {
        terminal.draw(|f| ui::draw::<CrosstermBackend>(f, &app))?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('r') => app.refresh(),
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
