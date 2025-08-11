use anyhow::Result;
use crossbeam::channel::{Receiver, Sender, unbounded};
use std::{
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use crate::{
    app::InputEvent,
    collection::{CGroupCollector, CGroupMetrics},
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
            loop {
                // sleep for 100ms
                // TODO: use the proper collection logic
                thread::sleep(Duration::from_millis(200));

                let collector = CGroupCollector::new(PathBuf::from("/sys/fs/cgroup"));

                if let Ok(metrics) = collector.collect_metrics() {
                    // TODO: handle metrics
                    if let Err(e) = event_tx1.send(CGroupEvent::Update(Box::new(metrics))) {
                        break;
                    }
                } else {
                    // TODO: handle error
                }
            }
        }));

        Ok(event_rx)
    }
}

pub struct ThreadManager {
    pub input_handle: Option<thread::JoinHandle<()>>,
    pub collection_handle: Option<thread::JoinHandle<()>>,
    pub cleanup_handle: Option<thread::JoinHandle<()>>,
}

impl ThreadManager {
    pub fn new() -> Self {
        Self {
            input_handle: None,
            collection_handle: None,
            cleanup_handle: None,
        }
    }

    pub fn start_threads(&mut self) -> Result<(Receiver<InputEvent>, Receiver<CGroupMetrics>)> {
        // Create channels
        let (input_tx, input_rx) = unbounded::<InputEvent>();
        let (data_tx, data_rx) = unbounded::<CGroupMetrics>();
        let (cleanup_tx, cleanup_rx) = unbounded::<CleanupMessage>();

        // Start input thread
        self.input_handle = Some(thread::spawn(move || {
            // input_thread_worker(input_tx);
        }));

        // Start collection thread
        let data_tx_clone = data_tx.clone();
        let cleanup_tx_clone = cleanup_tx.clone();
        self.collection_handle = Some(thread::spawn(move || {
            collection_thread_worker(data_tx_clone, cleanup_tx_clone);
        }));

        // Start cleanup thread
        self.cleanup_handle = Some(thread::spawn(move || {
            cleanup_thread_worker(cleanup_rx);
        }));

        Ok((input_rx, data_rx))
    }

    pub fn stop_threads(mut self) {
        if let Some(handle) = self.input_handle.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.collection_handle.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.cleanup_handle.take() {
            let _ = handle.join();
        }
    }
}

#[derive(Debug)]
pub enum CleanupMessage {
    OldData(Instant),
    Shutdown,
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

fn collection_thread_worker(
    data_sender: Sender<CGroupMetrics>,
    cleanup_sender: Sender<CleanupMessage>,
) {
    log::info!("Collection thread started");

    let cgroup_root = PathBuf::from("/sys/fs/cgroup");
    let collector = CGroupCollector::new(cgroup_root);

    let mut last_collection = Instant::now();
    let mut last_cleanup_signal = Instant::now();

    loop {
        let now = Instant::now();

        // Collect data at regular intervals
        if now.duration_since(last_collection) >= Duration::from_secs(1) {
            match collector.collect_metrics() {
                Ok(metrics) => {
                    if let Err(e) = data_sender.send(metrics) {
                        log::error!("Failed to send metrics: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    log::warn!("Failed to collect metrics: {}", e);
                }
            }
            last_collection = now;
        }

        // Signal cleanup thread every 30 seconds
        if now.duration_since(last_cleanup_signal) >= Duration::from_secs(30) {
            let cutoff_time = now - Duration::from_secs(300); // Keep 5 minutes of data
            let _ = cleanup_sender.send(CleanupMessage::OldData(cutoff_time));
            last_cleanup_signal = now;
        }

        // Sleep briefly to prevent busy waiting
        thread::sleep(Duration::from_millis(100));
    }

    let _ = cleanup_sender.send(CleanupMessage::Shutdown);
    log::info!("Collection thread stopped");
}

fn cleanup_thread_worker(receiver: Receiver<CleanupMessage>) {
    log::info!("Cleanup thread started");

    loop {
        match receiver.recv() {
            Ok(CleanupMessage::OldData(cutoff_time)) => {
                // In a real implementation, this would clean up old data
                // from a data store or cache
                log::debug!("Cleaning up data older than {:?}", cutoff_time);
            }
            Ok(CleanupMessage::Shutdown) => {
                break;
            }
            Err(e) => {
                log::error!("Cleanup thread receiver error: {}", e);
                break;
            }
        }
    }

    log::info!("Cleanup thread stopped");
}
