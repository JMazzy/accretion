//! Simulation plugin and systems for Bevy ECS
//!
//! This module owns the core physics systems (gravity, cluster formation, culling)
//! and the camera zoom / mouse-aim input handling.  Rendering logic lives in
//! [`crate::rendering`]; player systems live in [`crate::player`].

use crate::asteroid::{
    compute_convex_hull_from_points, rescale_vertices_to_area, Asteroid, AsteroidSize,
    GravityForce, NeighborCount, Planet, Vertices,
};
use crate::asteroid_rendering::{attach_asteroid_mesh_system, sync_asteroid_render_mode_system};
use crate::config::PhysicsConfig;
use crate::menu::GameState;
use crate::player::{
    aim_snap_system, apply_player_intent_system, attach_ion_cannon_shot_mesh_system,
    attach_missile_mesh_system, attach_player_ship_mesh_system, attach_player_ui_system,
    attach_projectile_mesh_system, camera_follow_system, cleanup_player_ui_system,
    despawn_old_ion_cannon_shots_system, despawn_old_missiles_system,
    despawn_old_projectiles_system, gamepad_connection_system, gamepad_to_intent_system,
    ion_cannon_fire_system, ion_cannon_hit_enemy_system, ion_shot_particles_system,
    keyboard_to_intent_system, missile_acceleration_system, missile_asteroid_hit_system,
    missile_fire_system, missile_trail_particles_system, player_collision_damage_system,
    player_intent_clear_system, player_oob_damping_system, player_respawn_system,
    projectile_asteroid_hit_system, projectile_fire_system, projectile_missile_planet_hit_system,
    stunned_enemy_particles_system, sync_aim_indicator_system,
    sync_player_and_projectile_mesh_visibility_system, sync_player_health_bar_system,
    sync_projectile_outline_visibility_system, sync_projectile_rotation_system,
    sync_ship_outline_visibility_and_color_system, tractor_beam_force_system, AimDirection,
    AimIdleTimer, IonCannonCooldown, IonCannonLevel, MissileAmmo, MissileCooldown, PlayerIntent,
    PlayerLives, PlayerScore, PlayerUiEntities, PreferredGamepad, TractorBeamLevel,
};
use crate::rendering::{
    debug_panel_button_system, hud_score_display_system, lives_hud_display_system,
    missile_hud_display_system, ore_hud_display_system, physics_inspector_display_system,
    profiler_display_system, stats_display_system, sync_boundary_ring_visibility_system,
    sync_debug_line_layers_system, sync_physics_inspector_visibility_system,
    sync_profiler_visibility_system, sync_stats_overlay_visibility_system, OverlayState,
};
use crate::spatial_partition::{rebuild_spatial_grid_system, SpatialGrid};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use std::time::Instant;

/// Tracks simulation statistics: active asteroids, culled count, merged count, split count, destroyed count
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct SimulationStats {
    pub live_count: u32,
    pub culled_total: u32,
    pub merged_total: u32,
    pub split_total: u32,
    pub destroyed_total: u32,
}

/// Aggregated missile combat telemetry used for balancing and test logs.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct MissileTelemetry {
    pub shots_fired: u32,
    pub hits: u32,
    pub instant_destroy_events: u32,
    pub split_events: u32,
    pub full_decompose_events: u32,
    pub destroyed_mass_total: u32,
    pub decomposed_mass_total: u32,
}

/// Camera state for zoom control (pan is replaced by player-follow camera)
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct CameraState {
    pub zoom: f32,
}

/// Per-schedule timing breakdown for in-game profiler display.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct ProfilerStats {
    pub update_group1_ms: f32,
    pub update_group2a_ms: f32,
    pub update_group2b_ms: f32,
    pub update_total_ms: f32,
    pub fixed_update_ms: f32,
    pub post_update_ms: f32,
}

#[derive(Resource, Default)]
struct ProfilerClock {
    update_mark: Option<Instant>,
    fixed_mark: Option<Instant>,
    post_mark: Option<Instant>,
}

