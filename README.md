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
- **Event-driven Multi-threaded Architecture** (Bottom-style)
  - **Main Thread**: UI Rendering and Event Processing Only
  - **Input Thread**: Keyboard/Mouse Event Capture → CGroupEvents
  - **Collection Thread**: cgroup Data Collection → Update Events
  - **Cleanup Thread**: Periodic Cleanup → Clean Events
  - **Event System**: Unified `CGroupEvent` enum with channel communication

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
- `q` / `Esc`: Quit application ✅ **WORKING - Instant response!**
- `r`: Manual refresh (background collection continues automatically)
- `j` / `↓`: Navigate down
- `k` / `↑`: Navigate up  
- `Tab`: Switch between panels
- `Enter`: Select/expand cgroup
- `?`: Help (placeholder)

### Key Improvements ✨
- **🚀 Performance**: Multi-threaded design prevents UI blocking
- **⚡ Responsiveness**: Input thread ensures instant key response
- **🔄 Auto-refresh**: Background data collection every 2 seconds
- **🛡️ Reliability**: Proper thread coordination and clean shutdown
- **📦 Fallback**: Mock data when cgroups unavailable (demos, containers)

## Building

```bash
cargo build --release
```

## Usage

```bash
cargo run
```

**Note:** The application will attempt to read from `/sys/fs/cgroup` to collect cgroup v2 information. If cgroups are not available (e.g., in containers or restricted environments), the application will automatically use mock data for demonstration purposes.

## ✅ Recent Fixes & Improvements

### Multi-threaded Event Architecture - IMPLEMENTED
- **Improvement**: Refactored from single-threaded to proper event-driven architecture
- **Inspiration**: Follows Bottom's proven multi-threaded event system design
- **Benefits**: Clean separation of concerns, better performance, proper thread coordination
- **Implementation**: 
  - `CGroupEvent` enum for all inter-thread communication
  - Dedicated input thread with crossterm event polling
  - Background data collection with automatic fallback
  - Event-driven main loop with timeout handling

### Quit Hanging Issue - RESOLVED
- **Problem**: Pressing 'q' would hang the application
- **Solution**: Input thread properly sends `Terminate` events and exits cleanly
- **Result**: 'q' and 'Esc' now work instantly with proper thread shutdown

### "Always Loading" Issue - RESOLVED  
- **Problem**: UI always showed "Loading..." even when data was available
- **Solution**: Background collection thread sends `Update` events with data/fallback
- **Result**: UI displays data immediately and updates every 2 seconds automatically

## Architecture

### Event Flow (Bottom-inspired)
```
┌─────────────────┐    ┌──────────────────────┐    ┌─────────────────┐
│   Input Thread  │    │     Main Thread      │    │Collection Thread│
│                 │───→│                      │←───│                 │
│ • Keyboard      │    │ • UI Rendering       │    │ • cgroup Data   │
│ • Mouse         │    │ • Event Processing   │    │ • Process Map   │
│ • Terminal      │    │ • State Updates      │    │ • Auto-refresh  │
└─────────────────┘    └──────────────────────┘    └─────────────────┘
         │                        │                          │
         ▼                        ▼                          ▼
    CGroupEvent::              CGroupEvent::              CGroupEvent::
    KeyInput(key)              Update(metrics)           Update(metrics)
    Terminate                                            Clean
```

### Module Structure
- `src/app/`: Application state management
- `src/events/`: Event system (`CGroupEvent` enum and utilities)
- `src/threads/`: Multi-threaded event workers
- `src/collection/`: cgroup v2 data collection and parsing
- `src/canvas/`: UI rendering and layout
- `src/widgets/`: Individual UI widget implementations

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