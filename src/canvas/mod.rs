use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::widgets::{CGroupTreeWidget, ResourceGraphWidget};

pub struct Canvas;

impl Canvas {
    pub fn draw(f: &mut Frame, app: &mut App) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title bar
                Constraint::Min(0),     // Main content
                Constraint::Length(3),  // Status bar
            ])
            .split(f.area());

        Self::draw_title_bar(f, app, chunks[0]);
        Self::draw_main_content(f, app, chunks[1]);
        Self::draw_status_bar(f, app, chunks[2]);
    }

    fn draw_title_bar(f: &mut Frame, app: &mut App, area: Rect) {
        // Truncate long paths to keep title readable
        let root_path = app.config.cgroup_root.display().to_string();
        
        let title_line = Line::from(vec![
            Span::styled(
                "cgroup Monitor v0.1.0 - ",
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                root_path,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]);
        let title = Paragraph::new(title_line)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::Blue)),
            );
        f.render_widget(title, area);
    }

    fn draw_main_content(f: &mut Frame, app: &mut App, area: Rect) {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Left side: cgroup tree
        {
            let tree_area = main_chunks[0];
            app.ui_state.tree_state.adjust_scroll_for_area_height(tree_area.height as usize);
            CGroupTreeWidget::draw(f, app, &app.ui_state.tree_state, tree_area);
        }

        // Right side: resource usage
        ResourceGraphWidget::draw(f, app, main_chunks[1]);
    }

    fn draw_status_bar(f: &mut Frame, app: &mut App, area: Rect) {
        let status_text = if let Some(ref data) = app.cgroup_data.metrics {
            format!(
                "Last update: {:?} ago | cgroups: {} | Press 'q' to quit",
                app.cgroup_data.last_update
                    .map(|t| t.elapsed())
                    .unwrap_or_default(),
                data.resource_usage.len()
            )
        } else {
            "Collecting data... | Press 'q' to quit".to_string()
        };

        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::Blue)),
            );
        f.render_widget(status, area);
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

pub fn format_duration_usec(usec: u64) -> String {
    let seconds = usec as f64 / 1_000_000.0;
    if seconds < 1.0 {
        format!("{:.1}ms", usec as f64 / 1000.0)
    } else if seconds < 60.0 {
        format!("{:.1}s", seconds)
    } else {
        let minutes = seconds / 60.0;
        format!("{:.1}m", minutes)
    }
}