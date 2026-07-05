---
name: hostel
description: Use when starting, listing, opening, or annotating localhost services with Hostel. Helps agents tag live dev servers with titles, memos, tags, sources, schemes, and open paths through the Hostel CLI or MCP server.
---

# Hostel

Hostel is the local inventory for loopback dev services. Use it to keep
human-readable context attached to servers you or another agent starts.

## Workflow

After starting a local server, identify its port from command output or
`hostel list --json`, then annotate it:

```bash
hostel label --port 5173 --title "Checkout frontend" --memo "Vite app for checkout edits" --tag frontend --tag vite --source codex
```

Use short titles, lowercase tags, and a source matching the agent or tool:
`codex`, `claude`, `opencode`, or another clear name.

Good titles name the useful thing, not the process: `Docs site`, `API server`,
`Checkout frontend`, `Storybook`.

For API docs, admin screens, or debug routes, set an open path:

```bash
hostel label --port 8000 --title "API server" --path /docs --tag api --source codex
```

For HTTPS-only local services, set the scheme explicitly:

```bash
hostel label --port 8443 --title "Local HTTPS app" --scheme https --source codex
```

## Commands

```bash
hostel list
hostel list --json
hostel label --port 5173 --title "Frontend" --memo "Vite app" --tag vite --source codex
hostel label --port 8000 --title "API server" --url /docs --tag api --source codex
hostel open 5173
hostel open --port 5173 --pid 12345
hostel clear 5173
hostel clear --port 5173 --pid 12345
```

`label` requires `--port` and at least one metadata field. Supported fields are
`--title`, `--memo`, repeated `--tag`, comma-separated `--tags`, `--url` or
`--path`, `--scheme http|https`, and `--source`.

If more than one live service matches a port, add `--pid`.

## Normalization

- Titles are one line and limited to 80 characters.
- Memos are one line and limited to 100 characters.
- Tags are lowercased, sanitized, sorted, and deduplicated.
- Open paths normalize `docs` to `/docs`; paths beginning with `/`, `?`, or `#`
  are kept as-is.
- Empty saved values clear that metadata field.

## MCP

If Hostel is configured as an MCP server with `hostel mcp`, prefer these tools
when available:

- `list_services`
- `set_service_metadata`
- `clear_service_metadata`
- `open_service`

The `hostel://services` resource exposes the same live service list.

Use `set_service_metadata` after launching a server. Use `open_service` only
when the user wants the service opened.

Do not kill services through this skill. Hostel keeps process termination as a
human TUI action with confirmation and a fresh PID/port verification scan.
