# HOSTEL

*The front desk for localhost.*

A calm monochrome TUI, CLI, and MCP server for loopback localhost services.

HOSTEL is for developers and coding agents running many local servers at once:
frontend dev servers, API servers, docs sites, dashboards, tunnel helpers, and
scratch experiments. It is intentionally not a general process manager.

## What HOSTEL Sees

HOSTEL lists TCP listeners bound to loopback addresses on ports `1024..9999`.

Included by default:

- `127.*`
- `::1`
- `localhost`, when scanner output uses that name

Excluded by default:

- wildcard listeners such as `0.0.0.0` or `*`
- public or LAN interface listeners
- ports below `1024` or above `9999`

On macOS, HOSTEL scans `lsof -nP -iTCP -sTCP:LISTEN`. On Linux, it parses
`/proc/net/tcp` and `/proc/net/tcp6`, then maps socket inodes back to PIDs.

## TUI

Run:

```bash
hostel
```

```text
                                   HOSTEL

                        localhost services 1024-9999

╭──────────────────────────────────────────────────────────────────────────╮
│ PORT   PID       SERVICE                                                 │
├──────────────────────────────────────────────────────────────────────────┤
│ 3000   48213     node                                                    │
│ 5173   47102     Checkout frontend  node  Vite  [vite]  @codex           │
│                  Vite app for checkout edits                             │
│ 8000   46711     API server  python3  API  → /docs                       │
│                                                                          │
╰──────────────────────────────────────────────────────────────────────────╯

                  ↑↓ select   Enter open   ? help   q quit
                               HOSTEL I.0245
```

On first launch, HOSTEL asks you to choose regular or vim-style keybindings.
The main screen shows matching services sorted by port, with PID, title or
process name, automatic service kind badges, tags, source, open path, command
details when there is room, and memo subtitles.

Regular mode:

```text
Up/Down select   Enter open   k kill   t title   m memo   g tags   u url   f filter   r refresh   ? help   q quit
```

Vim mode:

```text
j/k select       Enter open   K kill   t title   m memo   g tags   u url   f filter   r refresh   ? help   q quit
```

Press `?` for the in-app help overlay. Editors save with `Enter`, cancel with
`Esc`, and clear the field when saved empty.

## Metadata

HOSTEL can attach metadata to a live service:

- title: short display name, up to 80 characters
- memo: one-line note, up to 100 characters
- tags: lowercase, sanitized labels
- open path: appended to the localhost URL, such as `/docs`
- scheme: `http` or `https`, set through CLI or MCP
- source: agent or tool name, such as `codex` or `claude`

Open paths are normalized: `docs` becomes `/docs`, while values beginning with
`/`, `?`, or `#` are kept as-is. The default open URL is
`http://localhost:{port}/`.

Metadata is keyed by a stable service fingerprint based on port, process name,
and command. Stale metadata is pruned when services disappear.

## Safe Stop

Stopping a service is TUI-only. The CLI and MCP server cannot kill processes.

When you request a stop, HOSTEL:

- refuses unsafe PIDs such as `0` and `1`
- opens a confirmation overlay
- performs a fresh scan before sending a signal
- verifies the same PID is still listening on the same port
- sends `SIGTERM` only
- clears metadata for the stopped service
- reports whether the service disappeared or kept listening

## CLI

```bash
hostel --version
hostel list
hostel list --json
hostel label --port 5173 --title "Frontend" --memo "Vite app" --tag vite --source codex
hostel label --port 8000 --title "API server" --path /docs --scheme http --tag api
hostel open 5173
hostel open --port 5173 --pid 12345
hostel clear 5173
hostel clear --port 5173 --pid 12345
hostel mcp
```

`label` requires `--port` plus at least one metadata field. Supported fields are
`--title`, `--memo`, repeated `--tag`, comma-separated `--tags`, `--url` or
`--path`, `--scheme http|https`, and `--source`.

If more than one live service matches a port, add `--pid`.

## MCP

Run `hostel mcp` to expose HOSTEL over stdio to MCP clients.

Tools:

- `list_services`
- `set_service_metadata`
- `clear_service_metadata`
- `open_service`

Resource:

- `hostel://services`

MCP responses use the same service view as `hostel list --json`, including
`id`, `pid`, `port`, `address`, `process_name`, `command`, `kind`, `title`,
`memo`, `tags`, `url_path`, `scheme`, `source`, and `url`.

## Config And Data

HOSTEL stores files under the platform config directory plus `hostel`, unless
`HOSTEL_CONFIG_DIR` is set.

Common paths:

- macOS: `~/Library/Application Support/hostel`
- Linux: `~/.config/hostel`

`config.json` supports:

```json
{
  "keybind_mode": "regular",
  "hidden_keywords": []
}
```

Unknown config keys are rejected. `data.json` stores live-service metadata and
legacy memo/open-path maps that are migrated and pruned during scans.

## Versioning

The user-facing product version is not Cargo SemVer. Its source of truth is
`PRODUCT_VERSION` in `src/main.rs`; it is shown by the TUI footer,
`hostel --version`, and MCP `serverInfo.version`.

`Cargo.toml` package `version` remains Cargo-compatible SemVer.

## Development

```bash
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

Install the release binary as `~/.local/bin/hostel`:

```bash
make install
```

Check the product version from source:

```bash
make version
```
