use cgtop::collection::{CpuStats, IoStats, MemoryStats, PidStats, ResourceStats};
use hashbrown::HashMap;

/// Create mock resource stats for testing
pub fn create_mock_resource_stats() -> ResourceStats {
    ResourceStats {
        memory: MemoryStats {
            current: 1024 * 1024,        // 1MB
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
    let paths = vec![
        "/sys/fs/cgroup",
        "/sys/fs/cgroup/system.slice",
        "/sys/fs/cgroup/system.slice/systemd-logind.service",
        "/sys/fs/cgroup/system.slice/ssh.service",
        "/sys/fs/cgroup/system.slice/nginx.service",
        "/sys/fs/cgroup/user.slice",
        "/sys/fs/cgroup/user.slice/user-1000.slice",
        "/sys/fs/cgroup/user.slice/user-1000.slice/session-2.scope",
        "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service",
        "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/app.slice",
        "/sys/fs/cgroup/init.scope",
    ];

    paths
        .into_iter()
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
    let paths = vec![
        "/sys/fs/cgroup",
        "/sys/fs/cgroup/test1",
        "/sys/fs/cgroup/test2",
        "/sys/fs/cgroup/test1/child1",
        "/sys/fs/cgroup/test1/child2",
    ];

    paths
        .into_iter()
        .map(|path| (path.to_string(), create_mock_resource_stats()))
        .collect()
}
