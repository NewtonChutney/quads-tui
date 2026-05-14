# quads-tui Design

A Rust TUI for QUADS infrastructure scheduling, built with ratatui + crossterm.

## Architecture

```
src/
├── main.rs              # Entry point, tokio runtime, terminal setup, key handling
├── app.rs               # App state: Screen enum, selection indices, popups
├── config.rs            # ~/.config/quads/quads-tui.toml read/write
├── event.rs             # Crossterm event polling (key, tick, resize)
├── session.rs           # Multi-session manager, per-session cached data
├── api/
│   ├── mod.rs           # Re-exports
│   ├── models.rs        # Serde structs (Host, Cloud, Assignment, Schedule, etc.)
│   └── endpoints.rs     # ApiClient: reqwest + JWT auth, typed endpoint methods
└── ui/
    ├── mod.rs            # Top-level render dispatch
    ├── dashboard.rs      # Server list with status, summary stats
    ├── hosts.rs          # Hosts table with filters, search, multi-select
    ├── assignments.rs    # Sidebar + detail pane
    ├── clouds.rs         # Clouds table (mine/all filter)
    └── widgets.rs        # Status bar, help bar, tab bar, popups
```

## Screens

- **Dashboard** — server list with status indicators (● green=authenticated, ● red=read-only, ○ grey=disconnected), summary stats. Enter selects/logs in/logs out the focused server. `n` to add, `e` to edit servers.
- **Hosts** — table with Name, Model, Cloud, Status columns. Fuzzy search (`/`) across host names. Filter popup (`f`) toggles available/scheduled/broken/retired. Tab cycles all/self-schedulable view. Multi-select with Space for self-scheduling.
- **Assignments** — sidebar lists assignments, right pane shows detail. Tab toggles mine/all. Fuzzy search (`/`) across all columns. `t` to terminate (Enter/Esc confirm).
- **Clouds** — table with mine/all filter. Fuzzy search (`/`) across name, owner, ticket, description.

## Navigation

| Key   | Dashboard    | Tab views              |
|-------|--------------|------------------------|
| h     | → Hosts      | → Hosts                |
| a     | → Assign     | → Assign               |
| c     | → Clouds     | → Clouds               |
| d     | —            | → Dashboard            |
| Esc   | quit         | clear search / → Dashboard |
| q     | quit         | quit                   |
| j/k   | nav servers  | nav rows               |
| /     | —            | fuzzy search           |
| f     | —            | filter popup (hosts)   |
| Tab   | —            | toggle view mode       |
| t     | —            | terminate (assignments)|
| Space | —            | select host (hosts)    |
| s     | —            | schedule hosts (hosts) |
| r     | refresh      | refresh                |
| x     | auto-refresh | auto-refresh           |

## Workflows

### Authentication

Server connections are managed from the Dashboard. Enter on a disconnected server opens a login form. On auth failure, the user is prompted to register. Soft logout (Enter on authenticated server) clears auth state but preserves cached public data (hosts, clouds, assignments, schedules).

### Self-Schedule

1. In Hosts tab, press Tab to switch to self-schedulable view
2. Press Space to multi-select hosts (or act on the single host under cursor)
3. Press `s` to open the assignment picker
4. Choose an existing self-schedule assignment, or create a new one (description + wipe/Q-in-Q toggles)
5. Hosts are scheduled to the assignment's cloud via the API, then data refreshes

## API

Talks to QUADS v3 REST API (`/api/v3/`). JWT Bearer auth via `POST /login/` (Basic auth).

Key endpoints: `/hosts/`, `/clouds/`, `/clouds/summary/`, `/assignments/active/`, `/schedules/current/`, `/assignments/self/` (create self-schedule assignment), `/schedules/` (schedule host to cloud), `/assignments/terminate/{id}/`.

## Config

```toml
default_server = "my-quads"

[servers.my-quads]
url = "https://quads.example.com"
username = "user@example.com"
password = "..."
verify_ssl = true
```
