# Accretion Architecture & Physics

## Overview

Accretion is an ECS-based **asteroid simulation engine** built on **Bevy 0.17** with physics powered by **Rapier2D**. The simulation features natural asteroid aggregation through N-body gravity attraction and collision-based merging into larger composite structures.

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
├── config.rs             - PhysicsConfig Bevy resource; loaded from assets/physics.toml at startup and hot-reloaded at runtime
├── menu.rs               - GameState enum (MainMenu / ScenarioSelect / Playing / Paused / GameOver), SelectedScenario resource, MainMenuPlugin, splash screen + scenario-select + pause menu UI
├── asteroid.rs           - Unified asteroid components and spawn functions; convex hull computation
├── simulation.rs         - Physics systems: N-body gravity, cluster detection, composite formation
├── spatial_partition.rs  - KD-tree spatial index for O(K + log N) neighbour lookup (replaces flat grid)
├── rendering.rs          - OverlayState resource, debug overlay panel UI, gizmo rendering (asteroids, boundary, force/velocity vectors)
├── asteroid_rendering.rs - Mesh2d filled-polygon rendering for asteroids (attach-on-spawn, wireframe_only sync)
├── save.rs               - Slot-based save/load snapshot schema, TOML I/O, and world restore systems
├── player/               - Player ship entity, WASD controls, projectile firing, Mesh2d ship/projectile rendering, camera follow
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

## Implemented Upgrade Systems

Upgrades are implemented as ECS resources and purchased in the unified ore shop (`GameState::OreShop` in `src/menu.rs`).

- **Primary weapon upgrades** (`PrimaryWeaponLevel` in `src/player/state.rs`): raises projectile full-destroy threshold by level; larger asteroids are chipped.
- **Secondary weapon upgrades** (`SecondaryWeaponLevel` in `src/player/state.rs`): raises missile full-destroy threshold and increases split fragment count (`pieces = display_level + 1`, clamped by `missile_split_max_pieces`) for targets above threshold. If `display_level >= asteroid_size`, impacts fully decompose into unit fragments. Split geometry is impact-weighted: center hits trend toward equal-area fragments, edge hits produce asymmetric mass distributions.
- **Missile telemetry** (`MissileTelemetry` in `src/simulation.rs`): tracks shots/hits, outcome counts (destroy/split/decompose), and mass-based totals. Periodic frame logs expose outcome distribution and a simple `frames_per_kill` TTK proxy for balancing passes.
- **Ore magnet upgrades** (`OreAffinityLevel` in `src/mining.rs`): increases ore magnet radius and pull strength per level via `radius_at_level()` and `strength_at_level()`.
- **Tractor beam scaling** (`TractorBeamLevel` in `src/player/state.rs`): scales beam force/range plus max affected asteroid size/speed envelope.
- **Economy coupling**: weapon/missile/magnet/tractor upgrades spend from shared `PlayerOre` and use `try_upgrade(&mut ore)` style resource methods.
- **Session scope**: upgrade resources reset when returning to `MainMenu`; persistent progression depends on the planned save/load system.

## Save / Load Architecture

- **Persistence format**: versioned TOML snapshots under `saves/slot_N.toml` (`N = 1..3`).
- **Schema** (`src/save.rs`):
  - `SaveSnapshot` root (`version`, `scenario`, `player`, `asteroids`, `resources`)
  - `PlayerSnapshot` captures transform/velocity + health state
  - `AsteroidSnapshot` captures transform/velocity + `AsteroidSize` + local-space `Vertices`
  - `ResourceSnapshot` captures score/lives/ore/ammo and upgrade levels (weapon, missile, magnet, tractor)
- **Save trigger**: pause-menu `SAVE 1/2/3` buttons emit `SaveSlotRequest`; `handle_save_slot_requests_system` serializes current ECS state while paused.
- **Load trigger**: main-menu `LOAD GAME` opens `LoadGameMenu`; selecting a slot reads TOML into `PendingLoadedSnapshot` and transitions to `Playing`.
- **Load apply**: `apply_pending_loaded_snapshot_system` restores resources, respawns asteroids from local-space hull vertices, and respawns the player with saved physics/health state.

