use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table},
    Frame,
};

use crate::app::App;
use crate::canvas::{format_bytes, format_duration_usec};

pub struct CGroupTreeWidget;

impl CGroupTreeWidget {
    pub fn draw(f: &mut Frame, app: &App, area: Rect) {
        let items: Vec<ListItem> = if let Some(ref metrics) = app.cgroup_data.metrics {
            metrics.resource_usage.iter()
                .map(|(path, stats)| {
                    let memory_info = format_bytes(stats.memory.current);
                    let cpu_info = format_duration_usec(stats.cpu.usage_usec);
                    
                    let line = Line::from(vec![
                        Span::styled(
                            Self::format_path_display(path),
                            Style::default().fg(Color::Green),
                        ),
                        Span::raw(" - "),
                        Span::styled(
                            format!("Mem: {}", memory_info),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::raw(" | "),
                        Span::styled(
                            format!("CPU: {}", cpu_info),
                            Style::default().fg(Color::Cyan),
                        ),
                    ]);
                    ListItem::new(line)
                })
                .collect()
        } else {
            vec![ListItem::new("Loading cgroup data...")]
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title("cgroup Tree")
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::Blue)),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(list, area);
    }

    fn format_path_display(path: &str) -> String {
        // Remove /sys/fs/cgroup prefix and format as tree-like structure
        let trimmed = path.strip_prefix("/sys/fs/cgroup").unwrap_or(path);
        if trimmed.is_empty() || trimmed == "/" {
            "root".to_string()
        } else {
            let depth = trimmed.matches('/').count();
            let indent = "  ".repeat(depth.saturating_sub(1));
            let name = trimmed.split('/').last().unwrap_or(trimmed);
            format!("{}{}", indent, name)
        }
    }
}

pub struct ProcessListWidget;

impl ProcessListWidget {
    pub fn draw(f: &mut Frame, app: &App, area: Rect) {
        let rows: Vec<Row> = if let Some(ref metrics) = app.cgroup_data.metrics {
            // Collect and sort processes by PID first
            let mut process_data: Vec<_> = metrics.processes.iter().collect();
            process_data.sort_by_key(|(pid, _)| **pid);
            
            // Create rows from sorted data, limiting to first 100 for performance
            process_data
                .into_iter()
                .take(100)
                .map(|(pid, cgroup_path)| {
                    Row::new(vec![
                        pid.to_string(),
                        format!("pid-{}", pid), // Simple process identifier
                        Self::format_cgroup_display(cgroup_path),
                    ])
                })
                .collect()
        } else {
            vec![Row::new(vec!["Loading...", "", ""])]
        };

        let widths = [
            Constraint::Length(8),
            Constraint::Length(20),
            Constraint::Min(20),
        ];

        let table = Table::new(rows, widths)
            .header(
                Row::new(vec!["PID", "Command", "cgroup"])
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title("Process List")
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::Blue)),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(table, area);
    }

    fn format_cgroup_display(path: &str) -> String {
        path.strip_prefix("/sys/fs/cgroup").unwrap_or(path).to_string()
    }
}

pub struct ResourceGraphWidget;

impl ResourceGraphWidget {
    pub fn draw(f: &mut Frame, app: &App, area: Rect) {
        let content = if let Some(ref metrics) = app.cgroup_data.metrics {
            if let Some(selected_path) = &app.ui_state.selected_cgroup {
                if let Some(stats) = metrics.resource_usage.get(selected_path) {
                    format!(
                        "Selected cgroup: {}\n\nMemory: {}\nCPU Time: {}\nIO Read: {} | Write: {}\nPIDs: {}",
                        selected_path,
                        format_bytes(stats.memory.current),
                        format_duration_usec(stats.cpu.usage_usec),
                        format_bytes(stats.io.rbytes),
                        format_bytes(stats.io.wbytes),
                        stats.pids.current
                    )
                } else {
                    "Selected cgroup not found".to_string()
                }
            } else {
                format!(
                    "Total cgroups: {}\n\nSelect a cgroup from the tree above to view detailed resource usage.",
                    metrics.resource_usage.len()
                )
            }
        } else {
            "Loading resource data...".to_string()
        };

        let paragraph = Paragraph::new(content)
            .block(
                Block::default()
                    .title("Resource Usage")
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::Blue)),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(paragraph, area);
    }
}