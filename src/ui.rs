use crate::app::{App, InputMode, PopupState};
use crate::sys::{format_bytes, format_duration_secs, ProcessSort};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    widgets::{
        Axis, Block, BorderType, Borders, Cell, Chart, Clear, Dataset, Gauge, GraphType, Paragraph, Row, Sparkline, Table,
    },
    layout::Alignment,
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.size();
    
    // 1. Top (CPU) vs Bottom
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35), // CPU Module
            Constraint::Percentage(65), // Bottom Modules
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

    // Draw popup if needed
    if let PopupState::None = app.popup {
        // No popup
    } else {
        draw_popup(f, app);
    }
}

fn draw_popup(f: &mut Frame, app: &App) {
    let area = f.size();
    
    let text = match &app.popup {
        PopupState::Kill { pid, name } => vec![
            format!("Are you sure you want to kill process {} ({})?", pid, name),
            "Press 'Y' to confirm, 'N' to cancel".to_string(),
        ],
        PopupState::Help => vec![
            "Help Menu".to_string(),
            "".to_string(),
            "k: Kill Process".to_string(),
            "s/Tab: Toggle Sort (Cpu/Mem)".to_string(),
            "/: Search Process".to_string(),
            "?: Toggle Help".to_string(),
            "Esc: Close Popup / Clear Search".to_string(),
            "q: Quit".to_string(),
        ],
        _ => return,
    };
    
    // Centered float
    let width = 60;
    let height = text.len() as u16 + 4;
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - 40) / 2),
            Constraint::Length(height),
            Constraint::Percentage((100 - 40) / 2),
        ])
        .split(area);

    let popup_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - 40) / 2),
            Constraint::Length(width),
            Constraint::Percentage((100 - 40) / 2),
        ])
        .split(popup_layout[1])[1];

    f.render_widget(Clear, popup_area);
    
    let title = match app.popup {
         PopupState::Kill{..} => " Confirm Kill ",
         PopupState::Help => " Help ",
         _ => "",
    };

    let p = Paragraph::new(text.join("\n"))
        .block(Block::default().title(title).borders(Borders::ALL).border_type(BorderType::Rounded))
        .alignment(Alignment::Center);
    
    f.render_widget(p, popup_area);
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
        .title(format!(" CPU: {} | Uptime: {} ", app.sys().cpu_model, format_duration_secs(app.sys().uptime)));
    
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner_area);

    // Left: History Chart
    let data: Vec<(f64, f64)> = app.cpu_history.iter().enumerate()
        .map(|(i, &v)| (i as f64, v as f64))
        .collect();

    let datasets = vec![
        Dataset::default()
            .name("Usage")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(get_color(app.sys().cpu_global)))
            .data(&data),
    ];

    let chart = Chart::new(datasets)
        .block(Block::default().borders(Borders::RIGHT))
        .x_axis(Axis::default().bounds([0.0, 100.0])) // Window size
        .y_axis(Axis::default().bounds([0.0, 100.0]).labels(vec!["0%".into(), "50%".into(), "100%".into()]));
    
    f.render_widget(chart, chunks[0]);

    // Right: Cores
    let cores = &app.sys().cpu_cores;
    // Display as many cores as fit
    let core_count = cores.len();
    let rows_needed = core_count.min(inner_area.height as usize); 
    
    let core_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1); rows_needed])
        .split(chunks[1]);

    for (i, &usage) in cores.iter().take(rows_needed).enumerate() {
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
    
    let block = Block::default()
        .title(" Memory ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Just a big gauge for now, or list if we had Swap
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(get_color(percent as f32)))
        .percent(percent)
        .label(format!("{}/{}", format_bytes(sys.used_mem), format_bytes(sys.total_mem)));
    
    // Center vertically in the block
    let v_layout = Layout::default().direction(Direction::Vertical).constraints([Constraint::Min(1), Constraint::Length(1), Constraint::Min(1)]).split(inner);
    f.render_widget(gauge, v_layout[1]);
}

fn draw_network_module(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Network ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    
    let inner = block.inner(area);
    f.render_widget(block, area);
    
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(50), Constraint::Percentage(50)]).split(inner);

    let rx_label = format!("RX: {}/s", format_bytes(app.sys().rx_rate));
    let tx_label = format!("TX: {}/s", format_bytes(app.sys().tx_rate));

    let rx_spark = Sparkline::default()
        .block(Block::default().title(rx_label))
        .data(&app.net_rx_history)
        .style(Style::default().fg(Color::Green));
        
    let tx_spark = Sparkline::default()
        .block(Block::default().title(tx_label))
        .data(&app.net_tx_history)
        .style(Style::default().fg(Color::Red));

    f.render_widget(rx_spark, chunks[0]);
    f.render_widget(tx_spark, chunks[1]);
}

fn draw_disk_module(f: &mut Frame, area: Rect, app: &App) {
    let disks = app.sys().disks();
    let block = Block::default().title(" Disks ").borders(Borders::ALL).border_type(BorderType::Rounded);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let constraints = vec![Constraint::Length(2); disks.len().min(10)];
    let chunks = Layout::default().direction(Direction::Vertical).constraints(constraints).split(inner);

    for (i, disk) in disks.iter().take(chunks.len()).enumerate() {
        let used = disk.total - disk.available;
        let percent = (used as f64 / disk.total as f64 * 100.0) as u16;
        let g = Gauge::default()
            .percent(percent)
            .label(format!("{} {}", disk.mount_point, format_bytes(used)))
            .gauge_style(Style::default().fg(get_color(percent as f32)));
        f.render_widget(g, chunks[i]);
    }
}

pub fn draw_processes_module(f: &mut Frame, area: Rect, app: &App) {
    let sort_label = match app.sys().sort_by {
        ProcessSort::Cpu => "Sort: CPU",
        ProcessSort::Memory => "Sort: Mem",
        ProcessSort::Pid => "Sort: PID",
    };

    let title = match app.input_mode {
        InputMode::Normal => format!(" Processes (Press '/' search, '?' help) [{}] ", sort_label),
        InputMode::Editing => format!(" Search: {}_ ", app.search_query),
        InputMode::Popup => format!(" Processes (Popup Active) "),
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let query = app.search_query.to_lowercase();
    let processes: Vec<_> = app.sys().processes().iter()
        .filter(|p| p.name.to_lowercase().contains(&query) || p.pid.to_string().contains(&query))
        .collect();

    let rows = processes.iter().map(|p| {
        Row::new(vec![
            Cell::from(p.pid.to_string()),
            Cell::from(p.name.clone()),
            Cell::from(p.user.clone()),
            Cell::from(format!("{:.1}%", p.cpu)),
            Cell::from(format_bytes(p.mem_bytes)),
        ])
    });

    let table = Table::new(rows, vec![
        Constraint::Length(6),
        Constraint::Percentage(30),
        Constraint::Percentage(20),
        Constraint::Length(10),
        Constraint::Length(10),
    ])
    .header(Row::new(vec!["PID", "Name", "User", "CPU", "Mem"]).style(Style::default().add_modifier(Modifier::BOLD)))
    .block(block)
    .highlight_style(Style::default().bg(Color::White).fg(Color::Black));

    f.render_stateful_widget(table, area, &mut app.table_state.clone());
}