## Physics Rules

### Gravity System (`nbody_gravity_system`)

- **Constant**: `GRAVITY_CONST` (`src/constants.rs`) — mutual attraction strength
- **Minimum distance threshold**: `MIN_GRAVITY_DIST` — asteroids closer than this are excluded; Rapier handles contact physics below this range to prevent energy injection during close encounters
- **Maximum gravity distance**: `MAX_GRAVITY_DIST` — matches `CULL_DISTANCE` so culled asteroids exert no phantom forces
- **Force**: Applied between pairs as `F = GRAVITY_CONST / distance²`
- **Tidal torque**: In addition to the centre-of-mass force, a differential (tidal) torque is applied to each body.  For each pair, the gravitational acceleration at each vertex of body i is compared to the acceleration at its COM; the resulting lever-arm cross-products are summed to give a net torque that spins asymmetric composites realistically.  Scaled by `TIDAL_TORQUE_SCALE` (set to 0 to disable).
- **Optimization**: Uses `SpatialGrid` (KD-tree) for O(N·K + N log N) candidate lookup instead of O(N²) brute-force

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

### Density & Visual Size Scaling

- **Constant**: `ASTEROID_DENSITY` (`src/constants.rs`, default `0.1`) — mass units per world-unit².
- **Purpose**: Ensures a predictable, consistent relationship between an asteroid's `AsteroidSize` (gravitational mass in unit-triangle equivalents) and its visual polygon area on screen:
  ```
  target_area = AsteroidSize / ASTEROID_DENSITY
  ```
- **Applied to**: Merged composites (formation system) and split/chip fragments (combat system). Initial spawns retain their randomised size-scale for visual variety.
- **Helpers** (`src/asteroid.rs`):
  - `polygon_area(vertices)` — shoelace formula for polygon area
  - `rescale_vertices_to_area(vertices, target_area)` — scales all vertices radially from the centroid so the polygon encloses exactly `target_area`
- **Tunable** via `assets/physics.toml` (`asteroid_density`). Lower → bigger polygons; higher → smaller polygons for the same mass.

### Culling & Boundary

- **Soft boundary**: `SOFT_BOUNDARY_RADIUS` — asteroids beyond this distance feel a linear inward spring force (`soft_boundary_system`) that nudges them back toward the centre.  Force = `SOFT_BOUNDARY_STRENGTH × (dist − SOFT_BOUNDARY_RADIUS)` inward.
- **Hard-cull distance**: `HARD_CULL_DISTANCE` — safety net; only asteroids that escape the soft spring entirely are removed outright.  In normal operation almost no asteroids reach this distance.
- **Stats boundary**: `CULL_DISTANCE` — reference for the live-count display; asteroids within this radius are shown as "live".
- Artificial velocity damping ramps have been removed; energy loss occurs only through collisions and the outer soft spring.

### Player Tractor Beam

- **Activation**:
  - hold `Q` to pull
  - hold `E` to push
  - hold `Q+E` for freeze mode
- **System**: `tractor_beam_force_system` runs in `FixedUpdate` after `nbody_gravity_system` so beam forces are added on top of gravity each physics step.
- **Target filter**:
  - asteroid must be within beam range and outside `TRACTOR_BEAM_MIN_DISTANCE`
  - asteroid must be inside a ship-forward cone (`TRACTOR_BEAM_AIM_CONE_DOT`) around the ship forward vector
  - asteroid size and speed must be below level-scaled limits (`TRACTOR_BEAM_MAX_TARGET_SIZE_*`, `TRACTOR_BEAM_MAX_TARGET_SPEED_*`)
