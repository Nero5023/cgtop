mod common;

use cgtop::collection::{CGroupCollector, CGroupMetrics, ResourceStats, MemoryStats, CpuStats, IoStats, PidStats};
use tempfile::TempDir;
use std::fs;
use std::path::PathBuf;
use pretty_assertions::assert_eq;

fn create_mock_cgroup_filesystem(temp_dir: &TempDir) -> PathBuf {
    let cgroup_root = temp_dir.path().join("cgroup");
    fs::create_dir_all(&cgroup_root).unwrap();
    
    // Create mock cgroup hierarchy
    let system_slice = cgroup_root.join("system.slice");
    fs::create_dir_all(&system_slice).unwrap();
    
    let ssh_service = system_slice.join("ssh.service");
    fs::create_dir_all(&ssh_service).unwrap();
    
    // Create mock cgroup files
    fs::write(cgroup_root.join("memory.current"), "1048576").unwrap(); // 1MB
    fs::write(cgroup_root.join("memory.max"), "10485760").unwrap(); // 10MB
    fs::write(cgroup_root.join("cpu.stat"), "usage_usec 1000000\nuser_usec 500000\nsystem_usec 200000\n").unwrap();
    fs::write(cgroup_root.join("io.stat"), "8:0 rbytes=1024 wbytes=512 rios=10 wios=5\n").unwrap();
    fs::write(cgroup_root.join("pids.current"), "42").unwrap();
    fs::write(cgroup_root.join("pids.max"), "100").unwrap();
    
    // Create files for system.slice
    fs::write(system_slice.join("memory.current"), "2097152").unwrap(); // 2MB
    fs::write(system_slice.join("cpu.stat"), "usage_usec 2000000\nuser_usec 1000000\nsystem_usec 400000\n").unwrap();
    fs::write(system_slice.join("io.stat"), "8:0 rbytes=2048 wbytes=1024 rios=20 wios=10\n").unwrap();
    fs::write(system_slice.join("pids.current"), "5").unwrap();
    
    // Create files for ssh.service
    fs::write(ssh_service.join("memory.current"), "524288").unwrap(); // 0.5MB
    fs::write(ssh_service.join("cpu.stat"), "usage_usec 500000\nuser_usec 300000\nsystem_usec 100000\n").unwrap();
    fs::write(ssh_service.join("io.stat"), "8:0 rbytes=512 wbytes=256 rios=5 wios=2\n").unwrap();
    fs::write(ssh_service.join("pids.current"), "2").unwrap();
    
    cgroup_root
}

#[test]
fn test_cgroup_collector_creation() {
    let collector = CGroupCollector::new(PathBuf::from("/tmp"));
    assert_eq!(collector.cgroup_root, PathBuf::from("/tmp"));
}

#[test]
fn test_memory_stats_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let cgroup_root = create_mock_cgroup_filesystem(&temp_dir);
    
    let collector = CGroupCollector::new(cgroup_root.clone());
    let stats = collector.read_memory_stats(&cgroup_root).unwrap();
    
    assert_eq!(stats.current, 1048576); // 1MB
    assert_eq!(stats.max, Some(10485760)); // 10MB
}

#[test]
fn test_cpu_stats_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let cgroup_root = create_mock_cgroup_filesystem(&temp_dir);
    
    let collector = CGroupCollector::new(cgroup_root.clone());
    let stats = collector.read_cpu_stats(&cgroup_root).unwrap();
    
    assert_eq!(stats.usage_usec, 1000000);
    assert_eq!(stats.user_usec, 500000);
    assert_eq!(stats.system_usec, 200000);
}

#[test]
fn test_io_stats_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let cgroup_root = create_mock_cgroup_filesystem(&temp_dir);
    
    let collector = CGroupCollector::new(cgroup_root.clone());
    let stats = collector.read_io_stats(&cgroup_root).unwrap();
    
    assert_eq!(stats.rbytes, 1024);
    assert_eq!(stats.wbytes, 512);
    assert_eq!(stats.rios, 10);
    assert_eq!(stats.wios, 5);
}

#[test]
fn test_pid_stats_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let cgroup_root = create_mock_cgroup_filesystem(&temp_dir);
    
    let collector = CGroupCollector::new(cgroup_root.clone());
    let stats = collector.read_pid_stats(&cgroup_root).unwrap();
    
    assert_eq!(stats.current, 42);
    assert_eq!(stats.max, Some(100));
}

