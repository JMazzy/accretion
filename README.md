# Particle

A simulation video game based on particle-based destructible environments, inspired by games like "The Powder Toy" and "Noita".

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
cargo run --bin particle
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
- `src/particle.rs` - Core particle types and simulation
- `tests/` - Integration tests
- `examples/` - Example programs
