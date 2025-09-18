mod common;

use cgtop::widgets::CGroupTreeState;
use common::{create_mock_resource_stats, create_simple_cgroup_paths, create_test_cgroup_paths};
use pretty_assertions::assert_eq;

#[test]
fn test_tree_state_creation() {
    let tree_state = CGroupTreeState::default();

    assert!(tree_state.nodes.is_empty());
    assert!(tree_state.visible_nodes.is_empty());
    assert!(tree_state.selected.is_none());
    assert!(tree_state.expanded_nodes.is_empty());
}

#[test]
fn test_build_from_paths_simple() {
    let mut tree_state = CGroupTreeState::default();
    let paths = create_simple_cgroup_paths();

    tree_state.build_from_paths(&paths);

    // Should create nodes for all paths
    assert_eq!(tree_state.nodes.len(), 5); // root + 4 actual paths

    // Root should exist and be expanded
    let root = tree_state.nodes.get("").unwrap();
    assert_eq!(root.name, "root");
    assert_eq!(root.depth, 0);
    assert!(root.expanded);
    assert_eq!(root.children.len(), 2); // test1, test2

    // test1 should have children
    let test1 = tree_state.nodes.get("test1").unwrap();
    assert_eq!(test1.name, "test1");
    assert_eq!(test1.depth, 1);
    assert_eq!(test1.children.len(), 2); // child1, child2

    // child1 should have no children
    let child1 = tree_state.nodes.get("test1/child1").unwrap();
    assert_eq!(child1.name, "child1");
    assert_eq!(child1.depth, 2);
    assert!(child1.children.is_empty());
}

#[test]
fn test_tree_navigation() {
    let mut tree_state = CGroupTreeState::default();
    let paths = create_simple_cgroup_paths();

    tree_state.build_from_paths(&paths);

    // Initially should have a selection
    assert!(tree_state.selected.is_some());
    let initial_selection = tree_state.selected.clone();

    // Navigate down
    tree_state.select_next();
    assert_ne!(tree_state.selected, initial_selection);

    // Navigate up
    tree_state.select_previous();
    assert_eq!(tree_state.selected, initial_selection);
}

#[test]
fn test_tree_expansion() {
    let mut tree_state = CGroupTreeState::default();
    let paths = create_simple_cgroup_paths();

    tree_state.build_from_paths(&paths);

    // test1 should start collapsed (depth 1 nodes are expanded by default, but their children aren't visible)
    let initial_visible_count = tree_state.visible_nodes.len();

    // Expand test1 (which should already be expanded, so this should collapse it)
    let test1_expanded_before = tree_state.nodes.get("test1").unwrap().expanded;
    tree_state.toggle_expand("test1");
    let test1_expanded_after = tree_state.nodes.get("test1").unwrap().expanded;

    assert_ne!(test1_expanded_before, test1_expanded_after);

    // If we collapsed it, visible nodes should decrease
    // If we expanded it, visible nodes should stay same or increase
    let final_visible_count = tree_state.visible_nodes.len();
    if !test1_expanded_after {
        assert!(final_visible_count <= initial_visible_count);
    }
}

#[test]
fn test_state_persistence_across_updates() {
    let mut tree_state = CGroupTreeState::default();
    let mut paths = create_simple_cgroup_paths();

    // Initial build
    tree_state.build_from_paths(&paths);

    // Expand a node
    tree_state.toggle_expand("test1/child1"); // This should expand child1 if it has no children, or do nothing

    // Actually, let's expand test1 first, then expand something visible
    if tree_state.nodes.get("test1").unwrap().expanded {
        tree_state.toggle_expand("test1"); // Collapse it
    }
    tree_state.toggle_expand("test1"); // Now expand it

    let expanded_state_before = tree_state.expanded_nodes.clone();
    let selection_before = tree_state.selected.clone();

    // Add a new path and rebuild
    paths.insert(
        "/sys/fs/cgroup/test3".to_string(),
        common::create_mock_resource_stats(),
    );
    tree_state.build_from_paths(&paths);

    // Expansion state should be preserved
    assert_eq!(tree_state.expanded_nodes, expanded_state_before);

    // Selection should be preserved if the node still exists
    if let Some(ref sel) = selection_before {
        if tree_state.nodes.contains_key(sel) {
            assert_eq!(tree_state.selected, selection_before);
        }
    }

    // New node should exist
    assert!(tree_state.nodes.contains_key("test3"));
}

#[test]
fn test_complex_hierarchy() {
    let mut tree_state = CGroupTreeState::default();
    let paths = create_test_cgroup_paths();

    tree_state.build_from_paths(&paths);

    // Should create all nodes
    assert!(tree_state.nodes.len() >= 10);

    // Check specific hierarchy relationships
    let system_slice = tree_state.nodes.get("system.slice").unwrap();
    assert!(
        system_slice
            .children
            .contains(&"system.slice/systemd-logind.service".to_string())
    );
    assert!(
        system_slice
            .children
            .contains(&"system.slice/ssh.service".to_string())
    );

    let user_slice = tree_state.nodes.get("user.slice").unwrap();
    assert!(
        user_slice
            .children
            .contains(&"user.slice/user-1000.slice".to_string())
    );

    // Check depth calculation
    let deep_node = tree_state
        .nodes
        .get("user.slice/user-1000.slice/user@1000.service/app.slice")
        .unwrap();
    assert_eq!(deep_node.depth, 4);
}

#[test]
fn test_visible_nodes_calculation() {
    let mut tree_state = CGroupTreeState::default();
    let paths = create_simple_cgroup_paths();

    tree_state.build_from_paths(&paths);

    // Count visible nodes - should include expanded nodes and their immediate children
    let visible_count = tree_state.visible_nodes.len();
    assert!(visible_count > 0);

    // All visible nodes should exist in the tree
    for visible_path in &tree_state.visible_nodes {
        assert!(tree_state.nodes.contains_key(visible_path));
    }

    // Root should not be in visible nodes (it's not displayed)
    assert!(!tree_state.visible_nodes.contains(&"".to_string()));
}

#[test]
fn test_node_path_normalization() {
    let mut tree_state = CGroupTreeState::default();

    // Test with various path formats
    let mut paths = hashbrown::HashMap::new();
    paths.insert("/sys/fs/cgroup".to_string(), create_mock_resource_stats());
    paths.insert(
        "/sys/fs/cgroup/test".to_string(),
        create_mock_resource_stats(),
    );
    paths.insert("test".to_string(), create_mock_resource_stats()); // Should be normalized

    tree_state.build_from_paths(&paths);

    // Should handle normalization correctly
    assert!(tree_state.nodes.contains_key(""));
    assert!(tree_state.nodes.contains_key("test"));
}

#[test]
fn test_edge_cases() {
    let mut tree_state = CGroupTreeState::default();

    // Test with empty paths
    let empty_paths = hashbrown::HashMap::new();
    tree_state.build_from_paths(&empty_paths);
    assert!(tree_state.nodes.is_empty());

    // Test navigation with no nodes
    tree_state.select_next();
    tree_state.select_previous();
    assert!(tree_state.selected.is_none());

    // Test expansion with invalid path
    tree_state.toggle_expand("nonexistent");
    assert!(tree_state.expanded_nodes.is_empty());
}
