---
name: hostel
description: Use when starting, listing, opening, or annotating localhost services with Hostel. Helps agents tag live dev servers with titles, memos, tags, sources, and open paths through the Hostel CLI or MCP server.
---

# Hostel

Hostel is the local inventory for loopback dev services. Use it to keep human-readable context attached to servers you or another agent starts.

## Workflow

After starting a local server, identify its port from command output or `hostel list --json`, then annotate it:

```bash
hostel label --port 5173 --title "Checkout frontend" --memo "Vite app for checkout edits" --tag frontend --tag vite --source codex
```

Use short titles, lowercase tags, and a source matching the agent or tool: `codex`, `claude`, `opencode`, or another clear name.

Good titles name the useful thing, not the process: `Docs site`, `API server`, `Checkout frontend`, `Storybook`.

## Commands

```bash
hostel list --json
hostel label --port 5173 --title "Frontend" --memo "Vite app" --tag vite --source codex
hostel label --port 8000 --title "API server" --url /docs --tag api --source codex
hostel open 5173
hostel clear 5173
```

If more than one live service matches a port, add `--pid`.

## MCP

If Hostel is configured as an MCP server with `hostel mcp`, prefer these tools when available:

- `list_services`
- `set_service_metadata`
- `clear_service_metadata`
- `open_service`

Use `set_service_metadata` after launching a server. Use `open_service` only when the user wants the service opened.

Do not kill services through this skill. Hostel keeps process termination as a human TUI action.

