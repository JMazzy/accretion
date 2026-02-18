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

```text
src/
├── main.rs               - Bevy app setup, window configuration, test mode routing
├── asteroid.rs           - Unified asteroid components and spawn functions; convex hull computation
├── simulation.rs         - Physics systems: N-body gravity, cluster detection, composite formation
├── spatial_partition.rs  - Spatial grid for O(1) neighbor lookup (replaces O(N²) brute-force)
├── graphics.rs           - Camera setup for 2D rendering
├── testing.rs            - Automated test scenarios for physics validation
└── lib.rs                - Library exports
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

- **Constant**: `gravity_const = 10.0` (mutual attraction)
- **Minimum distance threshold**: `min_gravity_dist = 5.0` units
  - **Why**: Prevents energy injection during close encounters; Rapier handles collision physics below this range
- **Maximum gravity distance**: `max_gravity_dist = 1000.0` units (matches cull distance)
- **Force**: Applied between pairs as `F = gravity_const / distance²`
- **Optimization**: Uses `SpatialGrid` for O(N·K) grid-based candidate lookup instead of O(N²) brute-force iteration
- **Behavior**:
  - Asteroids at 100 units apart attract smoothly
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

- **Removed**: Artificial environmental and settling damping has been removed.
- **Philosophy**: Energy loss occurs only through natural physics: collision restitution (Rapier) and gravity dynamics.
- Asteroids may spin and bounce indefinitely if no collision occurs — this is correct physical behavior.

### Culling & Boundary

- **Culling distance**: 1000 units from origin
- **Purpose**: Prevents asteroids from flying indefinitely; cleans up far objects
- Note: artificial velocity damping ramps outside 600 units have been removed.

## ECS Systems Execution Order

### Update Schedule

1. **`stats_counting_system`** - Counts live/culled asteroids
2. **`rebuild_spatial_grid_system`** - Rebuilds spatial grid from current positions (O(N))
3. **`culling_system`** - Removes asteroids beyond 1000 units
4. **`neighbor_counting_system`** - Counts nearby asteroids using grid (O(N·K))
5. **`particle_locking_system`** - Synchronizes velocities of slow touching asteroids via Rapier contact_pairs iterator (O(C), C = active contacts)
6. **`user_input_system`** - Left-click spawns asteroids; arrow keys pan; wheel zooms
7. **`gizmo_rendering_system`** - Renders wireframe outlines; skips force vectors at >200 asteroids

### FixedUpdate Schedule (chained in order)

1. **`rebuild_spatial_grid_system`** - Rebuilds grid with physics-step positions
2. **`nbody_gravity_system`** - Applies mutual gravity using spatial grid (O(N·K))
3. **Rapier physics** - Solves all collision, integrates velocities, populates contact manifolds

### PostUpdate Schedule (CRITICAL TIMING)

1. **`asteroid_formation_system`** - Must run AFTER Rapier physics populates contacts
2. **`test_logging_system`** & **`test_verification_system`** - Runs after merging to see final states

**Critical**: System scheduling ensures proper data consistency. Asteroid formation must run *after* physics updates contacts.

## Spatial Grid (`spatial_partition.rs`)

The `SpatialGrid` resource partitions world space into 100-unit cells for O(1) neighbor queries.

- **Cell size**: 100 units (large enough to span typical gravity interaction distances)
- **Lookup**: `get_neighbors_excluding(entity, pos, max_distance)` returns candidates from nearby cells
- **Rebuild**: Called at the start of each Update and FixedUpdate frame
- **Complexity**: O(N) rebuild, O(K) lookup where K = avg entities per cell neighborhood
- **Impact**: Reduces N-body gravity and neighbor counting from O(N²) to O(N·K)

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
