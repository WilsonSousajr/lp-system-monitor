use crate::app::{App, SortColumn};
use crate::sys::{format_bytes, format_duration_secs, ProcessInfo};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        Axis, Block, Borders, Cell, Chart, Dataset, Gauge, GraphType, Paragraph, Row, Sparkline,
        Table,
    },
    Frame,
};

const COLOR_BG: Color = Color::Rgb(26, 27, 38);

const COLOR_BORDER: Color = Color::Rgb(160, 160, 160);

const COLOR_ACCENT: Color = Color::Rgb(0, 255, 127);

const COLOR_HIGH: Color = Color::Rgb(255, 85, 85);

const COLOR_TEXT_MAIN: Color = Color::Rgb(192, 202, 245);
const COLOR_HEADER_BG: Color = Color::Rgb(65, 72, 104);
const COLOR_HEADER_FG: Color = Color::White;

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.size();

    let bg_block = Block::default().style(Style::default().bg(COLOR_BG));
    f.render_widget(bg_block, size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Percentage(30),
            Constraint::Min(0),
        ])
        .spacing(0)
        .split(size);

    draw_top_bar(f, chunks[0], app);
    draw_cpu_row(f, chunks[1], app);
    draw_bottom_row(f, chunks[2], app);
}

fn draw_top_bar(f: &mut Frame, area: Rect, app: &App) {
    let now = chrono::Local::now();
    let time_str = now.format("%H:%M:%S").to_string();
    let bat_str = if let Some(bat) = app.sys().battery_percentage() {
        format!("BAT: {:.0}%", bat)
    } else {
        "BAT: N/A".to_string()
    };

    let style = Style::default().bg(COLOR_BG).fg(COLOR_TEXT_MAIN);
    let uptime = format_duration_secs(app.sys().uptime);

    let text = Line::from(vec![
        Span::styled(
            format!(" sysdash "),
            style.add_modifier(Modifier::BOLD).fg(COLOR_ACCENT),
        ),
        Span::raw(" | "),
        Span::styled(format!(" {} ", time_str), style),
        Span::raw(" | "),
        Span::styled(format!(" {} ", bat_str), style),
        Span::raw(" | "),
        Span::styled(format!(" Uptime: {} ", uptime), style),
    ]);

    f.render_widget(
        Paragraph::new(text)
            .alignment(ratatui::layout::Alignment::Left)
            .style(style),
        area,
    );
}

fn draw_cpu_row(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .spacing(0)
        .split(area);

    draw_cpu_graph(f, chunks[0], app);
    draw_cpu_cores(f, chunks[1], app);
}

fn draw_cpu_graph(f: &mut Frame, area: Rect, app: &App) {
    let block = make_block(" CPU History ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let data: Vec<(f64, f64)> = app
        .cpu_history
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, v as f64))
        .collect();

    let datasets = vec![Dataset::default()
        .name("Total")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(COLOR_ACCENT))
        .data(&data)];

    let chart = Chart::new(datasets)
        .x_axis(Axis::default().bounds([0.0, 100.0]))
        .y_axis(Axis::default().bounds([0.0, 100.0]).labels(vec![
            Span::styled("0", Style::default().fg(Color::DarkGray)),
            Span::styled("100", Style::default().fg(Color::DarkGray)),
        ]))
        .style(Style::default().bg(COLOR_BG));

    f.render_widget(chart, inner);
}

fn draw_cpu_cores(f: &mut Frame, area: Rect, app: &App) {
    let block = make_block(" Cores ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let cores = &app.sys().cpu_cores;
    let rows_max = inner.height as usize;

    let constraints = vec![Constraint::Length(1); rows_max];
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, &usage) in cores.iter().take(rows_max).enumerate() {
        if i >= chunks.len() {
            break;
        }

        render_usage_bar(f, chunks[i], format!("C{}", i), usage);
    }
}

fn draw_bottom_row(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .spacing(0)
        .split(area);

    draw_resources(f, chunks[0], app);
    draw_processes(f, chunks[1], app);
}

fn draw_resources(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .spacing(0)
        .split(area);

    draw_memory(f, chunks[0], app);
    draw_disks(f, chunks[1], app);
    draw_network(f, chunks[2], app);
}

fn draw_memory(f: &mut Frame, area: Rect, app: &App) {
    let block = make_block(" Memory ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let sys = app.sys();
    let total = sys.total_mem;
    let used = sys.used_mem;
    let percent = if total > 0 {
        (used as f64 / total as f64 * 100.0) as f32
    } else {
        0.0
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    let text = format!("{}/{}", format_bytes(used), format_bytes(total));
    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(COLOR_TEXT_MAIN)),
        chunks[0],
    );
    render_usage_bar(f, chunks[1], "RAM".into(), percent);
}

