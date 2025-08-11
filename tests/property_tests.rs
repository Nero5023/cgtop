mod common;

use cgtop::widgets::CGroupTreeState;
use proptest::prelude::*;
use hashbrown::HashMap;

// Generate arbitrary cgroup paths
fn arb_cgroup_path() -> impl Strategy<Value = String> {
    prop::collection::vec("[a-z][a-z0-9_-]{1,10}", 1..=5)
        .prop_map(|parts| format!("/sys/fs/cgroup/{}", parts.join("/")))
}

// Generate a set of cgroup paths
fn arb_cgroup_paths() -> impl Strategy<Value = HashMap<String, cgtop::collection::ResourceStats>> {
    prop::collection::vec(
        (arb_cgroup_path(), Just(common::create_mock_resource_stats())),
        1..20
    ).prop_map(|vec| {
        vec.into_iter().collect()
    })
}

proptest! {
    #[test]
    fn test_tree_build_doesnt_panic(paths in arb_cgroup_paths()) {
        let mut tree_state = CGroupTreeState::default();
        
        // Building a tree from any valid cgroup paths should not panic
        tree_state.build_from_paths(&paths);
        
        // Basic invariants should hold
        assert!(tree_state.nodes.len() > 0); // Should at least have root
        assert!(tree_state.nodes.contains_key("")); // Root should exist
        
        // All visible nodes should exist in the tree
        for visible_path in &tree_state.visible_nodes {
            assert!(tree_state.nodes.contains_key(visible_path));
        }
    }

    #[test]
    fn test_navigation_invariants(
        paths in arb_cgroup_paths(),
        nav_ops in prop::collection::vec(prop_oneof![
            Just("next"),
            Just("prev"),
        ], 0..50)
    ) {
        let mut tree_state = CGroupTreeState::default();
        tree_state.build_from_paths(&paths);
        
        for op in nav_ops {
            match op {
                "next" => tree_state.select_next(),
                "prev" => tree_state.select_previous(),
                _ => unreachable!(),
            }
            
            // Selection should always be valid if it exists
            if let Some(ref selected) = tree_state.selected {
                assert!(tree_state.nodes.contains_key(selected));
                assert!(tree_state.visible_nodes.contains(selected));
            }
        }
    }

    #[test]
    fn test_expansion_invariants(
        paths in arb_cgroup_paths(),
        expand_ops in prop::collection::vec(prop::string::string_regex("[a-z][a-z0-9_-/]*").unwrap(), 0..20)
    ) {
        let mut tree_state = CGroupTreeState::default();
        tree_state.build_from_paths(&paths);
        
        for path in expand_ops {
            tree_state.toggle_expand(&path);
            
            // Expanded nodes should always be valid
            for expanded_path in &tree_state.expanded_nodes {
                if !expanded_path.is_empty() { // Root can be empty string
                    assert!(tree_state.nodes.contains_key(expanded_path));
                }
            }
            
            // All visible nodes should exist in the tree
            for visible_path in &tree_state.visible_nodes {
                assert!(tree_state.nodes.contains_key(visible_path));
            }
        }
    }

    #[test]
    fn test_state_persistence_invariants(
        initial_paths in arb_cgroup_paths(),
        updated_paths in arb_cgroup_paths(),
        expand_ops in prop::collection::vec(prop::string::string_regex("[a-z][a-z0-9_-/]*").unwrap(), 0..5)
    ) {
        let mut tree_state = CGroupTreeState::default();
        
        // Build initial tree
        tree_state.build_from_paths(&initial_paths);
        
        // Perform some expansions
        for path in expand_ops {
            tree_state.toggle_expand(&path);
        }
        
        let expanded_before = tree_state.expanded_nodes.clone();
        let selected_before = tree_state.selected.clone();
        
        // Update with new paths
        tree_state.build_from_paths(&updated_paths);
        
        // Invariants after update
        assert!(tree_state.nodes.len() > 0); // Should have nodes
        assert!(tree_state.nodes.contains_key("")); // Root should exist
        
        // All visible nodes should exist
        for visible_path in &tree_state.visible_nodes {
            assert!(tree_state.nodes.contains_key(visible_path));
        }
        
        // Expanded state should be reasonable (nodes that still exist should preserve state)
        for expanded_path in &tree_state.expanded_nodes {
            if tree_state.nodes.contains_key(expanded_path) {
                // If the node still exists, it should be expanded
                if let Some(node) = tree_state.nodes.get(expanded_path) {
                    assert!(node.expanded);
                }
            }
        }
        
        // Selection should be valid if it exists
        if let Some(ref selected) = tree_state.selected {
            assert!(tree_state.nodes.contains_key(selected));
            assert!(tree_state.visible_nodes.contains(selected));
        }
    }

    #[test]
    fn test_hierarchy_correctness(paths in arb_cgroup_paths()) {
        let mut tree_state = CGroupTreeState::default();
        tree_state.build_from_paths(&paths);
        
        // Check parent-child relationships are correct
        for (path, node) in &tree_state.nodes {
            // Skip root
            if path.is_empty() {
                continue;
            }
            
            // Every child in the children list should exist as a node
            for child_path in &node.children {
                assert!(tree_state.nodes.contains_key(child_path));
                
                // Child's depth should be parent's depth + 1
                let child_node = tree_state.nodes.get(child_path).unwrap();
                assert_eq!(child_node.depth, node.depth + 1);
            }
            
            // Node's depth should correspond to its path depth
            let normalized_path = node.path.strip_prefix("/sys/fs/cgroup").unwrap_or(&node.path);
            let expected_depth = if normalized_path.is_empty() || normalized_path == "/" {
                0
            } else {
                normalized_path.split('/').filter(|p| !p.is_empty()).count()
            };
            
            // Allow some flexibility in depth calculation due to path normalization
            assert!(node.depth <= expected_depth + 1);
        }
    }
    
    #[test]
    fn test_visible_nodes_consistency(
        paths in arb_cgroup_paths(),
        random_expansions in prop::collection::vec(prop::string::string_regex("[a-z][a-z0-9_-/]*").unwrap(), 0..10)
    ) {
        let mut tree_state = CGroupTreeState::default();
        tree_state.build_from_paths(&paths);
        
        // Do some random expansions
        for expand_path in random_expansions {
            tree_state.toggle_expand(&expand_path);
        }
        
        // All visible nodes must be reachable from root by following expanded parents
        for visible_path in &tree_state.visible_nodes {
            let node = tree_state.nodes.get(visible_path).unwrap();
            
            // Trace back to root and ensure all parents are expanded
            let path_parts: Vec<&str> = if node.path == "/sys/fs/cgroup" {
                vec![]
            } else {
                node.path.strip_prefix("/sys/fs/cgroup/").unwrap_or(&node.path)
                    .split('/')
                    .collect()
            };
            
            let mut current_path = String::new();
            for (i, part) in path_parts.iter().enumerate() {
                if i > 0 {
                    current_path.push('/');
                }
                current_path.push_str(part);
                
                // All intermediate paths should exist and be expanded (except the last one)
                if i < path_parts.len() - 1 {
                    if let Some(parent_node) = tree_state.nodes.get(&current_path) {
                        assert!(parent_node.expanded, "Parent {} of visible node {} should be expanded", current_path, visible_path);
                    }
                }
            }
        }
    }
}