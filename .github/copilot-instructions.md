# Project Guidelines

## Code Style

- **Language**: Rust
- **Formatting**: Use `rustfmt` for all code formatting. Format on save and before commits. Configuration in `rustfmt.toml` if needed.
- **Linting**: Use `clippy` with `cargo clippy -- -D warnings` to enforce all warnings as errors. Follow clippy suggestions for idiomatic Rust patterns.

## Architecture

The "grav-sim" project is an ECS-based **asteroid simulation engine** built on **Bevy** with physics powered by **Rapier2D**. All objects in the simulation are asteroids that naturally aggregate through N-body gravity into larger composite polygonal structures.

> For authoritative version numbers, module layout, ECS system execution order, and physics constants see **[ARCHITECTURE.md](../ARCHITECTURE.md)**.  
> For user controls, camera behaviour, and UI details see **[FEATURES.md](../FEATURES.md)**.

- **Purpose**: Pure asteroid-based simulation where asteroids naturally aggregate through gravitational attraction and collision to form larger composite asteroids (polygons) that visually rotate based on physics.
- **Framework**: Bevy (ECS, rendering, windowing) + bevy_rapier2d (collision detection, rigid body dynamics). See `Cargo.toml` for current versions.
- **Core Modules**: See "Module Structure" in `ARCHITECTURE.md`.
- **Key files**:
  - `src/constants.rs` - All tuneable physics and gameplay constants (single source of truth)
  - `src/main.rs` - Bevy app main entry and test mode routing
  - `src/asteroid.rs` - Core asteroid definitions and spawn functions
  - `src/simulation.rs` - All ECS systems
  - `src/spatial_partition.rs` - Spatial grid for O(N·K) neighbor lookup
  - `src/graphics.rs` - Camera/rendering setup
  - `src/player/` - Player ship, controls, combat, rendering
  - `src/testing.rs` - Automated test scenarios
- **Entity Types**: All asteroids are unified entities with local-space vertices.
  - Spawn as triangles or polygons depending on configuration.
  - Composite asteroids formed when 2+ asteroids touch and merge.
- **ECS Systems** (Execution Order - CRITICAL): See "ECS Systems Execution Order" in `ARCHITECTURE.md`.
  - Formation system **must** run in `PostUpdate` after Rapier `FixedUpdate` populates contacts.
  - Test systems **must** run in `PostUpdate` after formation to observe merged results.

## Physics Rules

> All numeric constants (restitution, gravity strength, distance thresholds, velocity thresholds, damping factors, culling distance, etc.) are defined in `src/constants.rs` and documented with current values in the "Physics Constants Reference" section of **[ARCHITECTURE.md](../ARCHITECTURE.md)**. Do not hard-code or repeat those values here.

### Small Asteroid Properties (Triangles)

- **Shape**: Equilateral triangle with relative vertices stored, rotated by transform.
- **Collider**: Ball collider; radius in `src/asteroid.rs`.
- **Restitution**: Lower than large asteroids (see `ARCHITECTURE.md`).
- **Color**: Random grey shade per asteroid.

### Large Asteroid Properties (Polygons)

- **Formation**: Created when 2+ asteroids touch and their relative velocity is below the merge threshold.
- **Shape**: Convex hull of constituent asteroids (gift-wrapping algorithm).
- **Collider**: Exact convex polygon.
- **Mass**: Sum of constituent masses.
- **Rendering**: White wireframe outline, vertices rotated by transform rotation.
- **Velocity**: Inherits averaged linear and angular velocity from constituents.

### N-Body Gravity

- Applied between all asteroid pairs within a maximum distance (beyond which forces are skipped).
- A minimum distance threshold prevents energy injection during close encounters; Rapier handles contact physics below that threshold.
- Uses `SpatialGrid` for O(N·K) lookup instead of O(N²) brute-force.

### Velocity Synchronization

- When two touching asteroids both move below a slow threshold, velocities are averaged (linear + angular).
- Prepares asteroids for smooth composite formation.

### Cluster Formation & Merging (CRITICAL IMPLEMENTATION)

- **Detection**: Flood-fill algorithm through Rapier contact manifolds.
- **Contact query**: MUST run in `PostUpdate` after Rapier `FixedUpdate` populates contacts.
- **Velocity threshold**: Cluster members may still merge even at moderate speeds (see `ARCHITECTURE.md`).
- **Hull computation**:
  - Collect ALL vertices from cluster members in **world-space**.
  - Apply transform rotation: `world_v = center + rotation * local_v`
  - Compute convex hull from the world-space vertex set.
  - Convert hull back to **local-space** relative to the new centroid.
  - Spawn composite with local-space hull for correct rendering.
- **Velocity inheritance**: Average linear and angular velocity from cluster members.
- **Prevention**: Processed asteroids tracked per-frame to prevent duplicate merging.

### Environmental Damping

- Artificial environmental damping has been **removed**. Energy dissipation occurs only via collision restitution.

### Culling & Boundary

