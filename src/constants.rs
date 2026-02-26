//! Centralised physics and gameplay constants.
//!
//! All tuneable values live here so they can be found, reasoned-about, and
//! modified in one place without source-diving across multiple modules.
//!
//! ## Tuning guidance
//!
//! Each constant includes the tested range and the observable consequence of
//! changing it.  After editing, run `./test_all.sh` to confirm physics
//! behaviour has not regressed.

// ── World Bounds ──────────────────────────────────────────────────────────────

/// Width of the initial asteroid spawn region (world units).
///
/// Asteroids are distributed within ±SIM_WIDTH/2 of the origin.
/// Increasing this spreads the initial field; decreasing creates denser opening clusters.
pub const SIM_WIDTH: f32 = 4000.0;

/// Height of the initial asteroid spawn region (world units).
pub const SIM_HEIGHT: f32 = 4000.0;

/// Margin kept clear between the spawn region edge and the outer simulation boundary.
pub const SPAWN_GRID_MARGIN: f32 = 150.0;

/// Radius around the player start position (origin) that is kept free of asteroids.
///
/// Increase to give the player more breathing room at startup.
/// Decrease to make encounters start faster.
pub const PLAYER_BUFFER_RADIUS: f32 = 100.0;

// ── Physics: Gravity ──────────────────────────────────────────────────────────

/// Inverse-square gravity strength constant.
///
/// Higher values → stronger mutual attraction → faster cluster formation.
/// Tested range: 5.0–15.0.  At 10.0 two asteroids 100 u apart collide in ~350 frames.
/// Values above ~20.0 cause runaway acceleration at close range.
/// Reset to 10.0 now that gravity is mass-scaled (F = G·m_i·m_j/r²); the mass
/// factor provides sufficient attraction without needing an inflated constant.
pub const GRAVITY_CONST: f32 = 10.0;

/// Asteroids closer than this distance are excluded from gravity calculations.
///
/// When two asteroids are within `MIN_GRAVITY_DIST` units Rapier handles the
/// contact physics; injecting additional gravity forces at this range creates
/// spurious energy that destabilises high-speed passes.
/// Tested range: 3.0–10.0.
pub const MIN_GRAVITY_DIST: f32 = 5.0;

/// Maximum distance at which gravity is applied between two asteroids.
/// Decreasing this value can improve performance at large asteroid counts.
pub const MAX_GRAVITY_DIST: f32 = 1000.0;

// ── Physics: Cluster Formation ────────────────────────────────────────────────

/// Maximum linear speed (u/s) at which a touching asteroid qualifies for
/// velocity synchronisation via `particle_locking_system`.
///
/// Lower values → only nearly-stationary asteroids are synced.
/// Higher values → more aggressive locking, may cause visible velocity jumps.
/// Note: cluster formation is now gated by gravitational binding energy rather
/// than a velocity threshold — this constant only governs velocity sync, not merging.
pub const VELOCITY_THRESHOLD_LOCKING: f32 = 5.0;

/// Maximum per-member hull extent (u) relative to cluster size beyond which a
/// proposed merged hull is rejected as corrupted data.
///
/// `allowed_extent = HULL_EXTENT_BASE + cluster_members × HULL_EXTENT_PER_MEMBER`
pub const HULL_EXTENT_BASE: f32 = 60.0;
pub const HULL_EXTENT_PER_MEMBER: f32 = 20.0;

// ── Physics: Collision ────────────────────────────────────────────────────────

/// Restitution coefficient for small (unit) asteroids.
/// 0.0 = perfectly inelastic; 1.0 = perfectly elastic.
pub const RESTITUTION_SMALL: f32 = 0.0;

/// Friction coefficient applied to asteroid–asteroid contacts.
pub const FRICTION_ASTEROID: f32 = 1.0;

// ── Physics: Culling ──────────────────────────────────────────────────────────

/// Inner radius of the soft boundary zone (world units).
///
/// Asteroids that drift beyond this distance feel a gentle inward spring force
/// nudging them back toward the simulation centre.  Set to 90 % of the gravity
/// horizon so the spring activates well before `HARD_CULL_DISTANCE`.
pub const SOFT_BOUNDARY_RADIUS: f32 = 1800.0;

