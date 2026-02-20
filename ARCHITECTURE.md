# GRAV-SIM Architecture & Physics

## Overview

GRAV-SIM is an ECS-based **asteroid simulation engine** built on **Bevy 0.17** with physics powered by **Rapier2D**. The simulation features natural asteroid aggregation through N-body gravity attraction and collision-based merging into larger composite structures.

### Core Purpose

Pure asteroid-based simulation where asteroids naturally aggregate through gravitational attraction and collision into larger composite polygonal structures that visually rotate based on physics.

## System Architecture

#### Frameworks & Dependencies

- **[Bevy 0.17](https://bevyengine.org/)** ([GitHub](https://github.com/bevyengine/bevy)): ECS architecture, rendering, event handling, windowing
- **[bevy_rapier2d 0.32](https://rapier.rs/)** ([GitHub](https://github.com/dimforge/rapier)): Physics engine for collision detection, rigid body dynamics, and impulse-based response
- **[Rand 0.8](https://docs.rs/rand/latest/rand/)**: Random number generation for asteroid coloring

### Module Structure

```text
src/
├── main.rs               - Bevy app setup, window configuration, test mode routing
├── constants.rs          - All tuneable physics and gameplay constants (compile-time defaults)
├── config.rs             - PhysicsConfig Bevy resource; loaded from assets/physics.toml at startup
├── asteroid.rs           - Unified asteroid components and spawn functions; convex hull computation
├── simulation.rs         - Physics systems: N-body gravity, cluster detection, composite formation
├── spatial_partition.rs  - Spatial grid for O(1) neighbor lookup (replaces O(N²) brute-force)
├── rendering.rs          - Gizmo wireframe rendering and stats overlay systems
├── player/               - Player ship entity, WASD controls, projectile firing, camera follow
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

- **Constant**: `GRAVITY_CONST` (`src/constants.rs`) — mutual attraction strength
- **Minimum distance threshold**: `MIN_GRAVITY_DIST` — asteroids closer than this are excluded; Rapier handles contact physics below this range to prevent energy injection during close encounters
- **Maximum gravity distance**: `MAX_GRAVITY_DIST` — matches `CULL_DISTANCE` so culled asteroids exert no phantom forces
- **Force**: Applied between pairs as `F = GRAVITY_CONST / distance²`
- **Optimization**: Uses `SpatialGrid` for O(N·K) grid-based candidate lookup instead of O(N²) brute-force iteration

### Collision Detection

- **Engine**: Rapier2D automatic contact manifold population
- **Range**: Activated below `MIN_GRAVITY_DIST` (where gravity is skipped)
- **Response**: Restitution coefficients defined in `src/constants.rs`:
  - `RESTITUTION_SMALL` — small/unit asteroids
  - Composite asteroids use Rapier's default (no override currently applied)

### Cluster Formation & Merging

- **Detection**: Flood-fill algorithm through Rapier contact manifolds (no velocity pre-filter)
- **Execution**: Must run in `PostUpdate` after Rapier `FixedUpdate` populates contacts
- **Merge criterion: gravitational binding energy**
  - A cluster merges only if its kinetic energy in the centre-of-mass frame falls below the sum of pairwise gravitational binding energies:
    - `E_binding = Σ_{i<j} G · mᵢ · mⱼ / rᵢⱼ`
    - `E_k_com = Σᵢ ½mᵢ|vᵢ − v_cm|² + Σᵢ ½Iᵢωᵢ²`
    - Merge condition: `E_k_com < E_binding`
  - Mass proxy: `AsteroidSize` units (uniform density → mass ∝ size)
  - Moment of inertia estimate per member: `I = ½ · m · r²` where `r = √(m / π)`
- **Velocity synchronisation** (pre-formation, `particle_locking_system`): `VELOCITY_THRESHOLD_LOCKING` — stabilises co-moving touching asteroids before the formation system runs
- **Hull computation**:
  1. Collect all vertices from cluster members in **world-space**
  2. Apply transform rotation to local vertices: `world_v = center + rotation * local_v`
  3. Compute convex hull from complete world-space vertex set
  4. Convert hull back to **local-space relative to center** for rendering
  5. Spawn composite with local-space hull for correct visualization
- **Velocity inheritance**: Centre-of-mass velocity (mass-weighted average of linear velocities); simple average for angular velocity

### Environmental Damping

- **Removed**: Artificial environmental and settling damping has been removed.
- **Philosophy**: Energy loss occurs only through natural physics: collision restitution (Rapier) and gravity dynamics.
- Asteroids may spin and bounce indefinitely if no collision occurs — this is correct physical behavior.

### Culling & Boundary

- **Culling distance**: `CULL_DISTANCE` (`src/constants.rs`) from origin
- **Purpose**: Prevents asteroids from flying indefinitely; cleans up far objects
- Artificial velocity damping ramps have been removed; the boundary is a hard cull.

## ECS Systems Execution Order

### Update Schedule

1. **`stats_counting_system`** - Counts live/culled asteroids
2. **`rebuild_spatial_grid_system`** - Rebuilds spatial grid from current positions (O(N))
3. **`culling_system`** - Removes asteroids beyond `CULL_DISTANCE`
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

The `SpatialGrid` resource partitions world space into fixed-size cells for efficient neighbor queries.

- **Cell size**: `GRID_CELL_SIZE` (`src/constants.rs`) — must be ≥ the largest query radius / 2 to avoid excessive cell-check overhead
- **Lookup**: `get_neighbors_excluding(entity, pos, max_distance)` returns candidates from nearby cells
- **Rebuild**: Called at the start of each Update and FixedUpdate frame
- **Complexity**: O(N) rebuild, O(K) lookup where K = avg entities per cell neighborhood
- **Impact**: Reduces N-body gravity and neighbor counting from O(N²) to O(N·K)

## Physics Constants Reference

All tuneable constants are centralised in **`src/constants.rs`** with doc-comments explaining each value's purpose, tested range, and observable effect. The file is the single source of truth — never duplicate values into documentation.

Key constant groups (see `src/constants.rs` for current values):

| Group | Constants |
|---|---|
| World bounds | `SIM_WIDTH`, `SIM_HEIGHT`, `PLAYER_BUFFER_RADIUS` |
| Gravity | `GRAVITY_CONST`, `MIN_GRAVITY_DIST`, `MAX_GRAVITY_DIST` |
| Cluster formation | `VELOCITY_THRESHOLD_LOCKING` (velocity sync), `GRAVITY_CONST` (binding energy) |
| Collision | `RESTITUTION_SMALL`, `FRICTION_ASTEROID` |
| Culling | `CULL_DISTANCE` |
| Spatial grid | `GRID_CELL_SIZE` |
| Camera | `MIN_ZOOM`, `MAX_ZOOM`, `ZOOM_SPEED` |
| Player movement | `THRUST_FORCE`, `REVERSE_FORCE`, `ROTATION_SPEED` |
| Player OOB | `OOB_RADIUS`, `OOB_DAMPING`, `OOB_RAMP_WIDTH` |
| Player combat | `PROJECTILE_SPEED`, `FIRE_COOLDOWN`, `PROJECTILE_LIFETIME` |
| Player health | `PLAYER_MAX_HP`, `DAMAGE_SPEED_THRESHOLD`, `INVINCIBILITY_DURATION` |
| Gamepad | `GAMEPAD_BRAKE_DAMPING`, `GAMEPAD_LEFT_DEADZONE`, etc. |

## Testing Framework

### Test System

- **Trigger**: `GRAV_SIM_TEST=<test_name>` environment variable
- **Runs**: Single test scenario for exact reproducibility
- **Framework**: Custom spawning functions in `src/testing.rs`
- **Player isolation**: In test mode the player entity is **not spawned** — player systems run but find no `Player` component and are no-ops. This ensures asteroid-only tests are not affected by the player ship's collider (radius = `PLAYER_COLLIDER_RADIUS`) or its input/damage systems.

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
- **Hard world boundary**: `CULL_DISTANCE` radius is fixed in source; asteroids beyond this are permanently removed
- **Spawn area**: Initial asteroids distributed within `SIM_WIDTH`×`SIM_HEIGHT` with a `PLAYER_BUFFER_RADIUS` exclusion zone at origin; values require recompilation to change
- **Max simulation density**: Gizmo-based wireframe rendering starts showing overhead beyond ~200 simultaneous live asteroids (force-vector annotations auto-disabled at this threshold)

#### Physics Simplifications
- **Convex-only colliders**: All asteroid shapes are convex polygons; concavities from impacts are approximated by their convex hull, not modelled directly
- **Gravity cutoff**: Gravity is disabled inside `MIN_GRAVITY_DIST` (Rapier handles close contacts) and beyond `MAX_GRAVITY_DIST`; there is no smooth transition
- **No rotational gravity torque**: Gravity applies only linear force (no torque based on off-centre mass distribution)
- **Cluster formation is discrete**: Merging is all-or-nothing per frame; a cluster either fully merges in one PostUpdate step or waits until the next frame
- **Single-pass hull computation**: Composite hull is computed once at merge time; subsequent impacts reduce vertex count but do not recompute the full hull from physics state

#### Runtime Configuration

Physics constants are defined in `src/constants.rs` as compile-time defaults and mirrored into a `PhysicsConfig` Bevy resource (`src/config.rs`) at startup. If `assets/physics.toml` is present it overrides the defaults — no recompilation is required. If the file is absent the compiled-in defaults are used silently. The resource is injected before all other startup systems so every system reads up-to-date values on the first frame.

#### Version Constraints
- **Bevy 0.17** + **bevy_rapier2d 0.32**: Current versions. Migration from 0.13 completed February 2026.
- **Contact manifold API**: `ReadRapierContext` system param; contacts queried via `ctx.single()?.simulation.contact_pairs(colliders, rigidbody_set)` in formation and particle-locking systems.

### Future Enhancement Roadmap

#### Physics Improvements
- **Concave asteroid deformation**: Track per-vertex damage state; move impact vertex inward and recompute hull to simulate craters and progressive destruction
- ~~**Gravitational binding energy merge criterion**: Replace velocity-threshold merging with a binding-energy check; clusters only merge if their kinetic energy is below the gravitational potential energy of the cluster, producing more physically realistic aggregation~~ ✅ Completed
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
- ~~**Configuration file support**: Load physics constants from an `assets/physics.toml` file at startup, enabling tuning without recompilation~~ ✅ **Completed** — `assets/physics.toml` loaded at startup via `PhysicsConfig` resource
- **Score and progression system**: Points for asteroid destruction scaled by size; wave-based difficulty ramp spawning more and larger asteroids over time
- **Power-up asteroids**: Special-coloured asteroids that grant the player temporary buffs (shield, rapid-fire, gravity bomb) on destruction
- **Boss asteroids**: Single very-large composite (size ≥ 20) with scripted split behaviour acting as a wave-ending target
- **Multiplayer (local co-op)**: Spawn a second player ship feeding off the same physics world; share the asteroid field and scoring

#### Test & Developer Tooling
- **Automated regression baseline**: Store golden frame-log snapshots in `tests/golden/` and compare on each test run, automatically catching physics constant drift
- **In-game physics inspector**: Toggle an overlay showing entity IDs, velocities, and contact counts on-screen for live debugging without restarting in test mode
- **Hot-reload constants**: Watch `assets/physics.toml` for changes at runtime and apply updated constants on the fly

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
- ✅ **Near-miss stability** (validates gravity fix: pass-by speed stays within expected range, not runaway acceleration)
- ✅ Long-range gravity dynamics
- ✅ Culling system (no phantom forces)
- ✅ Mixed-size interactions
- ✅ Gravity distance boundaries
- ✅ Complex N-body systems

### Critical Fix Validated

**Gravity Threshold Fix**: Changed minimum gravity distance behaviour from clamping to skipping entirely when below `MIN_GRAVITY_DIST`. This prevents energy injection during close encounters and high-speed passes, ensuring stable physics across all scenarios.
