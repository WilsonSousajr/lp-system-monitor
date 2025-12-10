use crate::sys::SysCache;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::TableState;
use std::time::Duration;

#[derive(PartialEq, Debug)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SortColumn {
    Pid,
    Name,
    User,
    Cpu,
    Mem,
}

pub struct App {
    sys: SysCache,
    _tick_rate: Duration,
    should_quit: bool,
    pub table_state: TableState,

    pub cpu_history: Vec<u64>,
    pub net_rx_history: Vec<u64>,
    pub net_tx_history: Vec<u64>,

    pub search_query: String,
    pub input_mode: InputMode,

    pub sort_col: SortColumn,
    pub sort_desc: bool,
    pub _tree_view: bool,
}

impl App {
    pub fn new(tick_rate: Duration) -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        Self {
            sys: SysCache::new(),
            _tick_rate: tick_rate,
            should_quit: false,
            table_state,
            cpu_history: vec![0; 100],
            net_rx_history: vec![0; 100],
            net_tx_history: vec![0; 100],
            search_query: String::new(),
            input_mode: InputMode::Normal,
            sort_col: SortColumn::Cpu,
            sort_desc: true,
            _tree_view: false,
        }
    }

    pub fn on_tick(&mut self) {
        self.sys.refresh();

        self.cpu_history.remove(0);
        self.cpu_history.push(self.sys.cpu_global as u64);

        self.net_rx_history.remove(0);
        self.net_rx_history.push(self.sys.rx_rate);

        self.net_tx_history.remove(0);
        self.net_tx_history.push(self.sys.tx_rate);
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::F(10) => self.should_quit = true,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.should_quit = true
                }
                KeyCode::Down | KeyCode::Char('n') => self.next(),
                KeyCode::Up | KeyCode::Char('p') => self.previous(),
                KeyCode::Char('k') | KeyCode::F(9) => self.kill(),
                KeyCode::Char('/') | KeyCode::F(3) => self.input_mode = InputMode::Editing,
                KeyCode::F(6) => {
                    self.cycle_sort();
                }
                KeyCode::Char('I') => {
                    self.sort_desc = !self.sort_desc;
                }
                KeyCode::Tab => {
                    self.cycle_sort();
                }
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Esc => self.input_mode = InputMode::Normal,
                KeyCode::Enter => self.input_mode = InputMode::Normal,
                KeyCode::Backspace => {
                    self.search_query.pop();
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                }
                _ => {}
            },
        }
    }

    fn cycle_sort(&mut self) {
        self.sort_col = match self.sort_col {
            SortColumn::Pid => SortColumn::Name,
            SortColumn::Name => SortColumn::User,
            SortColumn::User => SortColumn::Cpu,
            SortColumn::Cpu => SortColumn::Mem,
            SortColumn::Mem => SortColumn::Pid,
        };
    }

    fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => i + 1,
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    0
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
    pub fn sys(&self) -> &SysCache {
        &self.sys
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_app_new() {
        let app = App::new(Duration::from_millis(100));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(!app.should_quit());
        assert_eq!(app.sort_col, SortColumn::Cpu);
    }

    #[test]
    fn test_cycle_sort() {
        let mut app = App::new(Duration::from_millis(100));
        assert_eq!(app.sort_col, SortColumn::Cpu); // Initial
        app.cycle_sort();
        assert_eq!(app.sort_col, SortColumn::Mem);
        app.cycle_sort();
        assert_eq!(app.sort_col, SortColumn::Pid);
    }

    #[test]
    fn test_input_mode_switching() {
        let mut app = App::new(Duration::from_millis(100));
        
        let slash = KeyEvent{ code: KeyCode::Char('/'), modifiers: KeyModifiers::empty(), kind: crossterm::event::KeyEventKind::Press, state: crossterm::event::KeyEventState::NONE };
        app.on_key(slash);
        assert_eq!(app.input_mode, InputMode::Editing);

        let char_a = KeyEvent{ code: KeyCode::Char('a'), modifiers: KeyModifiers::empty(), kind: crossterm::event::KeyEventKind::Press, state: crossterm::event::KeyEventState::NONE };
        app.on_key(char_a);
        assert_eq!(app.search_query, "a");

        let esc = KeyEvent{ code: KeyCode::Esc, modifiers: KeyModifiers::empty(), kind: crossterm::event::KeyEventKind::Press, state: crossterm::event::KeyEventState::NONE };
        app.on_key(esc);
        assert_eq!(app.input_mode, InputMode::Normal);
    }
}