#[test]
fn test_collect_cgroup_tree() {
    let temp_dir = TempDir::new().unwrap();
    let cgroup_root = create_mock_cgroup_filesystem(&temp_dir);
    
    let collector = CGroupCollector::new(cgroup_root.clone());
    let metrics = collector.collect_metrics().unwrap();
    
    // Should collect stats for root, system.slice, and ssh.service
    assert!(metrics.resource_usage.len() >= 3);
    
    // Check that we have the expected paths
    let root_path = cgroup_root.to_string_lossy().to_string();
    let system_path = cgroup_root.join("system.slice").to_string_lossy().to_string();
    let ssh_path = cgroup_root.join("system.slice/ssh.service").to_string_lossy().to_string();
    
    assert!(metrics.resource_usage.contains_key(&root_path));
    assert!(metrics.resource_usage.contains_key(&system_path));
    assert!(metrics.resource_usage.contains_key(&ssh_path));
    
    // Verify stats are correctly parsed
    let root_stats = &metrics.resource_usage[&root_path];
    assert_eq!(root_stats.memory.current, 1048576);
    assert_eq!(root_stats.cpu.usage_usec, 1000000);
    assert_eq!(root_stats.pids.current, 42);
}

#[test]
fn test_collect_metrics_with_missing_files() {
    let temp_dir = TempDir::new().unwrap();
    let cgroup_root = temp_dir.path().join("incomplete_cgroup");
    fs::create_dir_all(&cgroup_root).unwrap();
    
    // Only create some files, leave others missing
    fs::write(cgroup_root.join("memory.current"), "1048576").unwrap();
    // memory.max is missing
    // cpu.stat is missing
    // io.stat is missing
    // pids.current is missing
    
    let collector = CGroupCollector::new(cgroup_root.clone());
    let result = collector.collect_metrics();
    
    // Should still succeed, just with default values for missing files
    assert!(result.is_ok());
    let metrics = result.unwrap();
    
    let root_path = cgroup_root.to_string_lossy().to_string();
    let stats = &metrics.resource_usage[&root_path];
    
    // Should have parsed memory.current but others should be default
    assert_eq!(stats.memory.current, 1048576);
    assert_eq!(stats.memory.max, None); // Default when file is missing
    assert_eq!(stats.cpu.usage_usec, 0); // Default
    assert_eq!(stats.pids.current, 0); // Default
}

#[test]
fn test_collect_metrics_nonexistent_path() {
    let collector = CGroupCollector::new(PathBuf::from("/nonexistent/path"));
    let result = collector.collect_metrics();
    
    // Should fail gracefully when path doesn't exist
    assert!(result.is_err());
}

#[test]
fn test_resource_stats_defaults() {
    let stats = ResourceStats::default();
    
    assert_eq!(stats.memory.current, 0);
    assert_eq!(stats.memory.max, None);
    assert_eq!(stats.cpu.usage_usec, 0);
    assert_eq!(stats.cpu.user_usec, 0);
    assert_eq!(stats.cpu.system_usec, 0);
    assert_eq!(stats.io.rbytes, 0);
    assert_eq!(stats.io.wbytes, 0);
    assert_eq!(stats.io.rios, 0);
    assert_eq!(stats.io.wios, 0);
    assert_eq!(stats.pids.current, 0);
    assert_eq!(stats.pids.max, None);
}

#[test]
fn test_malformed_file_content() {
    let temp_dir = TempDir::new().unwrap();
    let cgroup_root = temp_dir.path().join("malformed_cgroup");
    fs::create_dir_all(&cgroup_root).unwrap();
    
    // Create files with malformed content
    fs::write(cgroup_root.join("memory.current"), "not_a_number").unwrap();
    fs::write(cgroup_root.join("cpu.stat"), "invalid format\nno equals signs").unwrap();
    fs::write(cgroup_root.join("io.stat"), "8:0 malformed_key_value").unwrap();
    fs::write(cgroup_root.join("pids.current"), "also_not_a_number").unwrap();
    
    let collector = CGroupCollector::new(cgroup_root.clone());
    let result = collector.collect_metrics();
    
    // Should succeed but with default values due to parse errors
    assert!(result.is_ok());
    let metrics = result.unwrap();
    
    let root_path = cgroup_root.to_string_lossy().to_string();
    let stats = &metrics.resource_usage[&root_path];
    
    // All values should be defaults due to parse failures
    assert_eq!(stats.memory.current, 0);
    assert_eq!(stats.cpu.usage_usec, 0);
    assert_eq!(stats.io.rbytes, 0);
    assert_eq!(stats.pids.current, 0);
}