mod app;
mod collection;
mod canvas;
mod widgets;
mod threads;

use anyhow::Result;
use app::{App, InputEvent};
use canvas::Canvas;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::{
    io,
    time::{Duration, Instant},
};
use threads::ThreadManager;

fn main() -> Result<()> {
    env_logger::init();
    
    log::info!("cgroup TUI Monitor starting...");
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and thread manager
    let mut app = App::new();
    let mut thread_manager = ThreadManager::new();
    
    // Start background threads
    let (input_rx, data_rx) = thread_manager.start_threads()?;
    app.set_channels(input_rx, data_rx);
    
    // Run the application
    let result = run_app(&mut terminal, &mut app);
    
    // Stop threads
    thread_manager.stop_threads();

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
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(250);

    loop {
        terminal.draw(|f| Canvas::draw(f, app))?;

        // Process input events
        if let Some(ref input_rx) = app.input_receiver {
            if let Ok(input_event) = input_rx.try_recv() {
                match input_event {
                    InputEvent::Quit => return Ok(()),
                    InputEvent::Key(key_event) => {
                        handle_key_event(app, key_event);
                    }
                    InputEvent::Resize(w, h) => {
                        log::info!("Resize event: {}x{}", w, h);
                    }
                }
            }
        }

        // Process data updates
        if let Some(ref data_rx) = app.data_receiver {
            if let Ok(metrics) = data_rx.try_recv() {
                app.cgroup_data.metrics = Some(metrics);
                app.cgroup_data.last_update = Some(Instant::now());
                log::debug!("Updated cgroup metrics");
            }
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
