use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table},
};
use std::{collections::BTreeMap, path::PathBuf};

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
    root_path: PathBuf,
}

impl Default for CGroupTreeState {
    fn default() -> Self {
        Self {
            nodes: BTreeMap::new(),
            selected: None,
            expanded_nodes: std::collections::HashSet::new(),
            visible_nodes: Vec::new(),
            root_path: PathBuf::from("/sys/fs/cgroup"),
        }
    }
}

impl CGroupTreeState {
    pub fn new(root_path: PathBuf) -> Self {
        let mut state = Self::default();
        state.root_path = root_path;
        state
    }
}

impl CGroupTreeState {
    pub fn build_from_paths(
        &mut self,
        paths: &hashbrown::HashMap<String, crate::collection::ResourceStats>,
    ) {
        // Save current expansion state and selection before rebuilding
        let saved_expanded_nodes = self.expanded_nodes.clone();
        let saved_selection = self.selected.clone();
        let is_first_build = self.nodes.is_empty();

        self.nodes.clear();
        self.visible_nodes.clear();

        // Build tree structure from flat paths
        for path in paths.keys() {
            // log::info!("Processing path: {}", path);
            self.insert_path(path);
        }

        // log::info!("After building tree: {} nodes", self.nodes.len());

        // Restore expansion state from saved state, or set defaults for first build
        for (node_key, node) in self.nodes.iter_mut() {
            // For first build, expand root level nodes by default
            if is_first_build && node.depth == 1 {
                node.expanded = true;
                self.expanded_nodes.insert(node_key.clone());
            }
            // For subsequent builds, restore previous expansion state
            else if saved_expanded_nodes.contains(node_key) {
                node.expanded = true;
                self.expanded_nodes.insert(node_key.clone());
            }
            // Root is always expanded
            else if node_key.is_empty() {
                node.expanded = true;
                self.expanded_nodes.insert(node_key.clone());
            }
        }

        // Build visible nodes list
        self.rebuild_visible_nodes();

        // Restore selection, or select first visible node by default
        if let Some(saved_sel) = saved_selection {
            // Check if previously selected node still exists
            if self.nodes.contains_key(&saved_sel) && self.visible_nodes.contains(&saved_sel) {
                self.selected = Some(saved_sel);
            } else if !self.visible_nodes.is_empty() {
                // Fallback to first visible node if previous selection is no longer visible
                self.selected = Some(self.visible_nodes[0].clone());
            }
        } else if self.selected.is_none() && !self.visible_nodes.is_empty() {
            // First time: select first visible node
            self.selected = Some(self.visible_nodes[0].clone());
        }
    }

