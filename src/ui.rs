use crate::app::App;
use crate::sys::{format_bytes, format_duration_secs, ProcessInfo};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table, Wrap},
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
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let sys = app.sys();

    // CPU Gauge
    let cpu_val = sys.cpu_percent();
    let cpu_gauge = Gauge::default()
        .block(Block::default().title(" CPU ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Yellow))
        .percent(cpu_val as u16);
    f.render_widget(cpu_gauge, chunks[0]);

    // Memory Gauge
    let used = sys.used_mem_bytes();
    let total = sys.total_mem_bytes();
    let mem_percent = if total > 0 {
        (used as f64 / total as f64 * 100.0) as u16
    } else {
        0
    };
    let mem_label = format!("{}/{}", format_bytes(used), format_bytes(total));
    let mem_gauge = Gauge::default()
        .block(Block::default().title(" Memory ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Cyan))
        .percent(mem_percent)
        .label(mem_label);
    f.render_widget(mem_gauge, chunks[1]);
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
    .highlight_style(Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD));

    f.render_stateful_widget(table, area, &mut app.table_state.clone());
}

fn draw_help(f: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" q ", Style::default().bg(Color::DarkGray).fg(Color::White)),
        Span::raw(" quit  "),
        Span::styled(" \u{2191}/\u{2193} ", Style::default().bg(Color::DarkGray).fg(Color::White)),
        Span::raw(" select  "),
        Span::styled(" k ", Style::default().bg(Color::Red).fg(Color::White)),
        Span::raw(" kill process"),
    ]);
    let p = Paragraph::new(line)
        .block(Block::default().borders(Borders::TOP))
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}
