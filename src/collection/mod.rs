use anyhow::Result;
use crossbeam::channel::Sender;
use hashbrown::HashMap;
use procfs::process::{Process, all_processes};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct CGroupCollector {
    pub cgroup_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CGroupMetrics {
    pub hierarchies: Vec<CGroupHierarchy>,
    pub processes: HashMap<u32, String>, // PID -> cgroup path
    pub resource_usage: HashMap<String, ResourceStats>, // cgroup path -> stats
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
pub struct CGroupHierarchy {
    pub root: CGroupNode,
    pub flat_map: HashMap<String, CGroupNode>,
}

#[derive(Debug, Clone)]
pub struct CGroupNode {
    pub path: String,
    pub name: String,
    pub stats: ResourceStats,
    pub children: Vec<String>,
    pub processes: Vec<ProcessInfo>,
}

#[derive(Debug, Clone, Default)]
pub struct ResourceStats {
    pub memory: MemoryStats,
    pub cpu: CpuStats,
    pub io: IoStats,
    pub pids: PidStats,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    pub current: u64,
    pub max: Option<u64>,
    pub events: MemoryEvents,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryEvents {
    pub low: u64,
    pub high: u64,
    pub max: u64,
    pub oom: u64,
    pub oom_kill: u64,
}

#[derive(Debug, Clone, Default)]
pub struct CpuStats {
    pub usage_usec: u64,
    pub user_usec: u64,
    pub system_usec: u64,
    pub nr_periods: u64,
    pub nr_throttled: u64,
    pub throttled_usec: u64,
}

#[derive(Debug, Clone, Default)]
pub struct IoStats {
    pub rbytes: u64,
    pub wbytes: u64,
    pub rios: u64,
    pub wios: u64,
}

#[derive(Debug, Clone, Default)]
pub struct PidStats {
    pub current: u64,
    pub max: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub command: String,
    pub cgroup_path: String,
}

impl CGroupCollector {
    pub fn new(cgroup_root: PathBuf) -> Self {
        Self { cgroup_root }
    }

    pub fn collect_metrics(&self) -> Result<CGroupMetrics> {
        let mut metrics = CGroupMetrics {
            hierarchies: Vec::new(),
            processes: HashMap::new(),
            resource_usage: HashMap::new(),
            timestamp: Instant::now(),
        };

        // Collect cgroup tree and resource stats
        self.collect_cgroup_tree(&self.cgroup_root, &mut metrics)?;

        // Map processes to cgroups
        self.collect_process_mappings(&mut metrics)?;

        Ok(metrics)
    }

    fn collect_cgroup_tree(&self, path: &Path, metrics: &mut CGroupMetrics) -> Result<()> {
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "cgroup path does not exist: {}",
                path.display()
            ));
        }

        // Read basic cgroup information
        let path_str = path.to_string_lossy().to_string();
        let stats = self.read_cgroup_stats(path)?;

        metrics.resource_usage.insert(path_str.clone(), stats);

        // Recursively collect from subdirectories
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let _ = self.collect_cgroup_tree(&entry.path(), metrics);
                }
            }
        }

        Ok(())
    }

    fn read_cgroup_stats(&self, cgroup_path: &Path) -> Result<ResourceStats> {
        let mut stats = ResourceStats::default();

        // Read memory stats
        stats.memory = self.read_memory_stats(cgroup_path)?;

        // Read CPU stats
        stats.cpu = self.read_cpu_stats(cgroup_path)?;

        // Read IO stats
        stats.io = self.read_io_stats(cgroup_path)?;

        // Read PID stats
        stats.pids = self.read_pid_stats(cgroup_path)?;

        Ok(stats)
    }

    pub fn read_memory_stats(&self, cgroup_path: &Path) -> Result<MemoryStats> {
        let mut memory_stats = MemoryStats::default();

        // Read memory.current
        if let Ok(content) = fs::read_to_string(cgroup_path.join("memory.current")) {
            memory_stats.current = content.trim().parse().unwrap_or(0);
        }

        // Read memory.max
        if let Ok(content) = fs::read_to_string(cgroup_path.join("memory.max")) {
            if content.trim() != "max" {
                memory_stats.max = content.trim().parse().ok();
            }
        }

        Ok(memory_stats)
    }

    pub fn read_cpu_stats(&self, cgroup_path: &Path) -> Result<CpuStats> {
        let mut cpu_stats = CpuStats::default();

        if let Ok(content) = fs::read_to_string(cgroup_path.join("cpu.stat")) {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    match parts[0] {
                        "usage_usec" => cpu_stats.usage_usec = parts[1].parse().unwrap_or(0),
                        "user_usec" => cpu_stats.user_usec = parts[1].parse().unwrap_or(0),
                        "system_usec" => cpu_stats.system_usec = parts[1].parse().unwrap_or(0),
                        "nr_periods" => cpu_stats.nr_periods = parts[1].parse().unwrap_or(0),
                        "nr_throttled" => cpu_stats.nr_throttled = parts[1].parse().unwrap_or(0),
                        "throttled_usec" => {
                            cpu_stats.throttled_usec = parts[1].parse().unwrap_or(0)
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(cpu_stats)
    }

    pub fn read_io_stats(&self, cgroup_path: &Path) -> Result<IoStats> {
        let mut io_stats = IoStats::default();

        if let Ok(content) = fs::read_to_string(cgroup_path.join("io.stat")) {
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    // Format: device_id rbytes=value wbytes=value rios=value wios=value
                    for part in &parts[1..] {
                        if let Some((key, value)) = part.split_once('=') {
                            match key {
                                "rbytes" => io_stats.rbytes += value.parse().unwrap_or(0),
                                "wbytes" => io_stats.wbytes += value.parse().unwrap_or(0),
                                "rios" => io_stats.rios += value.parse().unwrap_or(0),
                                "wios" => io_stats.wios += value.parse().unwrap_or(0),
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        Ok(io_stats)
    }

    pub fn read_pid_stats(&self, cgroup_path: &Path) -> Result<PidStats> {
        let mut pid_stats = PidStats::default();

        if let Ok(content) = fs::read_to_string(cgroup_path.join("pids.current")) {
            pid_stats.current = content.trim().parse().unwrap_or(0);
        }

        if let Ok(content) = fs::read_to_string(cgroup_path.join("pids.max")) {
            if content.trim() != "max" {
                pid_stats.max = content.trim().parse().ok();
            }
        }

        Ok(pid_stats)
    }

    fn collect_process_mappings(&self, metrics: &mut CGroupMetrics) -> Result<()> {
        // Get all running processes
        match all_processes() {
            Ok(processes) => {
                for process in processes.filter_map(|p| p.ok()) {
                    if let Ok(process_info) = self.get_process_cgroup_info(process) {
                        metrics
                            .processes
                            .insert(process_info.pid, process_info.cgroup_path.clone());

                        // Add process to the corresponding cgroup's process list
                        if let Some(_resource_stats) =
                            metrics.resource_usage.get_mut(&process_info.cgroup_path)
                        {
                            // This would be where we'd add the process to the cgroup's process list
                            // For now, we'll just track the mapping in the main processes HashMap
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to read processes: {}", e);
            }
        }

        Ok(())
    }

    fn get_process_cgroup_info(&self, process: Process) -> Result<ProcessInfo> {
        let pid = process.pid() as u32;

        // Read process command
        let command = process
            .cmdline()
            .map(|cmd| cmd.join(" "))
            .unwrap_or_else(|_| {
                process
                    .stat()
                    .map(|s| s.comm)
                    .unwrap_or_else(|_| format!("[{}]", pid))
            });

        // Read process cgroup
        let cgroup_path = match process.cgroups() {
            Ok(cgroups) => {
                // In cgroup v2, there should be only one cgroup entry
                cgroups
                    .into_iter()
                    .find(|cgroup| cgroup.hierarchy == 0) // cgroup v2 has hierarchy 0
                    .map(|cgroup| format!("/sys/fs/cgroup{}", cgroup.pathname))
                    .unwrap_or_else(|| "/sys/fs/cgroup".to_string())
            }
            Err(_) => "/sys/fs/cgroup".to_string(), // Fallback to root
        };

        Ok(ProcessInfo {
            pid,
            command,
            cgroup_path,
        })
    }

    pub fn get_process_count_for_cgroup(
        &self,
        cgroup_path: &str,
        metrics: &CGroupMetrics,
    ) -> usize {
        metrics
            .processes
            .values()
            .filter(|path| path.starts_with(cgroup_path))
            .count()
    }
}
