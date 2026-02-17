//! Simulation plugin and systems for Bevy ECS

use crate::asteroid::{blend_colors, compute_convex_hull, Asteroid, NeighborCount, Vertices};
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                culling_system,                // FIRST: Remove far asteroids before physics
                neighbor_counting_system,
                nbody_gravity_system,
                particle_locking_system,
                environmental_damping_system,
                asteroid_formation_system,
                user_input_system,
                gizmo_rendering_system,
            ),
        );
    }
}

/// Lock and merge asteroids when they're slow and touching
/// This replaces the grouping system with direct pairwise merging
pub fn particle_locking_system(
    mut query: Query<(Entity, &Transform, &mut Velocity), With<Asteroid>>,
    rapier_context: Res<RapierContext>,
) {
    let velocity_threshold = 5.0;
    let mut pairs_to_merge: Vec<(Entity, Entity)> = Vec::new();

    // Find touching asteroids that are slow enough to merge
    let entities: Vec<_> = query.iter().map(|(e, _, _)| e).collect();
    
    for i in 0..entities.len() {
        let e1 = entities[i];
        for j in (i + 1)..entities.len() {
            let e2 = entities[j];
            
            if let (Ok((_, _, v1)), Ok((_, _, v2))) = (query.get(e1), query.get(e2)) {
                if v1.linvel.length() < velocity_threshold && v2.linvel.length() < velocity_threshold {
                    if let Some(contact) = rapier_context.contact_pair(e1, e2) {
                        if contact.has_any_active_contacts() {
                            pairs_to_merge.push((e1, e2));
                        }
                    }
                }
            }
        }
    }

    // Merge pairs: sync velocities
    for (e1, e2) in pairs_to_merge {
        if let Ok([(_, _, mut v1), (_, _, mut v2)]) = query.get_many_mut([e1, e2]) {
            let avg_linvel = (v1.linvel + v2.linvel) * 0.5;
            let avg_angvel = (v1.angvel + v2.angvel) * 0.5;
            v1.linvel = avg_linvel;
            v1.angvel = avg_angvel;
            v2.linvel = avg_linvel;
            v2.angvel = avg_angvel;
        }
    }
}

/// N-body gravity system: applies custom gravity between all asteroids
/// Only asteroids within viewport + buffer zone apply gravity
pub fn nbody_gravity_system(
    mut query: Query<(Entity, &Transform, &mut ExternalForce), With<Asteroid>>,
) {
    let gravity_const = 10.0;  // Reduced from 15.0 to prevent runaway acceleration
    let min_dist = 150.0;  // Increased to reduce instability at contact ranges
    let max_gravity_dist = 800.0; // Only apply gravity to nearby asteroids

    // Collect positions
    let mut entities: Vec<(Entity, Vec2)> = Vec::new();
    for (entity, transform, _) in query.iter_mut() {
        entities.push((entity, transform.translation.truncate()));
    }

    // Apply gravity between nearby asteroid pairs only
    for i in 0..entities.len() {
        let (entity_i, pos_i) = entities[i];
        for j in (i + 1)..entities.len() {
            let (entity_j, pos_j) = entities[j];
            let delta = pos_j - pos_i;
            let dist = delta.length();
            
            // Skip gravity if asteroids are too far apart (prevents phantom forces from distant asteroids)
            if dist > max_gravity_dist {
                continue;
            }
            
            let dist_sq = (dist * dist).max(min_dist * min_dist);
            let force_mag = gravity_const / dist_sq;
            let force = delta.normalize_or_zero() * force_mag;

            if let Ok((_, _, mut force_i)) = query.get_mut(entity_i) {
                force_i.force += force;
            }
            if let Ok((_, _, mut force_j)) = query.get_mut(entity_j) {
                force_j.force -= force;
            }
        }
    }
}

/// Handle user input for spawning asteroids (click only)
pub fn user_input_system(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
) {
    let window = windows.single();
    if let Some(cursor_pos) = window.cursor_position() {
        // Convert from screen coordinates to world coordinates
        let world_x = cursor_pos.x - window.width() / 2.0;
        let world_y = -(cursor_pos.y - window.height() / 2.0);
        let world_pos = Vec2::new(world_x, world_y);

        if buttons.just_pressed(MouseButton::Left) {
            crate::asteroid::spawn_asteroid(&mut commands, world_pos, Color::WHITE, 0);
        }
    }
}

/// Count neighbors for each asteroid (for environmental damping)
pub fn neighbor_counting_system(
    mut query: Query<(Entity, &Transform, &mut NeighborCount), With<Asteroid>>,
) {
    let neighbor_threshold = 3.0;

    // Collect all positions
    let particles: Vec<(Entity, Vec2)> = query
        .iter()
        .map(|(e, t, _)| (e, t.translation.truncate()))
        .collect();

    // Count neighbors for each particle
    for (i, &(entity_i, pos_i)) in particles.iter().enumerate() {
        let mut count = 0;
        for (j, &(_, pos_j)) in particles.iter().enumerate() {
            if i != j && (pos_i - pos_j).length() < neighbor_threshold {
                count += 1;
            }
        }
        if let Ok((_, _, mut nc)) = query.get_mut(entity_i) {
            nc.0 = count;
        }
    }
}

/// Apply environmental damping to tightly packed asteroids
pub fn environmental_damping_system(
    mut query: Query<(&NeighborCount, &mut Velocity), With<Asteroid>>,
) {
    let tight_packing_threshold = 6;
    let base_damping = 0.005; // 0.5% damping

    for (neighbor_count, mut velocity) in query.iter_mut() {
        if neighbor_count.0 > tight_packing_threshold {
            velocity.linvel *= 1.0 - base_damping;
            velocity.angvel *= 1.0 - base_damping;
        }
    }
}

