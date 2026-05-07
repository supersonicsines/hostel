# Agent Instructions

This repository is `localhostel`, a terminal UI for multitasking vibecoders who need to see, open, copy, and safely manage locally hosted services.

## Product Direction

- Build for developers running many local servers at once: frontend dev servers, API servers, docs sites, dashboards, tunnel helpers, and scratch experiments.
- Optimize for calm situational awareness, fast actions, and safety around process management.
- Favor clean production-quality Rust over demo code. Dangerous host operations need verification, clear state, and conservative defaults.
- The app should feel beautiful and composed, but never at the expense of correctness, legibility, or trust.

## Versioning

The user-facing product version uses a custom display format:

- Current display version source of truth: `PRODUCT_VERSION` in `src/main.rs`.
- Pre-major versions use `0.XYZN`, with exactly four digits after the decimal.
- Major versions use Roman numerals followed by a four-digit decimal suffix.
- Examples: `0.9847`, then `I.0012`, then eventually `II.0293`.
- Always zero-pad the numeric suffix to four digits.
- Treat the Roman numeral as the major product generation.
- Reset or advance the four-digit suffix according to the release plan for that generation.
- Do not replace this display format with SemVer in user-facing UI, docs, release notes, or screenshots unless the user explicitly asks.

Important Rust/Cargo constraint:

- `Cargo.toml` package `version` must remain Cargo-compatible SemVer.
- Product display version and Cargo package version are allowed to diverge.
- If `PRODUCT_VERSION` later moves to build metadata, keep one source of truth and make the TUI, `--version`, docs, and release notes read from that source.
- Never assume `Cargo.toml` alone fully represents the product versioning scheme once Roman-major display versions are introduced.

## Engineering Standards

- Preserve the safe process-management model: never turn an unknown PID into `kill 0`, never trust stale scan data for destructive actions, and never kill a process just because a lock file says so.
- Default to loopback-only visibility. Wildcard and public-interface listeners should be opt-in or clearly labeled.
- Keep config honest: unsupported config keys should either be implemented or rejected clearly.
- Add focused tests around parsers, filters, process-safety rules, config behavior, and version display.
- Before finishing code changes, run:

```bash
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

## Git And Release Hygiene

- Do not stage, commit, push, or tag unless the user explicitly asks.
- Keep `Cargo.lock` committed because this is a binary application.
- Use `make install` to build and copy the release binary to `~/.local/bin/hostel`.
- Use `hostel --version` or `make version` to verify which build is installed or being run.