- Asteroids beyond the cull distance (see `ARCHITECTURE.md`) are removed each frame.
- Artificial velocity damping ramps have been removed; the boundary is a hard cull.

## User Interaction

See **[FEATURES.md](../FEATURES.md)** for the full list of controls, camera behaviour, and UI details.

- **Left-click / Space**: Fire projectile or spawn asteroid (mode-dependent); see `FEATURES.md`.
- **WASD + mouse**: Player ship movement and aiming.
- **Mouse wheel**: Camera zoom.
- **Coordinate system**: Screen (0,0) top-left → World (0,0) center; X right, Y up.

## Current Implementation Status

- ✅ Player ship with WASD movement, mouse aiming, and projectile firing
- ✅ Asteroid destruction: destroy / scatter / split / chip based on size
- ✅ Pure asteroid-only unified system (all entities equal)
- ✅ Cluster-based formation with flood-fill contact detection
- ✅ Wireframe rendering with rotation (no sprite overlays)
- ✅ Velocity inheritance and composite stability
- ✅ Convex hull composition from all constituent vertices
- ✅ Gravity attraction → collision → merging pipeline
- ✅ Automated test framework with environment variable triggering
- ✅ PostUpdate system scheduling for physics-aware logic
- ✅ Spatial grid for O(N·K) gravity and neighbor queries

## Testing Strategy (Session-Learned Best Practices)

### Test Framework

- **Environment Variable**: `GRAV_SIM_TEST=<test_name>` triggers test mode from `main.rs`
- **Available Tests**: See "Available Tests" in `ARCHITECTURE.md` for the current list
- **Test Config Resource**: Tracks frame count, asteroid counts, test name for automated verification
- **Player isolation**: In test mode the player entity is **not spawned** — player systems run but are no-ops

### Test Logging Strategy

- Log at periodic key frames to understand physics behavior
- Log both **positions** and **velocities** to observe whether asteroids are attracting or repelling
- This reveals phase transitions: attraction → collision → merge

### Test Verification

- Compare `initial_asteroid_count` vs `final_asteroid_count`
- For merging tests: expect `final < initial` AND `final >= 1`
- Use frame logging to debug: if asteroids don't merge, watch velocity and position trends

### Physics Debugging via Tests

1. **Spawn asteroids precisely** (not randomly) to reproduce behavior
2. **Log positions/velocities** across frames to see trends
3. **Use consistent test runs** (same spawn positions) for reproducible results
4. **Adjust constants** (gravity_const, velocity_threshold) and re-run same test
5. **Watch for phase changes**: attraction → collision → merge

### Common Pitfalls Found

- **System scheduling**: Formation system must run AFTER physics updates contacts
- **Hull computation**: Must use constituent vertex positions, not just center positions
- **Overlapping spawns**: Cause Rapier separation impulses; spawn touching but not overlapping
- **Local vs world space**: Vertices must be stored local, but hull computation in world space
- **Contact detection timing**: Contacts aren't available in same frame entities spawn; need 1+ frame delay

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

- **File Structure**: Standard Rust layout (see "Module Structure" in `ARCHITECTURE.md` for the canonical list).
- **Naming Conventions**:
  - `snake_case` for functions, variables, modules
  - `PascalCase` for types, structs, enums, traits
  - `SCREAMING_SNAKE_CASE` for constants
- **Physics Tuning**: All constants are defined in `src/constants.rs` (see "Physics Constants Reference" in `ARCHITECTURE.md`) and require `cargo build` to change. Do **not** hard-code specific values in documentation or instructions outside of `ARCHITECTURE.md`.

## Integration Points

- **External APIs**: None - fully self-contained simulation
- **Dependencies** (see `Cargo.toml` for current versions):
  - `bevy` - ECS engine, rendering, windowing
  - `bevy_rapier2d` - Physics engine integration for Bevy
  - `rand` - Random grey shades for asteroid coloring
  - `glam` - Math library (Vec2, Quat) via Bevy
- **Cross-Component Communication**:
  - Components: `Asteroid`, `Vertices` (local-space), `NeighborCount`, plus Rapier/Bevy physics components
  - Systems read/write components; Rapier applies physics automatically in FixedUpdate
  - Formation system queries Rapier context for contacts (requires PostUpdate scheduling)
  - Gizmos system reads transforms and locally-stored vertices for rendering
  - User input system spawns via `Commands`

## Critical Implementation Notes

### Vertex Storage (LOCAL-SPACE Essential)

- **Why local-space?**: Enables correct rotation rendering; simplifies physics collider creation
- **Storage**: `Vertices(Vec<Vec2>)` component stores vertices relative to entity position
- **Rendering**: Gizmos system rotates local vertices by transform rotation before drawing
- **Hull Composition**: When merging, convert local vertices to world-space, compute hull, convert back to local

### System Scheduling Constraints

- **Rapier Physics**: Runs in FixedUpdate; solves all physics, populates contact manifolds
- **Formation System**: Must run in PostUpdate; queries populated contact manifolds
- **Test Systems**: Must run in PostUpdate after formation; sees results of merging
- **Violation Consequences**: If formation runs before physics, no contacts detected → no merging

