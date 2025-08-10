# cgroup TUI Monitor (cgtop)

A terminal user interface (TUI) application for monitoring cgroup v2 (Control Groups version 2) in Rust, inspired by the Bottom system monitor.

## Project Status

✅ **Phase 1: Core Infrastructure** - Complete
- ✅ Basic TUI framework with ratatui
- ✅ cgroup v2 file parsing implementation  
- ✅ Basic tree widget for cgroup hierarchy

✅ **Phase 2: Data Collection** - Complete
- ✅ Multi-threaded data collection system
- ✅ Process-to-cgroup mapping via procfs
- ✅ Real-time updates with event handling

## Features Implemented

### Core Architecture
- **Multi-threaded Event-driven Architecture**
  - Main Thread: UI Rendering
  - Input Thread: Event Handling
  - Collection Thread: Data Gathering
  - Cleanup Thread: Data Retention Management

### Data Collection
- cgroup v2 filesystem parsing (`/sys/fs/cgroup`)
- Resource metrics collection (Memory, CPU, IO, PIDs)
- Process-to-cgroup mapping via `/proc` filesystem
- Real-time data updates

### User Interface
- Tree view of cgroup hierarchy
- Process list showing PID, command, and cgroup association
- Resource usage display for selected cgroups
- Status bar with system information

### Keyboard Controls
- `q` / `Esc`: Quit application
- `r`: Manual refresh
- `j` / `↓`: Navigate down
- `k` / `↑`: Navigate up  
- `Tab`: Switch between panels
- `Enter`: Select/expand cgroup
- `?` / `F1`: Help (placeholder)

## Building

```bash
cargo build --release
```

## Usage

```bash
cargo run
```

**Note:** The application will attempt to read from `/sys/fs/cgroup` to collect cgroup v2 information. Ensure your system has cgroup v2 enabled and accessible.

## Architecture

The application follows a clean separation of concerns:

- `src/app/`: Application state management
- `src/collection/`: cgroup v2 data collection and parsing
- `src/canvas/`: UI rendering and layout
- `src/widgets/`: Individual UI widget implementations
- `src/threads/`: Multi-threaded coordination

## Dependencies

- **ratatui**: Modern TUI framework
- **crossterm**: Cross-platform terminal handling
- **procfs**: `/proc` filesystem parsing
- **tokio**: Async runtime
- **anyhow**: Error handling
- **serde**: Configuration serialization

## Future Enhancements

The foundation is in place for:
- Resource usage graphs and charts
- Configuration management
- Filtering and searching capabilities
- Export functionality
- Performance optimizations

## License

MIT License