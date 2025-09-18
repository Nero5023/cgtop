mod common;

use cgtop::{
    app::{App, UiState},
    collection::CGroupMetrics,
    events::CGroupEvent,
    threads::EventThreads,
};
use crossbeam::channel::{self, Receiver};
use pretty_assertions::assert_eq;
use std::time::Duration;

fn create_mock_metrics() -> Box<CGroupMetrics> {
    use cgtop::collection::{CpuStats, IoStats, MemoryStats, PidStats, ResourceStats};
    use hashbrown::HashMap;
    use std::time::Instant;

    let mut resource_usage = HashMap::new();
    let processes = HashMap::new();

    // Create a simple test hierarchy
    let paths = vec![
        "/sys/fs/cgroup",
        "/sys/fs/cgroup/test.slice",
        "/sys/fs/cgroup/test.slice/test.service",
    ];

    for (i, path) in paths.iter().enumerate() {
        let stats = ResourceStats {
            memory: MemoryStats {
                current: 1024 * (i as u64 + 1),
                max: Some(1024 * 10),
                ..Default::default()
            },
            cpu: CpuStats {
                usage_usec: 1000 * (i as u64 + 1),
                ..Default::default()
            },
            io: IoStats {
                rbytes: 512 * (i as u64 + 1),
                wbytes: 256 * (i as u64 + 1),
                ..Default::default()
            },
            pids: PidStats {
                current: i as u64 + 1,
                max: Some(100),
            },
        };

        resource_usage.insert(path.to_string(), stats);
    }

    Box::new(CGroupMetrics {
        hierarchies: Vec::new(),
        processes,
        resource_usage,
        timestamp: Instant::now(),
    })
}

#[test]
fn test_app_initialization() {
    let app = App::new();

    // App should initialize with default values
    assert!(app.cgroup_data.metrics.is_none());
    assert!(app.cgroup_data.last_update.is_none());
    assert_eq!(app.ui_state.current_tab, 0);
    assert!(app.ui_state.tree_state.nodes.is_empty());
    assert!(app.ui_state.selected_cgroup.is_none());
}

#[test]
fn test_app_metrics_update() {
    let mut app = App::new();
    let mock_metrics = create_mock_metrics();

    // Simulate receiving a metrics update
    app.ui_state
        .tree_state
        .build_from_paths(&mock_metrics.resource_usage);
    app.cgroup_data.metrics = Some(mock_metrics);
    app.cgroup_data.last_update = Some(std::time::Instant::now());

    // App should have updated state
    assert!(app.cgroup_data.metrics.is_some());
    assert!(app.cgroup_data.last_update.is_some());
    assert!(!app.ui_state.tree_state.nodes.is_empty());

    // Tree should have been built
    let metrics = app.cgroup_data.metrics.as_ref().unwrap();
    assert_eq!(metrics.resource_usage.len(), 3);
    assert!(app.ui_state.tree_state.nodes.len() >= 3);
}

#[test]
fn test_tree_state_updates_with_selection() {
    let mut app = App::new();
    let mock_metrics = create_mock_metrics();

    // Update the tree state
    app.ui_state
        .tree_state
        .build_from_paths(&mock_metrics.resource_usage);

    // Navigate to select a node
    app.ui_state.tree_state.select_next();
    let selected_before = app.ui_state.tree_state.selected.clone();

    // Update with same data (simulating refresh)
    app.ui_state
        .tree_state
        .build_from_paths(&mock_metrics.resource_usage);
    let selected_after = app.ui_state.tree_state.selected.clone();

    // Selection should be preserved
    assert_eq!(selected_before, selected_after);
}

#[test]
fn test_expansion_state_persistence() {
    let mut app = App::new();
    let mock_metrics = create_mock_metrics();

    // Initial build
    app.ui_state
        .tree_state
        .build_from_paths(&mock_metrics.resource_usage);

    // Expand a node
    let expandable_nodes: Vec<String> = app
        .ui_state
        .tree_state
        .nodes
        .iter()
        .filter(|(_, node)| !node.children.is_empty())
        .map(|(path, _)| path.clone())
        .collect();

    if let Some(node_to_expand) = expandable_nodes.first() {
        app.ui_state.tree_state.toggle_expand(node_to_expand);
        let expanded_before = app.ui_state.tree_state.expanded_nodes.clone();

        // Simulate data refresh
        app.ui_state
            .tree_state
            .build_from_paths(&mock_metrics.resource_usage);
        let expanded_after = app.ui_state.tree_state.expanded_nodes.clone();

        // Expansion state should be preserved (note: root "" may be added automatically)
        assert!(expanded_after.len() >= expanded_before.len());
        for expanded_path in &expanded_before {
            assert!(
                expanded_after.contains(expanded_path),
                "Expected {} to remain expanded",
                expanded_path
            );
        }
    }
}

