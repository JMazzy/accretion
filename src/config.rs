//! Runtime physics configuration loaded from `assets/physics.toml`.
//!
//! [`PhysicsConfig`] is a Bevy [`Resource`] that mirrors every constant in
//! [`crate::constants`].  At startup, [`load_physics_config`] reads
//! `assets/physics.toml` and overwrites the defaults with any values present in
//! the file.  Missing keys fall back to the compile-time defaults, so a minimal
//! TOML can override just the constants you care about.
//!
//! ## Usage in systems
//!
//! Add `config: Res<PhysicsConfig>` to any system parameter list and read values
//! with `config.gravity_const`, `config.cull_distance`, etc.
//!
//! ## Tuning workflow
//!
//! 1. Edit `assets/physics.toml`.
//! 2. Restart the simulation — no recompilation required.
//! 3. Run `./test_all.sh` to validate the new values.
//!
//! Keep `src/constants.rs` in sync: it remains the **authoritative default**
//! source used by `PhysicsConfig::default()`.

use crate::constants::*;
use bevy::prelude::*;
use serde::Deserialize;

/// Runtime-tunable physics and gameplay configuration.
///
/// All fields default to the corresponding compile-time constant from
/// `src/constants.rs`.  Override any subset by setting the value in
/// `assets/physics.toml`.
#[derive(Resource, Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PhysicsConfig {
    // ── World Bounds ─────────────────────────────────────────────────────────
    pub sim_width: f32,
    pub sim_height: f32,
    pub spawn_grid_margin: f32,
    pub player_buffer_radius: f32,

    // ── Physics: Gravity ──────────────────────────────────────────────────────
    pub gravity_const: f32,
    pub min_gravity_dist: f32,
    pub max_gravity_dist: f32,

    // ── Physics: Cluster Formation ────────────────────────────────────────────
    pub velocity_threshold_locking: f32,
    pub velocity_threshold_formation: f32,
    pub hull_extent_base: f32,
    pub hull_extent_per_member: f32,

    // ── Physics: Collision ────────────────────────────────────────────────────
    pub restitution_small: f32,
    pub friction_asteroid: f32,

    // ── Physics: Culling ──────────────────────────────────────────────────────
    pub cull_distance: f32,

    // ── Physics: Neighbor Counting ────────────────────────────────────────────
    pub neighbor_threshold: f32,

    // ── Spatial Grid ──────────────────────────────────────────────────────────
    pub grid_cell_size: f32,

    // ── Camera ────────────────────────────────────────────────────────────────
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub zoom_speed: f32,

    // ── Player: Movement ──────────────────────────────────────────────────────
    pub thrust_force: f32,
    pub reverse_force: f32,
    pub rotation_speed: f32,
    pub player_linear_damping: f32,
    pub player_angular_damping: f32,
    pub player_collider_radius: f32,
    pub player_restitution: f32,

    // ── Player: Out-of-Bounds ─────────────────────────────────────────────────
    pub oob_radius: f32,
    pub oob_damping: f32,
    pub oob_ramp_width: f32,

    // ── Player: Combat ────────────────────────────────────────────────────────
    pub projectile_speed: f32,
    pub fire_cooldown: f32,
    pub projectile_lifetime: f32,
    pub projectile_max_dist: f32,
    pub projectile_collider_radius: f32,

    // ── Player: Health ────────────────────────────────────────────────────────
    pub player_max_hp: f32,
    pub damage_speed_threshold: f32,
    pub invincibility_duration: f32,

    // ── Gamepad ───────────────────────────────────────────────────────────────
    pub gamepad_left_deadzone: f32,
    pub gamepad_brake_damping: f32,
    pub aim_idle_snap_secs: f32,
    pub gamepad_right_deadzone: f32,
    pub gamepad_fire_threshold: f32,
    pub gamepad_heading_snap_threshold: f32,

    // ── Rendering ─────────────────────────────────────────────────────────────
    pub force_vector_hide_threshold: u32,
    pub force_vector_display_scale: f32,
    pub force_vector_min_length: f32,
    pub stats_font_size: f32,

    // ── Asteroid Geometry ─────────────────────────────────────────────────────
    pub hull_dedup_min_dist: f32,
    pub triangle_base_side: f32,
    pub square_base_half: f32,
    pub polygon_base_radius: f32,
    pub asteroid_size_scale_min: f32,
    pub asteroid_size_scale_max: f32,
    pub asteroid_initial_velocity_range: f32,
    pub asteroid_initial_angvel_range: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            // World Bounds
            sim_width: SIM_WIDTH,
            sim_height: SIM_HEIGHT,
            spawn_grid_margin: SPAWN_GRID_MARGIN,
            player_buffer_radius: PLAYER_BUFFER_RADIUS,
            // Gravity
            gravity_const: GRAVITY_CONST,
            min_gravity_dist: MIN_GRAVITY_DIST,
            max_gravity_dist: MAX_GRAVITY_DIST,
            // Cluster Formation
            velocity_threshold_locking: VELOCITY_THRESHOLD_LOCKING,
            velocity_threshold_formation: VELOCITY_THRESHOLD_FORMATION,
            hull_extent_base: HULL_EXTENT_BASE,
            hull_extent_per_member: HULL_EXTENT_PER_MEMBER,
            // Collision
            restitution_small: RESTITUTION_SMALL,
            friction_asteroid: FRICTION_ASTEROID,
            // Culling
            cull_distance: CULL_DISTANCE,
            // Neighbor Counting
            neighbor_threshold: NEIGHBOR_THRESHOLD,
            // Spatial Grid
            grid_cell_size: GRID_CELL_SIZE,
            // Camera
            min_zoom: MIN_ZOOM,
            max_zoom: MAX_ZOOM,
            zoom_speed: ZOOM_SPEED,
            // Player: Movement
            thrust_force: THRUST_FORCE,
            reverse_force: REVERSE_FORCE,
            rotation_speed: ROTATION_SPEED,
            player_linear_damping: PLAYER_LINEAR_DAMPING,
            player_angular_damping: PLAYER_ANGULAR_DAMPING,
            player_collider_radius: PLAYER_COLLIDER_RADIUS,
            player_restitution: PLAYER_RESTITUTION,
            // Player: Out-of-Bounds
            oob_radius: OOB_RADIUS,
            oob_damping: OOB_DAMPING,
            oob_ramp_width: OOB_RAMP_WIDTH,
            // Player: Combat
            projectile_speed: PROJECTILE_SPEED,
            fire_cooldown: FIRE_COOLDOWN,
            projectile_lifetime: PROJECTILE_LIFETIME,
            projectile_max_dist: PROJECTILE_MAX_DIST,
            projectile_collider_radius: PROJECTILE_COLLIDER_RADIUS,
            // Player: Health
            player_max_hp: PLAYER_MAX_HP,
            damage_speed_threshold: DAMAGE_SPEED_THRESHOLD,
            invincibility_duration: INVINCIBILITY_DURATION,
            // Gamepad
            gamepad_left_deadzone: GAMEPAD_LEFT_DEADZONE,
            gamepad_brake_damping: GAMEPAD_BRAKE_DAMPING,
            aim_idle_snap_secs: AIM_IDLE_SNAP_SECS,
            gamepad_right_deadzone: GAMEPAD_RIGHT_DEADZONE,
            gamepad_fire_threshold: GAMEPAD_FIRE_THRESHOLD,
            gamepad_heading_snap_threshold: GAMEPAD_HEADING_SNAP_THRESHOLD,
            // Rendering
            force_vector_hide_threshold: FORCE_VECTOR_HIDE_THRESHOLD,
            force_vector_display_scale: FORCE_VECTOR_DISPLAY_SCALE,
            force_vector_min_length: FORCE_VECTOR_MIN_LENGTH,
            stats_font_size: STATS_FONT_SIZE,
            // Asteroid Geometry
            hull_dedup_min_dist: HULL_DEDUP_MIN_DIST,
            triangle_base_side: TRIANGLE_BASE_SIDE,
            square_base_half: SQUARE_BASE_HALF,
            polygon_base_radius: POLYGON_BASE_RADIUS,
            asteroid_size_scale_min: ASTEROID_SIZE_SCALE_MIN,
            asteroid_size_scale_max: ASTEROID_SIZE_SCALE_MAX,
            asteroid_initial_velocity_range: ASTEROID_INITIAL_VELOCITY_RANGE,
            asteroid_initial_angvel_range: ASTEROID_INITIAL_ANGVEL_RANGE,
        }
    }
}

/// Startup system: attempt to load `assets/physics.toml` and overwrite the
/// `PhysicsConfig` resource with any values present in the file.
///
/// Missing keys retain their compiled defaults.  TOML parse errors are printed
/// to stderr but do not abort the simulation.  A missing file is silently
/// ignored (defaults are already in place from `insert_resource`).
pub fn load_physics_config(mut config: ResMut<PhysicsConfig>) {
    let path = "assets/physics.toml";
    match std::fs::read_to_string(path) {
        Ok(contents) => match toml::from_str::<PhysicsConfig>(&contents) {
            Ok(loaded) => {
                *config = loaded;
                println!("✓ Loaded physics config from {path}");
            }
            Err(e) => {
                eprintln!("⚠ Failed to parse {path}: {e}; using defaults");
            }
        },
        Err(_) => {
            // File not present — defaults are already in place; not an error.
            println!("ℹ No {path} found; using compiled defaults");
        }
    }
}
