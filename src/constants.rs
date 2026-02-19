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
pub const SIM_WIDTH: f32 = 3000.0;

/// Height of the initial asteroid spawn region (world units).
pub const SIM_HEIGHT: f32 = 2000.0;

/// Margin kept clear between the spawn region edge and the outer simulation boundary.
pub const SPAWN_GRID_MARGIN: f32 = 150.0;

/// Radius around the player start position (origin) that is kept free of asteroids.
///
/// Increase to give the player more breathing room at startup.
/// Decrease to make encounters start faster.
pub const PLAYER_BUFFER_RADIUS: f32 = 400.0;

// ── Physics: Gravity ──────────────────────────────────────────────────────────

/// Inverse-square gravity strength constant.
///
/// Higher values → stronger mutual attraction → faster cluster formation.
/// Tested range: 5.0–15.0.  At 10.0 two asteroids 100 u apart collide in ~350 frames.
/// Values above ~20.0 cause runaway acceleration at close range.
pub const GRAVITY_CONST: f32 = 10.0;

/// Asteroids closer than this distance are excluded from gravity calculations.
///
/// When two asteroids are within `MIN_GRAVITY_DIST` units Rapier handles the
/// contact physics; injecting additional gravity forces at this range creates
/// spurious energy that destabilises high-speed passes.
/// Tested range: 3.0–10.0.
pub const MIN_GRAVITY_DIST: f32 = 5.0;

/// Maximum distance at which gravity is applied between two asteroids.
///
/// Set equal to `CULL_DISTANCE` so that culled asteroids exert no phantom forces.
/// Decreasing this value can improve performance at large asteroid counts.
pub const MAX_GRAVITY_DIST: f32 = 1000.0;

// ── Physics: Cluster Formation ────────────────────────────────────────────────

/// Maximum linear speed (u/s) at which a touching asteroid qualifies for
/// velocity synchronisation via `particle_locking_system`.
///
/// Lower values → only nearly-stationary asteroids are synced.
/// Higher values → more aggressive locking, may cause visible velocity jumps.
pub const VELOCITY_THRESHOLD_LOCKING: f32 = 5.0;

/// Maximum linear speed (u/s) at which a touching asteroid can join a merge cluster
/// in `asteroid_formation_system`.
///
/// Slightly higher than `VELOCITY_THRESHOLD_LOCKING` so that asteroids are first
/// velocity-synced and then eligible to merge in the next pass.
pub const VELOCITY_THRESHOLD_FORMATION: f32 = 10.0;

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

/// Distance from the world origin beyond which asteroids are permanently removed.
///
/// Matches `MAX_GRAVITY_DIST` so that nothing exerts gravity after being culled.
pub const CULL_DISTANCE: f32 = 1000.0;

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

/// Minimum camera zoom scale (zoom *out*).  At 0.5 the full cull circle fits comfortably.
pub const MIN_ZOOM: f32 = 0.5;

/// Maximum camera zoom scale (zoom *in*).  At 8.0 only ~150×100 world units are visible.
pub const MAX_ZOOM: f32 = 8.0;

/// Zoom scale change per one scroll-wheel event.
pub const ZOOM_SPEED: f32 = 0.1;

// ── Player: Movement ─────────────────────────────────────────────────────────

/// Forward thrust force (N) applied while W is held.
///
/// Increase for snappier acceleration; decrease for a floatier feel.
/// Tested range: 60.0–300.0.
pub const THRUST_FORCE: f32 = 120.0;

/// Reverse thrust force (N) applied while S / gamepad-B is held.
/// Intentionally weaker than `THRUST_FORCE`.
pub const REVERSE_FORCE: f32 = 60.0;

/// Fixed angular velocity (rad/s) applied while A / D are held.
pub const ROTATION_SPEED: f32 = 3.0;

/// Linear damping applied to the player ship by Rapier on every physics step.
/// Simulates inertial resistance in space (purely aesthetic — real space has none).
pub const PLAYER_LINEAR_DAMPING: f32 = 0.5;

/// Angular damping applied to the player ship.
pub const PLAYER_ANGULAR_DAMPING: f32 = 3.0;

/// Radius (u) of the player ship's ball collider.
pub const PLAYER_COLLIDER_RADIUS: f32 = 8.0;

/// Restitution coefficient for the player ship.
pub const PLAYER_RESTITUTION: f32 = 0.3;

// ── Player: Out-of-Bounds ─────────────────────────────────────────────────────

/// Distance from origin beyond which the player experiences extra velocity damping.
/// Matches `CULL_DISTANCE` so the soft boundary coincides with the asteroid hard boundary.
pub const OOB_RADIUS: f32 = 1000.0;

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
pub const PROJECTILE_MAX_DIST: f32 = 1000.0;

/// Radius (u) of projectile ball collider.
pub const PROJECTILE_COLLIDER_RADIUS: f32 = 2.0;

// ── Player: Health ────────────────────────────────────────────────────────────

/// Player ship starting and maximum HP.
pub const PLAYER_MAX_HP: f32 = 100.0;

/// Relative speed threshold (u/s) below which asteroid impacts deal zero damage.
/// Slow grazes are harmless; only high-velocity impacts hurt.
pub const DAMAGE_SPEED_THRESHOLD: f32 = 30.0;

/// Seconds of invincibility granted immediately after taking damage.
/// Prevents rapid-fire damage from a sustained graze contact.
pub const INVINCIBILITY_DURATION: f32 = 0.5;

// ── Gamepad ───────────────────────────────────────────────────────────────────

/// Left-stick dead zone: inputs smaller than this fraction are ignored.
pub const GAMEPAD_LEFT_DEADZONE: f32 = 0.15;

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

/// Minimum distance between two vertex points before they are considered duplicates
/// during convex hull deduplication.  Prevents degenerate Rapier colliders.
pub const HULL_DEDUP_MIN_DIST: f32 = 0.5;

/// Minimum side length for spawned equilateral triangle shape (at scale 1.0).
pub const TRIANGLE_BASE_SIDE: f32 = 6.0;

/// Half-extent for spawned square shape (at scale 1.0).
pub const SQUARE_BASE_HALF: f32 = 4.0;

/// Circumradius for spawned pentagon and hexagon shapes (at scale 1.0).
pub const POLYGON_BASE_RADIUS: f32 = 5.0;

/// Asteroid size scale range: minimum multiplier applied to base geometry.
pub const ASTEROID_SIZE_SCALE_MIN: f32 = 0.5;

/// Asteroid size scale range: maximum multiplier applied to base geometry.
pub const ASTEROID_SIZE_SCALE_MAX: f32 = 1.5;

/// Initial velocity range (u/s) assigned to each spawned asteroid axis component.
pub const ASTEROID_INITIAL_VELOCITY_RANGE: f32 = 15.0;

/// Initial angular velocity range (rad/s) assigned to each spawned asteroid.
pub const ASTEROID_INITIAL_ANGVEL_RANGE: f32 = 5.0;