/// Spring constant for the soft boundary restoring force.
///
/// Force per world-unit displacement past `SOFT_BOUNDARY_RADIUS` in the inward
/// direction.  At 2.0 an asteroid 200 u past the inner edge receives 400 u of
/// restoring acceleration per unit mass — enough to turn it around in a few
/// hundred frames without an abrupt bounce.
pub const SOFT_BOUNDARY_STRENGTH: f32 = 2.0;

/// Absolute hard-cull distance: asteroids beyond this are forcibly removed.
///
/// Acts as a safety net if the soft spring is insufficient (e.g., a very fast
/// projectile).  Set comfortably outside `SOFT_BOUNDARY_RADIUS` so normal
/// simulation objects almost never reach it.
pub const HARD_CULL_DISTANCE: f32 = 2500.0;

/// Distance from the world origin beyond which asteroids are permanently removed.
/// Now acts as the stats / stats-display reference boundary; hard removal happens
/// at `HARD_CULL_DISTANCE`.
pub const CULL_DISTANCE: f32 = 2000.0;

// ── Physics: Neighbor Counting ────────────────────────────────────────────────

/// Radius (u) used by `neighbor_counting_system` to decide whether two asteroids
/// are "close neighbours".  Currently informational; previously drove damping logic.
pub const NEIGHBOR_THRESHOLD: f32 = 3.0;

// ── Spatial Grid ─────────────────────────────────────────────────────────────

/// World-space size of each spatial grid cell.
///
/// Must be ≥ the largest query radius / 2 or the cell-check count explodes.
/// At 500 u, a 1000-u gravity query checks only 5×5 = 25 cells.
/// Using 100 u would require 21×21 = 441 cells, worse than O(N²) at low counts.
pub const GRID_CELL_SIZE: f32 = 500.0;

// ── Camera ───────────────────────────────────────────────────────────────────

/// Minimum camera zoom scale (zoom *out*).  Allows the full `CULL_DISTANCE` circle to fit comfortably.
pub const MIN_ZOOM: f32 = 0.5;

/// Maximum camera zoom scale (zoom *in*).  At 4.0 only ~150×100 world units are visible.
pub const MAX_ZOOM: f32 = 4.0;

/// Zoom scale change per one scroll-wheel event.
pub const ZOOM_SPEED: f32 = 0.1;

// ── Player: Movement ─────────────────────────────────────────────────────────

/// Forward thrust force (N) applied while W is held.
///
/// Player ball collider (radius=8, density=1) has mass ≈ 201 kg.
/// At 25 000 N: acceleration ≈ 125 px/s²; terminal velocity ≈ 1 250 px/s
/// (limited by linear_damping=0.1).  Old value of 60 N gave 0.3 px/s² —
/// imperceptibly slow and indistinguishable from "not working".
pub const THRUST_FORCE: f32 = 25_000.0;

/// Reverse thrust force (N) applied while S / gamepad-B is held.
/// Intentionally weaker than `THRUST_FORCE`.
pub const REVERSE_FORCE: f32 = 12_500.0;

/// Fixed angular velocity (rad/s) applied while A / D are held.
pub const ROTATION_SPEED: f32 = 3.0;

/// Linear damping applied to the player ship by Rapier on every physics step.
/// Simulates inertial resistance in space (purely aesthetic — real space has none).
pub const PLAYER_LINEAR_DAMPING: f32 = 0.1;

/// Angular damping applied to the player ship.
pub const PLAYER_ANGULAR_DAMPING: f32 = 10.0;

/// Radius (u) of the player ship's ball collider.
pub const PLAYER_COLLIDER_RADIUS: f32 = 8.0;

/// Restitution coefficient for the player ship.
pub const PLAYER_RESTITUTION: f32 = 0.3;

// ── Player: Out-of-Bounds ─────────────────────────────────────────────────────

/// Distance from origin beyond which the player experiences extra velocity damping.
/// Matches `CULL_DISTANCE` so the soft boundary coincides with the asteroid hard boundary.
pub const OOB_RADIUS: f32 = 2000.0;

