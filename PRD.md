# HOSTEL PRD

## Product Summary

HOSTEL is a beautiful, minimal terminal UI for AI vibecoders who need to see which local services are running on localhost.

HOSTEL shows only TCP services listening on localhost ports `1024..9999`. It lets the user select a service, open it in the browser, attach a short memo, or safely kill it.

HOSTEL is not a general process manager. It is a localhost service switchboard.

## HOSTEL Logo

The HOSTEL logo is part of the product identity. It must appear during loading and remain visible on the main screen whenever the terminal is wide enough.

```text
      ___           ___           ___                         ___                            
     /\  \         /\  \         /\__\                       /\__\                           
     \:\  \       /::\  \       /:/ _/_         ___         /:/ _/_                          
      \:\  \     /:/\:\  \     /:/ /\  \       /\__\       /:/ /\__\                         
  ___ /::\  \   /:/  \:\  \   /:/ /::\  \     /:/  /      /:/ /:/ _/_   ___     ___          
 /\  /:/\:\__\ /:/__/ \:\__\ /:/_/:/\:\__\   /:/__/      /:/_/:/ /\__\ /\  \   /\__\         
 \:\/:/  \/__/ \:\  \ /:/  / \:\/:/ /:/  /  /::\  \      \:\/:/ /:/  / \:\  \ /:/  /         
  \::/__/       \:\  /:/  /   \::/ /:/  /  /:/\:\  \      \::/_/:/  /   \:\  /:/  /          
   \:\  \        \:\/:/  /     \/_/:/  /   \/__\:\  \      \:\/:/  /     \:\/:/  /           
    \:\__\        \::/  /        /:/  /         \:\__\      \::/  /       \::/  /            
     \/__/         \/__/         \/__/           \/__/       \/__/         \/__/             
```

Fallback logo for narrow terminals:

```text
HOSTEL
```

## Target User

HOSTEL is for developers and AI vibecoders running many local dev servers at once:

- Vite
- Next.js
- Astro
- API servers
- docs servers
- dashboards
- local tools
- scratch experiments

The user wants calm situational awareness, not a complex operations dashboard.

## Core Product Principle

Show me what is running locally, let me open it, let me annotate it briefly, and let me stop it safely.

Everything else is noise.

## Scope

HOSTEL must:

- Show only localhost TCP listeners on ports `1024..9999`.
- Hide public-interface and wildcard listeners by default.
- Show a beautiful black-and-white TUI.
- Keep the HOSTEL logo visible during loading and on the main screen.
- Support regular and vim keybinding modes.
- Open selected services in the default browser.
- Kill selected services only after confirmation and verification.
- Support short inline memos for live services.

HOSTEL must not:

- Manage arbitrary system processes.
- Show every process from `ps` or `sysinfo`.
- Include workspaces.
- Include tags.
- Include themes.
- Include process spawning.
- Include log streaming.
- Include a split process/log pane.
- Include complex configuration for unsupported features.

## Service Definition

A service is a process with a listening TCP socket bound to localhost.

Valid bind addresses:

- `127.0.0.1`
- `::1`
- `localhost`, if present in scanner output

Valid ports:

- `1024..9999`

Invalid by default:

- `0.0.0.0`
- `::`
- LAN IPs
- public IPs
- ports below `1024`
- ports above `9999`

## Service Model

```rust
struct LocalService {
    pid: u32,
    port: u16,
    address: String,
    process_name: String,
    command: String,
    memo: Option<String>,
}
```

Services are sorted by port ascending.

## Main User Flow

1. User launches `hostel`.
2. HOSTEL shows a beautiful loading screen for about 1.5 to 2 seconds.
3. If first run, HOSTEL shows a full-screen keybinding selector.
4. HOSTEL enters the main service list.
5. User moves with arrow keys or vim keys.
6. User presses `Enter` to open `http://localhost:{port}`.
7. User presses `m` to add a short memo.
8. User presses `k` or `K` to kill a service after confirmation.

