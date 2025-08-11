use cgtop::widgets::{CGroupTreeState};
use cgtop::collection::ResourceStats;
use hashbrown::HashMap;

fn main() {
    env_logger::init();
    
    println!("Testing tree widget logic...");
    
    // Create mock data
    let mut resource_usage = HashMap::new();
    
    let mock_paths = vec![
        "/sys/fs/cgroup",
        "/sys/fs/cgroup/system.slice",
        "/sys/fs/cgroup/system.slice/systemd-logind.service",
        "/sys/fs/cgroup/system.slice/ssh.service", 
        "/sys/fs/cgroup/user.slice",
        "/sys/fs/cgroup/user.slice/user-1000.slice",
        "/sys/fs/cgroup/user.slice/user-1000.slice/session-2.scope",
        "/sys/fs/cgroup/init.scope",
    ];
    
    for path in mock_paths {
        resource_usage.insert(path.to_string(), ResourceStats::default());
    }
    
    // Test tree building
    let mut tree_state = CGroupTreeState::default();
    tree_state.build_from_paths(&resource_usage);
    
    println!("Tree state created with {} nodes", tree_state.nodes.len());
    println!("Visible nodes: {}", tree_state.visible_nodes.len());
    
    for (path, node) in &tree_state.nodes {
        println!("Node '{}' -> name: '{}', depth: {}, children: {}", 
                 path, node.name, node.depth, node.children.len());
    }
    
    println!("Visible nodes in order:");
    for visible_path in &tree_state.visible_nodes {
        if let Some(node) = tree_state.nodes.get(visible_path) {
            let indent = "  ".repeat(node.depth);
            println!("{}├─ {} ({})", indent, node.name, node.path);
        }
    }
    
    println!("Tree widget logic test completed successfully!");
}