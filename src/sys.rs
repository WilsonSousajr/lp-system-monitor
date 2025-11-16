use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Process, ProcessRefreshKind, RefreshKind, System};

#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub mem_bytes: u64,
}

pub struct SysCache {
    sys: System,
    cpu: f32,
    total_mem_bytes: u64,
    used_mem_bytes: u64,
    uptime_secs: u64,
    procs: Vec<ProcessInfo>,
}

impl SysCache {
    pub fn new() -> Self {
        let refresh = RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything())
            .with_processes(ProcessRefreshKind::everything());
        let mut sys = System::new_with_specifics(refresh);
        sys.refresh_all();
        let mut s = Self {
            sys,
            cpu: 0.0,
            total_mem_bytes: 0,
            used_mem_bytes: 0,
            uptime_secs: 0,
            procs: Vec::new(),
        };
        s.refresh();
        s
    }

    pub fn refresh(&mut self) {
        self.sys.refresh_cpu();
        self.sys.refresh_memory();
        self.sys
            .refresh_processes_specifics(ProcessRefreshKind::everything());

        self.cpu = self.sys.global_cpu_info().cpu_usage();
        let total = self.sys.total_memory();
        let avail = self.sys.available_memory();
        self.total_mem_bytes = total;
        self.used_mem_bytes = total.saturating_sub(avail);
        self.uptime_secs = self.sys.uptime();
        self.procs = top_processes(&self.sys, 8);
    }

    pub fn cpu_percent(&self) -> f32 {
        self.cpu
    }

    pub fn total_mem_bytes(&self) -> u64 {
        self.total_mem_bytes
    }

    pub fn used_mem_bytes(&self) -> u64 {
        self.used_mem_bytes
    }

    pub fn uptime_secs(&self) -> u64 {
        self.uptime_secs
    }

    pub fn processes(&self) -> &[ProcessInfo] {
        &self.procs
    }
}

fn top_processes(sys: &System, limit: usize) -> Vec<ProcessInfo> {
    let mut v: Vec<ProcessInfo> = sys
        .processes()
        .values()
        .map(|p: &Process| ProcessInfo {
            pid: p.pid().as_u32(),
            name: p.name().to_string(),
            cpu: p.cpu_usage(),
            mem_bytes: p.memory(),
        })
        .collect();
    v.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));
    v.truncate(limit);
    v
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes == 0 {
        return "0 B".into();
    }
    let mut size = bytes as f64;
    let mut unit = 0usize;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{:.0} {}", size, UNITS[unit])
    } else {
        format!("{:.1} {}", size, UNITS[unit])
    }
}

pub fn format_duration_secs(total_secs: u64) -> String {
    let days = total_secs / 86_400;
    let hours = (total_secs % 86_400) / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if days > 0 {
        format!("{}d {:02}h {:02}m {:02}s", days, hours, mins, secs)
    } else if hours > 0 {
        format!("{:02}h {:02}m {:02}s", hours, mins, secs)
    } else {
        format!("{:02}m {:02}s", mins, secs)
    }
}
