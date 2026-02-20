# GRAV-SIM Architecture & Physics

## Overview

GRAV-SIM is an ECS-based **asteroid simulation engine** built on **Bevy 0.17** with physics powered by **Rapier2D**. The simulation features natural asteroid aggregation through N-body gravity attraction and collision-based merging into larger composite structures.

### Core Purpose

Pure asteroid-based simulation where asteroids naturally aggregate through gravitational attraction and collision into larger composite polygonal structures that visually rotate based on physics.

## System Architecture

### Frameworks & Dependencies

- **Bevy 0.17**: ECS architecture, rendering, event handling, windowing
- **bevy_rapier2d 0.32** (Rapier 0.22): Physics engine for collision detection, rigid body dynamics, and impulse-based response
- **Rand 0.8**: Random number generation for asteroid coloring

### Module Structure

```text
src/
├── main.rs               - Bevy app setup, window configuration, test mode routing
├── asteroid.rs           - Unified asteroid components and spawn functions; convex hull computation
├── simulation.rs         - Physics systems: N-body gravity, cluster detection, composite formation
├── spatial_partition.rs  - Spatial grid for O(1) neighbor lookup (replaces O(N²) brute-force)
├── player.rs             - Player ship entity, WASD controls, projectile firing, camera follow
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
6. **`player_control_system`** - Applies WASD thrust/rotation to player ship
7. **`projectile_fire_system`** - Fires projectiles on spacebar (with cooldown)
8. **`despawn_old_projectiles_system`** - Expires projectiles after lifetime/distance limit
9. **`user_input_system`** - Left-click spawns asteroids; mouse wheel zooms (no more arrow-key pan)
10. **`camera_follow_system`** - Centres camera on player ship each frame
11. **`camera_zoom_system`** - Applies zoom scale to camera transform
12. **`gizmo_rendering_system`** - Renders asteroid wireframe outlines
13. **`player_gizmo_system`** - Renders ship polygon and projectile circles

### FixedUpdate Schedule (chained in order)

1. **`rebuild_spatial_grid_system`** - Rebuilds grid with physics-step positions
2. **`nbody_gravity_system`** - Applies mutual gravity using spatial grid (O(N·K))
3. **Rapier physics** - Solves all collision, integrates velocities, populates contact manifolds

### PostUpdate Schedule (CRITICAL TIMING)

1. **`asteroid_formation_system`** - Must run AFTER Rapier physics populates contacts
2. **`test_logging_system`** & **`test_verification_system`** - Runs after merging to see final states

**Critical**: System scheduling ensures proper data consistency. Asteroid formation must run *after* physics updates contacts.

## Spatial Grid (`spatial_partition.rs`)

The `SpatialGrid` resource partitions world space into 500-unit cells for efficient neighbor queries.

- **Cell size**: 500 units — deliberately large to avoid excessive cell-check overhead
  - A query for `max_gravity_dist=1000` checks only a 5×5=25 cell area
  - Using 100-unit cells with the same query would check 21×21=441 cells, worse than O(N²) at low asteroid counts
- **Lookup**: `get_neighbors_excluding(entity, pos, max_distance)` returns candidates from nearby cells
- **Rebuild**: Called at the start of each Update and FixedUpdate frame
- **Complexity**: O(N) rebuild, O(K) lookup where K = avg entities per cell neighborhood
- **Impact**: Reduces N-body gravity and neighbor counting from O(N²) to O(N·K)

## Physics Constants Reference

All constants defined in `src/simulation.rs`:

```rust
// Simulation physics (src/simulation.rs)
gravity_const           = 10.0    // Mutual attraction strength
min_gravity_dist        = 5.0     // Skip gravity if closer (Rapier handles it)
max_gravity_dist        = 1000.0  // Gravity works across entire simulation
cull_distance           = 1000.0  // Remove entities beyond this
min_zoom                = 0.5     // Minimum camera zoom (full circle visible)
max_zoom                = 8.0     // Maximum camera zoom (detail view)

// Spatial grid (src/spatial_partition.rs)
grid_cell_size          = 500.0   // Must be >= max_query_distance / 2