## Visual Direction

HOSTEL should feel like a premium monochrome TUI.

Style:

- black background
- white foreground
- restrained gray borders
- clean spacing
- centered composition
- no clutter
- no bright cyberpunk colors
- no overdecorated dashboard UI

The UI should be simple enough for AI vibecoders to understand instantly.

## Loading Screen

On launch, show a loading screen for about 1.5 to 2 seconds.

Requirements:

- black background
- white HOSTEL logo
- subtle animated loading treatment
- enjoyable enough to be seen, not flashed
- skippable with `Enter` or `Space`

Use the full ASCII logo when terminal width allows. Use the compact `HOSTEL` fallback on narrow terminals.

## First-Run Keybinding Selector

If `~/.config/hostel/config.json` does not exist, show a full-screen split selector.

The screen is divided into two large selectable halves:

```text
╭──────────────────────────────╮╭──────────────────────────────╮
│                              ││                              │
│          REGULAR             ││             VIM              │
│                              ││                              │
│      Arrow keys to move      ││         j / k to move        │
│                              ││                              │
│      Press Enter or ←        ││        Press Enter or →      │
│                              ││                              │
╰──────────────────────────────╯╰──────────────────────────────╯
```

Behavior:

- Left/right arrows move selection.
- Enter confirms.
- Save the selected mode to config.
- Continue into the main TUI.

## Main Screen Layout

The HOSTEL logo must always be visible at the top or top-middle of the main screen. Use the full logo when there is room and the compact logo otherwise.

Example layout:

```text
                         HOSTEL
                  localhost services 1024-9999

        ╭────────────────────────────────────────────╮
        │ PORT   PID      SERVICE                    │
        │ 5173   91231    node                       │
        │                 frontend                   │
        │ 3000   88410    next-server                │
        │                 dashboard                   │
        │ 8080   77221    python3                    │
        │                 api                         │
        ╰────────────────────────────────────────────╯

             ↑↓ select   Enter open   k kill   m memo   q quit
```

Requirements:

- The service list is centered.
- The list is scrollable.
- The selected row is clearly highlighted.
- Memos appear inline as subtitles under their process.
- Borders must be enforced.
- Text must never bleed through borders.
- Long text must truncate cleanly with ellipsis.
- Layout must adapt to narrow terminals.
- If the terminal is too small, show a centered message.

Empty state:

```text
No localhost services on ports 1024-9999
```

## Keybindings

Regular mode:

- `Up` / `Down`: move selection
- `Enter`: open selected service in browser
- `k`: open kill confirmation
- `m`: edit memo
- `r`: refresh service list
- `q`: quit
- `?`: optional small help overlay

Vim mode:

- `j` / `k`: move selection
- Arrow keys also work
- `Enter`: open selected service in browser
- `K`: open kill confirmation
- `m`: edit memo
- `r`: refresh service list
- `q`: quit
- `?`: optional small help overlay

Do not add more bindings unless necessary.

## Opening Services

When the user presses `Enter`, open:

```text
http://localhost:{port}
```

Use the platform default browser:

- macOS: `open`
- Linux: `xdg-open`

After opening, show a short status message.

## Memos

Pressing `m` opens a small centered memo editor for the selected service.

Memo requirements:

- Maximum length: 100 characters.
- Single-line memo only.
- Memo appears inline as a subtitle under the process row.
- Memo is attached to the live service identity.
- Recommended memo key: `{pid}:{port}:{process_name}`.
- Memo is wiped when the service disappears.
- Memo is wiped immediately after HOSTEL successfully kills that service.
- Memos should not outlive the process they describe.
- If memos are persisted, stale memo entries must be removed on scan when their service is no longer present.

Memo editor behavior:

- Typing edits the memo.
- `Enter` saves.
- `Esc` cancels.
- Input beyond 100 characters is ignored or rejected cleanly.
- Empty memo clears the memo.

