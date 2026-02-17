//! Simulation plugin and systems for Bevy ECS

use crate::particle::{spawn_particle, GroupId, Locked, NeighborCount, Particle};
use crate::rigid_body::rigid_body_formation_system;
use bevy::input::ButtonInput;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::Rng;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_initial_particles)
            .add_systems(
                Update,
                (
                    crate::graphics::debug_simulation_state,
                    random_particle_spawn_system,
                    neighbor_counting_system,
                    nbody_gravity_system,
                    collision_response_system,
                    particle_locking_system,
                    environmental_damping_system,
                    culling_system,
                    user_input_system,
                    rigid_body_formation_system,
                ),
            );
    }
}

// System: Spawns a batch of initial particles randomly distributed
pub fn spawn_initial_particles(mut commands: Commands) {
    let mut rng = rand::thread_rng();
    eprintln!("[SPAWN] Starting initial particle spawn...");
    for _ in 0..200 {
        let x = rng.gen_range(-100.0..100.0);
        let y = rng.gen_range(-80.0..80.0);
        let color = Color::rgb(
            rng.gen_range(0.3..1.0),
            rng.gen_range(0.3..1.0),
            rng.gen_range(0.3..1.0),
        );
        spawn_particle(&mut commands, Vec2::new(x, y), color, 0);
    }
    eprintln!("[SPAWN] Created 200 particles");
}

// System: Spawns a particle at random every second (for demo/testing)
pub fn random_particle_spawn_system(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: Local<Timer>,
) {
    if timer.tick(time.delta()).just_finished() {
        let mut rng = rand::thread_rng();
        let x = rng.gen_range(-120.0..120.0);
        let y = rng.gen_range(-100.0..100.0);
        let color = Color::rgb(
            rng.gen_range(0.3..1.0),
            rng.gen_range(0.3..1.0),
            rng.gen_range(0.3..1.0),
        );
        spawn_particle(&mut commands, Vec2::new(x, y), color, 0);
    }
}

// System: lock particles together when slow and in contact
pub fn particle_locking_system(
    mut query: Query<(Entity, &mut Velocity, &mut Locked, &mut GroupId), With<Particle>>,
    rapier_context: Res<RapierContext>,
) {
    let velocity_threshold = 5.0;
    let mut group_counter = 1u32;
    let mut locked_pairs = Vec::new();
    let entities: Vec<_> = query
        .iter_mut()
        .map(|(e, v, l, g)| (e, v.linvel.length(), l.0, g.0))
        .collect();

    #[allow(clippy::needless_range_loop)]
    for i in 0..entities.len() {
        let (e1, v1, _l1, _g1) = entities[i];
        for j in (i + 1)..entities.len() {
            let (e2, v2, _l2, _g2) = entities[j];
            if v1 < velocity_threshold && v2 < velocity_threshold {
                if let Some(contact) = rapier_context.contact_pair(e1, e2) {
                    if contact.has_any_active_contacts() {
                        locked_pairs.push((e1, e2));
                    }
                }
            }
        }
    }
    for (e1, e2) in locked_pairs {
        if let Ok([(_, mut _v1, mut l1, mut g1), (_, mut _v2, mut l2, mut g2)]) =
            query.get_many_mut([e1, e2])
        {
            l1.0 = true;
            l2.0 = true;
            if g1.0 == 0 && g2.0 == 0 {
                g1.0 = group_counter;
                g2.0 = group_counter;
                group_counter += 1;
            } else if g1.0 == 0 {
                g1.0 = g2.0;
            } else if g2.0 == 0 {
                g2.0 = g1.0;
            }
        }
    }
}