#[test]
fn test_ui_state_navigation() {
    let mut ui_state = UiState::default();
    let mock_metrics = create_mock_metrics();

    // Build tree state
    ui_state
        .tree_state
        .build_from_paths(&mock_metrics.resource_usage);

    // Test navigation
    let initial_selection = ui_state.tree_state.selected.clone();

    ui_state.tree_state.select_next();
    let after_next = ui_state.tree_state.selected.clone();

    ui_state.tree_state.select_previous();
    let after_prev = ui_state.tree_state.selected.clone();

    // Should return to initial selection after next->previous
    assert_eq!(initial_selection, after_prev);

    // Middle selection should be different (if there are multiple nodes)
    if ui_state.tree_state.visible_nodes.len() > 1 {
        assert_ne!(initial_selection, after_next);
    }
}

#[test]
fn test_cgroup_path_to_selection_sync() {
    let mut app = App::new();
    let mock_metrics = create_mock_metrics();

    app.ui_state
        .tree_state
        .build_from_paths(&mock_metrics.resource_usage);

    // Simulate selecting a node and updating selected_cgroup
    if !app.ui_state.tree_state.visible_nodes.is_empty() {
        let first_visible = app.ui_state.tree_state.visible_nodes[0].clone();
        app.ui_state.tree_state.selected = Some(first_visible.clone());

        // Update selected_cgroup based on tree selection
        app.ui_state.selected_cgroup = app
            .ui_state
            .tree_state
            .selected
            .clone()
            .and_then(|path| app.ui_state.tree_state.nodes.get(&path))
            .map(|node| node.path.clone());

        // Should have a selected cgroup
        assert!(app.ui_state.selected_cgroup.is_some());

        // The selected cgroup should exist in the metrics
        let selected_path = app.ui_state.selected_cgroup.as_ref().unwrap();
        if let Some(ref metrics) = mock_metrics.resource_usage.get(selected_path) {
            assert!(metrics.memory.current > 0);
        }
    }
}

#[test]
fn test_event_handling_mock() {
    // Test that we can create and handle events without starting actual threads
    let (sender, receiver) = channel::unbounded::<CGroupEvent>();
    let mock_metrics = create_mock_metrics();

    // Send a mock update event
    sender.send(CGroupEvent::Update(mock_metrics)).unwrap();

    // Receive and process the event
    match receiver.try_recv() {
        Ok(CGroupEvent::Update(metrics)) => {
            assert_eq!(metrics.resource_usage.len(), 3);
            assert!(metrics.resource_usage.contains_key("/sys/fs/cgroup"));
        }
        Ok(_) => panic!("Unexpected event type"),
        Err(_) => panic!("Failed to receive event"),
    }
}

#[test]
fn test_app_with_multiple_updates() {
    let mut app = App::new();

    // First update
    let mut mock_metrics1 = create_mock_metrics();
    app.ui_state
        .tree_state
        .build_from_paths(&mock_metrics1.resource_usage);
    app.cgroup_data.metrics = Some(mock_metrics1);

    // Expand a node
    if !app.ui_state.tree_state.nodes.is_empty() {
        let first_expandable = app
            .ui_state
            .tree_state
            .nodes
            .iter()
            .find(|(_, node)| !node.children.is_empty())
            .map(|(path, _)| path.clone());

        if let Some(path) = first_expandable {
            app.ui_state.tree_state.toggle_expand(&path);
        }
    }

    let expanded_after_first = app.ui_state.tree_state.expanded_nodes.clone();

    // Second update with additional cgroups
    let mut mock_metrics2 = create_mock_metrics();
    mock_metrics2.resource_usage.insert(
        "/sys/fs/cgroup/new.slice".to_string(),
        common::create_mock_resource_stats(),
    );

    app.ui_state
        .tree_state
        .build_from_paths(&mock_metrics2.resource_usage);
    app.cgroup_data.metrics = Some(mock_metrics2);

    let expanded_after_second = app.ui_state.tree_state.expanded_nodes.clone();

    // Expansion state should be preserved (note: root "" may be added automatically)
    assert!(expanded_after_second.len() >= expanded_after_first.len());
    for expanded_path in &expanded_after_first {
        assert!(
            expanded_after_second.contains(expanded_path),
            "Expected {} to remain expanded",
            expanded_path
        );
    }

    // New cgroup should be present
    assert!(
        app.cgroup_data
            .metrics
            .as_ref()
            .unwrap()
            .resource_usage
            .contains_key("/sys/fs/cgroup/new.slice")
    );
}
