//! Simulation plugin and systems for Bevy ECS
//!
//! This module owns the core physics systems (gravity, cluster formation, culling)
//! and the camera zoom / mouse-aim input handling.  Rendering logic lives in
//! [`crate::rendering`]; player systems live in [`crate::player`].

use crate::asteroid::{
    compute_convex_hull_from_points, Asteroid, AsteroidSize, NeighborCount, Vertices,
};
use crate::constants::{
    CULL_DISTANCE, GRAVITY_CONST, HULL_EXTENT_BASE, HULL_EXTENT_PER_MEMBER, MAX_GRAVITY_DIST,
    MAX_ZOOM, MIN_GRAVITY_DIST, MIN_ZOOM, NEIGHBOR_THRESHOLD, VELOCITY_THRESHOLD_FORMATION,
    VELOCITY_THRESHOLD_LOCKING, ZOOM_SPEED,
};
use crate::player::{
    camera_follow_system, despawn_old_projectiles_system, gamepad_connection_system,
    gamepad_movement_system, player_collision_damage_system, player_control_system,
    player_force_reset_system, player_gizmo_system, player_oob_damping_system,
    projectile_asteroid_hit_system, projectile_fire_system, AimDirection, PreferredGamepad,
};
use crate::rendering::{gizmo_rendering_system, stats_display_system};
use crate::spatial_partition::{rebuild_spatial_grid_system, SpatialGrid};
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use std::collections::HashMap;

/// Tracks simulation statistics: active asteroids, culled count, merged count
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct SimulationStats {
    pub live_count: u32,
    pub culled_total: u32,
    pub merged_total: u32,
}

/// Camera state for zoom control (pan is replaced by player-follow camera)
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct CameraState {
    pub zoom: f32,
}

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimulationStats::default())
            .insert_resource(CameraState { zoom: 1.0 })
            .insert_resource(AimDirection::default())
            .insert_resource(PreferredGamepad::default())
            .insert_resource(SpatialGrid::default())
            .add_systems(
                Update,
                (
                    stats_counting_system, // FIRST: Count asteroids for tracking
                    culling_system,        // Remove far asteroids before physics
                    neighbor_counting_system,
                    particle_locking_system,
                    gamepad_connection_system, // Update PreferredGamepad on connect/disconnect
                    // Force must be reset BEFORE any input system adds to it; chain enforces order.
                    player_force_reset_system,
                    player_control_system,          // WASD ship thrust/rotation
                    gamepad_movement_system,        // Gamepad left stick movement + B reverse
                    mouse_aim_system,               // Mouse cursor updates AimDirection
                    projectile_fire_system,         // Space/click/right-stick fires projectiles
                    despawn_old_projectiles_system, // Expire old projectiles
                    user_input_system,              // Mouse wheel zoom
                    camera_follow_system,           // Camera tracks player
                    camera_zoom_system,             // Apply zoom scale to camera
                    gizmo_rendering_system,         // Render asteroids + boundary
                    player_gizmo_system,            // Render ship + aim indicator + projectiles
                    stats_display_system,           // Render stats text
                    player_oob_damping_system,      // Slow player outside cull radius
                    player_collision_damage_system, // Player takes damage from asteroids
                )
                    .chain(),
            )
            // Rebuild grid then run gravity in FixedUpdate (same schedule as Rapier physics)
            // Grid rebuild only here — Update systems reuse the last FixedUpdate grid
            .add_systems(
                FixedUpdate,
                (rebuild_spatial_grid_system, nbody_gravity_system).chain(),
            )
            // asteroid_formation_system must run AFTER Rapier (PostUpdate) populates contacts.
            // apply_deferred between the two systems flushes formation's despawns/spawns so
            // projectile_asteroid_hit_system never double-processes an entity that was already
            // merged and despawned by the formation system in the same frame.
            .add_systems(
                PostUpdate,
                (
                    asteroid_formation_system,
                    apply_deferred,
                    projectile_asteroid_hit_system,
                )
                    .chain(),
            );
    }
}

