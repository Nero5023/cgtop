# Testing Guide for cgtop

This document describes the comprehensive testing strategy for the cgtop application.

## Overview

The cgtop application uses multiple testing approaches to ensure reliability and performance:

1. **Unit Tests** - Test individual components in isolation
2. **Integration Tests** - Test component interactions and workflows  
3. **Property-Based Tests** - Test invariants with generated input
4. **Benchmarks** - Measure performance of critical operations
5. **Mock Tests** - Test with simulated cgroup data

## Test Structure

```
tests/
├── common/
│   └── mod.rs              # Shared test utilities and mock data
├── tree_state_tests.rs     # Unit tests for tree state management
├── collection_tests.rs     # Tests for cgroup data collection
├── integration_tests.rs    # Integration tests for app workflows
└── property_tests.rs       # Property-based tests with generated data

benches/
└── tree_benchmarks.rs      # Performance benchmarks
```

## Running Tests

### Quick Start
```bash
# Run all tests
./test_runner.sh

# Run specific test suites
./test_runner.sh unit
./test_runner.sh integration
./test_runner.sh property
./test_runner.sh bench
```

### Manual Commands
```bash
# Unit tests
cargo test --lib

# Specific test files
cargo test --test tree_state_tests
cargo test --test collection_tests
cargo test --test integration_tests
cargo test --test property_tests

# Benchmarks
cargo bench --bench tree_benchmarks

# With output
cargo test -- --nocapture

# Run tests in release mode (faster)
cargo test --release
```

## Test Categories

### 1. Unit Tests (`tree_state_tests.rs`)

Tests the core tree state management functionality:

- **Tree Construction**: Building trees from flat cgroup paths
- **Navigation**: Moving selection up/down through visible nodes
- **Expansion**: Expanding/collapsing tree nodes
- **State Persistence**: Preserving state across data updates
- **Edge Cases**: Empty data, invalid paths, etc.

**Key Test Cases:**
```rust
test_tree_state_creation()              // Basic initialization
test_build_from_paths_simple()          // Tree building from paths
test_tree_navigation()                  // Selection navigation
test_tree_expansion()                   // Node expansion/collapse
test_state_persistence_across_updates() // State preservation
test_complex_hierarchy()               // Deep tree structures
test_edge_cases()                      // Error conditions
```

### 2. Collection Tests (`collection_tests.rs`)

Tests cgroup data collection and parsing:

- **File Parsing**: Memory, CPU, IO, PID statistics parsing
- **Error Handling**: Missing files, malformed data, permissions
- **Mock Filesystem**: Testing with temporary cgroup structures
- **Statistics Calculation**: Aggregation and normalization

**Key Test Cases:**
```rust
test_memory_stats_parsing()     // Parse memory.current, memory.max
test_cpu_stats_parsing()        // Parse cpu.stat file
test_io_stats_parsing()         // Parse io.stat file  
test_collect_cgroup_tree()      // Full hierarchy collection
test_malformed_file_content()   // Error handling
```

### 3. Integration Tests (`integration_tests.rs`)

Tests complete application workflows:

- **App State Management**: Full app initialization and updates
- **Event Processing**: Handling metrics updates and UI events
- **Component Integration**: Tree state + UI state synchronization
- **Multi-Update Scenarios**: Multiple data refreshes

**Key Test Cases:**
```rust
test_app_initialization()           // App startup state
test_app_metrics_update()          // Processing cgroup data
test_tree_state_updates_with_selection() // UI consistency
test_expansion_state_persistence()  // State across updates
test_app_with_multiple_updates()   // Continuous operation
```

### 4. Property-Based Tests (`property_tests.rs`)

Uses `proptest` to test invariants with generated data:

- **Tree Invariants**: Parent-child relationships always valid
- **Navigation Safety**: Selection always points to valid nodes
- **Expansion Consistency**: Visible nodes always reachable
- **State Preservation**: Expansion state correctly maintained

