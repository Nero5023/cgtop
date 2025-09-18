use cgtop::collection::ResourceStats;
use cgtop::widgets::CGroupTreeState;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use hashbrown::HashMap;

fn create_large_cgroup_hierarchy(depth: usize, breadth: usize) -> HashMap<String, ResourceStats> {
    let mut paths = HashMap::new();
    let base_stats = ResourceStats::default();

    // Add root
    paths.insert("/sys/fs/cgroup".to_string(), base_stats.clone());

    // Generate a tree with specified depth and breadth
    let mut current_level = vec!["".to_string()]; // Start with root

    for level in 0..depth {
        let mut next_level = Vec::new();

        for parent in &current_level {
            for i in 0..breadth {
                let child_name = format!("level{}_node{}", level, i);
                let child_path = if parent.is_empty() {
                    child_name.clone()
                } else {
                    format!("{}/{}", parent, child_name)
                };

                let full_path = format!("/sys/fs/cgroup/{}", child_path);
                paths.insert(full_path, base_stats.clone());
                next_level.push(child_path);
            }
        }

        current_level = next_level;
        if current_level.is_empty() {
            break;
        }
    }

    paths
}

fn bench_tree_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("tree_build");

    for size in [10, 50, 100, 500].iter() {
        let paths = create_large_cgroup_hierarchy(4, *size / 4);

        group.bench_with_input(BenchmarkId::new("build_from_paths", size), size, |b, _| {
            let mut tree_state = CGroupTreeState::default();
            b.iter(|| {
                tree_state.build_from_paths(black_box(&paths));
            });
        });
    }

    group.finish();
}

fn bench_tree_navigation(c: &mut Criterion) {
    let mut group = c.benchmark_group("tree_navigation");

    let paths = create_large_cgroup_hierarchy(5, 10);
    let mut tree_state = CGroupTreeState::default();
    tree_state.build_from_paths(&paths);

    group.bench_function("select_next", |b| {
        b.iter(|| {
            tree_state.select_next();
        });
    });

    group.bench_function("select_previous", |b| {
        b.iter(|| {
            tree_state.select_previous();
        });
    });

    group.finish();
}

fn bench_tree_expansion(c: &mut Criterion) {
    let mut group = c.benchmark_group("tree_expansion");

    let paths = create_large_cgroup_hierarchy(4, 15);
    let mut tree_state = CGroupTreeState::default();
    tree_state.build_from_paths(&paths);

    // Get a node that can be expanded
    let expandable_node = tree_state
        .nodes
        .iter()
        .find(|(_, node)| !node.children.is_empty())
        .map(|(path, _)| path.clone())
        .unwrap_or_else(|| "level0_node0".to_string());

    group.bench_function("toggle_expand", |b| {
        b.iter(|| {
            tree_state.toggle_expand(black_box(&expandable_node));
        });
    });

    group.finish();
}

fn bench_state_persistence(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_persistence");

    let initial_paths = create_large_cgroup_hierarchy(4, 12);
    let mut tree_state = CGroupTreeState::default();
    tree_state.build_from_paths(&initial_paths);

    // Expand some nodes
    for (i, (path, node)) in tree_state.nodes.clone().iter().enumerate() {
        if i >= 5 {
            break;
        }
        if !node.children.is_empty() {
            tree_state.toggle_expand(path);
        }
    }

    // Create updated paths with some additions
    let mut updated_paths = initial_paths.clone();
    for i in 0..10 {
        updated_paths.insert(
            format!("/sys/fs/cgroup/new_slice_{}", i),
            ResourceStats::default(),
        );
    }

    group.bench_function("rebuild_with_state_preservation", |b| {
        b.iter(|| {
            tree_state.build_from_paths(black_box(&updated_paths));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_tree_build,
    bench_tree_navigation,
    bench_tree_expansion,
    bench_state_persistence
);
criterion_main!(benches);
