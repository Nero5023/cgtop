# Building a Rust TUI for cgroup v2 Monitoring

## Project Overview

This guide provides architectural guidance for creating a terminal user interface (TUI) application to monitor cgroup v2 (Control Groups version 2) in Rust, inspired by the Bottom system monitor.

## Essential Crates

### Core TUI Framework
- **ratatui** (formerly tui-rs) - Modern, feature-rich TUI framework
  - Provides widgets, layouts, and rendering capabilities
  - Active development and good performance
- **crossterm** - Cross-platform terminal manipulation
  - Event handling (keyboard/mouse)
  - Terminal setup and teardown

### cgroup v2 Interaction
- **procfs** - Parse `/proc` filesystem for process-to-cgroup mapping
- **std::fs** + custom parsing - Direct cgroup v2 filesystem interaction
- Consider creating a custom crate for cgroup v2 APIs if none exist

### System Integration
- **sysinfo** - Additional system information (processes, CPU, memory)
- **nix** - Unix system calls for advanced operations
- **libc** - Low-level system interfaces

### Async and Threading
- **tokio** - Async runtime for non-blocking I/O operations
- **crossbeam** - Lock-free data structures and channels
- **parking_lot** - High-performance synchronization primitives

### Data Processing
- **serde** - Serialization/deserialization for configuration
- **anyhow** - Error handling
- **hashbrown** - High-performance HashMap implementation

## Architecture Pattern: Multi-threaded Event-driven

### Threading Model (Based on Bottom)

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Main Thread   │    │   Input Thread   │    │ Collection Thread│
│   (UI Render)   │◄──►│ (Event Handling) │    │ (Data Gathering) │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         ▲                                               │
         │               ┌─────────────────┐             │
         └──────────────►│ Cleaning Thread │◄────────────┘
                        │ (Data Retention) │
                        └─────────────────┘
```

### Key Components

#### 1. Application State (`src/app/`)
```rust
pub struct App {
    pub cgroup_data: CGroupData,
    pub ui_state: UiState,
    pub config: Config,
    pub filters: FilterState,
    // Event channels
    pub input_receiver: Receiver<InputEvent>,
    pub data_receiver: Receiver<CGroupMetrics>,
}
```

#### 2. Data Collection (`src/collection/`)
```rust
pub struct CGroupCollector {
    pub cgroup_root: PathBuf,  // Usually /sys/fs/cgroup
    pub collection_interval: Duration,
    pub metrics_sender: Sender<CGroupMetrics>,
}

pub struct CGroupMetrics {
    pub hierarchies: Vec<CGroupHierarchy>,
    pub processes: HashMap<Pid, CGroupPath>,
    pub resource_usage: HashMap<CGroupPath, ResourceStats>,
    pub timestamp: Instant,
}
```

#### 3. UI Rendering (`src/canvas/` and `src/widgets/`)
```rust
pub struct CGroupWidget {
    pub tree_state: TreeState,
    pub selected_cgroup: Option<CGroupPath>,
    pub scroll_offset: usize,
}
```

## Data Flow Management

### 1. Collection Pipeline
```
cgroup filesystem → Parser → Aggregator → Filter → App State → UI
     (/sys/fs/cgroup)                                    (ratatui)
```

### 2. Event Handling
```
User Input → Input Thread → Event Queue → Main Thread → State Update → Re-render
```

### 3. Data Structures
```rust
// Core data types
pub struct CGroupPath(String);  // e.g., "/user.slice/user-1000.slice"
pub struct ResourceStats {
    pub memory: MemoryStats,
    pub cpu: CpuStats, 
    pub io: IoStats,
    pub pids: PidStats,
}

// Tree representation
pub struct CGroupHierarchy {
    pub root: CGroupNode,
    pub flat_map: HashMap<CGroupPath, CGroupNode>,
}

pub struct CGroupNode {
    pub path: CGroupPath,
    pub name: String,
    pub stats: ResourceStats,
    pub children: Vec<CGroupPath>,
    pub processes: Vec<ProcessInfo>,
}
```

## cgroup v2 Specific Considerations

### File System Structure
- **Root:** `/sys/fs/cgroup/` (unified hierarchy)
- **Controllers:** `cgroup.controllers`, `cgroup.subtree_control`
- **Resource files:** `memory.current`, `cpu.stat`, `io.stat`, `pids.current`

### Key Metrics to Monitor
```rust
pub struct CGroupMetrics {
    // Memory controller
    pub memory_current: u64,
    pub memory_max: Option<u64>,
    pub memory_events: MemoryEvents,
    
    // CPU controller  
    pub cpu_usage_usec: u64,
    pub cpu_user_usec: u64,
    pub cpu_system_usec: u64,
    
    // IO controller
    pub io_rbytes: u64,
    pub io_wbytes: u64,
    pub io_rios: u64,
    pub io_wios: u64,
    
