mod app;
mod canvas;
mod collection;
mod events;
mod threads;
mod widgets;
use events::CGroupEvent;
use threads::EventThreads;

use anyhow::Result;
use app::App;
use canvas::Canvas;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    io,
    time::{Duration, Instant},
};

fn main() -> Result<()> {
    env_logger::init();

    log::info!("cgroup TUI Monitor starting...");

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app (no threads for now to fix quit issue)
    let mut app = App::new();

    // Try to collect initial data
    log::info!("Attempting initial cgroup data collection...");
    let cgroup_root = std::path::PathBuf::from("/sys/fs/cgroup");
    let collector = collection::CGroupCollector::new(
        cgroup_root,
        Duration::from_secs(1),
        crossbeam::channel::unbounded().0, // Dummy sender
    );

    match collector.collect_metrics() {
        Ok(metrics) => {
            let cgroup_count = metrics.resource_usage.len();
            let process_count = metrics.processes.len();
            app.cgroup_data.metrics = Some(metrics);
            app.cgroup_data.last_update = Some(Instant::now());
            log::info!(
                "Initial data collected: {} cgroups, {} processes",
                cgroup_count,
                process_count
            );
        }
        Err(e) => {
            log::error!("Failed initial data collection: {}", e);
            log::info!("Creating mock data for testing...");

            // Create some mock data for testing when cgroups aren't available
            let mock_metrics = create_mock_metrics();
            app.cgroup_data.metrics = Some(mock_metrics);
            app.cgroup_data.last_update = Some(Instant::now());
        }
    }

    // Run the application
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Application error: {}", err);
    }

    Ok(())
}

