# Project Guidelines

## Code Style

- **Language**: Rust
- **Formatting**: Use `rustfmt` for all code formatting. Format on save and before commits. Configuration in `rustfmt.toml` if needed.
- **Linting**: Use `clippy` with `cargo clippy -- -D warnings` to enforce all warnings as errors. Follow clippy suggestions for idiomatic Rust patterns.

## Architecture

The "grav-sim" project is an ECS-based **asteroid simulation engine** built on **Bevy** with physics powered by **Rapier2D**. All objects in the simulation are asteroids that naturally aggregate through N-body gravity into larger composite polygonal structures.

- **Purpose**: Pure asteroid-based simulation where asteroids naturally aggregate through gravitational attraction and collision to form larger composite asteroids (polygons) that visually rotate based on physics.
- **Framework**: 
  - **Bevy 0.14**: Game engine providing ECS architecture, rendering, and event handling
  - **Rapier2D 0.18**: Physics engine for collision detection, rigid body dynamics, and impulse-based response
- **Core Modules**:
  - `asteroid.rs` - Unified asteroid components and spawn functions; convex hull computation
  - `simulation.rs` - Physics systems: N-body gravity, cluster detection, composite formation
  - `graphics.rs` - Camera setup for 2D rendering
  - `testing.rs` - Automated test scenarios for physics validation
  - `main.rs` - Bevy app setup, window configuration, and test mode routing
- **Entity Types**: All asteroids are unified entities with local-space vertices
  - Spawn as triangles or polygons depending on configuration
  - Composite asteroids formed when 2+ asteroids touch and merge
- **ECS Systems** (Execution Order - CRITICAL):
  - **Update Schedule**:
    1. **Culling** - Removes asteroids beyond 1000 units
    2. **Neighbor counting** - Counts nearby asteroids
    3. **N-body gravity** - Applies mutual attraction
    4. **Settling damping** - Applies friction to slow asteroids
    5. **Particle locking** - Velocity synchronization for slow asteroids
    6. **Environmental damping** - Stabilizes dense clusters
    7. **User input** - Left-click spawns asteroids
    8. **Gizmo rendering** - Renders wireframe outlines
  - **PostUpdate Schedule**:
    9. **Asteroid formation** - MUST run AFTER Rapier physics (FixedUpdate) populates contacts
    10. **Test logging & verification** - MUST run after formation to see merged results

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
- **Constant**: 2.0 (gentle mutual attraction to avoid velocity blowup)
- **Minimum distance threshold**: 2.0 units (clamps distance-squared to prevent singularities)
- **Maximum gravity distance**: 300.0 units (prevents phantom forces from distant asteroids)
- **Application**: Applied uniformly between all asteroid pairs within range
- **Observed Behavior**: Asteroids at 100 units apart attract over ~350 frames, reach velocities ~28+ m/s, then collide and merge

### Velocity Synchronization
- **Activation**: When two asteroids touch and both move < 5.0 u/s
- **Effect**: Velocities averaged between them (linear and angular)
- **Purpose**: Prepares asteroids for smooth composite formation

### Cluster Formation & Merging (CRITICAL IMPLEMENTATION)
- **Detection**: Flood-fill algorithm through Rapier contact manifolds
- **Contact query**: MUST run in PostUpdate after Rapier FixedUpdate populates contacts
- **Velocity threshold**: 10.0 u/s (allows faster asteroids to merge if in contact)
- **Hull computation**: 
  - Collect ALL vertices from cluster members in WORLD-SPACE
  - Apply transform rotation to local vertices: `world_v = offset + rotation.mul_vec3(local_v.extend(0.0)).truncate()`
  - Compute convex hull from complete world-space vertex set
  - Convert hull back to LOCAL-SPACE: `hull_local = hull.iter().map(|v| v - center).collect()`
  - Spawn composite with local-space hull for correct rendering
- **Velocity inheritance**: Average linear and angular velocity from cluster members
- **Prevention**: Processed asteroids tracked per-frame to prevent duplicate merging

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

## Current Implementation Status
- ✅ Pure asteroid-only unified system (all entities equal)
- ✅ Cluster-based formation with flood-fill contact detection
- ✅ Wireframe rendering with rotation (no sprite overlays)
- ✅ Velocity inheritance and composite stability
- ✅ Convex hull composition from all constituent vertices
- ✅ Gravity attraction → collision → merging pipeline
- ✅ Automated test framework with environment variable triggering
- ✅ PostUpdate system scheduling for physics-aware logic

## Testing Strategy (Session-Learned Best Practices)

### Test Framework
- **Environment Variable**: `GRAV_SIM_TEST=<test_name>` triggers test mode from `main.rs`
- **Available Tests**:
  - `two_triangles`: Verifies 2 touching asteroids merge into 1 composite
  - `three_triangles`: Verifies 3-asteroid cluster merges into 1 composite
  - `gravity`: Verifies distant asteroids attract, collide, and merge over time
- **Test Config Resource**: Tracks frame count, asteroid counts, test name for automated verification

### Test Logging Strategy
- Log at key frames: 1, 10, 30, 50, 100, 150, 200, 250, 300, etc.
- Log both **positions** and **velocities** to understand physics behavior
- Example output: `[Frame 300] pos: (-11.3, 0.0), vel_len: 28.565` for gravity test
- This reveals whether asteroids are attracting (velocity increasing, distance decreasing) or repelling

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
- **Tested range**: 2.0 works well; too high (15.0+) causes instability
- **Observable metrics**:
  - At 100 units separation: ~350 frames to collision
  - Velocity reaches ~28-30 m/s at collision
  - Collided asteroids merge immediately into stable composite
- **Indicates system is healthy**: Smooth acceleration, no bouncing apart after contact

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
- **Example format for changes**: "Updated gravity constant from 2.0 to 10.0 in `nbody_gravity_system` (documented in ARCHITECTURE.md)"

### PhysicsConstants Updates
When tuning physics constants:
1. Update the constant value in the source code (e.g., `src/simulation.rs`)
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
- **Build errors**: Check Rust edition (2021), Bevy version (0.13), dependencies in Cargo.toml
- **Clippy warnings**: Follow suggestions; most are idiomatic Rust improvements
- **Test failures**: Check test output frame-by-frame; log positions/velocities for physics issues
- **Unexpected physics**: Verify system execution order (see ECS Systems section); check constant values match source code