- **Push/Pull force model**: distance-falloff force applied along player-target axis.
- **Freeze force model**: anchored-offset spring-damper hold force
  - on freeze engage, each target stores a held offset `r_hold` relative to ship position (clamped by `tractor_beam_freeze_max_hold_offset`)
  - per step, desired position is `p_target = p_ship + r_hold`
  - `F_freeze = clamp((p_target - p_ast) * tractor_beam_freeze_offset_stiffness - v_rel * tractor_beam_freeze_velocity_damping, force_limit)`
  - `force_limit = force_at_level * tractor_beam_freeze_force_multiplier`
  - damping is reduced above `tractor_beam_freeze_max_relative_speed` to prevent spikes
- **VFX**: tractor force emits directional light-blue particles via `spawn_tractor_beam_particles`:
  - pull (cyan), push (blue), freeze (aqua-white)
  - emission is burst-throttled and capped per fixed-step to keep frame-time stable
- **Stability controls**: strict speed/mass/range/cone gating, frozen-mode size/speed multipliers, and freeze force caps avoid runaway acceleration.

## ECS Systems Execution Order

### Update Schedule

1. **`stats_counting_system`** - Counts live (within `CULL_DISTANCE`) / hard-culled (beyond `HARD_CULL_DISTANCE`) asteroids
2. **`soft_boundary_system`** - Applies inward spring force to asteroids beyond `SOFT_BOUNDARY_RADIUS`
3. **`culling_system`** - Hard-removes asteroids beyond `HARD_CULL_DISTANCE`
4. **`neighbor_counting_system`** - Counts nearby asteroids using grid (O(N·K))
5. **`particle_locking_system`** - Synchronizes velocities of slow touching asteroids via Rapier contact_pairs iterator (O(C), C = active contacts)
6. **`player_control_system`** - Applies WASD thrust/rotation to player ship
7. **`projectile_fire_system`** - Fires projectiles on spacebar (with cooldown)
8. **`despawn_old_projectiles_system`** - Expires projectiles after lifetime/distance limit
9. **`user_input_system`** - Left-click spawns asteroids; mouse wheel zooms
10. **`camera_follow_system`** - Centres camera on player ship each frame
11. **`camera_zoom_system`** - Applies zoom scale to camera transform
12. **`attach_asteroid_mesh_system`** - Attaches `Mesh2d` filled polygon to newly spawned asteroids (`Added<Asteroid>`)
13. **`sync_asteroid_mesh_visibility_system`** - Propagates `wireframe_only` toggle to asteroid mesh visibility
14. **`attach_player_ship_mesh_system`** - Attaches `Mesh2d` filled polygon to the player ship on spawn (`Added<Player>`)
15. **`attach_projectile_mesh_system`** - Attaches `Mesh2d` disc mesh to each new projectile (`Added<Projectile>`)
16. **`sync_player_and_projectile_mesh_visibility_system`** - Propagates `wireframe_only` to ship and projectile mesh visibility
17. **`gizmo_rendering_system`** - Renders asteroid gizmo overlays (wireframes, forces, velocity, boundary)
18. **`player_gizmo_system`** - Renders optional ship outline, aim indicator, health bar, projectile outlines

### FixedUpdate Schedule (chained in order)

1. **`rebuild_spatial_grid_system`** - Rebuilds grid with physics-step positions
2. **`nbody_gravity_system`** - Applies mutual gravity using spatial grid (O(N·K))
3. **`tractor_beam_force_system`** - Applies player beam pull/push forces to eligible asteroids
4. **`neighbor_counting_system`** - Counts nearby asteroids using current fixed-step positions
5. **Rapier physics** - Solves all collision, integrates velocities, populates contact manifolds

### PostUpdate Schedule (CRITICAL TIMING)

1. **`asteroid_formation_system`** - Must run AFTER Rapier physics populates contacts
2. **`test_logging_system`** & **`test_verification_system`** - Runs after merging to see final states

**Critical**: System scheduling ensures proper data consistency. Asteroid formation must run *after* physics updates contacts.

## Spatial Index (`spatial_partition.rs`)

