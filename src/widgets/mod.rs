use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, StatefulWidget, Table},
};
use std::collections::BTreeMap;

use crate::app::App;
use crate::canvas::{format_bytes, format_duration_usec};

#[derive(Debug, Clone)]
pub struct CGroupTreeNode {
    pub path: String,
    pub name: String,
    pub children: Vec<String>,
    pub expanded: bool,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct CGroupTreeState {
    pub nodes: BTreeMap<String, CGroupTreeNode>,
    pub selected: Option<String>,
    pub expanded_nodes: std::collections::HashSet<String>,
    pub visible_nodes: Vec<String>,
}

impl Default for CGroupTreeState {
    fn default() -> Self {
        Self {
            nodes: BTreeMap::new(),
            selected: None,
            expanded_nodes: std::collections::HashSet::new(),
            visible_nodes: Vec::new(),
        }
    }
}

impl CGroupTreeState {
    pub fn build_from_paths(
        &mut self,
        paths: &hashbrown::HashMap<String, crate::collection::ResourceStats>,
    ) {
        log::info!("build_from_paths called with {} paths", paths.len());

        self.nodes.clear();
        self.visible_nodes.clear();

        // Build tree structure from flat paths
        for path in paths.keys() {
            log::info!("Processing path: {}", path);
            self.insert_path(path);
        }

        log::info!("After building tree: {} nodes", self.nodes.len());

        // Expand root level nodes by default for better visibility
        for node in self.nodes.values_mut() {
            if node.depth == 1 {
                node.expanded = true;
                self.expanded_nodes.insert(
                    node.path
                        .strip_prefix("/sys/fs/cgroup/")
                        .unwrap_or(&node.path)
                        .to_string(),
                );
            }
        }

        // Build visible nodes list
        self.rebuild_visible_nodes();

        // Select first visible node by default
        if self.selected.is_none() && !self.visible_nodes.is_empty() {
            self.selected = Some(self.visible_nodes[0].clone());
        }

        log::info!(
            "Visible nodes: {} ({:?})",
            self.visible_nodes.len(),
            self.visible_nodes
        );
    }

    fn insert_path(&mut self, path: &str) {
        let normalized_path = path.strip_prefix("/sys/fs/cgroup").unwrap_or(path);
        let parts: Vec<&str> = normalized_path
            .split('/')
            .filter(|p| !p.is_empty())
            .collect();

        // Insert root if not exists
        if !self.nodes.contains_key("") {
            self.nodes.insert(
                "".to_string(),
                CGroupTreeNode {
                    path: "/sys/fs/cgroup".to_string(),
                    name: "root".to_string(),
                    children: Vec::new(),
                    expanded: true, // Root is always expanded
                    depth: 0,
                },
            );
            self.expanded_nodes.insert("".to_string());
        }

        // Build path incrementally
        let mut current_path = String::new();
        let mut parent_path = String::new();

        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                current_path.push('/');
            }
            current_path.push_str(part);

            if !self.nodes.contains_key(&current_path) {
                let full_path = if current_path.is_empty() {
                    "/sys/fs/cgroup".to_string()
                } else {
                    format!("/sys/fs/cgroup/{}", current_path)
                };

                self.nodes.insert(
                    current_path.clone(),
                    CGroupTreeNode {
                        path: full_path,
                        name: part.to_string(),
                        children: Vec::new(),
                        expanded: false,
                        depth: i + 1,
                    },
                );

                // Add to parent's children
                if let Some(parent) = self.nodes.get_mut(&parent_path) {
                    if !parent.children.contains(&current_path) {
                        parent.children.push(current_path.clone());
                        parent.children.sort();
                    }
                }
            }

            parent_path = current_path.clone();
        }
    }

    fn rebuild_visible_nodes(&mut self) {
        self.visible_nodes.clear();
        self.add_visible_children("");
    }

    fn add_visible_children(&mut self, path: &str) {
        if let Some(node) = self.nodes.get(path) {
            if !path.is_empty() {
                self.visible_nodes.push(path.to_string());
            }

            if node.expanded || path.is_empty() {
                let mut children = node.children.clone();
                children.sort();
                for child in children {
                    self.add_visible_children(&child);
                }
            }
        }
    }

    pub fn toggle_expand(&mut self, path: &str) {
        if let Some(node) = self.nodes.get_mut(path) {
            node.expanded = !node.expanded;
            if node.expanded {
                self.expanded_nodes.insert(path.to_string());
            } else {
                self.expanded_nodes.remove(path);
            }
        }
        self.rebuild_visible_nodes();
    }

    pub fn select_next(&mut self) {
        if self.visible_nodes.is_empty() {
            return;
        }

        let current_idx = self
            .selected
            .as_ref()
            .and_then(|s| self.visible_nodes.iter().position(|n| n == s))
            .unwrap_or(0);

        let next_idx = if current_idx >= self.visible_nodes.len() - 1 {
            0
        } else {
            current_idx + 1
        };

        self.selected = self.visible_nodes.get(next_idx).cloned();
    }

    pub fn select_previous(&mut self) {
        if self.visible_nodes.is_empty() {
            return;
        }

        let current_idx = self
            .selected
            .as_ref()
            .and_then(|s| self.visible_nodes.iter().position(|n| n == s))
            .unwrap_or(0);

        let prev_idx = if current_idx == 0 {
            self.visible_nodes.len() - 1
        } else {
            current_idx - 1
        };

        self.selected = self.visible_nodes.get(prev_idx).cloned();
    }
}

