//! Simulation plugin and systems for Bevy ECS

use crate::asteroid::{compute_convex_hull_from_points, Asteroid, NeighborCount, Vertices};
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
                settling_damping_system,       // Apply settling damping to slow asteroids
                particle_locking_system,
                environmental_damping_system,
                user_input_system,
                gizmo_rendering_system,
            ),
        )
        // Move asteroid_formation_system to PostUpdate so it runs AFTER Rapier physics computes contacts
        .add_systems(PostUpdate, asteroid_formation_system);
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
    let gravity_const = 2.0;  // Very gentle to allow contact without velocity blowup
    let min_dist = 2.0;  // Minimum distance before clamping
    let max_gravity_dist = 300.0; // Shorter range

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

/// Apply settling damping to slow asteroids to reduce spinning and help them settle into clusters
pub fn settling_damping_system(
    mut query: Query<&mut Velocity, With<Asteroid>>,
) {
    let slow_threshold = 3.0; // Asteroids moving slower than this experience settling damping
    let settling_damping = 0.01; // 1% velocity reduction per frame (much gentler)

    for mut velocity in query.iter_mut() {
        if velocity.linvel.length() < slow_threshold {
            // Slow asteroids lose energy through "settling"
            velocity.linvel *= 1.0 - settling_damping;
            velocity.angvel *= 1.0 - settling_damping;
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

/// Cull asteroids far off-screen
pub fn culling_system(mut commands: Commands, query: Query<(Entity, &Transform), With<Asteroid>>) {
    let cull_distance = 1000.0; // Cull just beyond the gravity interaction range
    
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
    query: Query<(Entity, &Transform, &Velocity, &Vertices), With<Asteroid>>,
    rapier_context: Res<RapierContext>,
) {
    // Find clusters of slow-moving asteroids that are touching
    let velocity_threshold = 10.0;
    let mut processed = std::collections::HashSet::new();
    
    let asteroids: Vec<_> = query.iter().collect();
    
    for i in 0..asteroids.len() {
        let (e1, t1, v1, _verts1) = asteroids[i];
        
        if processed.contains(&e1) {
            continue;
        }
        
        // Only start clusters from slow asteroids
        if v1.linvel.length() > velocity_threshold {
            continue;
        }
        
        // Flood-fill to find all connected slow asteroids
        // Store: (entity, transform, vertices)
        let mut cluster: Vec<(Entity, &Transform, &Vertices)> = vec![(e1, t1, _verts1)];
        let mut queue = vec![e1];
        let mut visited = std::collections::HashSet::new();
        visited.insert(e1);
        
        while let Some(current) = queue.pop() {
            for j in 0..asteroids.len() {
                let (e2, t2, v2, verts2) = asteroids[j];
                
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
                        cluster.push((e2, t2, verts2));
                    }
                }
            }
        }
        
        // If we have 2+ asteroids in the cluster, merge them
        if cluster.len() >= 2 {
            // Mark all as processed
            for (entity, _, _) in &cluster {
                processed.insert(*entity);
            }
            
            // Compute center position
            let mut center = Vec2::ZERO;
            let mut avg_linvel = Vec2::ZERO;
            let mut avg_angvel = 0.0;
            
            for (entity, t, _) in &cluster {
                let pos = t.translation.truncate();
                center += pos;
                // Get velocity from the cached query data
                for (e, _, vel, _) in &asteroids {
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
            for (_entity, transform, vertices) in &cluster {
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
                    // Convert hull back to local-space relative to center
                    let hull_local: Vec<Vec2> = hull.iter()
                        .map(|v| *v - center)
                        .collect();
                    
                    let avg_color = Color::rgb(0.5, 0.5, 0.5);
                    let _composite = crate::asteroid::spawn_asteroid_with_vertices(&mut commands, center, &hull_local, avg_color);
                    
                    // Update velocity
                    if let Some(mut cmd) = commands.get_entity(_composite) {
                        cmd.insert(Velocity {
                            linvel: avg_linvel,
                            angvel: avg_angvel,
                        });
                    }
                    
                    // Despawn all source asteroids
                    for (entity, _, _) in cluster {
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