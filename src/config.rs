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
use std::time::SystemTime;

const PHYSICS_CONFIG_PATH: &str = "assets/physics.toml";
const HOT_RELOAD_POLL_SECS: f32 = 0.5;

/// Tracks state for runtime hot-reloading of `assets/physics.toml`.
#[derive(Resource, Debug, Clone)]
pub struct PhysicsConfigHotReloadState {
    pub last_seen_modified: Option<SystemTime>,
    pub poll_timer: f32,
}

impl Default for PhysicsConfigHotReloadState {
    fn default() -> Self {
        Self {
            last_seen_modified: None,
            poll_timer: 0.0,
        }
    }
}

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
    pub hull_extent_base: f32,
    pub hull_extent_per_member: f32,

    // ── Physics: Collision ────────────────────────────────────────────────────
    pub restitution_small: f32,
    pub friction_asteroid: f32,

    // ── Physics: Culling ──────────────────────────────────────────────────────
    pub cull_distance: f32,
    pub soft_boundary_radius: f32,
    pub soft_boundary_strength: f32,
    pub hard_cull_distance: f32,

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

    // ── Player: Tractor Beam ─────────────────────────────────────────────────
    pub tractor_beam_range_base: f32,
    pub tractor_beam_range_per_level: f32,
    pub tractor_beam_force_base: f32,
    pub tractor_beam_force_per_level: f32,
    pub tractor_beam_max_target_size_base: u32,
    pub tractor_beam_max_target_size_per_level: u32,
    pub tractor_beam_max_target_speed_base: f32,
    pub tractor_beam_max_target_speed_per_level: f32,
    pub tractor_beam_min_distance: f32,
    pub tractor_beam_aim_cone_dot: f32,
    pub tractor_beam_freeze_velocity_damping: f32,
    pub tractor_beam_freeze_max_relative_speed: f32,
    pub tractor_beam_freeze_force_multiplier: f32,
    pub tractor_beam_freeze_offset_stiffness: f32,
    pub tractor_beam_freeze_max_hold_offset: f32,
    pub tractor_beam_freeze_max_target_size_multiplier: f32,
    pub tractor_beam_freeze_max_target_speed_multiplier: f32,

    // ── Player: Combat ────────────────────────────────────────────────────────
    pub projectile_speed: f32,
    pub fire_cooldown: f32,
    pub projectile_lifetime: f32,
    pub projectile_max_dist: f32,
    pub projectile_collider_radius: f32,

    // ── Player: Missiles ─────────────────────────────────────────────────────
    pub missile_ammo_max: u32,
    pub missile_initial_speed: f32,
    pub missile_speed: f32,
    pub missile_acceleration: f32,
    pub missile_cooldown: f32,
    pub missile_lifetime: f32,
    pub missile_max_dist: f32,
    pub missile_collider_radius: f32,
    pub missile_recharge_secs: f32,
    pub missile_split_max_pieces: u32,

    // ── Player: Health ────────────────────────────────────────────────────────
    pub player_max_hp: f32,
    pub damage_speed_threshold: f32,
    pub invincibility_duration: f32,

    // ── Player: Lives & Respawn ────────────────────────────────────────────────
    pub player_lives: i32,
    pub respawn_delay_secs: f32,
    pub respawn_invincibility_secs: f32,

    // ── Player: Passive Healing ────────────────────────────────────────────────
    pub passive_heal_delay_secs: f32,
    pub passive_heal_rate: f32,

    // ── Ore Magnet ────────────────────────────────────────────────────────────
    pub ore_magnet_radius: f32,
    pub ore_magnet_strength: f32,
    /// HP restored per ore unit spent on healing (`H` key).
    pub ore_heal_amount: f32,

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
    pub heptagon_base_radius: f32,
    pub octagon_base_radius: f32,
    pub planetoid_base_radius: f32,
    pub planetoid_unit_size: u32,
    pub asteroid_size_scale_min: f32,
    pub asteroid_size_scale_max: f32,
    pub asteroid_initial_velocity_range: f32,
    pub asteroid_initial_angvel_range: f32,

    // ── Physics: Density ──────────────────────────────────────────────────────
    /// Mass units per world-unit² used to scale polygon visual area to match mass.
    /// See `ASTEROID_DENSITY` in `src/constants.rs` for full documentation.
    pub asteroid_density: f32,
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
            hull_extent_base: HULL_EXTENT_BASE,
            hull_extent_per_member: HULL_EXTENT_PER_MEMBER,
            // Collision
            restitution_small: RESTITUTION_SMALL,
            friction_asteroid: FRICTION_ASTEROID,
            // Culling
            cull_distance: CULL_DISTANCE,
            soft_boundary_radius: SOFT_BOUNDARY_RADIUS,
            soft_boundary_strength: SOFT_BOUNDARY_STRENGTH,
            hard_cull_distance: HARD_CULL_DISTANCE,
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
            // Player: Tractor Beam
            tractor_beam_range_base: TRACTOR_BEAM_RANGE_BASE,
            tractor_beam_range_per_level: TRACTOR_BEAM_RANGE_PER_LEVEL,
            tractor_beam_force_base: TRACTOR_BEAM_FORCE_BASE,
            tractor_beam_force_per_level: TRACTOR_BEAM_FORCE_PER_LEVEL,
            tractor_beam_max_target_size_base: TRACTOR_BEAM_MAX_TARGET_SIZE_BASE,
            tractor_beam_max_target_size_per_level: TRACTOR_BEAM_MAX_TARGET_SIZE_PER_LEVEL,
            tractor_beam_max_target_speed_base: TRACTOR_BEAM_MAX_TARGET_SPEED_BASE,
            tractor_beam_max_target_speed_per_level: TRACTOR_BEAM_MAX_TARGET_SPEED_PER_LEVEL,
            tractor_beam_min_distance: TRACTOR_BEAM_MIN_DISTANCE,
            tractor_beam_aim_cone_dot: TRACTOR_BEAM_AIM_CONE_DOT,
            tractor_beam_freeze_velocity_damping: TRACTOR_BEAM_FREEZE_VELOCITY_DAMPING,
            tractor_beam_freeze_max_relative_speed: TRACTOR_BEAM_FREEZE_MAX_RELATIVE_SPEED,
            tractor_beam_freeze_force_multiplier: TRACTOR_BEAM_FREEZE_FORCE_MULTIPLIER,
            tractor_beam_freeze_offset_stiffness: TRACTOR_BEAM_FREEZE_OFFSET_STIFFNESS,
            tractor_beam_freeze_max_hold_offset: TRACTOR_BEAM_FREEZE_MAX_HOLD_OFFSET,
            tractor_beam_freeze_max_target_size_multiplier:
                TRACTOR_BEAM_FREEZE_MAX_TARGET_SIZE_MULTIPLIER,
            tractor_beam_freeze_max_target_speed_multiplier:
                TRACTOR_BEAM_FREEZE_MAX_TARGET_SPEED_MULTIPLIER,
            // Player: Combat
            projectile_speed: PROJECTILE_SPEED,
            fire_cooldown: FIRE_COOLDOWN,
            projectile_lifetime: PROJECTILE_LIFETIME,
            projectile_max_dist: PROJECTILE_MAX_DIST,
            projectile_collider_radius: PROJECTILE_COLLIDER_RADIUS,
            // Player: Missiles
            missile_ammo_max: MISSILE_AMMO_MAX,
            missile_initial_speed: MISSILE_INITIAL_SPEED,
            missile_speed: MISSILE_SPEED,
            missile_acceleration: MISSILE_ACCELERATION,
            missile_cooldown: MISSILE_COOLDOWN,
            missile_lifetime: MISSILE_LIFETIME,
            missile_max_dist: MISSILE_MAX_DIST,
            missile_collider_radius: MISSILE_COLLIDER_RADIUS,
            missile_recharge_secs: MISSILE_RECHARGE_SECS,
            missile_split_max_pieces: MISSILE_SPLIT_MAX_PIECES,
            // Player: Health
            player_max_hp: PLAYER_MAX_HP,
            damage_speed_threshold: DAMAGE_SPEED_THRESHOLD,
            invincibility_duration: INVINCIBILITY_DURATION,
            // Player: Lives & Respawn
            player_lives: PLAYER_LIVES,
            respawn_delay_secs: RESPAWN_DELAY_SECS,
            respawn_invincibility_secs: RESPAWN_INVINCIBILITY_SECS,
            // Player: Passive Healing
            passive_heal_delay_secs: PASSIVE_HEAL_DELAY_SECS,
            passive_heal_rate: PASSIVE_HEAL_RATE,
            // Ore Magnet
            ore_magnet_radius: ORE_MAGNET_BASE_RADIUS,
            ore_magnet_strength: ORE_MAGNET_BASE_STRENGTH,
            ore_heal_amount: ORE_HEAL_AMOUNT,
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
            heptagon_base_radius: HEPTAGON_BASE_RADIUS,
            octagon_base_radius: OCTAGON_BASE_RADIUS,
            planetoid_base_radius: PLANETOID_BASE_RADIUS,
            planetoid_unit_size: PLANETOID_UNIT_SIZE,
            asteroid_size_scale_min: ASTEROID_SIZE_SCALE_MIN,
            asteroid_size_scale_max: ASTEROID_SIZE_SCALE_MAX,
            asteroid_initial_velocity_range: ASTEROID_INITIAL_VELOCITY_RANGE,
            asteroid_initial_angvel_range: ASTEROID_INITIAL_ANGVEL_RANGE,
            // Density
            asteroid_density: ASTEROID_DENSITY,
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
    match read_physics_config_file(PHYSICS_CONFIG_PATH) {
        Ok(loaded) => {
            *config = loaded;
            println!("✓ Loaded physics config from {PHYSICS_CONFIG_PATH}");
        }
        Err(err) => {
            // File absent or malformed — defaults are already in place.
            println!("ℹ {err}; using current/default physics config");
        }
    }
}

