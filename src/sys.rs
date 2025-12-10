use sysinfo::{
    Components, CpuRefreshKind, Disks, MemoryRefreshKind, Networks, Pid,
    ProcessRefreshKind, RefreshKind, System, Users,
};

#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub user: String, // Added field
    pub cmd: String, // Path/Command
    pub cpu: f32,
    pub mem_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total: u64,
    pub available: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProcessSort {
    Cpu,
    Memory,
    Pid,
}

pub struct SysCache {
    sys: System,
    users: Users,
    networks: Networks,
    disks: Disks,
    components: Components, // For battery/temp
    pub cpu_model: String,
    pub cpu_cores: Vec<f32>, // Usage per core
    pub cpu_global: f32,
    pub total_mem: u64,
    pub used_mem: u64,
    pub uptime: u64,
    // Network rates
    pub rx_rate: u64,
    pub tx_rate: u64,
    prev_rx: u64,
    prev_tx: u64,
    procs: Vec<ProcessInfo>,
    pub sort_by: ProcessSort,
}

impl SysCache {
    pub fn new() -> Self {
        let refresh = RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything())
            .with_processes(ProcessRefreshKind::everything());
        
        let mut sys = System::new_with_specifics(refresh);
        
        let users = Users::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let disks = Disks::new_with_refreshed_list();
        let components = Components::new_with_refreshed_list();

        sys.refresh_all();
        
        let cpu_model = sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_default();

        let mut s = Self {
            sys,
            users,
            networks,
            disks,
            components,
            cpu_model,
            cpu_cores: Vec::new(),
            cpu_global: 0.0,
            total_mem: 0,
            used_mem: 0,
            uptime: 0,
            rx_rate: 0,
            tx_rate: 0,
            prev_rx: 0,
            prev_tx: 0,
            procs: Vec::new(),
            sort_by: ProcessSort::Cpu,
        };
        s.refresh();
        s
    }

    pub fn refresh(&mut self) {
        self.sys.refresh_cpu();
        self.sys.refresh_memory();
        self.sys.refresh_processes_specifics(ProcessRefreshKind::everything());
        self.networks.refresh();
        self.disks.refresh();
        self.components.refresh();

        // CPU
        self.cpu_global = self.sys.global_cpu_info().cpu_usage();
        self.cpu_cores = self.sys.cpus().iter().map(|c| c.cpu_usage()).collect();

        // Memory
        self.total_mem = self.sys.total_memory();
        self.used_mem = self.total_mem.saturating_sub(self.sys.available_memory());
        self.uptime = System::uptime();

        // Network Rate Calculation
        let (current_rx, current_tx) = self.networks.iter().fold((0, 0), |acc, (_, n)| (acc.0 + n.total_received(), acc.1 + n.total_transmitted()));
        
        // Calculate diff. If prev is 0 (first run), rate is 0 to avoid spikes.
        if self.prev_rx > 0 {
            self.rx_rate = current_rx.saturating_sub(self.prev_rx);
        }
        if self.prev_tx > 0 {
            self.tx_rate = current_tx.saturating_sub(self.prev_tx);
        }

        self.prev_rx = current_rx;
        self.prev_tx = current_tx;

        // Processes
        self.procs = top_processes(&self.sys, &self.users, self.sort_by);
    }

    pub fn kill_process(&self, pid: u32) {
        if let Some(process) = self.sys.process(Pid::from_u32(pid)) {
            process.kill();
        }
    }

    pub fn processes(&self) -> &[ProcessInfo] { &self.procs }
    pub fn disks(&self) -> Vec<DiskInfo> {
        self.disks.iter().map(|d| DiskInfo {
            name: d.name().to_string_lossy().to_string(),
            mount_point: d.mount_point().to_string_lossy().to_string(),
            total: d.total_space(),
            available: d.available_space(),
        }).collect()
    }
    
    // Helper to get battery % (first battery found)
    pub fn battery_percentage(&self) -> Option<f32> {
        None 
    }
}

fn top_processes(sys: &System, users: &Users, sort_by: ProcessSort) -> Vec<ProcessInfo> {
    let mut v: Vec<ProcessInfo> = sys.processes().values().map(|p| {
        let user = p.user_id()
            .and_then(|uid| users.get_user_by_id(uid))
            .map(|u| u.name().to_string())
            .unwrap_or_else(|| "root".to_string());

        ProcessInfo {
            pid: p.pid().as_u32(),
            name: p.name().to_string(),
            user,
            cmd: p.exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            cpu: p.cpu_usage(),
            mem_bytes: p.memory(),
        }
    }).collect();
    
    // Sort
    match sort_by {
        ProcessSort::Cpu => v.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal)),
        ProcessSort::Memory => v.sort_by(|a, b| b.mem_bytes.cmp(&a.mem_bytes)),
        ProcessSort::Pid => v.sort_by(|a, b| a.pid.cmp(&b.pid)),
    }
    v
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes == 0 { return "0 B".into(); }
    let mut size = bytes as f64;
    let mut unit = 0usize;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{:.1} {}", size, UNITS[unit])
}

pub fn format_duration_secs(total_secs: u64) -> String {
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    format!("{}h {}m", hours, mins)
}