The `SpatialGrid` resource is backed by a balanced 2-D KD-tree for efficient range queries.

- **Build**: `rebuild(points)` constructs a balanced KD-tree each frame — O(N log N) via median-split on alternating X/Y axes
- **Lookup**: `get_neighbors_excluding(entity, pos, max_distance)` returns all entities within an exact Euclidean sphere — O(K + log N) where K is the result count
- **Accuracy**: The KD-tree performs an exact spherical range query; the old grid returned square-cell over-approximations that callers had to re-filter
- **Non-uniform efficiency**: Unlike a fixed grid, the KD-tree adapts to where asteroids actually are.  Dense clusters do not degrade into O(N_cell²) behaviour.
- **Rebuild system**: `rebuild_spatial_grid_system` — called at the start of each FixedUpdate before the gravity system

## Physics Constants Reference

All tuneable constants are centralised in **`src/constants.rs`** with doc-comments explaining each value's purpose, tested range, and observable effect. The file is the single source of truth — never duplicate values into documentation.

Key constant groups (see `src/constants.rs` for current values):

| Group | Constants |
|---|---|
| World bounds | `SIM_WIDTH`, `SIM_HEIGHT`, `PLAYER_BUFFER_RADIUS` |
| Gravity | `GRAVITY_CONST`, `MIN_GRAVITY_DIST`, `MAX_GRAVITY_DIST`, `TIDAL_TORQUE_SCALE` |
| Cluster formation | `VELOCITY_THRESHOLD_LOCKING` (velocity sync), `GRAVITY_CONST` (binding energy) |
| Collision | `RESTITUTION_SMALL`, `FRICTION_ASTEROID` |
| Boundary | `SOFT_BOUNDARY_RADIUS`, `SOFT_BOUNDARY_STRENGTH`, `HARD_CULL_DISTANCE`, `CULL_DISTANCE` |
| Camera | `MIN_ZOOM`, `MAX_ZOOM`, `ZOOM_SPEED` |
| Player movement | `THRUST_FORCE`, `REVERSE_FORCE`, `ROTATION_SPEED` |
| Tractor beam | `TRACTOR_BEAM_RANGE_*`, `TRACTOR_BEAM_FORCE_*`, `TRACTOR_BEAM_MAX_TARGET_SIZE_*`, `TRACTOR_BEAM_MAX_TARGET_SPEED_*`, `TRACTOR_BEAM_MIN_DISTANCE`, `TRACTOR_BEAM_AIM_CONE_DOT` |
| Player OOB | `OOB_RADIUS`, `OOB_DAMPING`, `OOB_RAMP_WIDTH` |
| Player combat | `PROJECTILE_SPEED`, `FIRE_COOLDOWN`, `PROJECTILE_LIFETIME`, `MISSILE_INITIAL_SPEED`, `MISSILE_ACCELERATION`, `MISSILE_SPEED` |
| Weapon upgrades | `PRIMARY_WEAPON_MAX_LEVEL`, `WEAPON_UPGRADE_BASE_COST`, `SECONDARY_WEAPON_MAX_LEVEL`, `SECONDARY_WEAPON_UPGRADE_BASE_COST`, `TRACTOR_BEAM_MAX_LEVEL`, `TRACTOR_BEAM_UPGRADE_BASE_COST` |
| Ore economy & magnet upgrades | `ORE_HEAL_AMOUNT`, `ORE_MAGNET_BASE_RADIUS`, `ORE_MAGNET_BASE_STRENGTH`, `ORE_AFFINITY_MAX_LEVEL`, `ORE_AFFINITY_UPGRADE_BASE_COST` |
| Player health | `PLAYER_MAX_HP`, `DAMAGE_SPEED_THRESHOLD`, `INVINCIBILITY_DURATION` |
| Gamepad | `GAMEPAD_BRAKE_DAMPING`, `GAMEPAD_LEFT_DEADZONE`, etc. |
| Asteroid geometry | `TRIANGLE_BASE_SIDE`, `SQUARE_BASE_HALF`, `POLYGON_BASE_RADIUS`, `HEPTAGON_BASE_RADIUS`, `OCTAGON_BASE_RADIUS`, `PLANETOID_BASE_RADIUS`, `PLANETOID_UNIT_SIZE` |
| Asteroid density | `ASTEROID_DENSITY` — mass units per world-unit² (default `0.1`); governs visual area of merged/split polygons |

