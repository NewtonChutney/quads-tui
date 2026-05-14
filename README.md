# quads-tui

A terminal UI for [QUADS](https://github.com/redhat-performance/quads) bare-metal infrastructure scheduling, built with Rust, [ratatui](https://github.com/ratatui/ratatui), and [crossterm](https://github.com/crossterm-rs/crossterm).

## Features

- Multi-server management with saved credentials
- Host browsing with status filters and fuzzy search
- Self-schedule workflow: select hosts, pick or create assignments, schedule in bulk
- Assignment and cloud management with mine/all views
- Auto-refresh and manual refresh
- Single-instance enforcement (only one copy runs at a time)

## Supported Platforms

- Linux
- macOS
- Windows

All dependencies are cross-platform. No platform-specific code is used.

## Requirements

- Rust 1.85+ (edition 2024)
- Network access to a QUADS v3 API server (VPN may be required)

## Building

```sh
cargo build --release
```

The binary will be at `target/release/quads-tui`.

## Running

```sh
cargo run
# or
./target/release/quads-tui
```

On first run, add a server from the Dashboard with `n`, then connect with `Enter`.

## Configuration

Config is stored at `~/.config/quads/quads-tui.toml`:

```toml
default_server = "my-quads"

[servers.my-quads]
url = "https://quads.example.com"
username = "user@example.com"
password = "..."
verify_ssl = true
```

## Logging

Logs are written to `~/.config/quads/quads-tui.log` (overwritten each launch).

Default log level is `Info`. To enable debug logging, set the `RUST_LOG` environment variable:

```sh
RUST_LOG=debug cargo run
```

To watch logs while the app is running:

```sh
tail -f ~/.config/quads/quads-tui.log
```

## Key Bindings

| Key       | Action                                  |
|-----------|-----------------------------------------|
| `q`       | Quit                                    |
| `h/a/c/d` | Navigate to Hosts/Assignments/Clouds/Dashboard |
| `Left/Right` | Navigate between tabs                |
| `j/k`     | Move up/down                            |
| `Enter`   | Select / confirm / view details         |
| `Esc`     | Back / cancel / clear search            |
| `/`       | Fuzzy search                            |
| `Tab`     | Toggle view mode (all/mine, all/self-schedulable) |
| `f`       | Filter popup (hosts, all view only)     |
| `Space`   | Multi-select host (self-schedulable view) |
| `s`       | Schedule selected hosts                 |
| `t`       | Terminate assignment                    |
| `u`       | Unschedule host from assignment         |
| `r`       | Refresh data                            |
| `x`       | Toggle auto-refresh                     |

## Contributing

1. Fork the repo and create a feature branch
2. Make your changes
3. Run `cargo build` and `cargo clippy` to check for errors and warnings
4. Test against a QUADS server (or mock one)
5. Submit a pull request with a description of the changes

### Project Structure

```
src/
  main.rs           - Entry point, key handling, async task management
  app.rs            - App state, screen/popup enums, filtering logic
  config.rs         - Config file read/write
  event.rs          - Terminal event polling
  session.rs        - Multi-session manager with cached data
  api/
    endpoints.rs    - HTTP client for QUADS v3 API
    models.rs       - Serde structs for API responses
  ui/
    mod.rs          - Top-level render dispatch
    dashboard.rs    - Server list and summary stats
    hosts.rs        - Hosts table
    assignments.rs  - Assignment sidebar + detail pane
    clouds.rs       - Clouds table
    widgets.rs      - Help bar, popups, shared UI components
```

## License

See [LICENSE](LICENSE) for details.
