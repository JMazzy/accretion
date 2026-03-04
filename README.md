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

### Test Selection Guide

Use the smallest relevant test set based on what changed.

Core physics scenario regression (fast, script-based):

```bash
./test_all.sh
```

Core scenario integration tests (binary-driven, ignored by default):

```bash
cargo test --test physics_scenarios_integration -- --ignored --nocapture --test-threads=1
```

Extended integration scenarios (orbit/enemy scripted/perf sample; ignored by default):

```bash
cargo test --test physics_extended_integration -- --ignored --nocapture --test-threads=1
```

Run one specific scenario test:

```bash
cargo test --test physics_scenarios_integration scenario_two_triangles -- --ignored --nocapture --test-threads=1
```

Logs from script/integration scenario runs are written under `artifacts/test_logs/`.

#### What to run for common changes

- `src/asteroid.rs`, `src/simulation.rs`, `src/spatial_partition.rs`, `src/constants.rs`, `src/config.rs`
	- Run: `./test_all.sh`
	- Add: `physics_extended_integration` when touching orbit/perf-sensitive logic.
- `src/enemy.rs`, `src/player/combat.rs`, `src/player/control.rs`, `src/player/ion_cannon.rs`
	- Run: `./test_all.sh`
	- Add: `cargo test --test physics_extended_integration scenario_enemy_combat_scripted -- --ignored --nocapture --test-threads=1`
- `src/menu.rs`, `src/menu/`, `src/save.rs`, state wiring in `src/main.rs`
	- Run: `cargo test --test menu_tests -- --nocapture`
	- Add scenario integration tests if startup/test-mode wiring changed.
- Performance tuning only
	- Run targeted extended tests first (e.g. `scenario_baseline_100`, `scenario_mixed_content_225_enemy8`), then optionally full `physics_extended_integration`.

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
- `src/menu.rs` + `src/menu/` - Game state orchestration and menu UI modules
- `src/test_mode.rs` - Test-mode wiring from `main`
- `src/testing.rs` + `src/testing/` - Test façade + scenario/verification modules
- `tests/` - Integration tests
- `examples/` - Example programs

For detailed architecture and controls, see [ARCHITECTURE.md](ARCHITECTURE.md) and [FEATURES.md](FEATURES.md).
