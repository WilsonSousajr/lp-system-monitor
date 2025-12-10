use sysinfo::{
    Components, CpuRefreshKind, Disks, MemoryRefreshKind, Networks, Pid, ProcessRefreshKind,
    RefreshKind, System, Users,
};

#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub user: String,
    pub cmd: String,
    pub cpu: f32,
    pub mem_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct DiskInfo {
    pub _name: String,
    pub mount_point: String,
    pub total: u64,
    pub available: u64,
}

pub struct SysCache {
    sys: System,
    users: Users,
    networks: Networks,
    disks: Disks,
    components: Components,
    pub _cpu_model: String,
    pub cpu_cores: Vec<f32>,
    pub cpu_global: f32,
    pub cpu_temp: f32,
    pub total_mem: u64,
    pub used_mem: u64,
    pub uptime: u64,
    pub rx_rate: u64,
    pub tx_rate: u64,

    procs: Vec<ProcessInfo>,
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

        let cpu_model = sys
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_default();

        let mut temp_sum = 0.0;
        let mut temp_count = 0;
        for component in &components {
            let label = component.label().to_lowercase();
            if label.contains("cpu") || label.contains("core") || label.contains("package") {
                temp_sum += component.temperature();
                temp_count += 1;
            }
        }
        let cpu_temp = if temp_count > 0 {
            temp_sum / temp_count as f32
        } else {
            0.0
        };

        let mut s = Self {
            sys,
            users,
            networks,
            disks,
            components,
            _cpu_model: cpu_model,
            cpu_cores: Vec::new(),
            cpu_global: 0.0,
            cpu_temp,
            total_mem: 0,
            used_mem: 0,
            uptime: 0,
            rx_rate: 0,
            tx_rate: 0,

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
        self.networks.refresh();
        self.disks.refresh();
        self.components.refresh();

        self.cpu_global = self.sys.global_cpu_info().cpu_usage();
        self.cpu_cores = self.sys.cpus().iter().map(|c| c.cpu_usage()).collect();

        let mut temp_sum = 0.0;
        let mut temp_count = 0;
        for component in &self.components {
            let label = component.label().to_lowercase();
            if label.contains("cpu") || label.contains("core") || label.contains("package") {
                temp_sum += component.temperature();
                temp_count += 1;
            }
        }
        self.cpu_temp = if temp_count > 0 {
            temp_sum / temp_count as f32
        } else {
            0.0
        };

        self.total_mem = self.sys.total_memory();
        self.used_mem = self.total_mem.saturating_sub(self.sys.available_memory());
        self.uptime = System::uptime();

        let (rx, tx) = self.networks.iter().fold((0, 0), |acc, (_, n)| {
            (acc.0 + n.received(), acc.1 + n.transmitted())
        });
        
        self.rx_rate = rx;
        self.tx_rate = tx;

        self.procs = top_processes(&self.sys, &self.users);
    }

    pub fn kill_process(&self, pid: u32) {
        if let Some(process) = self.sys.process(Pid::from_u32(pid)) {
            process.kill();
        }
    }

    pub fn processes(&self) -> &[ProcessInfo] {
        &self.procs
    }

    pub fn disks(&self) -> Vec<DiskInfo> {
        self.disks
            .iter()
            .map(|d| DiskInfo {
                _name: d.name().to_string_lossy().to_string(),
                mount_point: d.mount_point().to_string_lossy().to_string(),
                total: d.total_space(),
                available: d.available_space(),
            })
            .collect()
    }

    pub fn battery_percentage(&self) -> Option<f32> {
        None
    }
}

fn top_processes(sys: &System, users: &Users) -> Vec<ProcessInfo> {
    let mut v: Vec<ProcessInfo> = sys
        .processes()
        .values()
        .map(|p| {
            let user = p
                .user_id()
                .and_then(|uid| users.get_user_by_id(uid))
                .map(|u| u.name().to_string())
                .unwrap_or_else(|| "root".to_string());

            ProcessInfo {
                pid: p.pid().as_u32(),
                name: p.name().to_string(),
                user,
                cmd: p
                    .exe()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
                cpu: p.cpu_usage(),
                mem_bytes: p.memory(),
            }
        })
        .collect();
    v.sort_by(|a, b| {
        b.cpu
            .partial_cmp(&a.cpu)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    v
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    if bytes == 0 {
        return "0B".into();
    }
    let mut size = bytes as f64;
    let mut unit = 0usize;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{:.1}{}", size, UNITS[unit])
}

pub fn format_duration_secs(total_secs: u64) -> String {
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0B");
        assert_eq!(format_bytes(512), "512.0B");
        assert_eq!(format_bytes(1024), "1.0K");
        assert_eq!(format_bytes(1024 * 1024), "1.0M");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0G");
    }

    #[test]
    fn test_format_duration_secs() {
        assert_eq!(format_duration_secs(0), "00:00:00");
        assert_eq!(format_duration_secs(59), "00:00:59");
        assert_eq!(format_duration_secs(60), "00:01:00");
        assert_eq!(format_duration_secs(3661), "01:01:01");
    }

    #[test]
    fn test_sys_cache_new() {
        let sys = SysCache::new();
        assert!(sys.total_mem > 0);
    }
}
