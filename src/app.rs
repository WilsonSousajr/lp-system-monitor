use crate::sys::{SysCache, ProcessSort};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use std::time::Duration;

#[derive(Clone)]
pub enum PopupState {
    None,
    Help,
    Kill { pid: u32, name: String },
}

pub enum InputMode {
    Normal,
    Editing,
    Popup,
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
    pub popup: PopupState,
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
            popup: PopupState::None,
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
                KeyCode::Char('k') => self.try_kill(), // Open modal
                KeyCode::Char('/') => self.input_mode = InputMode::Editing, // Enter search mode
                KeyCode::Tab | KeyCode::Char('s') => self.toggle_sort(),
                KeyCode::Char('?') => self.open_help(),
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Esc => self.input_mode = InputMode::Normal,
                KeyCode::Enter => self.input_mode = InputMode::Normal,
                KeyCode::Backspace => { self.search_query.pop(); },
                KeyCode::Char(c) => { self.search_query.push(c); },
                _ => {}
            },
            InputMode::Popup => match key.code {
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => self.close_popup(),
                KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => self.confirm_popup(),
                _ => {}
            }
        }
    }

    fn toggle_sort(&mut self) {
        self.sys.sort_by = match self.sys.sort_by {
            ProcessSort::Cpu => ProcessSort::Memory,
            ProcessSort::Memory => ProcessSort::Pid,
            ProcessSort::Pid => ProcessSort::Cpu,
        };
    }

    fn open_help(&mut self) {
        self.popup = PopupState::Help;
        self.input_mode = InputMode::Popup;
    }

    fn try_kill(&mut self) {
        // Get selected PID logic
        // We need to find the actual PID from the filtered list (if searching) or full list.
        // For simplicity reusing the logic from drawing (filtering) might be expensive here?
        // Let's replicate the filter logic or just assume no filter for now? 
        // Actually, we should be consistent. The UI displays filtered list.
        // We will do a quick filter here to find the correct process.
        
        let query = self.search_query.to_lowercase();
        let processes: Vec<_> = self.sys.processes().iter()
            .filter(|p| p.name.to_lowercase().contains(&query) || p.pid.to_string().contains(&query))
            .collect();

        if let Some(i) = self.table_state.selected() {
            if let Some(proc) = processes.get(i) {
                 self.popup = PopupState::Kill { pid: proc.pid, name: proc.name.clone() };
                 self.input_mode = InputMode::Popup;
            }
        }
    }

    fn confirm_popup(&mut self) {
        if let PopupState::Kill { pid, .. } = self.popup {
            self.sys.kill_process(pid);
        }
        self.close_popup();
    }

    fn close_popup(&mut self) {
        self.popup = PopupState::None;
        self.input_mode = InputMode::Normal;
    }

    fn next(&mut self) {
        // Replicating filter for bounds
        let query = self.search_query.to_lowercase();
        let count = self.sys.processes().iter()
            .filter(|p| p.name.to_lowercase().contains(&query) || p.pid.to_string().contains(&query))
            .count();
            
        if count == 0 { return; }

        let i = match self.table_state.selected() {
            Some(i) => if i >= count - 1 { 0 } else { i + 1 },
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn previous(&mut self) {
        let query = self.search_query.to_lowercase();
        let count = self.sys.processes().iter()
            .filter(|p| p.name.to_lowercase().contains(&query) || p.pid.to_string().contains(&query))
            .count();
        
        if count == 0 { return; }

        let i = match self.table_state.selected() {
            Some(i) => if i == 0 { count - 1 } else { i - 1 },
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    // Removing old kill method as it's replaced by try_kill and confirm_popup
    // pub fn kill(&mut self) { ... }

    pub fn request_quit(&mut self) { self.should_quit = true; }
    pub fn should_quit(&self) -> bool { self.should_quit }
    pub fn sys(&self) -> &SysCache { &self.sys }
}