fn draw_disks(f: &mut Frame, area: Rect, app: &App) {
    let block = make_block(" Disks ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let disks = app.sys().disks();
    let rows = inner.height as usize;
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1); rows])
        .split(inner);

    for (i, disk) in disks.iter().take(rows).enumerate() {
        if i >= layout.len() {
            break;
        }
        let used = disk.total - disk.available;
        let p = if disk.total > 0 {
            (used as f64 / disk.total as f64 * 100.0) as f32
        } else {
            0.0
        };
        render_usage_bar(f, layout[i], disk.mount_point.clone(), p);
    }
}

fn draw_network(f: &mut Frame, area: Rect, app: &App) {
    let block = make_block(" Network ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let rx_data: Vec<u64> = app.net_rx_history.iter().map(|&x| x).collect();
    let tx_data: Vec<u64> = app.net_tx_history.iter().map(|&x| x).collect();

    let rx_spark = Sparkline::default()
        .block(
            Block::default()
                .title(format!("RX: {}/s", format_bytes(app.sys().rx_rate)))
                .title_style(Style::default().fg(COLOR_ACCENT)),
        )
        .data(&rx_data)
        .style(Style::default().fg(COLOR_ACCENT));

    let tx_spark = Sparkline::default()
        .block(
            Block::default()
                .title(format!("TX: {}/s", format_bytes(app.sys().tx_rate)))
                .title_style(Style::default().fg(COLOR_HIGH)),
        )
        .data(&tx_data)
        .style(Style::default().fg(COLOR_HIGH));

    f.render_widget(rx_spark, chunks[0]);
    f.render_widget(tx_spark, chunks[1]);
}

fn draw_processes(f: &mut Frame, area: Rect, app: &mut App) {
    let block = make_block(" Processes ");

    let query = app.search_query.to_lowercase();
    let mut procs: Vec<&ProcessInfo> = app
        .sys()
        .processes()
        .iter()
        .filter(|p| p.name.to_lowercase().contains(&query) || p.pid.to_string().contains(&query))
        .collect();

    procs.sort_by(|a, b| {
        let ord = match app.sort_col {
            SortColumn::Pid => a.pid.cmp(&b.pid),
            SortColumn::Name => a.name.cmp(&b.name),
            SortColumn::User => a.user.cmp(&b.user),
            SortColumn::Cpu => a
                .cpu
                .partial_cmp(&b.cpu)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortColumn::Mem => a.mem_bytes.cmp(&b.mem_bytes),
        };
        if app.sort_desc {
            ord.reverse()
        } else {
            ord
        }
    });

    let rows: Vec<Row> = procs
        .iter()
        .map(|p| {
            Row::new(vec![
                Cell::from(p.pid.to_string()),
                Cell::from(p.name.clone()),
                Cell::from(p.cmd.chars().take(20).collect::<String>()),
                Cell::from(p.user.clone()),
                Cell::from(format_bytes(p.mem_bytes)),
                Cell::from(format!("{:.1}", p.cpu)),
            ])
        })
        .collect();

    let highlight_style = Style::default()
        .bg(Color::Cyan)
        .fg(Color::Black)
        .add_modifier(Modifier::BOLD);

    let table = Table::new(
        rows,
        vec![
            Constraint::Length(6),
            Constraint::Percentage(20),
            Constraint::Percentage(30),
            Constraint::Percentage(15),
            Constraint::Length(10),
            Constraint::Length(8),
        ],
    )
    .header(
        Row::new(vec!["PID", "Prog", "Command", "User", "MemB", "Cpu%"])
            .style(
                Style::default()
                    .bg(COLOR_HEADER_BG)
                    .fg(COLOR_HEADER_FG)
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(0),
    )
    .block(block)
    .highlight_style(highlight_style);

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_usage_bar(f: &mut Frame, area: Rect, label: String, percent: f32) {
    let gauge_block = Block::default();

    let gauge = Gauge::default()
        .block(gauge_block)
        .gauge_style(
            Style::default()
                .fg(if percent > 80.0 {
                    COLOR_HIGH
                } else {
                    COLOR_ACCENT
                })
                .bg(Color::DarkGray),
        )
        .label(format!("{} {:.1}%", label, percent))
        .ratio(percent as f64 / 100.0)
        .use_unicode(true);

    f.render_widget(gauge, area);
}

fn make_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER))
        .title(Span::styled(
            format!(" {} ", title),
            Style::default()
                .bg(COLOR_HEADER_BG)
                .fg(COLOR_HEADER_FG)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(COLOR_BG))
}