/// Minimum velocity decay factor applied per frame at maximum out-of-bounds depth.
/// Range: (0.0, 1.0) — values closer to 1.0 are gentler; closer to 0.0 are harsher.
pub const OOB_DAMPING: f32 = 0.97;

/// Width of the OOB ramp zone (u).  Damping increases linearly from 0% at `OOB_RADIUS`
/// to `(1.0 - OOB_DAMPING) × 100%` at `OOB_RADIUS + OOB_RAMP_WIDTH`.
pub const OOB_RAMP_WIDTH: f32 = 200.0;

// ── Player: Combat ────────────────────────────────────────────────────────────

/// Speed (u/s) of fired projectiles.
pub const PROJECTILE_SPEED: f32 = 500.0;

/// Minimum seconds between consecutive shots.
pub const FIRE_COOLDOWN: f32 = 0.2;

/// Seconds after which a projectile is automatically despawned.
pub const PROJECTILE_LIFETIME: f32 = 3.0;

/// Distance from origin beyond which a live projectile is despawned.
/// Kept at `CULL_DISTANCE` so projectiles don't outlive the asteroids they can hit.
pub const PROJECTILE_MAX_DIST: f32 = 2000.0;

/// Radius (u) of projectile ball collider.
pub const PROJECTILE_COLLIDER_RADIUS: f32 = 2.0;

// ── Player: Missiles ─────────────────────────────────────────────────────────

/// Maximum number of missiles the player can carry.
pub const MISSILE_AMMO_MAX: u32 = 5;

/// Speed (u/s) of fired missiles — slower than bullets, heavier impact.
pub const MISSILE_SPEED: f32 = 380.0;

/// Minimum seconds between consecutive missile shots.
pub const MISSILE_COOLDOWN: f32 = 0.5;

/// Seconds after which a missile is automatically despawned.
pub const MISSILE_LIFETIME: f32 = 4.0;

/// Distance from origin beyond which a live missile is despawned.
pub const MISSILE_MAX_DIST: f32 = 2000.0;

/// Radius (u) of missile ball collider — larger than a bullet.
pub const MISSILE_COLLIDER_RADIUS: f32 = 5.0;

/// Seconds for one missile to recharge automatically.
pub const MISSILE_RECHARGE_SECS: f32 = 12.0;

// ── Player: Health ────────────────────────────────────────────────────────────

/// Player ship starting and maximum HP.
pub const PLAYER_MAX_HP: f32 = 100.0;

/// Relative speed threshold (u/s) below which asteroid impacts deal zero damage.
/// Slow grazes are harmless; only high-velocity impacts hurt.
pub const DAMAGE_SPEED_THRESHOLD: f32 = 30.0;

/// Seconds of invincibility granted immediately after taking damage.
/// Prevents rapid-fire damage from a sustained graze contact.
pub const INVINCIBILITY_DURATION: f32 = 0.5;

/// Number of lives the player starts with.  Decrements on each death;
/// the game ends when it reaches zero.
pub const PLAYER_LIVES: i32 = 3;

/// Seconds to wait after death before the ship re-materialises.
pub const RESPAWN_DELAY_SECS: f32 = 2.5;

/// Seconds of invincibility granted on respawn — longer than a normal hit so
/// the player has time to orientate before taking damage again.
pub const RESPAWN_INVINCIBILITY_SECS: f32 = 4.0;

/// Seconds without any damage before passive HP regeneration begins.
pub const PASSIVE_HEAL_DELAY_SECS: f32 = 6.0;

/// HP regenerated per second once the passive heal delay has elapsed.
///
/// NOTE: Passive healing is no longer used in the default game — ore-based
/// healing replaces it.  This constant is retained so the PhysicsConfig field
/// keeps a meaningful default should passive healing be re-enabled via TOML.
pub const PASSIVE_HEAL_RATE: f32 = 6.0;

/// HP restored when the player spends one ore unit on healing (`H` key).
///
/// Chosen as a meaningful chunk that rewards ore collection without trivialising
/// combat: at 30 HP per ore a player with 5 ore can fully restore from near-zero.
pub const ORE_HEAL_AMOUNT: f32 = 30.0;