    fn insert_path(&mut self, path: &str) {
        let normalized_path = path.strip_prefix(&self.root_path_string()).unwrap_or(path);
        let parts: Vec<&str> = normalized_path
            .split('/')
            .filter(|p| !p.is_empty())
            .collect();

        // Insert root if not exists
        if !self.nodes.contains_key("") {
            self.nodes.insert(
                "".to_string(),
                CGroupTreeNode {
                    path: self.root_path.to_string_lossy().to_string(),
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
                    self.root_path.to_string_lossy().to_string()
                } else {
                    format!("{}/{}", self.root_path.to_string_lossy(), current_path)
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

    pub fn root_path_string(&self) -> String {
        self.root_path.to_string_lossy().to_string()
    }
}

pub struct CGroupTreeWidget;

impl CGroupTreeWidget {
    pub fn draw(f: &mut Frame, app: &App, tree_state: &CGroupTreeState, area: Rect) {
        // log::info!(
        //     "CGroupTreeWidget::draw called with tree_state.visible_nodes: {} nodes",
        //     tree_state.visible_nodes.len()
        // );

        let items: Vec<ListItem> = if let Some(ref metrics) = app.cgroup_data.metrics {
            tree_state
                .visible_nodes
                .iter()
                .filter_map(|node_path| {
                    let node = tree_state.nodes.get(node_path)?;
                    let stats = metrics.resource_usage.get(&node.path)?;

                    let memory_current_info = format_bytes(stats.memory.current);
                    let memory_peak_info = format_bytes(stats.memory.peak);
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
                            format!("Mem: {}", memory_current_info),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::styled(
                            format!("/{}", memory_peak_info),
                            Style::default().fg(Color::DarkGray),
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
                    .title("cgroup Tree (↑↓: navigate, →: expand, ←: collapse, Enter/Space: toggle)")
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
        let node_path_parts: Vec<&str> = if node.path == tree_state.root_path.to_string_lossy() {
            vec![]
        } else {
            node.path
                .strip_prefix(&tree_state.root_path_string())
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

    fn has_more_siblings(_path: &str, depth: usize, _tree_state: &CGroupTreeState) -> bool {
        // This is a simplified check - in a full implementation, you'd track sibling relationships
        // For now, we'll assume most intermediate nodes have siblings
        depth > 1
    }

    fn is_last_child(node: &CGroupTreeNode, tree_state: &CGroupTreeState) -> bool {
        // Find parent and check if this is the last child
        let node_path = node
            .path
            .strip_prefix(&tree_state.root_path_string())
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
                        Self::format_cgroup_display(cgroup_path, &app.config.cgroup_root),
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

    fn format_cgroup_display(path: &str, root_path: &PathBuf) -> String {
        path.strip_prefix(root_path.to_string_lossy().as_ref())
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
                    let header = format!("Selected cgroup: {}", selected_path);
                    
                    let memory_overview = format!(
                        "MEMORY OVERVIEW:\n\
                        • Current: {current} | Peak: {peak}\n\
                        • Limit: {limit}",
                        current = format_bytes(stats.memory.current),
                        peak = format_bytes(stats.memory.peak),
                        limit = stats.memory.max.map_or("unlimited".to_string(), |m| format_bytes(m))
                    );
                    
                    let memory_breakdown = format!(
                        "MEMORY BREAKDOWN (memory.stat):\n\
                        • Anonymous (heap/stack): {anon}\n\
                        • File Cache: {file}\n\
                        • Kernel Stack: {kernel_stack}\n\
                        • Slab (kernel structures): {slab}\n\
                        • Socket Buffers: {sock}",
                        anon = format_bytes(stats.memory.anon),
                        file = format_bytes(stats.memory.file),
                        kernel_stack = format_bytes(stats.memory.kernel_stack),
                        slab = format_bytes(stats.memory.slab),
                        sock = format_bytes(stats.memory.sock)
                    );
                    
                    let memory_activity = format!(
                        "MEMORY ACTIVITY:\n\
                        • Active Anonymous: {active_anon}\n\
                        • Inactive Anonymous: {inactive_anon}\n\
                        • Active File Cache: {active_file}\n\
                        • Inactive File Cache: {inactive_file}",
                        active_anon = format_bytes(stats.memory.active_anon),
                        inactive_anon = format_bytes(stats.memory.inactive_anon),
                        active_file = format_bytes(stats.memory.active_file),
                        inactive_file = format_bytes(stats.memory.inactive_file)
                    );
                    
                    let page_faults = format!(
                        "PAGE FAULTS:\n\
                        • Total: {total} | Major: {major}",
                        total = stats.memory.pgfault,
                        major = stats.memory.pgmajfault
                    );
                    
                    let memory_pressure = if let Some(ref pressure) = stats.memory.pressure {
                        format!(
                            "MEMORY PRESSURE (PSI):\n\
                            • Some Tasks Delayed:\n\
                            \x20\x20- 10s: {some_avg10}% | 1m: {some_avg60}% | 5m: {some_avg300}%\n\
                            \x20\x20- Total: {some_total_ms}ms\n\
                            • All Tasks Delayed:\n\
                            \x20\x20- 10s: {full_avg10}% | 1m: {full_avg60}% | 5m: {full_avg300}%\n\
                            \x20\x20- Total: {full_total_ms}ms",
                            some_avg10 = pressure.some_avg10,
                            some_avg60 = pressure.some_avg60,
                            some_avg300 = pressure.some_avg300,
                            some_total_ms = pressure.some_total / 1000, // Convert microseconds to milliseconds
                            full_avg10 = pressure.full_avg10,
                            full_avg60 = pressure.full_avg60,
                            full_avg300 = pressure.full_avg300,
                            full_total_ms = pressure.full_total / 1000, // Convert microseconds to milliseconds
                        )
                    } else {
                        "MEMORY PRESSURE (PSI):\n• Not available (memory.pressure file not found)".to_string()
                    };
                    
                    let cgroup_processes = if stats.cgroup_procs.is_empty() {
                        "CGROUP PROCESSES:\n• No processes in this cgroup".to_string()
                    } else {
                        let process_list = if stats.cgroup_procs.len() <= 10 {
                            // Show all PIDs if 10 or fewer
                            stats.cgroup_procs.iter()
                                .map(|pid| pid.to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        } else {
                            // Show first 10 PIDs and count
                            let first_ten = stats.cgroup_procs.iter()
                                .take(10)
                                .map(|pid| pid.to_string())
                                .collect::<Vec<_>>()
                                .join(", ");
                            format!("{} ... (+{} more)", first_ten, stats.cgroup_procs.len() - 10)
                        };
                        
                        format!(
                            "CGROUP PROCESSES:\n\
                            • Count: {count}\n\
                            • PIDs: {pids}",
                            count = stats.cgroup_procs.len(),
                            pids = process_list
                        )
                    };

                    let other_resources = format!(
                        "OTHER RESOURCES:\n\
                        • CPU Time: {cpu_time}\n\
                        • IO Read/Write: {io_read} / {io_write}\n\
                        • PIDs: {pids}",
                        cpu_time = format_duration_usec(stats.cpu.usage_usec),
                        io_read = format_bytes(stats.io.rbytes),
                        io_write = format_bytes(stats.io.wbytes),
                        pids = stats.pids.current
                    );
                    
                    format!(
                        "{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}",
                        header,
                        memory_overview,
                        memory_breakdown,
                        memory_activity,
                        page_faults,
                        memory_pressure,
                        cgroup_processes,
                        other_resources
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
