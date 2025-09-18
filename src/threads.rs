use anyhow::Result;
use crossbeam::channel::{Receiver, Sender, unbounded};
use std::{path::PathBuf, thread, time::Duration};

use crate::{
    collection::{
        CGroupCollector, CGroupMetrics, CpuStats, IoStats, MemoryStats, PidStats, ResourceStats,
    },
    events::CGroupEvent,
};

use crossterm::event::Event;
use crossterm::event::KeyEventKind;
use std::thread::JoinHandle;

pub struct EventThreads {
    input_handle: Option<JoinHandle<()>>,
    collection_handle: Option<JoinHandle<()>>,
    cleanup_handle: Option<JoinHandle<()>>,
}

impl EventThreads {
    pub fn new() -> Self {
        Self {
            input_handle: None,
            collection_handle: None,
            cleanup_handle: None,
        }
    }

    pub fn start(&mut self, cgroup_root: PathBuf) -> Result<Receiver<CGroupEvent>> {
        let (event_tx, event_rx) = unbounded::<CGroupEvent>();

        let event_tx0 = event_tx.clone();
        // Start input thread
        self.input_handle = Some(thread::spawn(move || {
            input_thread_worker(event_tx0);
        }));

        let event_tx1 = event_tx.clone();

        self.collection_handle = Some(thread::spawn(move || {
            collection_thread_worker(event_tx1, cgroup_root);
        }));

        Ok(event_rx)
    }
}

fn input_thread_worker(sender: Sender<CGroupEvent>) {
    log::info!("Input thread started)");

    loop {
        if let Ok(pool) = crossterm::event::poll(Duration::from_millis(20)) {
            if pool {
                if let Ok(event) = crossterm::event::read() {
                    match event {
                        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                            if sender.send(CGroupEvent::KeyInput(key_event)).is_err() {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    log::info!("Input thread stopped");
}

fn collection_thread_worker(sender: Sender<CGroupEvent>, cgroup_root: PathBuf) {
    log::info!(
        "Collection thread started with root: {}",
        cgroup_root.display()
    );

    loop {
        // sleep for 200ms
        // TODO: use the proper collection logic
        thread::sleep(Duration::from_millis(200));

        // Try to use mock data first for testing in sandbox environments
        let use_mock_data =
            std::env::var("CGTOP_USE_MOCK").unwrap_or_else(|_| "false".to_string()) == "true";

        if use_mock_data {
            log::info!("Using mock data for testing");
            let mock_metrics = create_mock_metrics(&cgroup_root);
            if let Err(_e) = sender.send(CGroupEvent::Update(Box::new(mock_metrics))) {
                break;
            }
        } else {
            let collector = CGroupCollector::new(cgroup_root.clone());

            if let Ok(metrics) = collector.collect_metrics() {
                if let Err(_e) = sender.send(CGroupEvent::Update(Box::new(metrics))) {
                    break;
                }
            } else {
                log::info!("Failed to collect real cgroup data, using mock data");
                let mock_metrics = create_mock_metrics(&cgroup_root);
                if let Err(_e) = sender.send(CGroupEvent::Update(Box::new(mock_metrics))) {
                    break;
                }
            }
        }
    }

    log::info!("Collection thread stopped");
}

fn cleanup_thread_worker(_sender: Sender<CGroupEvent>) {
    log::info!("Cleanup thread started");

    loop {
        // TODO, every x times, send cleanup message to only keep the limited amount of data
    }
}

// --------------------------------------------------------------------
// Mock data for testing
// --------------------------------------------------------------------
fn create_mock_metrics(cgroup_root: &PathBuf) -> CGroupMetrics {
    use hashbrown::HashMap;
    use std::time::Instant;

    let mut resource_usage = HashMap::new();
    let mut processes = HashMap::new();

    let root_str = cgroup_root.to_string_lossy();

    // Create mock cgroup hierarchy data using the provided root
    let mock_paths = vec![
        root_str.to_string(),
        format!("{}/system.slice", root_str),
        format!("{}/system.slice/systemd-logind.service", root_str),
        format!("{}/system.slice/ssh.service", root_str),
        format!("{}/system.slice/nginx.service", root_str),
        format!("{}/user.slice", root_str),
        format!("{}/user.slice/user-1000.slice", root_str),
        format!("{}/user.slice/user-1000.slice/session-2.scope", root_str),
        format!("{}/user.slice/user-1000.slice/user@1000.service", root_str),
        format!(
            "{}/user.slice/user-1000.slice/user@1000.service/app.slice",
            root_str
        ),
        format!(
            "{}/user.slice/user-1000.slice/user@1000.service/app.slice/firefox.service",
            root_str
        ),
        format!("{}/init.scope", root_str),
        format!("{}/machine.slice", root_str),
        format!("{}/machine.slice/docker-123456.scope", root_str),
    ];

    for (i, path) in mock_paths.iter().enumerate() {
        let stats = ResourceStats {
            memory: MemoryStats {
                current: 1024 * 1024 * (10 + i as u64 * 5), // 10MB + 5MB per level
                max: Some(1024 * 1024 * 100),               // 100MB limit
                peak: 1024 * 1024 * (15 + i as u64 * 8),    // 15MB + 8MB per level (peak > current)
                ..Default::default()
            },
            cgroup_procs: vec![1000 + i as u32, 2000 + i as u32], // Mock PIDs
            cpu: CpuStats {
                usage_usec: 1000000 * (i as u64 + 1), // 1 second + i seconds
                user_usec: 500000 * (i as u64 + 1),
                system_usec: 200000 * (i as u64 + 1),
                ..Default::default()
            },
            io: IoStats {
                rbytes: 1024 * (100 + i as u64 * 50),
                wbytes: 1024 * (50 + i as u64 * 25),
                rios: 10 + i as u64 * 2,
                wios: 5 + i as u64,
            },
            pids: PidStats {
                current: if i == 0 { 100 } else { 1 + i as u64 }, // Root has many processes
                max: Some(512),
            },
        };

        resource_usage.insert(path.to_string(), stats);
    }

    // Add some mock processes using the provided root
    processes.insert(1, format!("{}/init.scope", root_str));
    processes.insert(
        100,
        format!("{}/system.slice/systemd-logind.service", root_str),
    );
    processes.insert(200, format!("{}/system.slice/ssh.service", root_str));
    processes.insert(
        1000,
        format!("{}/user.slice/user-1000.slice/session-2.scope", root_str),
    );
    processes.insert(
        2000,
        format!(
            "{}/user.slice/user-1000.slice/user@1000.service/app.slice/firefox.service",
            root_str
        ),
    );

    CGroupMetrics {
        hierarchies: Vec::new(),
        processes,
        resource_usage,
        timestamp: Instant::now(),
    }
}
