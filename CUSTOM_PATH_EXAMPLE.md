# Custom cgroup Path Support

The cgtop application now supports monitoring custom cgroup filesystem paths instead of being hardcoded to `/sys/fs/cgroup`.

## Usage Examples

### Default behavior (monitors /sys/fs/cgroup)
```bash
cargo run
```

### Monitor a custom cgroup path
```bash
# Monitor a custom cgroup root directory
cargo run -- --path /custom/cgroup/root

# Monitor with verbose logging
cargo run -- --path /var/lib/lxc/containers --verbose

# Show help
cargo run -- --help
```

## Command Line Options

- `-p, --path <CGROUP_ROOT>`: Root cgroup filesystem path to monitor (default: `/sys/fs/cgroup`)
- `-v, --verbose`: Enable verbose logging
- `-h, --help`: Print help information

## Path Validation

The application validates that the specified cgroup path exists before starting:

```bash
$ cargo run -- --path /non/existent/path
Error: cgroup root path does not exist: /non/existent/path
```

## How It Works

1. **Command Line Parsing**: Uses `clap` to parse CLI arguments
2. **Path Validation**: Checks that the specified path exists before starting
3. **Data Collection**: The `CGroupCollector` uses the custom path to read cgroup statistics
4. **Mock Data**: When `CGTOP_USE_MOCK=true`, mock data is generated using the custom root path

## Testing

The application includes comprehensive tests that work with configurable cgroup paths:

```bash
# Run all tests
cargo test

# Run specific test suites
cargo test --test tree_state_tests
cargo test --test collection_tests
cargo test --test integration_tests
```

## Use Cases

This feature enables monitoring cgroup hierarchies in various environments:

- **Container Runtimes**: Monitor Docker containers under `/var/lib/docker/containers`
- **LXC Containers**: Monitor LXC containers under `/var/lib/lxc`
- **Custom Systemd**: Monitor custom systemd slice hierarchies
- **Testing**: Use temporary directories for testing and development
- **Sandboxed Environments**: Monitor restricted cgroup hierarchies

The custom path support makes cgtop more flexible and suitable for different deployment scenarios while maintaining full compatibility with the default behavior.