/// Initialize hot-reload file timestamp tracking after startup config load.
pub fn init_physics_hot_reload_state(mut state: ResMut<PhysicsConfigHotReloadState>) {
    state.last_seen_modified = physics_config_modified_time(PHYSICS_CONFIG_PATH);
}

/// Poll `assets/physics.toml` and hot-reload config when the file is modified.
pub fn hot_reload_physics_config(
    time: Res<Time>,
    mut state: ResMut<PhysicsConfigHotReloadState>,
    mut config: ResMut<PhysicsConfig>,
) {
    state.poll_timer += time.delta_secs();
    if state.poll_timer < HOT_RELOAD_POLL_SECS {
        return;
    }
    state.poll_timer = 0.0;

    let Some(modified) = physics_config_modified_time(PHYSICS_CONFIG_PATH) else {
        state.last_seen_modified = None;
        return;
    };

    let changed = match state.last_seen_modified {
        Some(previous) => modified > previous,
        None => {
            state.last_seen_modified = Some(modified);
            false
        }
    };

    if !changed {
        return;
    }

    match read_physics_config_file(PHYSICS_CONFIG_PATH) {
        Ok(loaded) => {
            *config = loaded;
            info!("Hot-reloaded physics config from {}", PHYSICS_CONFIG_PATH);
        }
        Err(err) => {
            eprintln!("⚠ Failed hot-reload from {PHYSICS_CONFIG_PATH}: {err}");
        }
    }

    // Advance watermark regardless of parse result to avoid repeated spam on the
    // same broken file version; next edit will trigger another reload attempt.
    state.last_seen_modified = Some(modified);
}

fn read_physics_config_file(path: &str) -> Result<PhysicsConfig, String> {
    let contents =
        std::fs::read_to_string(path).map_err(|err| format!("failed reading {path}: {err}"))?;
    toml::from_str::<PhysicsConfig>(&contents)
        .map_err(|err| format!("failed parsing {path}: {err}"))
}

fn physics_config_modified_time(path: &str) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}