/// Cull asteroids far off-screen and apply damping to distant ones
pub fn culling_system(mut commands: Commands, mut query: Query<(Entity, &Transform, &mut Velocity), With<Asteroid>>) {
    let cull_distance = 1000.0;  // Reduced to prevent accumulation
    let damping_distance = 400.0; // Start damping asteroids beyond viewport
    
    for (entity, transform, mut velocity) in query.iter_mut() {
        let dist = transform.translation.truncate().length();
        
        // Cull asteroids that are very far
        if dist > cull_distance {
            commands.entity(entity).despawn();
        }
        // Apply exponential damping to distant asteroids to prevent them from flying too far
        // This prevents asteroids from coasting indefinitely if they gain velocity from gravity
        else if dist > damping_distance {
            // Exponential ramp: starts at ~7% and reaches ~30% damping
            let t = (dist - damping_distance) / (cull_distance - damping_distance);
            let damping_factor = (0.93 * t * t) + (0.97 * (1.0 - t)); // Smooth exponential transition
            velocity.linvel *= damping_factor;
            velocity.angvel *= damping_factor;
        }
    }
}

/// Form large asteroids by detecting clusters of touching asteroids
/// and converting them into larger polygons
pub fn asteroid_formation_system(
    mut commands: Commands,
    query: Query<(Entity, &Transform, &Velocity, &crate::asteroid::AsteroidSize), With<Asteroid>>,
    rapier_context: Res<RapierContext>,
) {
    // Find clusters of slow-moving asteroids that are touching
    let velocity_threshold = 2.5; // Increased from 1.0 to make combining easier
    let mut processed = std::collections::HashSet::new();
    
    let asteroids: Vec<_> = query.iter().collect();
    
    for i in 0..asteroids.len() {
        if processed.contains(&asteroids[i].0) {
            continue; // Already processed/merged
        }
        
        let (e1, t1, v1, s1) = asteroids[i];
        
        // Only start clusters from small asteroids
        if !matches!(s1, crate::asteroid::AsteroidSize::Small) {
            continue;
        }
        
        if v1.linvel.length() > velocity_threshold {
            continue; // Too fast to merge
        }
        
        // Find all asteroids touching this one
        let mut cluster = vec![(e1, t1.translation.truncate(), *v1, s1)];
        
        for j in (i + 1)..asteroids.len() {
            if processed.contains(&asteroids[j].0) {
                continue;
            }
            
            let (e2, t2, v2, s2) = asteroids[j];
            
            // Only cluster small asteroids
            if !matches!(s2, crate::asteroid::AsteroidSize::Small) {
                continue;
            }
            
            if v2.linvel.length() > velocity_threshold {
                continue;
            }
            
            // Check if they're touching
            if let Some(contact) = rapier_context.contact_pair(e1, e2) {
                if contact.has_any_active_contacts() {
                    cluster.push((e2, t2.translation.truncate(), *v2, s2));
                }
            }
        }
        
        // If we have 2+ asteroids in the cluster, merge them
        if cluster.len() >= 2 {
            // Mark all as processed
            for (entity, _, _, _) in &cluster {
                processed.insert(*entity);
            }
            
            // Compute center and average velocity
            let mut center = Vec2::ZERO;
            let mut avg_linvel = Vec2::ZERO;
            let mut avg_angvel = 0.0;
            
            for (_, pos, vel, _) in &cluster {
                center += *pos;
                avg_linvel += vel.linvel;
                avg_angvel += vel.angvel;
            }
            
            let count = cluster.len() as f32;
            center /= count;
            avg_linvel /= count;
            avg_angvel /= count;
            
            // Build hull from cluster positions
            let positions: Vec<(Entity, Vec2, Color)> = cluster
                .iter()
                .map(|(entity, pos, _, _)| (*entity, *pos, Color::rgb(0.5, 0.5, 0.5)))
                .collect();
            
            // Compute convex hull
            if let Some(hull) = compute_convex_hull(&positions) {
                if hull.len() >= 2 {
                    let avg_color = blend_colors(&positions);
                    
                    // Spawn composite
                    let composite = crate::asteroid::spawn_large_asteroid(&mut commands, center, &hull, avg_color);
                    if let Some(mut cmd) = commands.get_entity(composite) {
                        cmd.insert(Velocity {
                            linvel: avg_linvel,
                            angvel: avg_angvel,
                        });
                    }
                    
                    // Despawn all source asteroids
                    for (entity, _, _, _) in cluster {
                        commands.entity(entity).despawn();
                    }
                }
            }
        }
    }
}

/// Render asteroid outlines using gizmos (wireframe visualization with rotation)
pub fn gizmo_rendering_system(
    mut gizmos: Gizmos,
    query: Query<(&Transform, &Vertices), With<Asteroid>>,
) {
    for (transform, vertices) in query.iter() {
        let pos = transform.translation.truncate();
        if vertices.0.len() < 2 {
            continue;
        }

        // Extract rotation from transform
        let rotation = transform.rotation;
        
        // Draw polygon outline with rotation applied
        for i in 0..vertices.0.len() {
            let v1 = vertices.0[i];
            let v2 = vertices.0[(i + 1) % vertices.0.len()];
            
            // Rotate vertices by transform rotation
            let p1 = pos + rotation.mul_vec3(v1.extend(0.0)).truncate();
            let p2 = pos + rotation.mul_vec3(v2.extend(0.0)).truncate();
            
            gizmos.line_2d(p1, p2, Color::WHITE);
        }
    }
}