## Scenarios

Built-in scenarios are variants of `SelectedScenario` (in `menu.rs`) and spawned by `spawn_initial_world` (in `main.rs`).

| Scenario | Spawn function | Description |
|----------|---------------|-------------|
| **Field** | `spawn_initial_asteroids` (100) + `spawn_planetoid` | Noise-clustered asteroid field with one large planetoid offset from the player |
| **Orbit** | `spawn_orbit_scenario` | Large central body ringed by debris: ring 1 triangles (r=280), ring 2 triangles+squares (r=480, scale 1.0–1.8), ring 3 pentagons/hexagons/heptagons (r=680, scale 1.0–2.2).  Each body's orbital speed is computed individually via `v = sqrt(G·AsteroidSize·M_central / (r·m_rapier))` |
| **Comets** | `spawn_comets_scenario` | 20 large (9–12 sided, scale 2.5–4.5) asteroids launched inward at 80–140 u/s.  High speed → fragmentation gameplay |
| **Shower** | `spawn_shower_scenario` | 250 unit triangles scattered uniformly within a 1 600-unit radius, near-zero velocity.  Shows natural accretion in real time |

## Testing Framework

### Test System

- **Trigger**: `ACCRETION_TEST=<test_name>` environment variable
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
- ~~**Hard world boundary**: `CULL_DISTANCE` (2000 units) radius; asteroids beyond this are permanently removed each frame~~ ✅ Replaced by soft boundary spring + safety hard-cull at `HARD_CULL_DISTANCE` (2500 units)
- **Spawn area**: Initial asteroids (100 default) distributed within `SIM_WIDTH`×`SIM_HEIGHT` (4000×4000) using noise-based clustering for natural asteroid field formations; distribution controlled by hash-based 2D noise with adjustable frequency; `PLAYER_BUFFER_RADIUS` exclusion zone at origin; values tunable via `assets/physics.toml` at runtime
- **Max simulation density**: Gizmo-based force-vector annotations auto-disabled at high count (> `force_vector_hide_threshold`); asteroid, ship, and projectile fills use retained `Mesh2d` GPU assets that scale efficiently with entity count

#### Physics Simplifications
- **Convex-only colliders**: All asteroid shapes are convex polygons; concavities from impacts are approximated by their convex hull, not modelled directly
- **Gravity cutoff**: Gravity is disabled inside `MIN_GRAVITY_DIST` (Rapier handles close contacts) and beyond `MAX_GRAVITY_DIST`; there is no smooth transition
- ~~**No rotational gravity torque**: Gravity applies only linear force (no torque based on off-centre mass distribution)~~ ✅ Implemented — tidal differential torques now applied per pair
- **Cluster formation is discrete**: Merging is all-or-nothing per frame; a cluster either fully merges in one PostUpdate step or waits until the next frame
- **Single-pass hull computation**: Composite hull is computed once at merge time; subsequent impacts reduce vertex count but do not recompute the full hull from physics state

#### Runtime Configuration

Physics constants are defined in `src/constants.rs` as compile-time defaults and mirrored into a `PhysicsConfig` Bevy resource (`src/config.rs`). At startup, `assets/physics.toml` (if present) overrides defaults. During runtime, the file is polled and hot-reloaded when its modification timestamp changes, so updated values are applied without restart or recompilation. If the file is absent the compiled-in defaults are used silently.

#### Version Constraints
- **Bevy 0.17** + **bevy_rapier2d 0.32**: Current versions. Migration from 0.13 completed February 2026.
- **Contact manifold API**: `ReadRapierContext` system param; contacts queried via `ctx.single()?.simulation.contact_pairs(colliders, rigidbody_set)` in formation and particle-locking systems.

