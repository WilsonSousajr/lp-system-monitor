use crate::app::App;
use crate::sys::{format_bytes, format_duration_secs, ProcessInfo};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(size);

    draw_summary(f, chunks[0], app);
    draw_processes(f, chunks[1], app);
    draw_help(f, chunks[2]);
}

fn draw_summary(f: &mut Frame, area: Rect, app: &App) {
    let sys = app.sys();
    let cpu = format!("{:.1}%", sys.cpu_percent());
    let used = format_bytes(sys.used_mem_bytes());
    let total = format_bytes(sys.total_mem_bytes());
    let uptime = format_duration_secs(sys.uptime_secs());

    let text = vec![
        Line::from(vec![
            Span::styled(" CPU: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(cpu),
        ]),
        Line::from(vec![
            Span::styled(" Mem: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(format!("{} / {}", used, total)),
        ]),
        Line::from(vec![
            Span::styled(" Uptime: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(uptime),
        ]),
    ];

    let block = Block::default().title(" System ").borders(Borders::ALL);
    let p = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    f.render_widget(p, area);
}

fn draw_processes(f: &mut Frame, area: Rect, app: &App) {
    let headers = Row::new(vec![
        Cell::from("PID"),
        Cell::from("Name"),
        Cell::from("CPU%"),
        Cell::from("Mem"),
    ])
    .style(Style::default().fg(Color::Gray).add_modifier(Modifier::BOLD));

    let rows = app.sys().processes().iter().map(|p: &ProcessInfo| {
         Row::new(vec![
             Cell::from(p.pid.to_string()),
             Cell::from(p.name.clone()),
             Cell::from(format!("{:.1}", p.cpu)),
             Cell::from(format_bytes(p.mem_bytes)),
         ])
    });

    let table = Table::new(rows, vec![
        Constraint::Length(8),
        Constraint::Percentage(50),
        Constraint::Length(8),
        Constraint::Length(12),
    ])
    .header(headers)
    .block(Block::default().title(" Top Processes ").borders(Borders::ALL))
    .highlight_style(Style::default().bg(Color::Blue).fg(Color::Black));

    f.render_widget(table, area);
}

fn draw_help(f: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" q ", Style::default().bg(Color::DarkGray).fg(Color::White)),
        Span::raw(" quit  "),
        Span::styled(" Ctrl-C ", Style::default().bg(Color::DarkGray).fg(Color::White)),
        Span::raw(" quit  "),
        Span::styled(" Esc ", Style::default().bg(Color::DarkGray).fg(Color::White)),
        Span::raw(" quit"),
    ]);
    let p = Paragraph::new(line)
        .block(Block::default().borders(Borders::TOP))
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}
