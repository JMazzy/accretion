# Project Guidelines

## Code Style

- **Language**: Rust
- **Formatting**: Use `rustfmt` for all code formatting. Format on save and before commits. Configuration in `rustfmt.toml` if needed.
- **Linting**: Use `clippy` with `cargo clippy -- -D warnings` to enforce all warnings as errors. Follow clippy suggestions for idiomatic Rust patterns.

## Architecture

The "particle" project is organized as a Rust project with the following structure:

- **Purpose**: A particle simulation engine featuring emergent structure formation through particle-to-particle gravity, collision physics, and locking mechanisms. Particles naturally aggregate and form complex structures through N-body gravitational interactions.
- **Core Components**:
  - `particle.rs` - Core particle struct with position, velocity, and resting state
  - `simulation.rs` - Main simulation engine with particle-to-particle gravity, collision detection, and culling
  - `graphics.rs` - Framebuffer renderer for visualizing particles
  - `main.rs` - Event loop and user interaction (paint/explode particles)
  - `rigid_body.rs` - Rigid body physics with convex hull geometry and rotational dynamics
- **Service Boundaries**: Clean separation between physics (simulation), rendering (graphics), and data structures (particle)
- **Data Flow**: 
  - User input → particle spawning/explosions
  - Simulation calculates gravitational forces and particle updates
  - Collisions lock particles together when both drop below velocity threshold
  - Particles are culled when far off-screen
  - Results are rendered each frame

## Physics Rules

### Particle Collisions
- **Detection**: Particles collide when distance < collision_distance (4.0)
- **Locking**: Particles lock together when:
  - Both are moving slower than velocity_threshold (5.0)
  - Distance ≤ sum of radii (contact)
  - They maintain relative offset constraints
- **Restitution**: 0.5 coefficient of restitution (bouncy, space-like)
- **Group System**: Locked particles join groups (group_id) to move/lock together
- **Breaking**: Groups break apart when impact force exceeds break_force_threshold (20.0)

### Rigid Body Formation
- **Formation**: When >= 3 particles in a group remain locked and at rest, convert to rigid body
- **Geometry**: 
  - Shape is the convex hull of the outermost particles
  - Bounding radius calculated from hull vertices (not inflated from particle offsets)
  - Hull vertices used for collision detection boundary
- **Mass**: Sum of all particle masses (each particle = mass 1.0)
- **Moment of Inertia**: Calculated from particle offsets: I = Σ(mass * distance²)
- **Particle Absorption**: Slow particles (< velocity_threshold) colliding with rigid body boundary get absorbed:
  - Center of mass recalculated
  - All particle offsets recomputed relative to new center
  - Convex hull and bounding radius regenerated
  - Colors blended
- **Rotation**: Generated naturally from contact point impulses in collisions

### Rigid Body Collisions
- **Contact Points**: Calculated on each body's surface at collision normal
- **Velocity at Contact**: v_contact = linear_velocity + (angular_velocity × r_contact)
- **Impulse Calculation**: Accounts for both linear and rotational inertia
- **Restitution**: 0.7 coefficient of restitution (bouncy, space-like)
- **Collision Damping**: 3% velocity damping post-collision (minimal, space physics)
- **Merging**: Two rigid bodies merge when:
  - Both linear speeds < velocity_threshold (5.0)
  - Both angular speeds < 1.0 rad/s
  - Distance < bounding_radius_sum * 1.1 (touching)
  - Merging converts back to locked particle groups

### Space Physics
- **No Global Damping**: Objects maintain momentum indefinitely (frictionless space)
- **Minimal Environmental Damping**: 
  - Only applies when particles very tightly packed (>6 neighbors within 3.0 units)
  - Base damping: 0.5% to prevent tunneling
- **Gravity**: 
  - Particle-to-particle: 15.0 (N-body attraction)
  - Minimum distance threshold: 100.0 (prevents singularities)
  - Applies uniformly to all bodies
- **Culling**: Both particles and rigid bodies culled when > 200 units off-screen

## Build and Test

Include the exact commands agents should use:

```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Format code
cargo fmt

# Lint code (all warnings as errors)
cargo clippy -- -D warnings

# Check without building artifacts
cargo check

# Run a binary/example
cargo run --bin <name>
```

## Project Conventions

Document patterns specific to this project that differ from common defaults:

- **File Structure**: Standard Rust layout with `src/` for library code, `src/main.rs` for binaries, `tests/` for integration tests, and `examples/` for example code.
- **Naming Conventions**: Use `snake_case` for functions, variables, and modules; `PascalCase` for types, structs, enums, and traits; `SCREAMING_SNAKE_CASE` for constants.
- **Configuration**: [How is the project configured? Environment variables? Config files?]
- **Error Handling**: Prefer `Result<T, E>` for fallible operations and `Option<T>` for optional values. Use custom error types with `thiserror` or `anyhow` crates when applicable.

## Integration Points

- **External APIs**: None - the simulation is self-contained
- **Dependencies**: 
  - `minifb` - Window creation and rendering (displays framebuffer to screen)
  - `glam` - Math library for 2D vectors and physics calculations
  - `rand` - Random number generation for particle spawning
- **Cross-Component Communication**: Graphics module receives particle state from simulation; main loop drives both

## Security

- **Authentication**: Not applicable - single-user simulation
- **Sensitive Data**: None - purely computational
- **Secret Management**: Not applicable