## Killing Services

Pressing kill opens a confirmation overlay.

Example:

```text
Kill node on localhost:5173?
PID 91231

Enter confirm   Esc cancel
```

Safety requirements:

- Never call `kill(0, ...)`.
- Never kill if PID is missing.
- Never kill if PID is zero.
- Never kill if PID is invalid.
- Before killing, rescan and verify that the same PID is still listening on the selected localhost port.
- If verification fails, do not kill.
- Show an error status if verification fails.
- Send SIGTERM only.
- Do not implement SIGKILL unless explicitly requested.
- After successful kill, wipe the memo for that service.
- After successful kill, refresh the service list.

## Scanner Requirements

Target platforms:

- macOS
- Linux

Scanner behavior:

- Detect TCP listening sockets.
- Include only localhost listeners.
- Include only ports `1024..9999`.
- Map each service to PID.
- Include process name.
- Include command if available.
- Refresh automatically every 2 seconds.
- Refresh manually with `r`.

macOS:

- Use `lsof -nP -iTCP -sTCP:LISTEN`.
- Parse PID, command/name, local address, and port.
- Include only loopback listeners.

Linux:

- Parse `/proc/net/tcp` and `/proc/net/tcp6`.
- Map socket inode to PID by scanning `/proc/[pid]/fd`.
- Decode IPv4 and IPv6 loopback addresses correctly.
- Include only loopback listeners.

## Config And Data

Config path:

```text
~/.config/hostel/config.json
```

Data path:

```text
~/.config/hostel/data.json
```

Config should contain only supported keys.

Suggested config:

```json
{
  "keybind_mode": "regular"
}
```

Suggested data:

```json
{
  "memos": {}
}
```

If memo persistence is implemented, stale memos must be cleaned automatically when services disappear.

## Versioning

Keep product display version separate from Cargo SemVer.

- `Cargo.toml` version must remain Cargo-compatible.
- Product display version source of truth must be `PRODUCT_VERSION` in `src/main.rs`.
- `hostel --version` must print the product display version.
- Use the custom display version format described in `AGENTS.md`.

## Code Quality

- Clean production Rust.
- No `unwrap()` in production paths.
- Use `anyhow::Result` where appropriate.
- Keep modules simple.
- Remove old features that do not match this PRD.
- Do not leave stubs or placeholders.
- UI code should render.
- App/state code should decide behavior.
- Scanner code should be testable.

Recommended simplified structure:

```text
src/
  main.rs
  app.rs
  config.rs
  scanner.rs
  service.rs
  ui.rs
```

Old modules for workspaces, tags, spawned processes, logs, and themes should be deleted or unused.

## Tests

Add focused tests for:

- macOS `lsof` parser
- Linux proc parser helpers
- loopback filtering
- port range filtering
- service sorting
- memo key generation
- 100-character memo limit
- stale memo cleanup
- PID safety validation
- kill verification logic

Before finishing, run:

```bash
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

## Acceptance Criteria

HOSTEL is acceptable when:

- It shows only localhost TCP listeners on ports `1024..9999`.
- It does not show arbitrary system processes.
- The loading screen is beautiful and lasts long enough to be seen.
- The logo is visible on the main screen.
- First run uses a full-screen split keybinding selector.
- Regular arrow-key mode works.
- Vim mode works.
- `Enter` opens the selected service in the browser.
- `k` or `K` opens a safe kill confirmation.
- Kill never targets PID 0.
- Kill verifies PID and port before sending SIGTERM.
- `m` edits a 100-character memo.
- Memos appear inline below processes.
- Memos are wiped when processes disappear or are killed.
- Layout remains clean and bounded at normal and narrow terminal sizes.
- No old workspace/tag/theme/log/spawn clutter remains.
- Tests cover scanner and safety behavior.
- All required cargo checks pass.
