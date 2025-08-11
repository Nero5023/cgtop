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

        let collector = CGroupCollector::new(PathBuf::from("/sys/fs/cgroup"));

        if let Ok(metrics) = collector.collect_metrics() {
            // TODO: handle metrics
            if let Err(_e) = sender.send(CGroupEvent::Update(Box::new(metrics))) {
                break;
            }
        } else {
            // TODO: handle error
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