/// Lock and merge asteroids when they're slow and touching.
/// Optimized: uses Rapier's contact pair iterator directly (O(C) where C = active contacts)
/// instead of iterating all N² entity pairs to find touching ones.
pub fn particle_locking_system(
    mut query: Query<(Entity, &mut Velocity), With<Asteroid>>,
    rapier_context: Res<RapierContext>,
) {
    let velocity_threshold = VELOCITY_THRESHOLD_LOCKING;
    let mut pairs_to_merge: Vec<(Entity, Entity)> = Vec::new();

    // Iterate only active contact pairs from Rapier (O(C) not O(N²))
    for contact_pair in rapier_context.contact_pairs() {
        if !contact_pair.has_any_active_contacts() {
            continue;
        }

        let e1 = contact_pair.collider1();
        let e2 = contact_pair.collider2();

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
pub(crate) fn gravity_force_between(
    pos_i: Vec2,
    pos_j: Vec2,
    gravity_const: f32,
    min_dist_sq: f32,
    max_dist_sq: f32,
) -> Option<Vec2> {
    let delta = pos_j - pos_i;
    let dist_sq = delta.length_squared();
    if dist_sq > max_dist_sq || dist_sq < min_dist_sq {
        return None;
    }
    Some(delta.normalize_or_zero() * (gravity_const / dist_sq))
}

/// N-body gravity system: applies custom gravity between all asteroids
/// Optimized with spatial grid: only checks neighbors in nearby cells (O(N·K) instead of O(N²))
pub fn nbody_gravity_system(
    mut query: Query<(Entity, &Transform, &mut ExternalForce), With<Asteroid>>,
    grid: Res<SpatialGrid>,
) {
    let gravity_const = GRAVITY_CONST;
    let min_gravity_dist = MIN_GRAVITY_DIST;
    let max_gravity_dist = MAX_GRAVITY_DIST;
    let min_gravity_dist_sq = min_gravity_dist * min_gravity_dist;
    let max_gravity_dist_sq = max_gravity_dist * max_gravity_dist;

    // CRITICAL: Reset all forces to zero first, then calculate fresh
    // This prevents accumulation bugs and ensures forces reflect current positions
    for (_, _, mut force) in query.iter_mut() {
        force.force = Vec2::ZERO;
        force.torque = 0.0;
    }

    // Collect all entities with positions (needed for pairwise calculations)
    let mut entity_data: Vec<(Entity, Vec2)> = Vec::new();
    // Also build an index map for O(1) lookups
    let mut entity_index: HashMap<Entity, usize> = HashMap::new();
    for (entity, transform, _) in query.iter() {
        entity_index.insert(entity, entity_data.len());
        entity_data.push((entity, transform.translation.truncate()));
    }

    // Collect all force pairs to apply (entity, force_delta)
    let mut force_deltas: HashMap<Entity, Vec2> = HashMap::new();

    // Calculate gravity between nearby asteroid pairs using spatial grid
    for (idx_i, (entity_i, pos_i)) in entity_data.iter().enumerate() {
        // Get all potential neighbors from spatial grid
        let candidates = grid.get_neighbors_excluding(*entity_i, *pos_i, max_gravity_dist);

        // Only process each pair once (if candidate index > current index)
        for entity_j in candidates {
            if let Some(&idx_j) = entity_index.get(&entity_j) {
                if idx_j <= idx_i {
                    continue;
                }

                let pos_j = entity_data[idx_j].1;

                if let Some(force) = gravity_force_between(
                    *pos_i,
                    pos_j,
                    gravity_const,
                    min_gravity_dist_sq,
                    max_gravity_dist_sq,
                ) {
                    // Apply Newton's third law: equal and opposite forces
                    *force_deltas.entry(*entity_i).or_insert(Vec2::ZERO) += force;
                    *force_deltas.entry(entity_j).or_insert(Vec2::ZERO) -= force;
                }
            }
        }
    }

    // Apply all collected forces
    for (entity, force_delta) in force_deltas {
        if let Ok((_, _, mut force)) = query.get_mut(entity) {
            force.force += force_delta;
        }
    }
}

/// Updates `AimDirection` every frame from the mouse cursor position.
/// The player is always at the screen centre (camera follows them), so the
/// normalised screen-space offset from the centre IS the aim direction in world space.
pub fn mouse_aim_system(mut aim: ResMut<AimDirection>, windows: Query<&Window>) {
    let Ok(window) = windows.get_single() else {
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
        aim.0 = dir;
    }
}

/// Handle user input: mouse wheel adjusts zoom.
/// Left-click now fires projectiles (handled in `projectile_fire_system`).
pub fn user_input_system(
    mut camera_state: ResMut<CameraState>,
    mut scroll_evr: EventReader<MouseWheel>,
) {
    // Zoom: mouse wheel (zoom value is applied each frame in camera_zoom_system)
    for ev in scroll_evr.read() {
        let delta = ev.y;
        camera_state.zoom = (camera_state.zoom - (delta * ZOOM_SPEED)).clamp(MIN_ZOOM, MAX_ZOOM);
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

/// Count neighbors for each asteroid (kept for potential future features and visualization hints)
/// Optimized with spatial grid: O(N·K) instead of O(N²) where K is avg neighbors per cell
pub fn neighbor_counting_system(
    mut query: Query<(Entity, &Transform, &mut NeighborCount), With<Asteroid>>,
    grid: Res<SpatialGrid>,
) {
    let neighbor_threshold = NEIGHBOR_THRESHOLD;

    // Collect all entity positions first as a HashMap for O(1) lookups
    let entity_positions: HashMap<Entity, Vec2> = query
        .iter()
        .map(|(e, t, _)| (e, t.translation.truncate()))
        .collect();

    for (entity, pos) in &entity_positions {
        // Get potential neighbors from grid (only checks nearby cells)
        let candidates = grid.get_neighbors_excluding(*entity, *pos, neighbor_threshold);

        // Count those actually within threshold distance
        let count = candidates
            .iter()
            .filter(|&&candidate| {
                entity_positions
                    .get(&candidate)
                    .map(|candidate_pos| (*pos - *candidate_pos).length() < neighbor_threshold)
                    .unwrap_or(false)
            })
            .count();

        if let Ok((_, _, mut nc)) = query.get_mut(*entity) {
            nc.0 = count;
        }
    }
}

/// Track statistics: live count, culled count, merged count
/// Must run BEFORE culling_system to detect which asteroids are about to be culled
pub fn stats_counting_system(
    mut stats: ResMut<SimulationStats>,
    query: Query<(Entity, &Transform), With<Asteroid>>,
) {
    let cull_distance = CULL_DISTANCE;
    let mut live_count = 0;
    let mut culled_this_frame = 0;

    // Count live asteroids and identify those about to be culled
    for (_, transform) in query.iter() {
        let dist = transform.translation.truncate().length();
        if dist <= cull_distance {
            live_count += 1;
        } else {
            culled_this_frame += 1;
        }
    }

    stats.live_count = live_count;
    stats.culled_total += culled_this_frame;
}

/// Cull asteroids far off-screen
pub fn culling_system(mut commands: Commands, query: Query<(Entity, &Transform), With<Asteroid>>) {
    let cull_distance = CULL_DISTANCE;

    for (entity, transform) in query.iter() {
        let dist = transform.translation.truncate().length();

        // Cull asteroids that drift very far
        if dist > cull_distance {
            commands.entity(entity).despawn();
        }
    }
}

/// Form large asteroids by detecting clusters of touching asteroids
/// and converting them into larger polygons
pub fn asteroid_formation_system(
    mut commands: Commands,
    query: Query<(Entity, &Transform, &Velocity, &Vertices, &AsteroidSize), With<Asteroid>>,
    rapier_context: Res<RapierContext>,
    mut stats: ResMut<SimulationStats>,
) {
    // Find clusters of slow-moving asteroids that are touching
    let velocity_threshold = VELOCITY_THRESHOLD_FORMATION;
    let mut processed = std::collections::HashSet::new();

    let asteroids: Vec<_> = query.iter().collect();

    for i in 0..asteroids.len() {
        let (e1, t1, v1, _verts1, sz1) = asteroids[i];

        if processed.contains(&e1) {
            continue;
        }

        // Only start clusters from slow asteroids
        if v1.linvel.length() > velocity_threshold {
            continue;
        }

        // Flood-fill to find all connected slow asteroids
        // Store: (entity, transform, vertices, size)
        let mut cluster: Vec<(Entity, &Transform, &Vertices, u32)> = vec![(e1, t1, _verts1, sz1.0)];
        let mut queue = vec![e1];
        let mut visited = std::collections::HashSet::new();
        visited.insert(e1);

        while let Some(current) = queue.pop() {
            for &(e2, t2, v2, verts2, sz2) in asteroids.iter() {
                if visited.contains(&e2) || processed.contains(&e2) {
                    continue;
                }

                // Only combine if slow
                if v2.linvel.length() > velocity_threshold {
                    continue;
                }

                // Check if they're touching via Rapier contact
                if let Some(contact) = rapier_context.contact_pair(current, e2) {
                    if contact.has_any_active_contacts() {
                        visited.insert(e2);
                        queue.push(e2);
                        cluster.push((e2, t2, verts2, sz2.0));
                    }
                }
            }
        }

        // If we have 2+ asteroids in the cluster, merge them
        if cluster.len() >= 2 {
            // Mark all as processed
            for (entity, _, _, _) in &cluster {
                processed.insert(*entity);
            }

            // Compute center position
            let mut center = Vec2::ZERO;
            let mut avg_linvel = Vec2::ZERO;
            let mut avg_angvel = 0.0;

            for (entity, t, _v, _) in &cluster {
                let pos = t.translation.truncate();
                center += pos;
                // Get velocity from the cached query data
                for (e, _, vel, _, _) in &asteroids {
                    if e == entity {
                        avg_linvel += vel.linvel;
                        avg_angvel += vel.angvel;
                        break;
                    }
                }
            }

            let count = cluster.len() as f32;
            center /= count;
            avg_linvel /= count;
            avg_angvel /= count;

            // Collect ALL vertices from ALL cluster members in world-space
            let mut world_vertices = Vec::new();
            for (_entity, transform, vertices, _sz) in &cluster {
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
                    let hull_centroid: Vec2 =
                        hull.iter().copied().sum::<Vec2>() / hull.len() as f32;
                    let hull_local: Vec<Vec2> = hull.iter().map(|v| *v - hull_centroid).collect();

                    // Sanity check: skip merges that would produce pathologically large shapes.
                    // A legitimate merge of N small asteroids should never produce a hull
                    // that extends more than ~50 units per constituent member from the center.
                    let max_extent = hull_local
                        .iter()
                        .map(|v| v.length())
                        .fold(0.0_f32, f32::max);
                    let extent_limit =
                        HULL_EXTENT_BASE + cluster.len() as f32 * HULL_EXTENT_PER_MEMBER;
                    if max_extent > extent_limit {
                        // Refuse to create this merge — it indicates corrupted vertex data.
                        // Despawn nothing; leave the source asteroids intact.
                        continue;
                    }

                    // Sum unit sizes of all cluster members
                    let total_size: u32 = cluster.iter().map(|(_, _, _, s)| s).sum();

                    let avg_color = Color::rgb(0.5, 0.5, 0.5);
                    let _composite = crate::asteroid::spawn_asteroid_with_vertices(
                        &mut commands,
                        hull_centroid,
                        &hull_local,
                        avg_color,
                        total_size,
                    );

                    // Update velocity
                    if let Some(mut cmd) = commands.get_entity(_composite) {
                        cmd.insert(Velocity {
                            linvel: avg_linvel,
                            angvel: avg_angvel,
                        });
                    }

                    // Track merge: N asteroids became 1, so we merged (N-1) asteroids
                    let merge_count = (cluster.len() - 1) as u32;
                    stats.merged_total += merge_count;

                    // Despawn all source asteroids
                    for (entity, _, _, _) in cluster {
                        commands.entity(entity).despawn();
                    }
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
        let f = gravity_force_between(Vec2::ZERO, Vec2::new(100.0, 0.0), 10.0, 1.0, 1_000_000.0)
            .expect("pair should be in range");
        assert!(f.x > 0.0, "force x should be positive (toward body j)");
        assert!(f.y.abs() < 1e-6, "no vertical component for horizontal pair");
    }

    #[test]
    fn gravity_inverse_square_law() {
        let g = 10.0_f32;
        let f1 = gravity_force_between(Vec2::ZERO, Vec2::new(10.0, 0.0), g, 1.0, 1_000_000.0).unwrap();
        let f2 = gravity_force_between(Vec2::ZERO, Vec2::new(20.0, 0.0), g, 1.0, 1_000_000.0).unwrap();
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
        let f = gravity_force_between(Vec2::ZERO, Vec2::new(d, 0.0), g, 1.0, 1_000_000.0).unwrap();
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
        );
        assert!(f.is_none(), "should be None when closer than min distance");
    }

    #[test]
    fn gravity_newtons_third_law() {
        let pos_i = Vec2::new(-50.0, 30.0);
        let pos_j = Vec2::new(70.0, -20.0);
        let f_ij = gravity_force_between(pos_i, pos_j, 10.0, 1.0, 1_000_000.0).unwrap();
        let f_ji = gravity_force_between(pos_j, pos_i, 10.0, 1.0, 1_000_000.0).unwrap();
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
        );
        assert!(f.is_some(), "exactly at boundary should still return force");
    }
}
