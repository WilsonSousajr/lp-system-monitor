use crate::sys::SysCache;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

pub struct App {
    sys: SysCache,
    tick_rate: Duration,
    should_quit: bool,
}

impl App {
    pub fn new(tick_rate: Duration) -> Self {
        Self {
            sys: SysCache::new(),
            tick_rate,
            should_quit: false,
        }
    }

    pub fn on_tick(&mut self) {
        self.sys.refresh();
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => self.should_quit = true,
            KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true
            }
            _ => {}
        }
    }

    pub fn request_quit(&mut self) {
        self.should_quit = true;
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn tick_rate(&self) -> Duration {
        self.tick_rate
    }

    pub fn sys(&self) -> &SysCache {
        &self.sys
    }
}
