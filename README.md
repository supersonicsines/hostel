# HOSTEL

A simple TUI for localhost services for vibecoders who iterate simultaneously.

HOSTEL shows TCP listeners bound to localhost on ports `1024..9999`, then lets you:

- move with arrows or vim keys
- press `Enter` to open `http://localhost:{port}`
- press `m` to attach a short live memo
- press `u` to set a service-specific open path like `/docs`
- press `f` to hide services by persistent keyword filters
- press `k`/`K` to safely SIGTERM after verification

It also detects small automatic badges like `Vite`, `Astro`, `Next`, `API`, and `Rust`.
It is intentionally not a general process manager.