### Gravity Constant Tuning

- See "Physics Constants Reference" in `ARCHITECTURE.md` for the current value and justification.
- **Observable metrics**: smooth acceleration, stable velocities, asteroids merge on contact.
- **Indicates system is healthy**: Smooth acceleration, no bouncing apart after contact.

## Documentation Maintenance

**Maintain living documentation rather than creating new files for each change.**

### Documentation Files

The project maintains three consolidated documentation files:

1. **`ARCHITECTURE.md`** - Core technical reference
   - System architecture and module organization
   - Physics rules and ECS system execution order
   - Physics constants and equations
   - Test framework documentation
   - Update when: Adding/modifying systems, changing physics constants, restructuring modules

2. **`FEATURES.md`** - User controls and runtime behavior
   - Asteroid spawning and camera controls
   - Visual feedback (stats display, boundary visualization)
   - Implementation details for user-facing systems
   - Update when: Adding/modifying user controls, changing UI behavior, adjusting camera limits

3. **`CHANGELOG.md`** - Concise summary of all changes
   - Major features and improvements
   - Bug fixes with root causes and solutions
   - Test results and validation summary
   - Build status and deployment info
   - Update when: Completing significant features, fixing critical bugs, reaching milestones

### Documentation Best Practices

- **Update docs with code changes**: When implementing a feature or fix, update the relevant documentation file immediately
- **Avoid temporary documentation files**: Do not create session-specific or change-specific markdown files
- **Keep it concise**: Consolidate related info; remove redundancy
- **Link to GitHub instructions**: Reference `copilot-instructions.md` for repeated architectural details
- **Example format for changes**: "Increased `GRAVITY_CONST` in `src/constants.rs` (documented in ARCHITECTURE.md)"

### PhysicsConstants Updates

When tuning physics constants:

1. Update the constant value in `src/constants.rs`
2. Update the constant reference in ARCHITECTURE.md with both the value and justification
3. Add a line to CHANGELOG.md explaining the change and its observable effect
4. Run relevant tests to validate the change (see Build Verification below)

## Build Verification for Code Changes

**Every significant code change must pass the following verification steps before proceeding to the next task.**

### Verification Checklist

For any change to physics systems, UI, core logic, or test framework:

1. **Compilation & Formatting**
   - ✅ `cargo check` passes with zero errors
   - ✅ `cargo fmt` (code is properly formatted)
   - ✅ `cargo clippy -- -D warnings` passes (zero warnings)
2. **Build Status**
   - ✅ `cargo build` succeeds (debug mode)
   - ✅ `cargo build --release` succeeds (optimized)
3. **Test Existence & Execution**
   - ✅ Relevant tests exist for the changed functionality
   - ✅ If modifying physics: run `./test_all.sh` or specific test with `GRAV_SIM_TEST=<name> cargo run --release`
   - ✅ If modifying UI/input: manually verify controls work as expected
   - ✅ All tests pass (compare initial vs final asteroid counts as applicable)

4. **Physics Validation** (if changing gravity/collision/merging logic)
   - ✅ Run at least 2 related tests to verify no regressions
   - ✅ Confirm asteroids behave predictably (no runaway acceleration, stable velocities)
   - ✅ Verify merging occurs when expected based on distance/velocity
   - ✅ Check that culled asteroids are completely removed

5. **Documentation Update**
   - ✅ Update ARCHITECTURE.md if constants or system order changed
   - ✅ Update FEATURES.md if user controls or UI changed
   - ✅ Update CHANGELOG.md with brief summary of the change

### When to Run Full Verification

- **Always**: Modifying physics systems (gravity, collision, merging, culling)
- **Always**: Adding new features or user controls
- **Always**: Changing physics constants
- **Before commit**: Any change that affects simulation behavior
- **Optional**: Minor formatting fixes, comment updates, refactoring without behavior change

### Example Verification Flow

```bash
# 1. Check compilation
cargo check
cargo fmt
cargo clippy -- -D warnings

# 2. Build both modes
cargo build
cargo build --release

# 3. Run relevant tests
GRAV_SIM_TEST=gravity cargo run --release
GRAV_SIM_TEST=near_miss cargo run --release
GRAV_SIM_TEST=culling_verification cargo run --release

# 4. Confirm all tests pass
./test_all.sh

# 5. Update documentation in ARCHITECTURE.md, FEATURES.md, CHANGELOG.md

# 6. Verify one final time
cargo clippy -- -D warnings
```

### Debug Tips for Failed Verification

- **Build errors**: Check Rust edition (2021), Bevy/bevy_rapier2d versions in `Cargo.toml`
- **Clippy warnings**: Follow suggestions; most are idiomatic Rust improvements
- **Test failures**: Check test output frame-by-frame; log positions/velocities for physics issues
- **Unexpected physics**: Verify system execution order (see ECS Systems section); check constant values match source code
