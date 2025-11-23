use crate::app::{App, InputMode};
use crate::sys::{format_bytes, format_duration_secs};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Gauge, Paragraph, Row, Sparkline, Table, Wrap,
    },
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.size();
    
    // 1. Top (CPU) vs Bottom
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30), // CPU Module
            Constraint::Percentage(70), // Bottom Modules
        ])
        .split(size);

    // 2. Bottom Split: Left (Mem/Net), Center (Disk), Right (Procs)
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25), // Left
            Constraint::Percentage(25), // Center
            Constraint::Percentage(50), // Right
        ])
        .split(main_chunks[1]);

    // 3. Left Split: Memory vs Network
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(bottom_chunks[0]);

    draw_cpu_module(f, main_chunks[0], app);
    draw_memory_module(f, left_chunks[0], app);
    draw_network_module(f, left_chunks[1], app);
    draw_disk_module(f, bottom_chunks[1], app);
    draw_processes_module(f, bottom_chunks[2], app);
}

fn get_color(percent: f32) -> Color {
    if percent < 50.0 { Color::Green }
    else if percent < 80.0 { Color::Yellow }
    else { Color::Red }
}

fn draw_cpu_module(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta))
        .title(format!(" CPU: {} | Uptime: {} ", app.sys().cpu_model, format_duration_secs(app.sys().uptime)));
    
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(inner_area);

    // History Graph
    let sparkline = Sparkline::default()
        .block(Block::default().title("History").borders(Borders::RIGHT))
        .data(&app.cpu_history)
        .style(Style::default().fg(Color::Green));
    f.render_widget(sparkline, chunks[0]);

    // Cores
    let cores = &app.sys().cpu_cores;
    // Simple visualization of first 8 cores to fit
    let core_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1); cores.len().min(8)])
        .split(chunks[1]);

    for (i, &usage) in cores.iter().take(8).enumerate() {
        let label = format!("C{:02}", i);
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(get_color(usage)))
            .label(label)
            .percent(usage as u16);
        f.render_widget(gauge, core_chunks[i]);
    }
}

fn draw_memory_module(f: &mut Frame, area: Rect, app: &App) {
    let sys = app.sys();
    let percent = (sys.used_mem as f64 / sys.total_mem as f64 * 100.0) as u16;
    
    let gauge = Gauge::default()
        .block(Block::default().title(" Memory ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Cyan)))
        .gauge_style(Style::default().fg(get_color(percent as f32)))
        .percent(percent)
        .label(format!("{}/{}", format_bytes(sys.used_mem), format_bytes(sys.total_mem)));
    
    f.render_widget(gauge, area);
}

fn draw_network_module(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Network ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));
    
    let inner = block.inner(area);
    f.render_widget(block, area);
    
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(50), Constraint::Percentage(50)]).split(inner);

    let rx_spark = Sparkline::default().data(&app.net_rx_history).style(Style::default().fg(Color::Green)).bar_set(ratatui::symbols::bar::NINE_LEVELS);
    let tx_spark = Sparkline::default().data(&app.net_tx_history).style(Style::default().fg(Color::Red)).bar_set(ratatui::symbols::bar::NINE_LEVELS);

    f.render_widget(rx_spark, chunks[0]);
    f.render_widget(tx_spark, chunks[1]);
}

fn draw_disk_module(f: &mut Frame, area: Rect, app: &App) {
    let disks = app.sys().disks();
    let block = Block::default().title(" Disks ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Blue));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let constraints = vec![Constraint::Length(2); disks.len().min(5)];
    let chunks = Layout::default().direction(Direction::Vertical).constraints(constraints).split(inner);

    for (i, disk) in disks.iter().take(5).enumerate() {
        let used = disk.total - disk.available;
        let percent = (used as f64 / disk.total as f64 * 100.0) as u16;
        let g = Gauge::default()
            .percent(percent)
            .label(format!("{} {}", disk.mount_point, format_bytes(used)))
            .gauge_style(Style::default().fg(get_color(percent as f32)));
        f.render_widget(g, chunks[i]);
    }
}

fn draw_processes_module(f: &mut Frame, area: Rect, app: &App) {
    let title = match app.input_mode {
        InputMode::Normal => format!(" Processes (Press '/' to search) "),
        InputMode::Editing => format!(" Search: {}_ ", app.search_query),
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::White));

    // Filter processes
    let query = app.search_query.to_lowercase();
    let processes: Vec<_> = app.sys().processes().iter()
        .filter(|p| p.name.to_lowercase().contains(&query) || p.pid.to_string().contains(&query))
        .collect();

    let rows = processes.iter().map(|p| {
        Row::new(vec![
            Cell::from(p.pid.to_string()),
            Cell::from(p.name.clone()),
            Cell::from(format!("{:.1}%", p.cpu)),
            Cell::from(format_bytes(p.mem_bytes)),
        ])
    });

    let table = Table::new(rows, vec![
        Constraint::Length(6),
        Constraint::Percentage(40),
        Constraint::Length(10),
        Constraint::Length(10),
    ])
    .header(Row::new(vec!["PID", "Name", "CPU", "Mem"]).style(Style::default().add_modifier(Modifier::BOLD)))
    .block(block)
    .highlight_style(Style::default().bg(Color::White).fg(Color::Black));

    // Note: We are passing the raw table state. 
    // If the filtered list is shorter than the selected index, it might look weird, 
    // but Ratatui handles out-of-bounds gracefully usually.
    f.render_stateful_widget(table, area, &mut app.table_state.clone());
}
