SysDash (MVP)

A minimal terminal dashboard that shows CPU usage, memory used/total, uptime, and a small process list. Updates at a fixed interval with no visible flicker and exits cleanly with q or Ctrl-C.

Build and run
- Requirements: Rust 1.70+ (stable)
- cargo run

Controls
- q, Esc, Ctrl-C: Quit

Roadmap (next)
- Configurable update interval
- Sorting and filtering processes
- More system panels (disk, network)
- Help overlay

Notes
- This uses sysinfo 0.30 where memory values are returned in bytes. If you change sysinfo version, verify units and adjust formatters accordingly.
- The event loop polls at 1s; you can change tick_rate in main.rs or expose it via CLI later.