**Key Properties Tested:**
```rust
tree_build_doesnt_panic()           // Never crashes on valid input
navigation_invariants()             // Selection stays valid
expansion_invariants()              // Expansion state consistent  
state_persistence_invariants()      // State properly preserved
hierarchy_correctness()             // Tree structure valid
visible_nodes_consistency()         // Visibility rules followed
```

### 5. Benchmarks (`tree_benchmarks.rs`)

Performance tests for critical operations:

- **Tree Building**: Time to build tree from various sizes of data
- **Navigation**: Performance of selection changes
- **Expansion**: Cost of expanding/collapsing nodes
- **State Persistence**: Overhead of preserving state across updates

**Benchmark Groups:**
```rust
bench_tree_build()         // Tree construction performance
bench_tree_navigation()    // Selection change performance
bench_tree_expansion()     // Expansion toggle performance
bench_state_persistence()  // State preservation overhead
```

## Mock Data and Utilities

### Common Test Utilities (`tests/common/mod.rs`)

Provides shared functionality for all tests:

```rust
create_mock_resource_stats()    // Generate realistic ResourceStats
create_test_cgroup_paths()      // Complex cgroup hierarchy
create_simple_cgroup_paths()    // Simple test hierarchy
```

### Mock Cgroup Filesystem

For testing data collection, we create temporary directories that mimic the cgroup filesystem structure:

```rust
let temp_dir = TempDir::new().unwrap();
let cgroup_root = create_mock_cgroup_filesystem(&temp_dir);
```

This allows testing file parsing without requiring actual cgroup access.

## Test Data Patterns

### Hierarchical Test Data
```
/sys/fs/cgroup/
├── system.slice/
│   ├── systemd-logind.service
│   ├── ssh.service
│   └── nginx.service
├── user.slice/
│   └── user-1000.slice/
│       ├── session-2.scope
│       └── user@1000.service/
│           └── app.slice/
│               └── firefox.service
└── init.scope
```

### Property Test Generators
- **Paths**: Generate valid cgroup path strings
- **Hierarchies**: Create nested directory structures
- **Operations**: Random navigation and expansion sequences

## Performance Expectations

Based on benchmarks, the application should handle:

- **Tree Building**: <10ms for 500 cgroups
- **Navigation**: <1μs per selection change
- **Expansion**: <5ms per toggle operation
- **State Persistence**: <20ms for 500 cgroups with 50 expanded nodes

## Coverage Goals

Target coverage levels:
- **Core Logic**: >95% (tree state, data collection)
- **Integration**: >80% (event handling, UI state)
- **Overall**: >85%

## Continuous Integration

For CI/CD pipelines:

```bash
# Fast test suite (for PRs)
cargo test --lib
cargo test --test tree_state_tests
cargo clippy -- -D warnings

# Full test suite (for releases)  
./test_runner.sh all
```

## Adding New Tests

### For New Features
1. Add unit tests in appropriate `*_tests.rs` file
2. Add integration test scenarios if feature affects app workflow
3. Consider property-based tests for complex logic
4. Add benchmarks for performance-critical code

### Test Naming Convention
- `test_<functionality>()` - Basic functionality tests
- `test_<component>_<scenario>()` - Specific scenario tests
- `test_<edge_case>()` - Error conditions and edge cases

### Mock Data Guidelines
- Use realistic cgroup paths and values
- Test both small and large hierarchies
- Include error conditions (missing files, parse failures)
- Vary data to test different code paths

## Debugging Tests

### Running Individual Tests
```bash
cargo test test_tree_state_creation -- --exact --nocapture
```

### Property Test Debugging
```bash
# Run with smaller input sizes for debugging
PROPTEST_CASES=10 cargo test property_tests
```

### Benchmark Analysis
```bash
cargo bench --bench tree_benchmarks
# Results in target/criterion/report/index.html
```

This comprehensive testing strategy ensures the cgtop application is reliable, performant, and maintainable across different environments and usage patterns.