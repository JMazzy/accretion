# Project Guidelines

## Code Style

- **Language**: Rust
- **Formatting**: Use `rustfmt` for all code formatting. Format on save and before commits. Configuration in `rustfmt.toml` if needed.
- **Linting**: Use `clippy` with `cargo clippy -- -D warnings` to enforce all warnings as errors. Follow clippy suggestions for idiomatic Rust patterns.

## Architecture

The "grav-sim" project is an ECS-based **asteroid simulation engine** built on **Bevy** with physics powered by **Rapier2D**. All objects in the simulation are asteroids that naturally aggregate through N-body gravity into larger composite polygonal structures.

- **Purpose**: Pure asteroid-based simulation where small asteroids (triangles) naturally aggregate through gravitational attraction and collision to form larger composite asteroids (polygons) that visually rotate based on physics.
- **Framework**: 
  - **Bevy 0.13**: Game engine providing ECS architecture, rendering, and event handling
  - **Rapier2D 0.18**: Physics engine for collision detection, rigid body dynamics, and impulse-based response
- **Core Modules**:
  - `asteroid.rs` - Unified asteroid components and spawn functions for small (triangles) and large (polygons) asteroids
  - `simulation.rs` - Physics systems: N-body gravity (with distance limits), velocity syncing, cluster detection, composite formation, culling with damping, and user input
  - `graphics.rs` - Camera setup for 2D rendering
  - `main.rs` - Bevy app setup, window configuration, and plugin initialization
- **Entity Types**: 
  - **Small Asteroids**: Equilateral triangles, 2.0 unit ball colliders, spawn via left-click
  - **Large Asteroids**: Convex polygons (wireframe with white outlines), polygon colliders, formed when 2+ small asteroids cluster together
- **ECS Systems** (Execution Order):
  1. **Culling** - Removes asteroids beyond 1000 units; applies damping beyond viewport
  2. **Neighbor counting** - Counts nearby asteroids for environmental damping eligibility
  3. **N-body gravity** - Applies mutual attraction only to nearby asteroids (< 800 units)
  4. **Velocity syncing** - Synchronizes velocities of touching slow-moving asteroids
  5. **Environmental damping** - Applies slight friction to tightly packed clusters (>6 neighbors within 3 units)
  6. **Cluster formation** - Detects clusters of 2+ touching slow (< 1.0 u/s) small asteroids and merges into composites
  7. **User input** - Left-click spawns triangle asteroids at cursor position
  8. **Gizmo rendering** - Renders asteroid wireframes with rotation applied

## Physics Rules

### Small Asteroid Properties (Triangles)
- **Shape**: Equilateral triangle (relative vertices stored, rotated by transform)
- **Collider**: 2.0 unit ball
- **Mass**: 1.0 unit
- **Restitution**: 0.5 (50% bouncy, space-like)
- **Color**: Random grey shade (0.3–0.9) per asteroid
- **Damping**: No linear or angular damping by default

### Large Asteroid Properties (Polygons)
- **Formation**: Created when 2+ small asteroids touch and move < 1.0 u/s
- **Shape**: Convex hull of constituent asteroids (computed via gift wrapping algorithm)
- **Collider**: Exact convex polygon from physics engine
- **Mass**: Sum of constituent masses
- **Restitution**: 0.7 (70% bouncy)
- **Color**: Random grey shade (0.3–0.9)
- **Rendering**: White wireframe outline, vertices rotated by transform's rotation
- **Velocity**: Inherits averaged linear and angular velocity from constituents

### N-Body Gravity
- **Constant**: 15.0 (strong mutual attraction)
- **Minimum distance threshold**: 150.0 units (reduces instability when touching)
- **Maximum gravity distance**: 800.0 units (prevents phantom forces from distant asteroids)
- **Application**: Applied uniformly between all asteroid pairs within range

### Velocity Synchronization
- **Activation**: When two asteroids touch and both move < 5.0 u/s
- **Effect**: Velocities averaged between them (linear and angular)
- **Purpose**: Prepares asteroids for smooth composite formation

### Cluster Formation & Merging
- **Detection**: Every frame, find all small asteroids touching each other that move < 1.0 u/s
- **Threshold**: 2+ asteroids in a cluster triggers merging
- **Process**:
  1. Compute center of mass and average velocities
  2. Calculate convex hull from constituent positions
  3. Spawn large asteroid inheriting averaged velocity
  4. **Immediately despawn** all constituent small asteroids
- **Prevention**: Processed asteroids tracked each frame to prevent duplicate merging

### Environmental Damping
- **Activation**: Applied to asteroids with >6 neighbors within 3.0 units
- **Damping factor**: 0.5% per frame (factor: 0.995)
- **Purpose**: Prevents numerical instability in extreme density clusters

### Culling & Damping
- **Damping zone**: Asteroids beyond 600 units from origin experience increasing damping
- **Culling distance**: 1000 units (asteroids removed when exceeding this)
- **Damping ramp**: Smoothly increases from 0% to 5% over 400-unit range
- **Purpose**: Prevents asteroids from flying indefinitely; cleans up far objects

## User Interaction

- **Left-click**: Spawns a small triangle asteroid at cursor position
- **No automatic spawning**: Simulation starts empty; user drives all spawning
- **Coordinate system**: Screen (0,0) top-left → World (0,0) center; X right, Y up

## Current Implementation
- ✅ Pure asteroid-only system (no particle/rigid_body distinction)
- ✅ Direct cluster-based formation (no GroupId state tracking)
- ✅ Wireframe rendering with rotation
- ✅ Velocity inheritance on composite formation
- ✅ Long-range gravity distance limits (prevents phantom forces)
- ✅ Culling with distance-based damping
- ✅ Click-only user control for clean testing

## Development Commands

```bash
# Build the project
cargo build

# Build in release mode (optimized)
cargo build --release

# Run the simulation
cargo run --release

# Format code
cargo fmt

# Lint code (all warnings as errors)
cargo clippy -- -D warnings

# Check without building artifacts
cargo check
```

## Project Conventions

- **File Structure**: Standard Rust layout:
  - `src/` - library and binary code
  - `src/main.rs` - Bevy app main entry
  - `src/asteroid.rs` - Core asteroid definitions and spawn functions
  - `src/simulation.rs` - All ECS systems
  - `src/graphics.rs` - Camera/rendering setup
- **Naming Conventions**: 
  - `snake_case` for functions, variables, modules
  - `PascalCase` for types, structs, enums, traits
  - `SCREAMING_SNAKE_CASE` for constants
- **Physics Tuning**: Constants defined at top of physics system functions:
  - Gravity constant and distance thresholds in `nbody_gravity_system`
  - Velocity thresholds in `particle_locking_system` and `asteroid_formation_system`
  - Damping factors in `environmental_damping_system` and `culling_system`

## Integration Points

- **External APIs**: None - fully self-contained simulation
- **Dependencies**: 
  - `bevy` (0.13) - ECS engine, rendering, windowing
  - `bevy_rapier2d` (0.25) - Physics engine integration for Bevy
  - `rapier2d` (0.18) - Core physics via SIMD-optimized convex hulls and collision detection
  - `rand` (0.8) - Random grey shades for asteroid coloring
  - `glam` - Math library (Vec2, Quat) via Bevy
- **Cross-Component Communication**: 
  - Components: `Asteroid`, `AsteroidSize`, `Vertices`, plus Rapier/Bevy physics components
  - Systems read/write components; Rapier applies physics automatically
  - Gizmos system reads transforms and vertices for rendering
  - User input system spawns via `Commands`
