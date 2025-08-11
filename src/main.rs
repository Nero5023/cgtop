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

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
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
                CGroupEvent::Update(metrics) => {
                    let cgroup_count = metrics.resource_usage.len();
                    let process_count = metrics.processes.len();

                    // Update tree state with new data
                    app.ui_state
                        .tree_state
                        .build_from_paths(&metrics.resource_usage);

                    // log::info!("metrics.resource_usage: {:?}", metrics.resource_usage);

                    app.cgroup_data.metrics = Some(metrics);
                    app.cgroup_data.last_update = Some(Instant::now());

                    log::info!(
                        "Updated cgroup metrics: {} cgroups, {} processes",
                        cgroup_count,
                        process_count
                    );
                }
                CGroupEvent::UpdateDummy => {}
                _ => {}
            },
            Err(e) => {
                log::error!("Error receiving event: {:?}", e);
            }
        }
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
            app.ui_state.tree_state.select_next();
            // Update selected cgroup for resource display
            app.ui_state.selected_cgroup = app
                .ui_state
                .tree_state
                .selected
                .clone()
                .and_then(|path| app.ui_state.tree_state.nodes.get(&path))
                .map(|node| node.path.clone());
        }
        KeyCode::Char('k') | KeyCode::Up => {
            // Navigate up in the tree
            app.ui_state.tree_state.select_previous();
            // Update selected cgroup for resource display
            app.ui_state.selected_cgroup = app
                .ui_state
                .tree_state
                .selected
                .clone()
                .and_then(|path| app.ui_state.tree_state.nodes.get(&path))
                .map(|node| node.path.clone());
        }
        KeyCode::Tab => {
            // Switch between tabs/panels
            app.ui_state.current_tab = (app.ui_state.current_tab + 1) % 3;
            log::info!("Switched to tab {}", app.ui_state.current_tab);
        }
        KeyCode::Enter | KeyCode::Right => {
            // Expand/collapse selected node
            if let Some(selected) = app.ui_state.tree_state.selected.clone() {
                app.ui_state.tree_state.toggle_expand(&selected);
                log::info!("Toggled expand for: {}", selected);
            }
        }
        KeyCode::Left => {
            // Collapse selected node
            if let Some(selected) = app.ui_state.tree_state.selected.clone() {
                if let Some(node) = app.ui_state.tree_state.nodes.get_mut(&selected) {
                    if node.expanded {
                        app.ui_state.tree_state.toggle_expand(&selected);
                        log::info!("Collapsed: {}", selected);
                    }
                }
            }
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