// ── Ore Magnet ────────────────────────────────────────────────────────────────

/// Base magnet pull radius at level 0 (world units).
/// Upgrades add +50 u per level (→ 700 u at level 9).
///
/// Intentionally small at base level to incentivize upgrades; ore collection
/// range feels limited until invested into.
pub const ORE_MAGNET_BASE_RADIUS: f32 = 250.0;

/// Base magnet pull strength at level 0 (velocity magnitude, u/s).
/// Upgrades add +16 u/s per level (→ 184 u/s at level 9).
///
/// Reduced from 120 → 40 u/s to make upgrades feel impactful.  Level 0 collection
/// is intentionally slow; by level 5–6 it's fast enough for comfortable play.
pub const ORE_MAGNET_BASE_STRENGTH: f32 = 40.0;

/// Maximum level the ore magnet can be upgraded to (1-indexed display; 0 = base).
///
/// At level N the magnet pulls ore from (250 + N × 50) u at (40 + N × 16) u/s.
pub const ORE_AFFINITY_MAX_LEVEL: u32 = 10;

/// Ore cost for the next magnet upgrade = `ORE_AFFINITY_UPGRADE_BASE_COST * next_level`.
///
/// Level 1 costs 5, Level 2 costs 10, …, Level 10 costs 50.
/// Total to max-level: 5 + 10 + … + 50 = 275 ore.
pub const ORE_AFFINITY_UPGRADE_BASE_COST: u32 = 5;

// ── Gamepad ───────────────────────────────────────────────────────────────────

/// Left-stick dead zone: inputs smaller than this fraction are ignored.
pub const GAMEPAD_LEFT_DEADZONE: f32 = 0.15;

/// Velocity decay factor applied to the player's linvel / angvel every frame
/// while the B (East) gamepad button is held.  Acts as an active brake.
/// Range: (0.0, 1.0) — 0.82 removes ≈18% of speed per frame (~60 fps), stopping
/// from full speed in roughly half a second.
pub const GAMEPAD_BRAKE_DAMPING: f32 = 0.82;

/// Seconds of inactivity (no mouse movement, left stick, or right stick) before
/// the aim direction is automatically snapped back to the ship's forward direction.
pub const AIM_IDLE_SNAP_SECS: f32 = 1.0;

/// Right-stick dead zone for aim updates.
pub const GAMEPAD_RIGHT_DEADZONE: f32 = 0.2;

/// Right-stick minimum deflection fraction required to trigger auto-fire.
pub const GAMEPAD_FIRE_THRESHOLD: f32 = 0.5;

/// Minimum angular error (rad) before the ship actively corrects its heading
/// toward the gamepad left-stick direction.
pub const GAMEPAD_HEADING_SNAP_THRESHOLD: f32 = 0.05;

// ── Rendering ─────────────────────────────────────────────────────────────────

/// Live asteroid count above which force-vector rendering is skipped.
/// Keeps CPU gizmo overhead manageable at high densities.
pub const FORCE_VECTOR_HIDE_THRESHOLD: u32 = 200;

/// Scale factor applied to force vectors when drawing them as gizmo lines.
/// Larger values make small forces more visible.
pub const FORCE_VECTOR_DISPLAY_SCALE: f32 = 80.0;

/// Minimum world-space force magnitude to bother drawing a force vector gizmo.
pub const FORCE_VECTOR_MIN_LENGTH: f32 = 0.1;

/// Font size for the on-screen statistics overlay.
pub const STATS_FONT_SIZE: f32 = 20.0;

// ── Asteroid Geometry ─────────────────────────────────────────────────────────

