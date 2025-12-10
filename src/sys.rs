use sysinfo::{
    Components, CpuRefreshKind, Disks, MemoryRefreshKind, Networks, Pid, ProcessRefreshKind,
    RefreshKind, System, Users,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub user: String,
    pub cmd: String,
    pub cpu: f32,
    pub mem_bytes: u64,
    pub parent: Option<u32>, // Parent PID
    pub indent: usize, // For tree view
}

#[derive(Clone, Debug)]
pub struct DiskInfo {
    pub _name: String,
    pub mount_point: String,
    pub total: u64,
    pub available: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProcessSort {
    Cpu,
    Memory,
    Pid,
    Tree,
}

pub struct SysCache {
    sys: System,
    users: Users,
    networks: Networks,
    disks: Disks,
    components: Components,
    pub cpu_model: String, 
    pub _cpu_model: String,
    pub cpu_cores: Vec<f32>,
    pub cpu_global: f32,
    pub cpu_temp: f32,
    pub total_mem: u64,
    pub used_mem: u64,
    pub uptime: u64,
    pub rx_rate: u64,
    pub tx_rate: u64,
    prev_rx: u64,
    prev_tx: u64,
    // Disk Stats
    prev_disk_stats: (u64, u64), // (sectors_read, sectors_written)
    pub disk_read_rate: f64, // bytes/s
    pub disk_write_rate: f64, // bytes/s
    // Sensors
    pub sensors: Vec<(String, f32)>,
    
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
            prev_rx: 0,
            prev_tx: 0,
            prev_disk_stats: (0, 0),
            disk_read_rate: 0.0,
            disk_write_rate: 0.0,
            sensors: Vec::new(),
            procs: Vec::new(),
            sort_by: ProcessSort::Cpu,
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
        
        // Sensors
        self.sensors = self.components.iter()
            .map(|c| (c.label().to_string(), c.temperature()))
            .filter(|(_, t)| *t > 0.0) // Filter invalid sensors
            .collect();

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

        // Network Rate
        let (current_rx, current_tx) = self.networks.iter().fold((0, 0), |acc, (_, n)| (acc.0 + n.total_received(), acc.1 + n.total_transmitted()));
        if self.prev_rx > 0 {
            self.rx_rate = current_rx.saturating_sub(self.prev_rx);
        }
        if self.prev_tx > 0 {
            self.tx_rate = current_tx.saturating_sub(self.prev_tx);
        }
        self.prev_rx = current_rx;
        self.prev_tx = current_tx;

        // Disk IO Rate (Linux specific logic via /proc/diskstats)
        if let Some((curr_rd, curr_wr)) = get_disk_io_stats() {
             if self.prev_disk_stats.0 > 0 {
                  // Sectors are usually 512 bytes
                  let diff_rd = curr_rd.saturating_sub(self.prev_disk_stats.0);
                  let diff_wr = curr_wr.saturating_sub(self.prev_disk_stats.1);
                  self.disk_read_rate = (diff_rd as f64) * 512.0;
                  self.disk_write_rate = (diff_wr as f64) * 512.0;
             }
             self.prev_disk_stats = (curr_rd, curr_wr);
        }

        // Processes
        self.procs = top_processes(&self.sys, &self.users, self.sort_by);
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

// Reads /proc/diskstats for total sectors read/written on physical devices
fn get_disk_io_stats() -> Option<(u64, u64)> {
    let content = std::fs::read_to_string("/proc/diskstats").ok()?;
    let mut read_sectors = 0u64;
    let mut write_sectors = 0u64;

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 14 { continue; }
        
        let name = parts[2];
        // Filter for physical devices (sd*, nvme*, vd*, xvd*) ignoring partitions (usually end in digit, except nvme)
        // Simple heuristic: if it ends in digit, it might be a partition, unless nvme p*.
        // A safer heuristic for total stats: just sum everything that looks like a root disk?
        // Let's sum sd[a-z], nvme[0-9]n[0-9], vd[a-z]. 
        // Actually, summing everything might double count partitions.
        // Let's look for devices that DON'T end in a digit (sda, vda) OR are nvme namespaces (nvme0n1).
        
        let is_physical = (name.starts_with("sd") && !name.chars().last().unwrap().is_numeric()) ||
                          (name.starts_with("vd") && !name.chars().last().unwrap().is_numeric()) ||
                          (name.starts_with("nvme") && name.contains("n") && !name.contains("p"));

        if is_physical {
             // Field 6: sectors read, Field 10: sectors written (1-indexed in docs, 0-indexed parts is 5 and 9)
             // /proc/diskstats format:
             //  major minor name ... read_sectors ... write_sectors ...
             // Fields indices (0-based):
             // 2: name
             // 5: sectors read
             // 9: sectors written
             if let (Ok(r), Ok(w)) = (parts[5].parse::<u64>(), parts[9].parse::<u64>()) {
                 read_sectors += r;
                 write_sectors += w;
             }
        }
    }
    