    // PIDs controller
    pub pids_current: u64,
    pub pids_max: Option<u64>,
}
```

### File Parsing Strategy
```rust
impl CGroupCollector {
    fn read_memory_stats(&self, cgroup_path: &Path) -> Result<MemoryStats> {
        let memory_current = fs::read_to_string(cgroup_path.join("memory.current"))?
            .trim()
            .parse::<u64>()?;
            
        let memory_stat = self.parse_key_value_file(
            &cgroup_path.join("memory.stat")
        )?;
        
        Ok(MemoryStats {
            current: memory_current,
            // ... parse other fields
        })
    }
}
```

## UI Design Considerations

### Layout Structure
```
┌─────────────────────────────────────────────────┐
│ Title Bar: cgroup Monitor v1.0                  │
├─────────────────────────────────────────────────┤
│ cgroup Tree          │ Process List            │
│ ├─ system.slice      │ PID   Command     cgroup │
│ │  ├─ systemd.ser... │ 1234  systemd     system │  
│ │  └─ ssh.service    │ 5678  sshd        system │
│ ├─ user.slice        │ 9012  firefox     user   │
│ │  └─ user-1000.s... │                         │
│ └─ machine.slice     │                         │
├─────────────────────────────────────────────────┤
│ Resource Usage (Selected cgroup)                │
│ Memory: 256MB/1GB    CPU: 15%    IO: 1.2MB/s   │
└─────────────────────────────────────────────────┘
```

### Widget Hierarchy
```rust
pub enum Widget {
    CGroupTree(TreeWidget),
    ProcessList(TableWidget),
    ResourceGraphs(GraphWidget),
    StatusBar(ParagraphWidget),
}
```

### Navigation and Controls
- **Tree navigation:** Arrow keys, Enter to expand/collapse
- **Tab switching:** Between tree, process list, graphs
- **Filtering:** Type to filter by cgroup name or process
- **Sorting:** Sort processes by memory, CPU, etc.

## Performance Considerations

### Efficient Data Collection
```rust
// Use async I/O for reading multiple cgroup files
async fn collect_cgroup_metrics(&self) -> Result<CGroupMetrics> {
    let futures: Vec<_> = self.cgroup_paths.iter()
        .map(|path| self.read_cgroup_stats(path))
        .collect();
    
    let results = futures::future::join_all(futures).await;
    // Aggregate results...
}
```

### Memory Management
- **Data retention:** Keep only recent data points (configurable window)
- **Lazy loading:** Load detailed stats only for visible cgroups
- **Caching:** Cache parsed data with timestamps

### Update Strategy
- **Incremental updates:** Only update changed cgroups
- **Adaptive polling:** Faster polling for active cgroups
- **Background cleanup:** Periodically clean stale data

## Configuration Management

### Configuration Structure
```rust
#[derive(Deserialize)]
pub struct Config {
    pub update_interval: Duration,
    pub data_retention: Duration,
    pub ui: UiConfig,
    pub filters: FilterConfig,
}

#[derive(Deserialize)]  
pub struct UiConfig {
    pub default_widget: Widget,
    pub colors: ColorScheme,
    pub tree_expanded: Vec<String>,
}
```

## Error Handling Strategy

### Graceful Degradation
```rust
// Continue monitoring even if some cgroups are inaccessible
match self.read_cgroup_stats(path) {
    Ok(stats) => metrics.insert(path, stats),
    Err(e) => {
        log::warn!("Failed to read cgroup {}: {}", path.display(), e);
        // Use cached data or mark as unavailable
    }
}
```

## Testing Strategy

### Unit Tests
- Parser functions for cgroup files
- Data structure transformations  
- UI state transitions

### Integration Tests
- End-to-end data collection
- UI rendering with mock data
- Configuration loading

### Platform Testing
- Different Linux distributions
- Various cgroup v2 configurations
- Permission scenarios

## Packaging and Distribution

### Build Configuration
```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = "symbols"

[features]
default = []
debug-logging = []
```

### Installation Methods
- **Cargo:** `cargo install cgrouptui`
- **Package managers:** .deb, .rpm packages
- **Static binary:** Self-contained executable

## Development Phases

### Phase 1: Core Infrastructure
1. Set up basic TUI with ratatui
2. Implement cgroup v2 file parsing
3. Create basic tree widget

### Phase 2: Data Collection
1. Multi-threaded data collection
2. Process-to-cgroup mapping
3. Real-time updates

### Phase 3: UI Enhancement
1. Resource usage graphs
2. Process list integration  
3. Keyboard navigation

### Phase 4: Advanced Features
1. Filtering and searching
2. Configuration management
3. Export capabilities

This architecture provides a solid foundation for building a performant and user-friendly cgroup v2 monitoring tool, leveraging the proven patterns from Bottom while adapting them for cgroup-specific requirements.