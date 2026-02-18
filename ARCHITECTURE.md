# GRAV-SIM Architecture & Physics

## Overview

GRAV-SIM is an ECS-based **asteroid simulation engine** built on **Bevy 0.13** with physics powered by **Rapier2D**. The simulation features natural asteroid aggregation through N-body gravity attraction and collision-based merging into larger composite structures.

### Core Purpose
Pure asteroid-based simulation where asteroids naturally aggregate through gravitational attraction and collision into larger composite polygonal structures that visually rotate based on physics.

## System Architecture

### Frameworks & Dependencies
- **Bevy 0.13**: ECS architecture, rendering, event handling, windowing
- **Rapier2D 0.18**: Physics engine for collision detection, rigid body dynamics, and impulse-based response
- **Rand 0.8**: Random number generation for asteroid coloring

### Module Structure
```
src/
├── main.rs          - Bevy app setup, window configuration, test mode routing
├── asteroid.rs      - Unified asteroid components and spawn functions; convex hull computation
├── simulation.rs    - Physics systems: N-body gravity, cluster detection, composite formation
├── graphics.rs      - Camera setup for 2D rendering
├── testing.rs       - Automated test scenarios for physics validation
└── lib.rs           - Library exports
```

## Entity Types & Components

### Asteroid Entity
All asteroids in the simulation are unified entities with locally-stored vertices (relative to position).

**Components**:
- `Transform` (Bevy) - Position and rotation in world space
- `Vertices` - Vec of local-space vertices (relative to entity position)
- `RigidBody` (Rapier) - Physics entity with mass and inertia
- `Collider` (Rapier) - Collision shape (convex polygon or ball)
- `Velocity` (Rapier) - Linear and angular velocity
- `ExternalForce` (Rapier) - For gravity force application
- `Sprite` or `Gizmo` - Visual rendering

**Properties**:
- Spawn as triangles (3 vertices) or polygons depending on configuration
- Composite asteroids formed when 2+ asteroids touch and move slowly
- Local-space vertices enable correct rotation rendering and hull computation

## Physics Rules

### Gravity System (`nbody_gravity_system`)
- **Constant**: `gravity_const = 2.0` (gentle mutual attraction)
- **Minimum distance threshold**: `min_gravity_dist = 20.0` units
  - **Why**: Prevents energy injection during close encounters; Rapier handles collision physics below this range
  - **If ≥ 20 units apart**: Skip gravity entirely (prevents unphysical high-speed acceleration)
- **Maximum gravity distance**: `max_gravity_dist = 300.0` units (prevents phantom forces from distant asteroids)
- **Force**: Applied uniformly between all asteroid pairs as `F = gravity_const / distance²`
- **Behavior**: 
  - Asteroids at 100 units apart attract smoothly with gravity_const=10.0
  - Reach collision speeds based on initial spacing
  - Collide and merge into stable composites

### Collision Detection
- **Engine**: Rapier2D automatic contact manifold population
- **Range**: Activated for distances < 20 units (where gravity is disabled)
- **Response**: Restitution coefficients:
  - Triangle asteroids: `0.5` (50% bouncy)
  - Composite asteroids: `0.7` (70% bouncy)

### Cluster Formation & Merging
- **Detection**: Flood-fill algorithm through Rapier contact manifolds
- **Execution**: Must run in `PostUpdate` after Rapier `FixedUpdate` populates contacts
- **Velocity threshold**: `10.0 u/s` (allows faster asteroids to merge if in contact)
- **Hull computation**:
  1. Collect all vertices from cluster members in **world-space**
  2. Apply transform rotation to local vertices: `world_v = center + rotation * local_v`
  3. Compute convex hull from complete world-space vertex set
  4. Convert hull back to **local-space relative to center** for rendering
  5. Spawn composite with local-space hull for correct visualization
- **Velocity inheritance**: Average linear and angular velocity from cluster members

### Environmental Damping
- **Activation**: Applied to asteroids with >6 neighbors within 3.0 units
- **Damping factor**: 0.5% per frame (multiply velocity by 0.995)
- **Purpose**: Prevents numerical instability in extreme density clusters

### Culling & Boundary
- **Culling distance**: 1000 units from origin
- **Damping zone**: Asteroids beyond 600 units experience increasing damping
- **Damping ramp**: Smoothly increases from 0% to 5% over 400-unit range
- **Purpose**: Prevents asteroids from flying indefinitely; cleans up far objects