    Some((read_sectors, write_sectors))
}

fn top_processes(sys: &System, users: &Users, sort_by: ProcessSort) -> Vec<ProcessInfo> {
    let mut infos: Vec<ProcessInfo> = sys.processes().values().map(|p| {
        let user = p.user_id()
            .and_then(|uid| users.get_user_by_id(uid))
            .map(|u| u.name().to_string())
            .unwrap_or_else(|| "root".to_string());
        
        let parent = p.parent().map(|pid| pid.as_u32());

        ProcessInfo {
            pid: p.pid().as_u32(),
            name: p.name().to_string(),
            user,
            cmd: p.exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            cpu: p.cpu_usage(),
            mem_bytes: p.memory(),
            parent,
            indent: 0,
        }
    }).collect();
    
    match sort_by {
        ProcessSort::Cpu => infos.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal)),
        ProcessSort::Memory => infos.sort_by(|a, b| b.mem_bytes.cmp(&a.mem_bytes)),
        ProcessSort::Pid => infos.sort_by(|a, b| a.pid.cmp(&b.pid)),
        ProcessSort::Tree => {
            // Sort by PID first to safeguard
            infos.sort_by(|a, b| a.pid.cmp(&b.pid));
            return build_process_tree(infos);
        }
    }
    infos
}

fn build_process_tree(flat: Vec<ProcessInfo>) -> Vec<ProcessInfo> {
    // 1. Build Adjacency List
    let mut children_map: HashMap<Option<u32>, Vec<u32>> = HashMap::new();
    let mut process_map: HashMap<u32, ProcessInfo> = HashMap::new();
    
    for p in flat {
        process_map.insert(p.pid, p.clone());
        children_map.entry(p.parent).or_default().push(p.pid);
    }

    // 2. DFS
    let mut result = Vec::new();
    
    // Find "roots" (parent is None OR parent is not in our list e.g. kernel threads or parent killed)
    // We treat 'None' as true root. For others, if parent not found, treat as root.
    
    // We iterate the map's keys. But easier: Iterate all processes, check if parent exists in map.
    let mut roots: Vec<u32> = process_map.values()
        .filter(|p| p.parent.is_none() || !process_map.contains_key(&p.parent.unwrap()))
        .map(|p| p.pid)
        .collect();
    
    roots.sort(); // Sort roots by PID

    for root_pid in roots {
        append_node(root_pid, 0, &mut result, &process_map, &children_map);
    }
    
    result
}

fn append_node(pid: u32, depth: usize, result: &mut Vec<ProcessInfo>, 
               pmap: &HashMap<u32, ProcessInfo>, children: &HashMap<Option<u32>, Vec<u32>>) {
    if let Some(p) = pmap.get(&pid) {
        let mut p_indent = p.clone();
        p_indent.indent = depth;
        result.push(p_indent);
        
        if let Some(kids) = children.get(&Some(pid)) {
             let mut sorted_kids = kids.clone();
             sorted_kids.sort(); // sort children by PID
             for kid in sorted_kids {
                 append_node(kid, depth + 1, result, pmap, children);
             }
        }
    }
}

pub fn format_duration_secs(total_secs: u64) -> String {
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    format!("{}h {}m", hours, mins)
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
