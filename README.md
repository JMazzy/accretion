# Accretion

An **ECS-based asteroid aggregation simulation game** built on [Bevy](https://bevyengine.org/) with physics powered by [Rapier](https://rapier.rs/). Asteroids naturally aggregate through N-body gravity attraction and collision-based merging into larger composite polygonal structures.

**Tech Stack**: [Rust](https://www.rust-lang.org/) · [Bevy 0.17](https://github.com/bevyengine/bevy) · [Rapier2D 0.32](https://github.com/dimforge/rapier)

## Building

Build the project:

```bash
cargo build
```

Build in release mode:

```bash
cargo build --release
```

## Running

Run the executable:

```bash
cargo run --bin accretion
```

## Testing

Run all tests:

```bash
cargo test
```

Run tests with output:

```bash
cargo test -- --nocapture
```

## Code Style

Format code:

```bash
cargo fmt
```

Lint code (all warnings as errors):

```bash
cargo clippy -- -D warnings
```

Check without building:

```bash
cargo check
```

## Project Structure

- `src/main.rs` - Binary entry point
- `src/lib.rs` - Library root
- `src/asteroid.rs` - Core asteroid types and simulation
- `tests/` - Integration tests
- `examples/` - Example programs
