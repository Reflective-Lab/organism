---
tags: [stack]
---
# Rust

Organism is Rust-first. Edition 2024, rust-version 1.94.

## Conventions

- `unsafe_code = "forbid"` — no exceptions
- Clippy pedantic
- `just lint` clean before considering work done
- No feature flags or backwards-compat shims unless asked

## Build

```bash
just build      # Build all crates
just test       # Run tests
just lint       # Format + clippy pedantic
```

See also: [[Building/Getting Started]]
