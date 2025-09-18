mod app;
mod canvas;
mod collection;
mod events;
mod notifications;
mod threads;
mod widgets;
use events::CGroupEvent;
use threads::EventThreads;

use anyhow::{Context, Result};
use app::App;
use canvas::Canvas;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use env_logger::{Env, Target, WriteStyle};
use log::LevelFilter;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    env,
    fs::OpenOptions,
    io,
    path::{Path, PathBuf},
    time::Instant,
};

// ===================================Set up logging=============================================

const PRIMARY_LOG_PATH: &str = "/var/log/cgtop.log";

fn fallback_log_path() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(env::temp_dir)
        .join("cgtop")
        .join("cgtop.log")
}

fn open_log_file(path: &Path) -> std::io::Result<std::fs::File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    OpenOptions::new().create(true).append(true).open(path)
}

fn init_logging(verbose: bool) -> Result<PathBuf> {
    let primary_path = PathBuf::from(PRIMARY_LOG_PATH);

    let (log_file, resolved_path, used_fallback) = match open_log_file(&primary_path) {
        Ok(file) => (file, primary_path.clone(), false),
        Err(primary_error) => {
            let fallback_path = fallback_log_path();
            let file = open_log_file(&fallback_path).with_context(|| {
                format!(
                    "failed to open primary log file {} (error: {}) and fallback {}",
                    primary_path.display(),
                    primary_error,
                    fallback_path.display()
                )
            })?;

            (file, fallback_path, true)
        }
    };

    let default_filter = if verbose { "debug" } else { "info" };
    let env = Env::default().default_filter_or(default_filter);
    let mut builder = env_logger::Builder::from_env(env);

    if env::var_os("RUST_LOG").is_none() {
        builder.filter_level(if verbose {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        });
    }

    builder
        .write_style(WriteStyle::Never)
        .format_timestamp_secs()
        .target(Target::Pipe(Box::new(log_file)));

    builder.init();

    if used_fallback {
        log::warn!(
            "Falling back to log file at {} because {} was unavailable",
            resolved_path.display(),
            PRIMARY_LOG_PATH
        );
    } else {
        log::info!("Logging to {}", resolved_path.display());
    }

    Ok(resolved_path)
}

// ===================================================================================================================

// we need to normalize the path to remove the trailing slash
// because the path is used as a prefix for other paths
fn normalize_path(s: &str) -> Result<PathBuf, String> {
    Ok(PathBuf::from(s.trim_end_matches('/')))
}

#[derive(Parser)]
#[command(name = "cgtop")]
#[command(about = "A top-like utility for cgroup v2 hierarchies")]
#[command(version = "0.1.0")]
struct Cli {
    /// Path to the cgroup filesystem root
    #[arg(long, short, default_value = "/sys/fs/cgroup", value_parser = normalize_path)]
    path: PathBuf,

    /// Enable verbose logging
    #[arg(long, short)]
    verbose: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    init_logging(cli.verbose)?;

    log::info!(
        "cgroup TUI Monitor starting with root path: {}",
        cli.path.display()
    );

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app with custom cgroup path
    let mut app = App::new_with_path(cli.path);

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
    let event_rx = event_threads.start(app.config.cgroup_root.clone())?;

    loop {
        // Update notifications (remove expired ones)
        app.update_notifications();

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
        KeyCode::Char('D') => {
            if let Some(selected_key) = &app.ui_state.tree_state.selected {
                if let Some(node) = app.ui_state.tree_state.nodes.get(selected_key) {
                    let parent_key = selected_key
                        .rsplit_once('/')
                        .map(|(parent, _)| parent.to_string())
                        .unwrap_or_default();

                    if parent_key.is_empty() {
                        let root_path = app.ui_state.tree_state.root_path_string();
                        let warning = format!("Cannot clean the root cgroup ({})", root_path);
                        log::warn!("{}", warning);
                        app.show_warning(warning);
                    } else if let Some(parent_node) = app.ui_state.tree_state.nodes.get(&parent_key)
                    {
                        let parent_path = parent_node.path.clone();
                        log::info!(
                            "Clean parent requested for cgroup: {} (selected child: {})",
                            parent_path,
                            node.path
                        );
                        handle_delete_cgroup(app, &parent_path);
                    } else {
                        let warning = format!("Parent cgroup not found for {}", node.path);
                        log::warn!("{}", warning);
                        app.show_warning(warning);
                    }
                }
            }
        }
        KeyCode::Char('d') => {
            // Execute recursive directory removal
            if let Some(selected) = &app.ui_state.tree_state.selected {
                if let Some(node) = app.ui_state.tree_state.nodes.get(selected) {
                    let path = node.path.clone();
                    handle_delete_cgroup(app, &path);
                }
            }
        }
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
        KeyCode::Enter | KeyCode::Right | KeyCode::Char(' ') => {
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

fn handle_delete_cgroup(app: &mut app::App, cgroup_path: &str) {
    use cgtop::utils::{is_safe_to_remove, remove_dir_recursive_safe};

    log::info!("Delete requested for cgroup: {}", cgroup_path);

    // Safety check
    if !is_safe_to_remove(cgroup_path) {
        let error_msg = format!("Unsafe path: {}", cgroup_path);
        log::error!("Refusing to delete unsafe path: {}", cgroup_path);
        app.show_error(error_msg);
        return;
    }

    // Attempt to remove the directory
    match remove_dir_recursive_safe(cgroup_path) {
        Ok(_) => {
            let success_msg = format!("Deleted: {}", cgroup_path);
            log::info!("Successfully deleted cgroup directory: {}", cgroup_path);
            app.show_success(success_msg);
        }
        Err(e) => {
            let error_msg = format!("Delete failed: {}", e);
            log::error!("Failed to delete cgroup directory {}: {}", cgroup_path, e);
            app.show_error(error_msg);
        }
    }
}