// Player (src/player.rs)
thrust_force            = 120.0   // Forward thrust (N) while W held
reverse_force           = 60.0    // Reverse thrust (N) while S held
rotation_speed          = 3.0     // Angular velocity (rad/s) while A or D held
projectile_speed        = 500.0   // Projectile speed (units/s)
fire_cooldown           = 0.2     // Seconds between shots
projectile_lifetime     = 3.0     // Seconds before projectile despawns
player_max_hp           = 100.0   // Player ship full health
damage_speed_threshold  = 30.0    // Minimum relative speed (u/s) that deals damage
invincibility_duration  = 0.5     // Seconds of damage immunity after a hit
oob_radius              = 1000.0  // Soft boundary beyond which player is damped
oob_damping             = 0.97    // Velocity decay factor applied per frame outside OOB_RADIUS
```

## Testing Framework

### Test System

- **Trigger**: `GRAV_SIM_TEST=<test_name>` environment variable
- **Runs**: Single test scenario for exact reproducibility
- **Framework**: Custom spawning functions in `src/testing.rs`
- **Player isolation**: In test mode the player entity is **not spawned** — player systems run but find no `Player` component and are no-ops. This ensures asteroid-only tests are not affected by the player ship's collider (8-unit ball at origin) or its input/damage systems.

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

## Known Limitations & Future Considerations

### Current Technical Constraints

#### Simulation Boundaries
- **2D only**: All physics operates on the XY plane; no Z-axis forces or rendering depth
- **Hard world boundary**: 1000-unit cull radius is fixed in source; asteroids beyond this are permanently removed
- **Spawn area**: Initial asteroids distributed in a 3000×2000 unit region with a 400-unit player buffer at origin; values require recompilation to change
- **Max simulation density**: Gizmo-based wireframe rendering starts showing overhead beyond ~200 simultaneous live asteroids (force-vector annotations auto-disabled at this threshold)

#### Physics Simplifications
- **Convex-only colliders**: All asteroid shapes are convex polygons; concavities from impacts are approximated by their convex hull, not modelled directly
- **Gravity cutoff**: Gravity is disabled inside 5 units (Rapier handles close contacts) and beyond 1000 units; there is no smooth transition
- **No rotational gravity torque**: Gravity applies only linear force (no torque based on off-centre mass distribution)
- **Cluster formation is discrete**: Merging is all-or-nothing per frame; a cluster either fully merges in one PostUpdate step or waits until the next frame
- **Single-pass hull computation**: Composite hull is computed once at merge time; subsequent impacts reduce vertex count but do not recompute the full hull from physics state

#### Hardcoded Configuration
All physics-tuning constants are defined directly in source files and require `cargo build` to change:
- Gravity, distance thresholds, velocity thresholds — `src/simulation.rs`
- Player movement, projectile, health constants — `src/player.rs`
- Grid cell size — `src/spatial_partition.rs`
- Asteroid spawn counts and bounds — `src/asteroid.rs`

#### Version Constraints
- **Bevy 0.17** + **bevy_rapier2d 0.32**: Current versions. Migration from 0.13 completed February 2026.
- **Contact manifold API**: `ReadRapierContext` system param; contacts queried via `ctx.single()?.simulation.contact_pairs(colliders, rigidbody_set)` in formation and particle-locking systems.

### Future Enhancement Roadmap

#### Physics Improvements
- **Concave asteroid deformation**: Track per-vertex damage state; move impact vertex inward and recompute hull to simulate craters and progressive destruction
- **Gravitational binding energy merge criterion**: Replace velocity-threshold merging with a binding-energy check; clusters only merge if their kinetic energy is below the gravitational potential energy of the cluster, producing more physically realistic aggregation
- **Rotational-inertia-aware gravity torque**: Include mass distribution (second moment of area) in gravity force application so oddly-shaped composites develop realistic rotation
- **Soft boundary with elastic reflection**: Replace hard cull-at-1000u removal with a potential-well boundary that gently reflects asteroids back toward the simulation centre
- **KD-tree neighbor search**: Replace the static 500-unit spatial grid with a dynamic KD-tree to better handle highly non-uniform asteroid distributions (dense cluster + sparse outer field)
- **Orbital presets**: Optional initial conditions (Keplerian orbits, accretion disk configuration) as alternative to random spawning

#### Visual & Rendering Enhancements
- **Particle effects system**: Impact dust clouds on projectile hits; merge vortex animations; debris trail on asteroid destruction
- **Level-of-Detail (LOD) rendering**: Large composites (>8 vertices) rendered as filled GPU mesh instead of CPU-drawn gizmo wireframe, removing the per-vertex CPU bottleneck at high count
- **Velocity heat-map coloring**: Tint asteroid wireframes from blue (slow) to red (fast) to give instant visual feedback on kinetic energy distribution
- **Crater / fracture overlays**: Draw cracks on asteroid surfaces proportional to accumulated damage (impacts that didn't yet destroy the asteroid)
- **Dynamic camera FOV**: Camera zoom automatically increases when the player moves fast, giving a wider field of view at speed
- **Post-processing effects**: Bloom on high-energy impacts and merges; chromatic aberration during player damage invincibility

#### Gameplay & Extensibility
- **Configuration file support**: Load physics constants from an `assets/physics.toml` file at startup, enabling tuning without recompilation
- **Score and progression system**: Points for asteroid destruction scaled by size; wave-based difficulty ramp spawning more and larger asteroids over time
- **Power-up asteroids**: Special-coloured asteroids that grant the player temporary buffs (shield, rapid-fire, gravity bomb) on destruction
- **Boss asteroids**: Single very-large composite (size ≥ 20) with scripted split behaviour acting as a wave-ending target
- **Multiplayer (local co-op)**: Spawn a second player ship feeding off the same physics world; share the asteroid field and scoring

#### Test & Developer Tooling
- **Automated regression baseline**: Store golden frame-log snapshots in `tests/golden/` and compare on each test run, automatically catching physics constant drift
- **In-game physics inspector**: Toggle an overlay showing entity IDs, velocities, and contact counts on-screen for live debugging without restarting in test mode
- **Hot-reload constants**: Watch `assets/physics.toml` for changes at runtime and apply updated constants on the fly (requires the configuration file feature above)

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
