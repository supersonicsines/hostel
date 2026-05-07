# Localhostel PRD

## Summary

Localhostel is a beautiful, safe terminal UI for multitasking vibecoders to manage all locally hosted services from one place. It should answer the developer's constant background question: "What is running on my machine, where is it, and what can I safely do with it?"

The product should make local development feel less scattered. Instead of juggling browser tabs, terminal panes, forgotten ports, and mystery processes, users get a clean live inventory of local services with fast actions for opening, copying, refreshing, and safely stopping services.

## Target Users

- Solo vibecoders running several local projects and AI-assisted experiments at once.
- Frontend/full-stack developers juggling Vite, Next.js, API servers, docs servers, local databases, and mock services.
- Builders who frequently lose track of which localhost tab belongs to which terminal process.
- Power users who want speed, but not at the cost of accidentally killing important background services.

## Problem

Local development is increasingly multi-process and messy. A single coding session can spawn many services across ports, tools, frameworks, and directories. Existing workflows rely on memory, `lsof`, `ps`, terminal scrollback, browser history, or ad hoc shell aliases.

This creates several pain points:

- Users forget which ports are active.
- Users open the wrong localhost URL.
- Users accidentally leave zombie dev servers running.
- Users kill the wrong process from stale or incomplete command output.
- Users cannot quickly distinguish loopback-only services from wildcard/public listeners.
- Users lack a composed, glanceable dashboard for local service state.

## Goals

- Show a live, legible list of locally hosted services.
- Make common actions fast: open, copy URL, refresh, stop.
- Make destructive actions conservative and verifiable.
- Provide a calm, polished TUI that developers actually enjoy leaving open.
- Support a future versioning/release workflow based on the custom product version format.
- Become the default local-service cockpit for multitasking developers.

## Non-Goals

- Do not become a full process manager for every system process.
- Do not show public/wildcard listeners by default without clear labeling or opt-in.
- Do not require a daemon for the first production-quality release.
- Do not hand-roll deep framework detection until the basic inventory and safety model are excellent.
- Do not prioritize decorative UI over trust, speed, and clarity.

## Product Principles

- Safety first: never make a dangerous process decision from stale or ambiguous data.
- Glanceability: users should understand the current local-service state in seconds.
- Fast hands: common actions should be one keystroke away.
- Honest config: every documented option must work or fail clearly.
- Beautiful restraint: polished terminal UI, clean spacing, clear hierarchy, no noise.
- Local-first: no network accounts, telemetry, or external service dependency by default.

## Core User Flows

### See What Is Running

The user opens `hostel` and sees a live list of local services with address, port, PID when known, and process label.

Acceptance criteria:

- Loopback services are shown by default.
- Wildcard/public listeners are hidden by default or clearly opt-in.
- Unknown PID is displayed as unavailable, not as `0`.
- IPv4 and IPv6 addresses are represented correctly.
- The app refreshes automatically at a configurable interval.

### Open A Service

The user selects a service and presses Enter to open it in the browser.

Acceptance criteria:

- The generated URL uses the actual bind address when appropriate.
- Wildcard listeners open as `localhost`.
- IPv6 URLs use bracket formatting.
- Failed open attempts produce a visible status message.

### Copy A Service URL

The user selects a service and presses `c` to copy its URL.

Acceptance criteria:

- The copied URL matches the URL that Open would use.
- Clipboard command failures are surfaced in the status line.

### Stop A Service

The user selects a service and presses `k` to stop it.

Acceptance criteria:

- Kill is disabled or rejected when PID is unknown.
- The app re-scans before signaling and confirms the selected service still matches.
- The app uses graceful termination first.
- The list refreshes after the signal attempt.
- Failure states are explicit and non-ambiguous.

### Debug Version

The user can verify which build is installed or running.

Acceptance criteria:

- The TUI visibly displays the product version.
- `hostel --version` prints the version.
- `make install` prints the installed binary version after copying.

## Functional Requirements

### Service Discovery

- Detect listening TCP services on macOS and Linux.
- Parse process name, PID when available, bind address, and port.
- Deduplicate sensibly without hiding distinct address bindings.
- Sort by port, then address.
- Filter by configured include ranges and excludes.
- Treat wildcard listeners as opt-in by default.

### Configuration

- Load config from app-specific user config locations.
- Support explicit config override through `LOCALHOSTEL_CONFIG`.
- Reject unknown config fields.
- Support theme preset, refresh interval, PID visibility, include ranges, excludes, and wildcard inclusion.

### Locking

- Prevent multiple app instances without killing an existing process.
- Store lock data in a user-owned runtime/cache location.
- Remove stale locks only after verifying the recorded PID is not running.

### Versioning

- Support the custom display version format:
  - `0.XYZN` before the first Roman major generation.
  - `I.XYZN`, `II.XYZN`, etc. for major product generations.
  - Four digits after the decimal in all cases.
- Maintain Cargo-compatible package versioning separately if needed.
- Use one source of truth for displayed product version. The current source is `PRODUCT_VERSION` in `src/main.rs`.

## UX Requirements

- First screen is the tool itself, not a landing or help view.
- Layout must remain stable while services refresh.
- Status messages should be concise and actionable.
- Destructive action failures should explain whether PID was unknown, selection changed, or signaling failed.
- The window title or status area should expose the product version.
- The UI should feel quiet, sharp, and developer-native.

## Future Features

- Project/directory detection for each service.
- Framework detection: Vite, Next.js, Rails, Django, FastAPI, Astro, Remix, etc.
- Group services by project.
- Search/filter by port, process, project, or framework.
- Health checks and response status.
- Port conflict warnings.
- Favorite/pin services.
- Recently opened services.
- Optional command reveal for known safe process metadata.
- Configurable keybindings after a typed, validated schema exists.
- Richer themes once theme customization is real.
- Optional service restart hooks.

## Success Metrics

- User can identify all relevant local services within 5 seconds.
- User can open or copy a local service URL with one command.
- User never accidentally signals PID 0 or an unknown/stale PID.
- Clippy, formatter, and tests remain clean before releases.
- The installed version can be verified without reading source files.

## Release Milestones

### 0.0001 To 0.0999: Safety Foundation

- Safe lock model.
- Loopback-focused scanner.
- Correct PID optionality.
- Version display and `--version`.
- Parser and filter tests.

### 0.1000 To 0.4999: Daily Driver

- Better service labels.
- Search/filter.
- More robust Linux/macOS parsing.
- Better error/status messaging.
- Installer polish.

### 0.5000 To 0.9847: Product Feel

- Project grouping.
- Framework detection.
- Config validation UX.
- Richer theme presets.
- Documentation and release notes.

### I.0000 And Beyond: Major Product Generation

- Stable product identity.
- Mature service cockpit workflows.
- Strong release process using Roman-major display versions.
- Optional advanced integrations without sacrificing local-first behavior.
