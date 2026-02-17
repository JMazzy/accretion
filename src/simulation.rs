//! Simulation plugin and systems for Bevy ECS

use crate::asteroid::{compute_convex_hull_from_points, Asteroid, NeighborCount, Vertices};
use bevy::input::ButtonInput;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

/// Tracks simulation statistics: active asteroids, culled count, merged count
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct SimulationStats {
    pub live_count: u32,
    pub culled_total: u32,
    pub merged_total: u32,
}

/// Camera state for pan/zoom controls
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct CameraState {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
}

const MAX_PAN_DISTANCE: f32 = 600.0;
const MIN_ZOOM: f32 = 0.5;  // 0.5 scale = see full ~2000u circle
const MAX_ZOOM: f32 = 8.0;   // 8.0 scale = 4x magnification
const ZOOM_SPEED: f32 = 0.1; // Speed of zoom per scroll

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimulationStats::default())
           .insert_resource(CameraState { pan_x: 0.0, pan_y: 0.0, zoom: 1.0 })
           .add_systems(
            Update,
            (
                stats_counting_system,         // FIRST: Count asteroids for tracking
                culling_system,                // Remove far asteroids before physics
                neighbor_counting_system,
                settling_damping_system,       // Apply settling damping to slow asteroids
                particle_locking_system,
                environmental_damping_system,
                user_input_system,             // Now handles keyboard + mouse input
                camera_pan_system,             // Update camera position from input
                camera_zoom_system,            // Update camera zoom from input
                gizmo_rendering_system,        // Render asteroids + boundary
                stats_display_system,          // Render stats text
            ),
        )
        // Run gravity in FixedUpdate (same schedule as Rapier physics) to prevent force accumulation
        .add_systems(FixedUpdate, nbody_gravity_system)
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
    let gravity_const = 10.0;  // Increased for more noticeable attraction
    let min_gravity_dist = 5.0;  // Skip gravity entirely if asteroids are closer than this - prevents runaway acceleration
    let max_gravity_dist = 300.0; // Shorter range

    // CRITICAL: Reset all forces to zero first, then calculate fresh
    // This prevents accumulation bugs and ensures forces reflect current positions
    for (_, _, mut force) in query.iter_mut() {
        force.force = Vec2::ZERO;
        force.torque = 0.0;
    }

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
            
            // Skip gravity if asteroids are too close or too far (prevents both singularities and phantom forces)
            if dist < min_gravity_dist || dist > max_gravity_dist {
                continue;
            }
            
            let dist_sq = dist * dist;
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

/// Handle user input: spawning (left-click), camera pan (arrow keys), zoom (mouse wheel)
pub fn user_input_system(
    mut commands: Commands,
    mut camera_state: ResMut<CameraState>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut scroll_evr: EventReader<MouseWheel>,
    windows: Query<&Window>,
) {
    let window = windows.single();
    
    // Spawning: left-click with camera-aware coordinates
    if let Some(cursor_pos) = window.cursor_position() {
        // Convert from screen coordinates to world coordinates, accounting for camera pan and zoom
        let norm_x = (cursor_pos.x - window.width() / 2.0) * camera_state.zoom;
        let norm_y = -(cursor_pos.y - window.height() / 2.0) * camera_state.zoom;
        let world_x = norm_x + camera_state.pan_x;
        let world_y = norm_y + camera_state.pan_y;
        let world_pos = Vec2::new(world_x, world_y);

        if buttons.just_pressed(MouseButton::Left) {
            crate::asteroid::spawn_asteroid(&mut commands, world_pos, Color::WHITE, 0);
        }
    }
    
    // Camera panning: arrow keys
    let pan_speed = 5.0;
    if keys.pressed(KeyCode::ArrowUp) {
        camera_state.pan_y = (camera_state.pan_y + pan_speed).clamp(-MAX_PAN_DISTANCE, MAX_PAN_DISTANCE);
    }
    if keys.pressed(KeyCode::ArrowDown) {
        camera_state.pan_y = (camera_state.pan_y - pan_speed).clamp(-MAX_PAN_DISTANCE, MAX_PAN_DISTANCE);
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        camera_state.pan_x = (camera_state.pan_x - pan_speed).clamp(-MAX_PAN_DISTANCE, MAX_PAN_DISTANCE);
    }
    if keys.pressed(KeyCode::ArrowRight) {
        camera_state.pan_x = (camera_state.pan_x + pan_speed).clamp(-MAX_PAN_DISTANCE, MAX_PAN_DISTANCE);
    }
    
    // Zoom: mouse wheel
    for ev in scroll_evr.read() {
        // In Bevy 0.13, MouseWheel has y field for scroll delta
        let delta = ev.y;
        camera_state.zoom = (camera_state.zoom - (delta * ZOOM_SPEED)).clamp(MIN_ZOOM, MAX_ZOOM);
    }
}

/// Apply camera pan/zoom state to the camera entity
pub fn camera_pan_system(
    camera_state: Res<CameraState>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
) {
    for mut transform in camera_query.iter_mut() {
        transform.translation = Vec3::new(camera_state.pan_x, camera_state.pan_y, transform.translation.z);
        transform.scale = Vec3::new(camera_state.zoom, camera_state.zoom, 1.0);
    }
}

