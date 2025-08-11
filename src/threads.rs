use anyhow::Result;
use crossbeam::channel::{Receiver, Sender, unbounded};
use std::{
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use crate::{
    app::InputEvent,
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

    pub fn start(&mut self) -> Result<Receiver<CGroupEvent>> {
        let (event_tx, event_rx) = unbounded::<CGroupEvent>();

        let event_tx0 = event_tx.clone();
        // Start input thread
        self.input_handle = Some(thread::spawn(move || {
            input_thread_worker(event_tx0);
        }));

        let event_tx1 = event_tx.clone();

        self.collection_handle = Some(thread::spawn(move || {
            collection_thread_worker(event_tx1);
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

fn collection_thread_worker(sender: Sender<CGroupEvent>) {
    log::info!("Collection thread started");

    loop {
        // sleep for 200ms
        // TODO: use the proper collection logic
        thread::sleep(Duration::from_millis(200));

        // Try to use mock data first for testing in sandbox environments
        let use_mock_data =
            std::env::var("CGTOP_USE_MOCK").unwrap_or_else(|_| "false".to_string()) == "true";

        if use_mock_data {
            log::info!("Using mock data for testing");
            let mock_metrics = create_mock_metrics();
            if let Err(_e) = sender.send(CGroupEvent::Update(Box::new(mock_metrics))) {
                break;
            }
        } else {
            let collector = CGroupCollector::new(PathBuf::from("/sys/fs/cgroup"));

            if let Ok(metrics) = collector.collect_metrics() {
                if let Err(_e) = sender.send(CGroupEvent::Update(Box::new(metrics))) {
                    break;
                }
            } else {
                log::info!("Failed to collect real cgroup data, using mock data");
                let mock_metrics = create_mock_metrics();
                if let Err(_e) = sender.send(CGroupEvent::Update(Box::new(mock_metrics))) {
                    break;
                }
            }
        }
    }

    log::info!("Collection thread stopped");
}

fn cleanup_thread_worker(sender: Sender<CGroupEvent>) {
    log::info!("Cleanup thread started");

    loop {
        // TODO, every x times, send cleanup message to only keep the limited amount of data
    }

    log::info!("Cleanup thread stopped");
}

// --------------------------------------------------------------------
// Mock data for testing
// --------------------------------------------------------------------
fn create_mock_metrics() -> CGroupMetrics {
    use hashbrown::HashMap;
    use std::time::Instant;

    let mut resource_usage = HashMap::new();
    let mut processes = HashMap::new();

    // Create mock cgroup hierarchy data
    let mock_paths = vec![
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
        "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/app.slice/firefox.service",
        "/sys/fs/cgroup/init.scope",
        "/sys/fs/cgroup/machine.slice",
        "/sys/fs/cgroup/machine.slice/docker-123456.scope",
    ];

    for (i, path) in mock_paths.iter().enumerate() {
        let stats = ResourceStats {
            memory: MemoryStats {
                current: 1024 * 1024 * (10 + i as u64 * 5), // 10MB + 5MB per level
                max: Some(1024 * 1024 * 100),               // 100MB limit
                ..Default::default()
            },
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

    // Add some mock processes
    processes.insert(1, "/sys/fs/cgroup/init.scope".to_string());
    processes.insert(
        100,
        "/sys/fs/cgroup/system.slice/systemd-logind.service".to_string(),
    );
    processes.insert(200, "/sys/fs/cgroup/system.slice/ssh.service".to_string());
    processes.insert(
        1000,
        "/sys/fs/cgroup/user.slice/user-1000.slice/session-2.scope".to_string(),
    );
    processes.insert(
        2000,
        "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/app.slice/firefox.service"
            .to_string(),
    );

    CGroupMetrics {
        hierarchies: Vec::new(),
        processes,
        resource_usage,
        timestamp: Instant::now(),
    }
}