// N-body gravity system: applies custom gravity between all particles
pub fn nbody_gravity_system(
    mut query: Query<(Entity, &Transform, &mut ExternalForce, Option<&GroupId>), With<Particle>>,
) {
    let gravity_const = 15.0;
    let min_dist = 100.0;
    let mut entities: Vec<(Entity, Vec2, Option<GroupId>)> = Vec::new();
    for (entity, transform, _force, group) in query.iter_mut() {
        entities.push((entity, transform.translation.truncate(), group.copied()));
    }
    #[allow(clippy::needless_range_loop)]
    for i in 0..entities.len() {
        let (entity_i, pos_i, _group_i) = entities[i];
        for j in (i + 1)..entities.len() {
            let (entity_j, pos_j, _group_j) = entities[j];
            let delta = pos_j - pos_i;
            let dist_sq = delta.length_squared().max(min_dist * min_dist);
            let force_mag = gravity_const / dist_sq;
            let force = delta.normalize_or_zero() * force_mag;
            if let Ok((_, _, mut force_i, _)) = query.get_mut(entity_i) {
                force_i.force += force;
            }
            if let Ok((_, _, mut force_j, _)) = query.get_mut(entity_j) {
                force_j.force -= force;
            }
        }
    }
}

// System: handle user input for spawning and explosions
pub fn user_input_system(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut query: Query<(&Transform, &mut Velocity), With<Particle>>,
) {
    let window = windows.single();
    if let Some(cursor_pos) = window.cursor_position() {
        let world_pos = Vec2::new(
            cursor_pos.x - window.width() / 2.0,
            cursor_pos.y - window.height() / 2.0,
        );
        if buttons.just_pressed(MouseButton::Left) {
            spawn_particle(&mut commands, world_pos, Color::rgb(1.0, 0.8, 0.8), 0);
        }
        if buttons.just_pressed(MouseButton::Right) {
            // Apply explosion force to nearby particles
            let explosion_force = 100.0;
            let explosion_radius = 50.0;
            for (transform, mut velocity) in query.iter_mut() {
                let particle_pos = transform.translation.truncate();
                let diff = particle_pos - world_pos;
                let dist = diff.length();
                if dist < explosion_radius && dist > 0.1 {
                    let force = (diff.normalize() * explosion_force) / (1.0 + dist / 10.0);
                    velocity.linvel += force;
                }
            }
        }
    }
}

// System: count neighbors for each particle (for environmental damping)
pub fn neighbor_counting_system(
    mut query: Query<(Entity, &Transform, &mut NeighborCount), With<Particle>>,
) {
    let neighbor_threshold = 3.0;

    // Collect all positions and entities
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

// System: apply environmental damping to tightly packed particles
pub fn environmental_damping_system(
    mut query: Query<(&NeighborCount, &mut Velocity), With<Particle>>,
) {
    let tight_packing_threshold = 6; // If >6 neighbors within 3.0 units
    let base_damping = 0.005; // 0.5% damping

    for (neighbor_count, mut velocity) in query.iter_mut() {
        if neighbor_count.0 > tight_packing_threshold {
            velocity.linvel *= 1.0 - base_damping;
            velocity.angvel *= 1.0 - base_damping;
        }
    }
}

// System: cull particles far off-screen
pub fn culling_system(mut commands: Commands, query: Query<(Entity, &Transform), With<Particle>>) {
    let cull_distance = 200.0;
    for (entity, transform) in query.iter() {
        if transform.translation.truncate().length() > cull_distance {
            commands.entity(entity).despawn();
        }
    }
}

// System: handle collision responses (restitution, minimal damping)
pub fn collision_response_system(
    rapier_context: Res<RapierContext>,
    mut query: Query<(&Transform, &mut Velocity, &Restitution), With<Particle>>,
) {
    let mut collisions = Vec::new();
    for contact_pair in rapier_context.contact_pairs() {
        if contact_pair.has_any_active_contacts() {
            collisions.push((contact_pair.collider1(), contact_pair.collider2()));
        }
    }

    // Apply post-collision damping (3% as per spec)
    for (_, mut velocity, _restitution) in query.iter_mut() {
        velocity.linvel *= 0.97; // 3% damping
    }
}
