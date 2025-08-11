use cgtop::collection::{ResourceStats, MemoryStats, CpuStats, IoStats, PidStats};
use hashbrown::HashMap;

/// Create mock resource stats for testing
pub fn create_mock_resource_stats() -> ResourceStats {
    ResourceStats {
        memory: MemoryStats {
            current: 1024 * 1024, // 1MB
            max: Some(1024 * 1024 * 10), // 10MB
            ..Default::default()
        },
        cpu: CpuStats {
            usage_usec: 1000000, // 1 second
            user_usec: 500000,
            system_usec: 200000,
            ..Default::default()
        },
        io: IoStats {
            rbytes: 1024,
            wbytes: 512,
            rios: 10,
            wios: 5,
        },
        pids: PidStats {
            current: 1,
            max: Some(100),
        },
    }
}

/// Create a set of mock cgroup paths for testing tree operations
pub fn create_test_cgroup_paths() -> HashMap<String, ResourceStats> {
    create_test_cgroup_paths_with_root("/sys/fs/cgroup")
}

/// Create a set of mock cgroup paths for testing tree operations with custom root
pub fn create_test_cgroup_paths_with_root(root: &str) -> HashMap<String, ResourceStats> {
    let paths = vec![
        root.to_string(),
        format!("{}/system.slice", root),
        format!("{}/system.slice/systemd-logind.service", root),
        format!("{}/system.slice/ssh.service", root),
        format!("{}/system.slice/nginx.service", root),
        format!("{}/user.slice", root),
        format!("{}/user.slice/user-1000.slice", root),
        format!("{}/user.slice/user-1000.slice/session-2.scope", root),
        format!("{}/user.slice/user-1000.slice/user@1000.service", root),
        format!("{}/user.slice/user-1000.slice/user@1000.service/app.slice", root),
        format!("{}/init.scope", root),
    ];

    paths.into_iter()
        .enumerate()
        .map(|(i, path)| {
            let mut stats = create_mock_resource_stats();
            // Vary the stats slightly for each cgroup
            stats.memory.current += i as u64 * 1024;
            stats.cpu.usage_usec += i as u64 * 100000;
            (path.to_string(), stats)
        })
        .collect()
}

/// Create a simple flat cgroup hierarchy for basic tests
pub fn create_simple_cgroup_paths() -> HashMap<String, ResourceStats> {
    create_simple_cgroup_paths_with_root("/sys/fs/cgroup")
}

/// Create a simple flat cgroup hierarchy for basic tests with custom root
pub fn create_simple_cgroup_paths_with_root(root: &str) -> HashMap<String, ResourceStats> {
    let paths = vec![
        root.to_string(),
        format!("{}/test1", root),
        format!("{}/test2", root),
        format!("{}/test1/child1", root),
        format!("{}/test1/child2", root),
    ];

    paths.into_iter()
        .map(|path| (path.to_string(), create_mock_resource_stats()))
        .collect()
}