/// Density of asteroid material (mass units per world-unit²).
///
/// Used to establish a predictable relationship between an asteroid's `AsteroidSize`
/// (gravitational mass in unit-triangle equivalents) and its visual area on screen:
///
/// ```text
/// target_area = asteroid_size / ASTEROID_DENSITY
/// ```
///
/// A lower density → larger visual size for the same mass.
/// A higher density → smaller visual size for the same mass.
///
/// Calibrated so that a single unit triangle (`AsteroidSize = 1`) has roughly the
/// same area as a triangle with `base_side ≈ 4 u` at scale 1.0 (≈ 7 u²),
/// meaning `density ≈ 1 / 10 = 0.1`.
pub const ASTEROID_DENSITY: f32 = 0.1;

/// Minimum distance between two vertex points before they are considered duplicates
/// during convex hull deduplication.  Prevents degenerate Rapier colliders.
pub const HULL_DEDUP_MIN_DIST: f32 = 0.5;

/// Minimum side length for spawned equilateral triangle shape (at scale 1.0).
pub const TRIANGLE_BASE_SIDE: f32 = 8.0;

/// Half-extent for spawned square shape (at scale 1.0).
pub const SQUARE_BASE_HALF: f32 = 6.0;

/// Circumradius for spawned pentagon and hexagon shapes (at scale 1.0).
pub const POLYGON_BASE_RADIUS: f32 = 7.0;

/// Circumradius for spawned heptagon (7-sided) shapes (at scale 1.0).
pub const HEPTAGON_BASE_RADIUS: f32 = 8.5;

/// Circumradius for spawned octagon (8-sided) shapes (at scale 1.0).
pub const OCTAGON_BASE_RADIUS: f32 = 10.0;

/// Circumradius for the large planetoid asteroid (at scale 1.0).
/// Planetoids are 16-sided near-circles and participate in full N-body physics.
pub const PLANETOID_BASE_RADIUS: f32 = 25.0;

/// Unit-size count assigned to a spawned planetoid.
/// Reflects the planetoid's large mass relative to small asteroids.
pub const PLANETOID_UNIT_SIZE: u32 = 16;

/// Asteroid size scale range: minimum multiplier applied to base geometry.
pub const ASTEROID_SIZE_SCALE_MIN: f32 = 0.5;

/// Asteroid size scale range: maximum multiplier applied to base geometry.
/// Increased to 2.5 to allow noticeably large individual asteroids.
pub const ASTEROID_SIZE_SCALE_MAX: f32 = 2.5;

/// Initial velocity range (u/s) assigned to each spawned asteroid axis component.
pub const ASTEROID_INITIAL_VELOCITY_RANGE: f32 = 15.0;

/// Initial angular velocity range (rad/s) assigned to each spawned asteroid.
pub const ASTEROID_INITIAL_ANGVEL_RANGE: f32 = 5.0;

// ── Primary Weapon Upgrades ───────────────────────────────────────────────────

/// Maximum level the primary weapon can be upgraded to (1-indexed display; 0 = base).
///
/// At level N the weapon fully destroys asteroids of size ≤ N.
/// Sizes above N are always chipped (1 vertex removed, 1-unit fragment spawned).
pub const PRIMARY_WEAPON_MAX_LEVEL: u32 = 10;

/// Ore cost for the next upgrade = `WEAPON_UPGRADE_BASE_COST * next_level`.
///
/// Level 1 costs 5, Level 2 costs 10, …, Level 10 costs 50.
/// Total to max-level: 5 + 10 + … + 50 = 275 ore.
pub const WEAPON_UPGRADE_BASE_COST: u32 = 5;

// ── Secondary Weapon (Missile) Upgrades ────────────────────────────────────────

/// Maximum level the secondary weapon (missile) can be upgraded to (1-indexed display; 0 = base).
///
/// At level N the missile destroys asteroids of size ≤ (2 + N) and chips (2 + N) units
/// for larger asteroids, each chipped unit becoming a size-1 fragment.
pub const SECONDARY_WEAPON_MAX_LEVEL: u32 = 10;

/// Ore cost for the next missile upgrade = `SECONDARY_WEAPON_UPGRADE_BASE_COST * next_level`.
///
/// Level 1 costs 5, Level 2 costs 10, …, Level 10 costs 50.
/// Total to max-level: 5 + 10 + … + 50 = 275 ore.
pub const SECONDARY_WEAPON_UPGRADE_BASE_COST: u32 = 5;