pub struct CGroupTreeWidget;

impl CGroupTreeWidget {
    pub fn draw(f: &mut Frame, app: &App, tree_state: &CGroupTreeState, area: Rect) {
        log::info!(
            "CGroupTreeWidget::draw called with tree_state.visible_nodes: {} nodes",
            tree_state.visible_nodes.len()
        );

        let items: Vec<ListItem> = if let Some(ref metrics) = app.cgroup_data.metrics {
            tree_state
                .visible_nodes
                .iter()
                .filter_map(|node_path| {
                    let node = tree_state.nodes.get(node_path)?;
                    let stats = metrics.resource_usage.get(&node.path)?;

                    let memory_info = format_bytes(stats.memory.current);
                    let cpu_info = format_duration_usec(stats.cpu.usage_usec);

                    // Create tree visualization with proper indentation and tree chars
                    let tree_prefix = Self::get_tree_prefix(node, tree_state);
                    let expand_indicator = if !node.children.is_empty() {
                        if node.expanded { "▼ " } else { "▶ " }
                    } else {
                        "  "
                    };

                    // Style based on selection
                    let name_style = if tree_state.selected.as_ref() == Some(node_path) {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Green)
                    };

                    let line = Line::from(vec![
                        Span::styled(tree_prefix, Style::default().fg(Color::DarkGray)),
                        Span::styled(expand_indicator, Style::default().fg(Color::Blue)),
                        Span::styled(&node.name, name_style),
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
                    Some(ListItem::new(line))
                })
                .collect()
        } else {
            vec![ListItem::new("Loading cgroup data...")]
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title("cgroup Tree (↑↓: navigate, →: expand, ←: collapse, Enter: select)")
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::Blue)),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(list, area);
    }

    fn get_tree_prefix(node: &CGroupTreeNode, tree_state: &CGroupTreeState) -> String {
        if node.depth == 0 {
            return String::new();
        }

        let mut prefix = String::new();
        let node_path_parts: Vec<&str> = if node.path == "/sys/fs/cgroup" {
            vec![]
        } else {
            node.path
                .strip_prefix("/sys/fs/cgroup/")
                .unwrap_or(&node.path)
                .split('/')
                .collect()
        };

        // Build prefix by checking each level
        for depth in 1..node.depth {
            let ancestor_path = if depth == 1 {
                node_path_parts[0].to_string()
            } else {
                node_path_parts[..depth].join("/")
            };

            // Check if this ancestor has more siblings at this level
            let has_more_siblings = Self::has_more_siblings(&ancestor_path, depth, tree_state);

            if has_more_siblings {
                prefix.push_str("│   ");
            } else {
                prefix.push_str("    ");
            }
        }

        // Add the final connector
        let is_last_child = Self::is_last_child(node, tree_state);
        if is_last_child {
            prefix.push_str("└── ");
        } else {
            prefix.push_str("├── ");
        }

        prefix
    }

    fn has_more_siblings(path: &str, depth: usize, tree_state: &CGroupTreeState) -> bool {
        // This is a simplified check - in a full implementation, you'd track sibling relationships
        // For now, we'll assume most intermediate nodes have siblings
        depth > 1
    }

    fn is_last_child(node: &CGroupTreeNode, tree_state: &CGroupTreeState) -> bool {
        // Find parent and check if this is the last child
        let node_path = node
            .path
            .strip_prefix("/sys/fs/cgroup")
            .unwrap_or(&node.path);
        if let Some(parent_path_end) = node_path.rfind('/') {
            let parent_path = if parent_path_end == 0 {
                ""
            } else {
                &node_path[1..parent_path_end] // Remove leading slash
            };

            if let Some(parent) = tree_state.nodes.get(parent_path) {
                return parent.children.last() == Some(&node_path[1..].to_string());
            }
        }
        false
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
        path.strip_prefix("/sys/fs/cgroup")
            .unwrap_or(path)
            .to_string()
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