/// Process zoom changes (handled in camera_pan_system)
pub fn camera_zoom_system() {
    // Zoom is now applied directly in camera_pan_system via transform.scale
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

/// Track statistics: live count, culled count, merged count
/// Must run BEFORE culling_system to detect which asteroids are about to be culled
pub fn stats_counting_system(
    mut stats: ResMut<SimulationStats>,
    query: Query<(Entity, &Transform), With<Asteroid>>,
) {
    let cull_distance = 1000.0;
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
    let cull_distance = 1000.0; // Cull just beyond the gravity interaction range
    
    for (entity, transform) in query.iter() {
        let dist = transform.translation.truncate().length();
        
        // Cull asteroids that drift very far
        if dist > cull_distance {
            commands.entity(entity).despawn();
        }
    }
}

/// Check if a point is inside a convex polygon using cross product method
/// Assumes vertices are in counter-clockwise order
fn is_point_in_convex_polygon(point: &Vec2, polygon: &[Vec2]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    
    // For a convex polygon, the point is inside if it's on the same side
    // of all edges (all cross products have the same sign)
    let mut sign = None;
    
    for i in 0..polygon.len() {
        let v1 = polygon[i];
        let v2 = polygon[(i + 1) % polygon.len()];
        
        // Edge vector
        let edge = v2 - v1;
        // Vector from edge start to point
        let to_point = *point - v1;
        
        // Cross product (2D: returns scalar)
        let cross = edge.x * to_point.y - edge.y * to_point.x;
        
        // Skip edges that are colinear with the point
        if cross.abs() < 0.0001 {
            continue;
        }
        
        match sign {
            None => sign = Some(cross > 0.0),
            Some(positive) => {
                if (cross > 0.0) != positive {
                    return false;  // Point is outside
                }
            }
        }
    }
    
    true  // Point is inside (or on boundary)
}

/// Form large asteroids by detecting clusters of touching asteroids
/// and converting them into larger polygons
pub fn asteroid_formation_system(
    mut commands: Commands,
    query: Query<(Entity, &Transform, &Velocity, &Vertices), With<Asteroid>>,
    rapier_context: Res<RapierContext>,
    mut stats: ResMut<SimulationStats>,
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
                    // IMPORTANT: Check if any other asteroids are inside this hull
                    // If so, add them to the cluster to prevent flinging
                    let mut additional_count = 0;
                    for j in 0..asteroids.len() {
                        let (e_check, t_check, v_check, verts_check) = asteroids[j];
                        
                        // Skip if already in cluster or processed
                        if processed.contains(&e_check) || cluster.iter().any(|(e, _, _)| *e == e_check) {
                            continue;
                        }
                        
                        // Check if this asteroid's center is inside the hull
                        let check_pos = t_check.translation.truncate();
                        if is_point_in_convex_polygon(&check_pos, &hull) {
                            // Mark as processed and add to cluster
                            processed.insert(e_check);
                            cluster.push((e_check, t_check, verts_check));
                            
                            // Add its vertices to the hull computation
                            let rotation = t_check.rotation;
                            let offset = t_check.translation.truncate();
                            for local_v in &verts_check.0 {
                                let world_v = offset + rotation.mul_vec3(local_v.extend(0.0)).truncate();
                                world_vertices.push(world_v);
                            }
                            
                            // Update center and velocity averages with incremental formula
                            let current_count = count + additional_count as f32;
                            let new_count = current_count + 1.0;
                            center = (center * current_count + check_pos) / new_count;
                            avg_linvel = (avg_linvel * current_count + v_check.linvel) / new_count;
                            avg_angvel = (avg_angvel * current_count + v_check.angvel) / new_count;
                            
                            additional_count += 1;
                        }
                    }
                    
                    // If we found additional asteroids inside, recompute hull with all vertices
                    let final_hull = if additional_count > 0 {
                        compute_convex_hull_from_points(&world_vertices).unwrap_or(hull.clone())
                    } else {
                        hull.clone()
                    };
                    
                    // Convert hull back to local-space relative to center
                    let hull_local: Vec<Vec2> = final_hull.iter()
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
                    
                    // Track merge: N asteroids became 1, so we merged (N-1) asteroids
                    let merge_count = (cluster.len() - 1) as u32;
                    stats.merged_total += merge_count;
                    
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
    query: Query<(&Transform, &Vertices, &ExternalForce), With<Asteroid>>,
) {
    // Draw asteroids
    for (transform, vertices, force) in query.iter() {
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
        
        // Draw force vector (red line from asteroid center showing current force)
        // Scale the force vector for visibility (multiply by 80 for better visibility)
        let force_vec = force.force * 80.0;
        if force_vec.length() > 0.1 {  // Only draw if force is significant
            gizmos.line_2d(pos, pos + force_vec, Color::rgb(1.0, 0.0, 0.0));
        }
    }
    
    // Draw culling boundary circle at origin (yellow)
    let cull_distance = 1000.0;
    gizmos.circle_2d(Vec2::ZERO, cull_distance, Color::rgb(1.0, 1.0, 0.0));
}

/// Marker component for the stats text display
#[derive(Component)]
pub struct StatsTextDisplay;

/// Initialize stats text display entity on startup
pub fn setup_stats_text(
    mut commands: Commands,
) {
    // Create UI text that stays fixed on screen (unaffected by camera zoom/pan)
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Live: 0 | Culled: 0 | Merged: 0",
                TextStyle {
                    font: Handle::default(),
                    font_size: 20.0,
                    color: Color::rgb(0.0, 1.0, 1.0),
                },
            ));
        })
        .insert(StatsTextDisplay);
}

/// Update stats text display each frame (content only - position is fixed in screen space)
pub fn stats_display_system(
    stats: Res<SimulationStats>,
    parent_query: Query<&Children, With<StatsTextDisplay>>,
    mut text_query: Query<&mut Text>,
) {
    // Find the Text child of our StatsTextDisplay parent node
    for children in parent_query.iter() {
        for &child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                text.sections[0].value = format!(
                    "Live: {} | Culled: {} | Merged: {}",
                    stats.live_count, stats.culled_total, stats.merged_total
                );
            }
        }
    }
}
