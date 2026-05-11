# HOSTEL

A calm TUI, CLI, and MCP surface for localhost services.

HOSTEL shows loopback TCP listeners on ports `1024..9999`, keeps short titles, tags, memos, and open paths, then lets you:

- move with arrows or vim keys
- press `Enter` to open `http://localhost:{port}`
- press `t` / `g` / `m` to title, tag, or memo a service
- press `u` to set a service-specific open path like `/docs`
- press `f` to hide services by persistent keyword filters
- press `k`/`K` to safely SIGTERM after verification

CLI:

```bash
hostel list --json
hostel label --port 5173 --title "Frontend" --memo "Vite app" --tag vite
hostel open 5173
```

MCP clients can run `hostel mcp` so agents can list and annotate live services. Kill stays TUI-only.