/// Per-frame scratch buffers reused across `neighbor_counting_system` to eliminate
/// per-call heap allocations.
///
/// Registered once at startup; all `Vec`s grow to hold the largest N ever seen
/// and are only cleared (never freed) each call, so steady-state operation
/// produces zero heap allocations per physics tick.
#[derive(Resource, Default)]
pub(crate) struct GravityScratch {
    /// Reusable output buffer for KD-tree neighbor queries.
    neighbor_buf: Vec<Entity>,
    /// Reusable position buffer for `neighbor_counting_system`.
    nc_positions: Vec<(Entity, Vec2)>,
}

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimulationStats::default())
            .insert_resource(MissileTelemetry::default())
            .insert_resource(OverlayState::default())
            .insert_resource(GravityScratch::default())
            .insert_resource(ProfilerStats::default())
            .insert_resource(ProfilerClock::default())
            .insert_resource(CameraState { zoom: 1.0 })
            .insert_resource(AimDirection::default())
            .insert_resource(AimIdleTimer::default())
            .insert_resource(PreferredGamepad::default())
            .insert_resource(PlayerIntent::default())
            .insert_resource(PlayerScore::default())
            .insert_resource(PlayerLives::default())
            .insert_resource(TractorBeamLevel::default())
            .insert_resource(IonCannonLevel::default())
            .insert_resource(IonCannonCooldown::default())
            .insert_resource(MissileAmmo::default())
            .insert_resource(MissileCooldown::default())
            .insert_resource(PlayerUiEntities::default())
            .insert_resource(SpatialGrid::default())
            .add_systems(
                Update,
                (
                    profiler_begin_update_system,
                    // ── Group 1: physics bookkeeping + input pipeline ─────────
                    (
                        stats_counting_system, // Count asteroids for stats
                        soft_boundary_system,  // Apply inward spring force near boundary
                        culling_system,        // Hard-remove asteroids past safety boundary
                        particle_locking_system,
                        gamepad_connection_system, // Track preferred gamepad
                        player_intent_clear_system, // Reset ExternalForce + PlayerIntent
                        keyboard_to_intent_system, // WASD/rotation keys → PlayerIntent
                        gamepad_to_intent_system,  // Gamepad left-stick + B → PlayerIntent
                        apply_player_intent_system, // PlayerIntent → ExternalForce / Velocity
                        mouse_aim_system,          // Mouse cursor updates AimDirection
                    )
                        .chain(),
                    profiler_mark_after_group1_system,
                    // ── Group 2a: input / camera / mesh attachment ───────────
                    (
                        (
                            projectile_fire_system,              // Space/click/right-stick fires
                            missile_fire_system,                 // X/right-click fires a missile
                            ion_cannon_fire_system,              // C fires a forward ion shot
                            missile_acceleration_system,         // Missiles ramp toward max speed
                            missile_trail_particles_system,      // Exhaust particles opposite velocity
                            ion_shot_particles_system,           // Ion shot particle trail
                            stunned_enemy_particles_system,      // Stunned enemy particle feedback
                            aim_snap_system,                     // Snap aim after idle timeout
                            despawn_old_projectiles_system,      // Expire old projectiles
                            despawn_old_missiles_system,         // Expire old missiles
                            despawn_old_ion_cannon_shots_system, // Expire old ion shots
                            user_input_system,                   // Mouse wheel zoom
                        )
                            .chain(),
                        (
                            camera_follow_system,                // Camera tracks player
                            camera_zoom_system,                  // Apply zoom scale
                            attach_asteroid_mesh_system,         // Attach Mesh2d to new asteroids
                            sync_asteroid_render_mode_system, // Swap fill/outline mesh on wireframe_only toggle
                            attach_player_ship_mesh_system,   // Attach Mesh2d to player ship
                            attach_player_ui_system,          // Spawn health bar + aim indicator
                            attach_projectile_mesh_system,    // Attach Mesh2d to new projectiles
                            attach_missile_mesh_system,       // Attach Mesh2d to new missiles
                            attach_ion_cannon_shot_mesh_system, // Attach Mesh2d to new ion shots
                            sync_projectile_rotation_system, // Update projectile rotation to match velocity
                            sync_player_and_projectile_mesh_visibility_system, // Propagate wireframe_only
                        )
                            .chain(),
                    )
                        .chain(),
                    profiler_mark_after_group2a_system,
                    // ── Group 2b: overlay sync + player logic + stats ────────
                    (
                        sync_boundary_ring_visibility_system, // Show/hide boundary ring
                        sync_debug_line_layers_system,        // Refresh retained debug line layers
                        sync_stats_overlay_visibility_system, // Show/hide stats overlay
                        sync_physics_inspector_visibility_system, // Show/hide physics inspector
                        sync_profiler_visibility_system,      // Show/hide profiler overlay
                        sync_ship_outline_visibility_and_color_system, // Show/hide ship outline + apply HP tint
                        sync_projectile_outline_visibility_system, // Show/hide projectile/missile ring outlines
                        sync_player_health_bar_system, // Update health bar position + colour
                        sync_aim_indicator_system,     // Update aim arrow orientation + visibility
                        hud_score_display_system,      // Refresh score HUD
                        lives_hud_display_system,      // Refresh lives + respawn-countdown HUD
                        missile_hud_display_system,    // Refresh missile ammo HUD
                        ore_hud_display_system,        // Refresh ore count HUD
                        stats_display_system,          // Render stats overlay text
                        physics_inspector_display_system, // Render physics inspector text
                        profiler_display_system,       // Render profiler text
                        player_oob_damping_system,     // Slow player outside boundary
                        player_collision_damage_system, // Player takes damage from asteroids
                        player_respawn_system,         // Re-spawn ship after countdown
                        cleanup_player_ui_system,      // Despawn UI on player death
                    )
                        .chain(),
                    profiler_mark_after_group2b_system,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                missile_telemetry_log_system.run_if(in_state(GameState::Playing)),
            )
            // Rebuild grid, run gravity, and count neighbors in FixedUpdate.
            // neighbor_counting_system was previously in Update (60 Hz) — moving it here
            // avoids 60 KD-tree scans per second that produced no visible difference.
            .add_systems(
                FixedUpdate,
                (
                    profiler_begin_fixed_update_system,
                    (
                        rebuild_spatial_grid_system,
                        nbody_gravity_system,
                        tractor_beam_force_system,
                        neighbor_counting_system,
                    )
                        .chain(),
                    profiler_end_fixed_update_system,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            // asteroid_formation_system must run AFTER Rapier (PostUpdate) populates contacts.
            // apply_deferred between the two systems flushes formation's despawns/spawns so
            // projectile_asteroid_hit_system never double-processes an entity that was already
            // merged and despawned by the formation system in the same frame.
            .add_systems(
                PostUpdate,
                (
                    profiler_begin_post_update_system,
                    (
                        asteroid_formation_system,
                        ion_cannon_hit_enemy_system,
                        projectile_asteroid_hit_system,
                        missile_asteroid_hit_system,
                        projectile_missile_planet_hit_system,
                    )
                        .chain(),
                    profiler_end_post_update_system,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            // debug_panel_button_system runs outside the Playing gate so the debug
            // overlay toggles remain functional while the game is paused.
            .add_systems(Update, debug_panel_button_system);
    }
}

fn profiler_begin_update_system(mut clock: ResMut<ProfilerClock>) {
    clock.update_mark = Some(Instant::now());
}

fn profiler_mark_after_group1_system(
    mut clock: ResMut<ProfilerClock>,
    mut stats: ResMut<ProfilerStats>,
) {
    if let Some(mark) = clock.update_mark {
        let now = Instant::now();
        stats.update_group1_ms = (now - mark).as_secs_f32() * 1000.0;
        clock.update_mark = Some(now);
    }
}

fn profiler_mark_after_group2a_system(
    mut clock: ResMut<ProfilerClock>,
    mut stats: ResMut<ProfilerStats>,
) {
    if let Some(mark) = clock.update_mark {
        let now = Instant::now();
        stats.update_group2a_ms = (now - mark).as_secs_f32() * 1000.0;
        clock.update_mark = Some(now);
    }
}

fn profiler_mark_after_group2b_system(
    mut clock: ResMut<ProfilerClock>,
    mut stats: ResMut<ProfilerStats>,
) {
    if let Some(mark) = clock.update_mark {
        let now = Instant::now();
        stats.update_group2b_ms = (now - mark).as_secs_f32() * 1000.0;
        stats.update_total_ms =
            stats.update_group1_ms + stats.update_group2a_ms + stats.update_group2b_ms;
        clock.update_mark = Some(now);
    }
}

fn profiler_begin_fixed_update_system(mut clock: ResMut<ProfilerClock>) {
    clock.fixed_mark = Some(Instant::now());
}

fn profiler_end_fixed_update_system(
    mut clock: ResMut<ProfilerClock>,
    mut stats: ResMut<ProfilerStats>,
) {
    if let Some(mark) = clock.fixed_mark.take() {
        stats.fixed_update_ms = (Instant::now() - mark).as_secs_f32() * 1000.0;
    }
}

fn profiler_begin_post_update_system(mut clock: ResMut<ProfilerClock>) {
    clock.post_mark = Some(Instant::now());
}

fn profiler_end_post_update_system(
    mut clock: ResMut<ProfilerClock>,
    mut stats: ResMut<ProfilerStats>,
) {
    if let Some(mark) = clock.post_mark.take() {
        stats.post_update_ms = (Instant::now() - mark).as_secs_f32() * 1000.0;
    }
}

fn missile_telemetry_log_system(
    mut frame_counter: Local<u32>,
    mut last_logged_shots: Local<u32>,
    telemetry: Res<MissileTelemetry>,
) {
    *frame_counter += 1;

    if !frame_counter.is_multiple_of(120) {
        return;
    }

    if telemetry.shots_fired == 0 || telemetry.shots_fired == *last_logged_shots {
        return;
    }

    let shots = telemetry.shots_fired as f32;
    let hits = telemetry.hits as f32;
    let hit_rate = if shots > 0.0 {
        100.0 * hits / shots
    } else {
        0.0
    };

    let outcomes_total =
        telemetry.instant_destroy_events + telemetry.split_events + telemetry.full_decompose_events;
    let outcomes_total_f = outcomes_total.max(1) as f32;

    let kill_events = telemetry.instant_destroy_events + telemetry.full_decompose_events;
    let frames_per_kill = if kill_events > 0 {
        *frame_counter as f32 / kill_events as f32
    } else {
        f32::INFINITY
    };

    info!(
        "[missile_telemetry] frame={} shots={} hits={} hit_rate={:.1}% outcomes{{destroy:{:.1}%, split:{:.1}%, decompose:{:.1}%}} mass{{destroyed:{}, decomposed:{}}} ttk_proxy={{frames_per_kill:{}}}",
        *frame_counter,
        telemetry.shots_fired,
        telemetry.hits,
        hit_rate,
        100.0 * telemetry.instant_destroy_events as f32 / outcomes_total_f,
        100.0 * telemetry.split_events as f32 / outcomes_total_f,
        100.0 * telemetry.full_decompose_events as f32 / outcomes_total_f,
        telemetry.destroyed_mass_total,
        telemetry.decomposed_mass_total,
        if frames_per_kill.is_finite() {
            format!("{frames_per_kill:.1}")
        } else {
            "n/a".to_string()
        }
    );

    *last_logged_shots = telemetry.shots_fired;
}

/// Lock and merge asteroids when they're slow and touching.
/// Optimized: uses Rapier's contact pair iterator directly (O(C) where C = active contacts)
/// instead of iterating all N² entity pairs to find touching ones.
pub fn particle_locking_system(
    mut query: Query<(Entity, &mut Velocity), With<Asteroid>>,
    rapier_context: ReadRapierContext,
    config: Res<PhysicsConfig>,
) {
    let velocity_threshold = config.velocity_threshold_locking;
    let mut pairs_to_merge: Vec<(Entity, Entity)> = Vec::new();

    // Iterate only active contact pairs from Rapier (O(C) not O(N²))
    let Ok(rapier) = rapier_context.single() else {
        return;
    };
    for contact_pair in rapier
        .simulation
        .contact_pairs(rapier.colliders, rapier.rigidbody_set)
    {
        if !contact_pair.has_any_active_contact() {
            continue;
        }

        let (Some(e1), Some(e2)) = (contact_pair.collider1(), contact_pair.collider2()) else {
            continue;
        };

        // Only sync velocities if both asteroids are slow
        if let (Ok((_, v1)), Ok((_, v2))) = (query.get(e1), query.get(e2)) {
            if v1.linvel.length() < velocity_threshold && v2.linvel.length() < velocity_threshold {
                pairs_to_merge.push((e1, e2));
            }
        }
    }

    // Sync velocities for qualifying pairs
    for (e1, e2) in pairs_to_merge {
        if let Ok([(_, mut v1), (_, mut v2)]) = query.get_many_mut([e1, e2]) {
            let avg_linvel = (v1.linvel + v2.linvel) * 0.5;
            let avg_angvel = (v1.angvel + v2.angvel) * 0.5;
            v1.linvel = avg_linvel;
            v1.angvel = avg_angvel;
            v2.linvel = avg_linvel;
            v2.angvel = avg_angvel;
        }
    }
}

/// Compute the gravitational force vector applied to body `i` (toward body `j`)
/// for a single pair, using an inverse-square law.
///
/// Returns `None` when the pair is outside the valid gravity range:
/// - `dist_sq < min_dist_sq` (Rapier handles the contact)
/// - `dist_sq > max_dist_sq` (too far to matter)
///
/// The reaction force on body `j` is the negation of the returned value (Newton's 3rd law).
///
/// `mass_i` and `mass_j` are the gravitational masses of the two bodies (typically
/// their `AsteroidSize` values cast to `f32`).  The returned force magnitude is
/// `G · mass_i · mass_j / r²`.
pub(crate) fn gravity_force_between(
    pos_i: Vec2,
    pos_j: Vec2,
    gravity_const: f32,
    min_dist_sq: f32,
    max_dist_sq: f32,
    mass_i: f32,
    mass_j: f32,
) -> Option<Vec2> {
    let delta = pos_j - pos_i;
    let dist_sq = delta.length_squared();
    if dist_sq > max_dist_sq || dist_sq < min_dist_sq {
        return None;
    }
    Some(delta.normalize_or_zero() * (gravity_const * mass_i * mass_j / dist_sq))
}

/// N-body gravity system: applies mass-scaled gravity between all asteroids.
///
/// Force magnitude: `G · m_i · m_j / r²` where `m_i` and `m_j` are the
/// `AsteroidSize` values cast to `f32`.  This means larger composite bodies
/// (and the Orbit scenario's massive central planetoid) are genuinely more
/// gravitationally dominant, producing stable orbital dynamics.
///
/// Uses O(N²/2) pair iteration.
pub(crate) fn nbody_gravity_system(
    mut query: Query<
        (
            Entity,
            &Transform,
            &AsteroidSize,
            &mut ExternalForce,
            &mut GravityForce,
        ),
        With<Asteroid>,
    >,
    config: Res<PhysicsConfig>,
) {
    let gravity_const = config.gravity_const;
    let min_gravity_dist = config.min_gravity_dist;
    let max_gravity_dist = config.max_gravity_dist;
    let min_gravity_dist_sq = min_gravity_dist * min_gravity_dist;
    let max_gravity_dist_sq = max_gravity_dist * max_gravity_dist;

    // CRITICAL: Reset all forces to zero first, then calculate fresh.
    for (_, _, _, mut force, mut grav) in query.iter_mut() {
        force.force = Vec2::ZERO;
        force.torque = 0.0;
        grav.0 = Vec2::ZERO;
    }

    // Collect all entities with positions and gravitational masses.
    let mut entity_data: Vec<(Entity, Vec2, f32)> = Vec::new();
    for (entity, transform, size, _, _) in query.iter() {
        entity_data.push((entity, transform.translation.truncate(), size.0 as f32));
    }

    // Collect all force pairs to apply (entity, force_delta)
    let mut force_deltas: std::collections::HashMap<Entity, Vec2> =
        std::collections::HashMap::new();

    // Calculate gravity between all asteroid pairs
    for idx_i in 0..entity_data.len() {
        let (entity_i, pos_i, mass_i) = entity_data[idx_i];

        for &(entity_j, pos_j, mass_j) in entity_data.iter().skip(idx_i + 1) {
            if let Some(force) = gravity_force_between(
                pos_i,
                pos_j,
                gravity_const,
                min_gravity_dist_sq,
                max_gravity_dist_sq,
                mass_i,
                mass_j,
            ) {
                // Apply Newton's third law: equal and opposite forces
                *force_deltas.entry(entity_i).or_insert(Vec2::ZERO) += force;
                *force_deltas.entry(entity_j).or_insert(Vec2::ZERO) -= force;
            }
        }
    }

    // Apply all collected forces
    for (entity, force_delta) in force_deltas {
        if let Ok((_, _, _, mut force, mut grav)) = query.get_mut(entity) {
            force.force += force_delta;
            grav.0 += force_delta;
        }
    }
}

/// Updates `AimDirection` every frame from the mouse cursor position.
/// The player is always at the screen centre (camera follows them), so the
/// normalised screen-space offset from the centre IS the aim direction in world space.
/// Also resets [`AimIdleTimer`] whenever the cursor position changes.
pub fn mouse_aim_system(
    mut aim: ResMut<AimDirection>,
    mut idle: ResMut<AimIdleTimer>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    // Compute the direction from the window centre toward the cursor.
    // Because the camera follows the player, this is identical to the world-space
    // direction from the player to the cursor (zoom scale cancels on .normalize()).
    let offset = Vec2::new(
        cursor.x - window.width() / 2.0,
        -(cursor.y - window.height() / 2.0), // flip Y: Bevy world +Y = screen up
    );
    let dir = offset.normalize_or_zero();
    if dir.length_squared() > 0.0 {
        // Detect cursor movement: compare to the stored last cursor position.
        let moved = idle
            .last_cursor
            .is_none_or(|prev: Vec2| prev.distance_squared(cursor) > 1.0);
        if moved {
            idle.last_cursor = Some(cursor);
            idle.secs = 0.0;
        }
        aim.0 = dir;
    }
}

/// Handle user input: mouse wheel adjusts zoom.
/// Left-click now fires projectiles (handled in `projectile_fire_system`).
pub fn user_input_system(
    mut camera_state: ResMut<CameraState>,
    mut scroll_evr: MessageReader<MouseWheel>,
    config: Res<PhysicsConfig>,
) {
    // Zoom: mouse wheel (zoom value is applied each frame in camera_zoom_system)
    for ev in scroll_evr.read() {
        let delta = ev.y;
        camera_state.zoom = (camera_state.zoom - (delta * config.zoom_speed))
            .clamp(config.min_zoom, config.max_zoom);
    }
}

/// Apply the current zoom scale to the camera transform each frame.
/// Camera translation is handled by player::camera_follow_system.
pub fn camera_zoom_system(
    camera_state: Res<CameraState>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
) {
    for mut transform in camera_query.iter_mut() {
        transform.scale = Vec3::new(camera_state.zoom, camera_state.zoom, 1.0);
    }
}

/// Count neighbors for each asteroid (kept for potential future features and visualization hints).
/// Uses the spatial grid (O(N·K)) and a reusable scratch buffer to avoid per-frame allocations.
pub(crate) fn neighbor_counting_system(
    mut query: Query<(Entity, &Transform, &mut NeighborCount), With<Asteroid>>,
    grid: Res<SpatialGrid>,
    config: Res<PhysicsConfig>,
    mut scratch: ResMut<GravityScratch>,
) {
    let neighbor_threshold = config.neighbor_threshold;

    // Collect (entity, pos) pairs into the reusable buffer — avoids creating a
    // new Vec each frame and lets us call get_mut on the query afterwards.
    scratch.nc_positions.clear();
    for (e, t, _) in query.iter() {
        scratch.nc_positions.push((e, t.translation.truncate()));
    }

    for ii in 0..scratch.nc_positions.len() {
        let (entity, pos) = scratch.nc_positions[ii];

        // The KD-tree performs an exact Euclidean query: every returned entity
        // is already within `neighbor_threshold`.  Just count the results.
        grid.query_neighbors_into(entity, pos, neighbor_threshold, &mut scratch.neighbor_buf);
        let count = scratch.neighbor_buf.len();

        if let Ok((_, _, mut nc)) = query.get_mut(entity) {
            nc.0 = count;
        }
    }
}

/// Track statistics: live count, culled count, merged count.
/// Must run BEFORE culling_system to detect which asteroids are about to be hard-culled.
pub fn stats_counting_system(
    mut stats: ResMut<SimulationStats>,
    query: Query<(Entity, &Transform), With<Asteroid>>,
    config: Res<PhysicsConfig>,
) {
    let cull_distance = config.cull_distance;
    let hard_cull_distance = config.hard_cull_distance;
    let mut live_count = 0;
    let mut hard_culled_this_frame = 0;

    for (_, transform) in query.iter() {
        let dist = transform.translation.truncate().length();
        if dist <= cull_distance {
            live_count += 1;
        }
        // Count only asteroids that will actually be removed this frame
        if dist > hard_cull_distance {
            hard_culled_this_frame += 1;
        }
    }

    stats.live_count = live_count;
    stats.culled_total += hard_culled_this_frame;
}

/// Hard-cull asteroids that have drifted past the safety boundary.
///
/// The soft boundary spring (`soft_boundary_system`) keeps most asteroids from
/// reaching this distance; hard-culling is a last resort for very fast objects.
pub fn culling_system(
    mut commands: Commands,
    query: Query<(Entity, &Transform), With<Asteroid>>,
    config: Res<PhysicsConfig>,
) {
    let hard_cull_distance = config.hard_cull_distance;

    for (entity, transform) in query.iter() {
        let dist = transform.translation.truncate().length();
        if dist > hard_cull_distance {
            commands.entity(entity).despawn();
        }
    }
}

/// Apply a restoring force to asteroids that have drifted past the soft boundary.
///
/// The force is a linear spring toward the origin:
/// ```text
/// F_inward = soft_boundary_strength × (dist − soft_boundary_radius) × (−pos / dist)
/// ```
/// This creates a reflecting potential well that nudges stray asteroids back toward
/// the simulation centre without the jarring discontinuity of hard-culling.  The
/// spring activates only when `dist > soft_boundary_radius`.
pub fn soft_boundary_system(
    mut query: Query<(&Transform, &mut ExternalForce), With<Asteroid>>,
    config: Res<PhysicsConfig>,
) {
    let inner_radius = config.soft_boundary_radius;
    let strength = config.soft_boundary_strength;

    for (transform, mut ext_force) in query.iter_mut() {
        let pos = transform.translation.truncate();
        let dist = pos.length();

        if dist > inner_radius && dist > 0.0 {
            let excess = dist - inner_radius;
            // Inward unit vector: −pos / dist
            let inward = -pos / dist;
            ext_force.force += inward * (strength * excess);
        }
    }
}

/// Form large asteroids by detecting clusters of touching asteroids
/// and converting them into larger polygons.
///
/// ## Merge criterion: gravitational binding energy
///
/// A cluster merges only when its kinetic energy in the centre-of-mass frame
/// is less than the sum of pairwise gravitational binding energies:
///
/// ```text
/// E_binding = Σ_{i<j}  G · mᵢ · mⱼ / rᵢⱼ
/// E_k_com   = Σᵢ  ½ · mᵢ · |vᵢ − v_cm|²  +  Σᵢ  ½ · Iᵢ · ωᵢ²
/// merge iff  E_k_com < E_binding
/// ```
///
/// Mass is approximated as `AsteroidSize` units (uniform density).
/// Moment of inertia per member: `I = ½ · m · r_eff²` where `r_eff = √(m / π)`.
#[allow(clippy::type_complexity)]
pub fn asteroid_formation_system(
    mut commands: Commands,
    query: Query<
        (Entity, &Transform, &Velocity, &Vertices, &AsteroidSize),
        (With<Asteroid>, Without<Planet>),
    >,
    rapier_context: ReadRapierContext,
    mut stats: ResMut<SimulationStats>,
    config: Res<PhysicsConfig>,
) {
    let gravity_const = config.gravity_const;
    let mut processed = std::collections::HashSet::new();

    let Ok(rapier) = rapier_context.single() else {
        return;
    };
    let asteroids: Vec<_> = query.iter().collect();

    for i in 0..asteroids.len() {
        let (e1, t1, v1, _verts1, sz1) = asteroids[i];

        if processed.contains(&e1) {
            continue;
        }

        // Flood-fill to find all connected (touching) asteroids.
        // No velocity pre-filter — the binding-energy check below gates the merge.
        // Cluster stores: (entity, transform, velocity, vertices, size_units)
        let mut cluster: Vec<(Entity, &Transform, &Velocity, &Vertices, u32)> =
            vec![(e1, t1, v1, _verts1, sz1.0)];
        let mut queue = vec![e1];
        let mut visited = std::collections::HashSet::new();
        visited.insert(e1);

        while let Some(current) = queue.pop() {
            for &(e2, t2, v2, verts2, sz2) in asteroids.iter() {
                if visited.contains(&e2) || processed.contains(&e2) {
                    continue;
                }

                // Check if they're touching via Rapier contact
                if let Some(contact) = rapier.contact_pair(current, e2) {
                    if contact.has_any_active_contact() {
                        visited.insert(e2);
                        queue.push(e2);
                        cluster.push((e2, t2, v2, verts2, sz2.0));
                    }
                }
            }
        }

        if cluster.len() < 2 {
            continue;
        }

        // ── Gravitational binding energy check ────────────────────────────────
        //
        // Use AsteroidSize units as a mass proxy (uniform density → mass ∝ size).
        let masses: Vec<f32> = cluster.iter().map(|&(_, _, _, _, s)| s as f32).collect();
        let total_mass: f32 = masses.iter().sum();

        // Centre-of-mass velocity (mass-weighted average)
        let v_cm: Vec2 = cluster
            .iter()
            .zip(masses.iter())
            .map(|(&(_, _, v, _, _), &m)| v.linvel * m)
            .sum::<Vec2>()
            / total_mass;

        // Kinetic energy in the COM frame: translational + rotational per member.
        // Moment of inertia for a uniform disk: I = ½ · m · r², r = √(m / π).
        let ke_com: f32 = cluster
            .iter()
            .zip(masses.iter())
            .map(|(&(_, _, v, _, _), &m)| {
                let dv = v.linvel - v_cm;
                let r_eff = (m / std::f32::consts::PI).sqrt();
                let inertia = 0.5 * m * r_eff * r_eff;
                0.5 * m * dv.length_squared() + 0.5 * inertia * v.angvel * v.angvel
            })
            .sum();

        // Pairwise gravitational binding energy: E = Σ_{i<j} G·mᵢ·mⱼ / rᵢⱼ
        let binding_energy: f32 = {
            let mut e = 0.0_f32;
            let n = cluster.len();
            for a in 0..n {
                for b in (a + 1)..n {
                    let pos_a = cluster[a].1.translation.truncate();
                    let pos_b = cluster[b].1.translation.truncate();
                    // Clamp distance to ≥1 to avoid division-by-zero on overlapping bodies
                    let dist = (pos_b - pos_a).length().max(1.0);
                    e += gravity_const * masses[a] * masses[b] / dist;
                }
            }
            e
        };

        // Reject merge if cluster has too much kinetic energy to be gravitationally bound
        if ke_com >= binding_energy {
            continue;
        }

        // ── Merge ─────────────────────────────────────────────────────────────
        for (entity, _, _, _, _) in &cluster {
            processed.insert(*entity);
        }

        // Composite inherits the centre-of-mass velocity (momentum-conserving).
        // Angular velocity: simple average (moment-of-inertia weighting negligible here).
        let avg_linvel = v_cm;
        let avg_angvel: f32 =
            cluster.iter().map(|&(_, _, v, _, _)| v.angvel).sum::<f32>() / cluster.len() as f32;

        // Collect ALL vertices from ALL cluster members in world-space
        let mut world_vertices = Vec::new();
        for (_entity, transform, _vel, vertices, _sz) in &cluster {
            let rotation = transform.rotation;
            let offset = transform.translation.truncate();

            for local_v in &vertices.0 {
                // Rotate local vertex by transform rotation
                let world_v = offset + rotation.mul_vec3(local_v.extend(0.0)).truncate();
                world_vertices.push(world_v);
            }
        }

        // Compute convex hull from all world-space vertices
        if let Some(hull) = compute_convex_hull_from_points(&world_vertices) {
            // Need at least 3 vertices for a valid composite asteroid
            if hull.len() >= 3 {
                // Use the hull's geometric centroid as the spawn position so that the
                // stored local vertices are centred on (0,0) in local space.
                // Using the average of cluster entity positions would leave the vertices
                // off-centre, causing the physics body and drawn outline to misalign.
                let hull_centroid: Vec2 = hull.iter().copied().sum::<Vec2>() / hull.len() as f32;
                let hull_local: Vec<Vec2> = hull.iter().map(|v| *v - hull_centroid).collect();

                // Sanity check: skip merges that would produce pathologically large shapes.
                // A legitimate merge of N small asteroids should never produce a hull
                // that extends more than ~50 units per constituent member from the center.
                let max_extent = hull_local
                    .iter()
                    .map(|v| v.length())
                    .fold(0.0_f32, f32::max);
                let extent_limit =
                    config.hull_extent_base + cluster.len() as f32 * config.hull_extent_per_member;
                if max_extent > extent_limit {
                    // Refuse to create this merge — it indicates corrupted vertex data.
                    // Despawn nothing; leave the source asteroids intact.
                    continue;
                }

                // Sum unit sizes of all cluster members
                let total_size: u32 = cluster.iter().map(|(_, _, _, _, s)| s).sum();

                // Scale the hull so its visual area matches total_size / density.
                // This ensures merged composites look proportional to their mass
                // regardless of how spread out the constituent asteroids were.
                let target_area = total_size as f32 / config.asteroid_density;
                let hull_local = rescale_vertices_to_area(&hull_local, target_area);

                let avg_color = Color::srgb(0.5, 0.5, 0.5);
                let composite = crate::asteroid::spawn_asteroid_with_vertices(
                    &mut commands,
                    hull_centroid,
                    &hull_local,
                    avg_color,
                    total_size,
                );

                // Update velocity
                if let Ok(mut cmd) = commands.get_entity(composite) {
                    cmd.insert(Velocity {
                        linvel: avg_linvel,
                        angvel: avg_angvel,
                    });
                }

                // Track merge: N asteroids became 1, so we merged (N-1) asteroids
                let merge_count = (cluster.len() - 1) as u32;
                stats.merged_total += merge_count;

                crate::particles::spawn_merge_particles(&mut commands, hull_centroid);

                // Despawn all source asteroids
                for (entity, _, _, _, _) in cluster {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── gravity_force_between ─────────────────────────────────────────────────

    #[test]
    fn gravity_attracts_toward_other_body() {
        let f = gravity_force_between(
            Vec2::ZERO,
            Vec2::new(100.0, 0.0),
            10.0,
            1.0,
            1_000_000.0,
            1.0,
            1.0,
        )
        .expect("pair should be in range");
        assert!(f.x > 0.0, "force x should be positive (toward body j)");
        assert!(
            f.y.abs() < 1e-6,
            "no vertical component for horizontal pair"
        );
    }

    #[test]
    fn gravity_inverse_square_law() {
        let g = 10.0_f32;
        let f1 = gravity_force_between(
            Vec2::ZERO,
            Vec2::new(10.0, 0.0),
            g,
            1.0,
            1_000_000.0,
            1.0,
            1.0,
        )
        .unwrap();
        let f2 = gravity_force_between(
            Vec2::ZERO,
            Vec2::new(20.0, 0.0),
            g,
            1.0,
            1_000_000.0,
            1.0,
            1.0,
        )
        .unwrap();
        let ratio = f1.x / f2.x;
        assert!(
            (ratio - 4.0).abs() < 1e-4,
            "force at 2× distance should be 4× weaker; ratio={ratio}"
        );
    }

    #[test]
    fn gravity_force_magnitude_matches_formula() {
        let d = 50.0_f32;
        let g = 10.0_f32;
        let f = gravity_force_between(Vec2::ZERO, Vec2::new(d, 0.0), g, 1.0, 1_000_000.0, 1.0, 1.0)
            .unwrap();
        let expected = g / (d * d);
        assert!(
            (f.x - expected).abs() < 1e-6,
            "magnitude: got {}, expected {expected}",
            f.x
        );
    }

    #[test]
    fn gravity_none_beyond_max_distance() {
        let max_dist = 100.0_f32;
        let f = gravity_force_between(
            Vec2::ZERO,
            Vec2::new(max_dist + 1.0, 0.0),
            10.0,
            1.0,
            max_dist * max_dist,
            1.0,
            1.0,
        );
        assert!(f.is_none(), "should be None beyond max distance");
    }

    #[test]
    fn gravity_none_within_min_distance() {
        let min_dist = 5.0_f32;
        let f = gravity_force_between(
            Vec2::ZERO,
            Vec2::new(2.0, 0.0),
            10.0,
            min_dist * min_dist,
            1_000_000.0,
            1.0,
            1.0,
        );
        assert!(f.is_none(), "should be None when closer than min distance");
    }

    #[test]
    fn gravity_newtons_third_law() {
        let pos_i = Vec2::new(-50.0, 30.0);
        let pos_j = Vec2::new(70.0, -20.0);
        let f_ij = gravity_force_between(pos_i, pos_j, 10.0, 1.0, 1_000_000.0, 1.0, 1.0).unwrap();
        let f_ji = gravity_force_between(pos_j, pos_i, 10.0, 1.0, 1_000_000.0, 1.0, 1.0).unwrap();
        assert!(
            (f_ij + f_ji).length() < 1e-5,
            "forces on i and j must sum to zero"
        );
    }

    #[test]
    fn gravity_at_boundary_distance_returns_some() {
        // Exactly at max boundary (dist_sq == max_dist_sq uses > not >=, so this is in-range)
        let max_dist = 100.0_f32;
        let f = gravity_force_between(
            Vec2::ZERO,
            Vec2::new(max_dist, 0.0),
            10.0,
            1.0,
            max_dist * max_dist,
            1.0,
            1.0,
        );
        assert!(f.is_some(), "exactly at boundary should still return force");
    }
}
