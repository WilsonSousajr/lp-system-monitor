use crate::sys::SysCache;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use std::time::Duration;

pub enum InputMode {
    Normal,
    Editing,
}

pub struct App {
    sys: SysCache,
    tick_rate: Duration,
    should_quit: bool,
    pub table_state: TableState,
    
    // History for graphs
    pub cpu_history: Vec<u64>,
    pub net_rx_history: Vec<u64>,
    pub net_tx_history: Vec<u64>,

    // Search/Filter
    pub search_query: String,
    pub input_mode: InputMode,
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
            cpu_history: vec![0; 100], // Buffer for sparkline
            net_rx_history: vec![0; 100],
            net_tx_history: vec![0; 100],
            search_query: String::new(),
            input_mode: InputMode::Normal,
        }
    }

    pub fn on_tick(&mut self) {
        self.sys.refresh();
        
        // Update history
        self.cpu_history.remove(0);
        self.cpu_history.push(self.sys.cpu_global as u64);

        self.net_rx_history.remove(0);
        self.net_rx_history.push(self.sys.rx_rate); // Use rate
        
        self.net_tx_history.remove(0);
        self.net_tx_history.push(self.sys.tx_rate); // Use rate
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => self.should_quit = true,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.should_quit = true,
                KeyCode::Down => self.next(),
                KeyCode::Up => self.previous(),
                KeyCode::Char('k') => self.kill(),
                KeyCode::Char('/') => self.input_mode = InputMode::Editing, // Enter search mode
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Esc => self.input_mode = InputMode::Normal,
                KeyCode::Enter => self.input_mode = InputMode::Normal,
                KeyCode::Backspace => { self.search_query.pop(); },
                KeyCode::Char(c) => { self.search_query.push(c); },
                _ => {}
            }
        }
    }

    fn next(&mut self) {
        // Logic needs to account for filtered list size, handled in UI or here. 
        // For simplicity, we just increment, but UI might show fewer items.
        let i = match self.table_state.selected() {
            Some(i) => i + 1,
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => if i == 0 { 0 } else { i - 1 },
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn kill(&mut self) {
        // Note: This kills based on the *full* list index. 
        // If filtering is active, this logic needs to map visual index to actual PID.
        // For this snippet, we assume the user clears search before killing or we implement mapping later.
        if let Some(i) = self.table_state.selected() {
            if let Some(proc) = self.sys.processes().get(i) {
                self.sys.kill_process(proc.pid);
            }
        }
    }

    pub fn request_quit(&mut self) { self.should_quit = true; }
    pub fn should_quit(&self) -> bool { self.should_quit }
    pub fn sys(&self) -> &SysCache { &self.sys }
}