### Future Enhancement Roadmap

#### Physics Improvements
- **Concave asteroid deformation**: Track per-vertex damage state; move impact vertex inward and recompute hull to simulate craters and progressive destruction
- ~~**Gravitational binding energy merge criterion**: Replace velocity-threshold merging with a binding-energy check; clusters only merge if their kinetic energy is below the gravitational potential energy of the cluster, producing more physically realistic aggregation~~ ✅ Completed
- ~~**Rotational-inertia-aware gravity torque**: Include mass distribution (second moment of area) in gravity force application so oddly-shaped composites develop realistic rotation~~ ✅ Completed
- ~~**Soft boundary with elastic reflection**: Replace hard cull-at-1000u removal with a potential-well boundary that gently reflects asteroids back toward the simulation centre~~ ✅ Completed
- ~~**KD-tree neighbor search**: Replace the static 500-unit spatial grid with a dynamic KD-tree to better handle highly non-uniform asteroid distributions (dense cluster + sparse outer field)~~ ✅ Completed
- **Orbital presets**: Optional initial conditions (Keplerian orbits, accretion disk configuration) as alternative to random spawning

#### Visual & Rendering Enhancements
- **Particle effects system**: Impact dust clouds on projectile hits; merge vortex animations; debris trail on asteroid destruction
- **Level-of-Detail (LOD) rendering**: All asteroids, the player ship, and projectiles now use retained `Mesh2d` filled GPU assets — the per-vertex CPU bottleneck has been removed. ✅ Implemented.
- **Velocity heat-map coloring**: Tint asteroid wireframes from blue (slow) to red (fast) to give instant visual feedback on kinetic energy distribution
- **Crater / fracture overlays**: Draw cracks on asteroid surfaces proportional to accumulated damage (impacts that didn't yet destroy the asteroid)
- **Dynamic camera FOV**: Camera zoom automatically increases when the player moves fast, giving a wider field of view at speed
- **Post-processing effects**: Bloom on high-energy impacts and merges; chromatic aberration during player damage invincibility

#### Gameplay & Extensibility
- ~~**Configuration file support**: Load physics constants from an `assets/physics.toml` file at startup, enabling tuning without recompilation~~ ✅ **Completed** — `assets/physics.toml` loaded at startup via `PhysicsConfig` resource
- ~~**Planet entity type**: Introduce an anchored high-mass world body that participates in gravity but is excluded from asteroid merge/split weapon destruction paths~~ ✅ **Completed**
- **Score and progression system**: Points for asteroid destruction scaled by size; wave-based difficulty ramp spawning more and larger asteroids over time
- **Power-up asteroids**: Special-coloured asteroids that grant the player temporary buffs (shield, rapid-fire, gravity bomb) on destruction
- **Boss asteroids**: Single very-large composite (size ≥ 20) with scripted split behaviour acting as a wave-ending target
- **Multiplayer (local co-op)**: Spawn a second player ship feeding off the same physics world; share the asteroid field and scoring

#### Test & Developer Tooling
- **Automated regression baseline**: Store golden frame-log snapshots in `tests/golden/` and compare on each test run, automatically catching physics constant drift
- ~~**Profiler integration**: Frame-time diagnostics plus in-game schedule timing breakdown for Update / FixedUpdate / PostUpdate~~ ✅ Completed
- ~~**In-game physics inspector**: Toggle an overlay showing entity IDs, velocities, and contact counts on-screen for live debugging without restarting in test mode~~ ✅ Completed
- ~~**Debug spatial grid visualization**: Toggle KD-tree split-cell line rendering to inspect spatial partition behavior in real time~~ ✅ Completed
- ~~**Hot-reload constants**: Watch `assets/physics.toml` for changes at runtime and apply updated constants on the fly~~ ✅ Completed

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
ACCRETION_TEST=near_miss cargo run --release

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