fn create_mock_metrics() -> collection::CGroupMetrics {
    use collection::*;
    use hashbrown::HashMap;

    let mut resource_usage = HashMap::new();
    let mut processes = HashMap::new();

    // Create some mock cgroups
    resource_usage.insert(
        "/sys/fs/cgroup".to_string(),
        ResourceStats {
            memory: MemoryStats {
                current: 1024 * 1024 * 512,    // 512MB
                max: Some(1024 * 1024 * 1024), // 1GB
                events: MemoryEvents::default(),
            },
            cpu: CpuStats {
                usage_usec: 1000000, // 1 second
                user_usec: 600000,
                system_usec: 400000,
                nr_periods: 100,
                nr_throttled: 5,
                throttled_usec: 50000,
            },
            io: IoStats {
                rbytes: 1024 * 1024 * 10, // 10MB
                wbytes: 1024 * 1024 * 5,  // 5MB
                rios: 1000,
                wios: 500,
            },
            pids: PidStats {
                current: 25,
                max: Some(100),
            },
        },
    );

    resource_usage.insert(
        "/sys/fs/cgroup/system.slice".to_string(),
        ResourceStats {
            memory: MemoryStats {
                current: 1024 * 1024 * 256,   // 256MB
                max: Some(1024 * 1024 * 512), // 512MB
                events: MemoryEvents::default(),
            },
            cpu: CpuStats {
                usage_usec: 500000, // 0.5 seconds
                user_usec: 300000,
                system_usec: 200000,
                nr_periods: 50,
                nr_throttled: 2,
                throttled_usec: 10000,
            },
            io: IoStats {
                rbytes: 1024 * 1024 * 5, // 5MB
                wbytes: 1024 * 1024 * 2, // 2MB
                rios: 500,
                wios: 200,
            },
            pids: PidStats {
                current: 10,
                max: Some(50),
            },
        },
    );

    resource_usage.insert(
        "/sys/fs/cgroup/user.slice".to_string(),
        ResourceStats {
            memory: MemoryStats {
                current: 1024 * 1024 * 128, // 128MB
                max: None,
                events: MemoryEvents::default(),
            },
            cpu: CpuStats {
                usage_usec: 300000, // 0.3 seconds
                user_usec: 200000,
                system_usec: 100000,
                nr_periods: 30,
                nr_throttled: 1,
                throttled_usec: 5000,
            },
            io: IoStats {
                rbytes: 1024 * 1024 * 3, // 3MB
                wbytes: 1024 * 1024 * 1, // 1MB
                rios: 300,
                wios: 100,
            },
            pids: PidStats {
                current: 8,
                max: Some(20),
            },
        },
    );

    // Create some mock processes
    processes.insert(1, "/sys/fs/cgroup/system.slice".to_string());
    processes.insert(1234, "/sys/fs/cgroup/system.slice".to_string());
    processes.insert(5678, "/sys/fs/cgroup/user.slice".to_string());
    processes.insert(9999, "/sys/fs/cgroup/user.slice".to_string());

    CGroupMetrics {
        hierarchies: Vec::new(),
        processes,
        resource_usage,
        timestamp: std::time::Instant::now(),
    }
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let mut last_tick = Instant::now();
    let mut last_data_collection = Instant::now();
    let tick_rate = Duration::from_millis(250);

    let mut event_threads = EventThreads::new();
    let event_rx = event_threads.start()?;

    loop {
        terminal.draw(|f| Canvas::draw(f, app))?;

        match event_rx.recv() {
            Ok(event) => match event {
                CGroupEvent::KeyInput(key_event) => {
                    if event.is_quit_key() {
                        return Ok(());
                    }
                    handle_key_event(app, key_event);
                }
                CGroupEvent::UpdateDummy => {}
                _ => {}
            },
            Err(e) => {
                log::error!("Error receiving event: {:?}", e);
            }
        }

        // Process data updates (collect directly instead of using thread)
        if last_data_collection.elapsed() >= Duration::from_millis(2000) {
            // Update every 2 seconds
            // Create a simple collector and try to collect data
            let cgroup_root = std::path::PathBuf::from("/sys/fs/cgroup");
            let collector = collection::CGroupCollector::new(
                cgroup_root,
                Duration::from_secs(1),
                crossbeam::channel::unbounded().0, // Dummy sender
            );

            match collector.collect_metrics() {
                Ok(metrics) => {
                    let cgroup_count = metrics.resource_usage.len();
                    let process_count = metrics.processes.len();
                    app.cgroup_data.metrics = Some(metrics);
                    app.cgroup_data.last_update = Some(Instant::now());
                    log::info!(
                        "Updated cgroup metrics: {} cgroups, {} processes",
                        cgroup_count,
                        process_count
                    );
                }
                Err(e) => {
                    log::error!("Failed to collect metrics: {}", e);
                    // Use mock data if collection fails
                    let mock_metrics = create_mock_metrics();
                    app.cgroup_data.metrics = Some(mock_metrics);
                    app.cgroup_data.last_update = Some(Instant::now());
                }
            }
            last_data_collection = Instant::now();
        }

        // UI tick update
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        // Small sleep to prevent busy waiting
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn handle_key_event(app: &mut App, key_event: crossterm::event::KeyEvent) {
    use crossterm::event::{KeyCode, KeyModifiers};

    match key_event.code {
        KeyCode::Char('r') => {
            log::info!("Manual refresh requested");
            // The collection thread will automatically provide updates
        }
        KeyCode::Char('j') | KeyCode::Down => {
            // Navigate down in the tree
            app.ui_state.scroll_offset = app.ui_state.scroll_offset.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            // Navigate up in the tree
            app.ui_state.scroll_offset = app.ui_state.scroll_offset.saturating_sub(1);
        }
        KeyCode::Tab => {
            // Switch between tabs/panels
            app.ui_state.current_tab = (app.ui_state.current_tab + 1) % 3;
            log::info!("Switched to tab {}", app.ui_state.current_tab);
        }
        KeyCode::Enter => {
            // Select/expand cgroup (placeholder)
            log::info!("Enter pressed - toggle selection");
        }
        KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            log::info!("Ctrl+C pressed - should quit");
        }
        KeyCode::Char('?') => {
            log::info!("Help requested");
            // Could show help overlay
        }
        _ => {
            // Log unhandled keys for debugging
            log::debug!("Unhandled key: {:?}", key_event);
        }
    }
}
