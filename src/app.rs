use crate::sys::SysCache;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use std::time::Duration;

pub struct App {
    sys: SysCache,
    tick_rate: Duration,
    should_quit: bool,
    pub table_state: TableState,
}

impl App {
    pub fn new(tick_rate: Duration) -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        Self {
            sys: SysCache::new(),
            tick_rate,
            should_quit: false,
            table_state,
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
            KeyCode::Down => self.next(),
            KeyCode::Up => self.previous(),
            KeyCode::Char('k') => self.kill(),
            _ => {}
        }
    }

    fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.sys.processes().len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.sys.processes().len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn kill(&mut self) {
        if let Some(i) = self.table_state.selected() {
            if let Some(proc) = self.sys.processes().get(i) {
                self.sys.kill_process(proc.pid);
            }
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