## ECS Systems Execution Order

### Update Schedule (Key Physics)
1. **`stats_counting_system`** - Counts live/culled asteroids
2. **`culling_system`** - Removes asteroids beyond 1000 units
3. **`neighbor_counting_system`** - Counts nearby asteroids (<3 units)
4. **`nbody_gravity_system`** - Applies mutual gravity forces (20-300 unit range)
5. **`settling_damping_system`** - Applies friction to slow asteroids
6. **`particle_locking_system`** - Synchronizes velocities of slow touching asteroids
7. **`environmental_damping_system`** - Stabilizes dense clusters (>6 neighbors)
8. **`user_input_system`** - Left-click spawns asteroids; arrow keys pan; wheel zooms
9. **`gizmo_rendering_system`** - Renders wireframe outlines

### FixedUpdate Schedule
- **Rapier physics**: Solves all collision, integrates velocities, populates contact manifolds

### PostUpdate Schedule (CRITICAL TIMING)
10. **`asteroid_formation_system`** - Must run AFTER Rapier physics populates contacts
11. **`test_logging_system`** & **`test_verification_system`** - Runs after merging to see final states

**Critical**: System scheduling ensures proper data consistency. Asteroid formation must run *after* physics updates contacts.

## Physics Constants Reference

All constants defined in `src/simulation.rs`:

```rust
gravity_const      = 10.0     // Mutual attraction strength
min_gravity_dist   = 5.0      // Skip gravity if closer (Rapier handles it)
max_gravity_dist   = 1000.0   // Gravity works across entire simulation
cull_distance      = 1000.0   // Remove entities beyond this
max_pan_distance   = 600.0    // Camera pan bounds
min_zoom           = 0.5      // Minimum camera zoom (full circle visible)
max_zoom           = 8.0      // Maximum camera zoom (detail view)
```

## Testing Framework

### Test System
- **Trigger**: `GRAV_SIM_TEST=<test_name>` environment variable
- **Runs**: Single test scenario for exact reproducibility
- **Framework**: Custom spawning functions in `src/testing.rs`

### Available Tests
- `two_triangles` - Verify 2 touching asteroids merge into 1
- `three_triangles` - Verify 3-asteroid cluster merges into 1-2
- `gentle_approach` - Verify smooth gravity-driven acceleration
- `high_speed_collision` - Verify high-velocity merge behavior
- `near_miss` - **Critical**: High-speed pass-by with gravity interaction (validates fix)
- `gravity` - Long-distance attraction and merge
- `culling_verification` - Off-screen removal and gravity isolation
- `large_small_pair` - Mixed-size asteroid gravity interaction
- `gravity_boundary` - Behavior at maximum gravity distance
- `mixed_size_asteroids` - Complex 5-body N-body system

### Test Logging
- Logs positions and velocities at key frames (1, 10, 30, 50, 100+)
- Compares initial vs final asteroid counts
- Validates: merging occurred (count decreased), physics stable (velocity reasonable)

## Code Quality Standards

- **Language**: Rust
- **Formatting**: `cargo fmt` (rustfmt)
- **Linting**: `cargo clippy -- -D warnings` (all warnings as errors)
- **File Structure**: Standard Rust layout with modules in `src/`

## Development Commands

```bash
# Build and check
cargo check
cargo build
cargo build --release

# Format and lint
cargo fmt
cargo clippy -- -D warnings

# Run simulation
cargo run --release

# Run specific test
GRAV_SIM_TEST=near_miss cargo run --release

# Run all tests
./test_all.sh
```

## Physics Validation Results

### All 10 Tests Passing ✅
- ✅ Basic merging (touching asteroids)
- ✅ Cluster detection (multiple asteroids)
- ✅ Gravity attraction (distance-based acceleration)
- ✅ High-speed collisions (impact merging)
- ✅ **Near-miss stability** (validates gravity fix: 20→38 u/s, not 20→426 u/s)
- ✅ Long-range gravity dynamics
- ✅ Culling system (no phantom forces)
- ✅ Mixed-size interactions
- ✅ Gravity distance boundaries
- ✅ Complex N-body systems

### Critical Fix Validated
**Gravity Threshold Fix**: Changed minimum gravity distance from clamping at 2 units to skipping entirely when <20 units apart. This prevents energy injection during close encounters and high-speed passes, ensuring stable physics across all scenarios.

