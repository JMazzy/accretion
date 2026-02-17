//! Rigid body ECS components and formation system for Bevy

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use std::collections::HashMap;

/// Marker for a rigid body group entity
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub struct RigidBodyGroup;

/// Stores color blend of forming rigid body
#[derive(Component, Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct RigidBodyColor(pub Color);

/// System: form rigid bodies from locked groups of particles
pub fn rigid_body_formation_system(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &Transform,
            &super::particle::GroupId,
            &super::particle::Locked,
            &super::particle::ParticleColor,
        ),
        With<super::particle::Particle>,
    >,
) {
    // Group particles by GroupId
    let mut groups: HashMap<u32, Vec<(Entity, Vec2, Color)>> = HashMap::new();
    for (entity, transform, group_id, locked, color) in query.iter() {
        if group_id.0 > 0 && locked.0 {
            let pos = transform.translation.truncate();
            groups
                .entry(group_id.0)
                .or_default()
                .push((entity, pos, color.0));
        }
    }

    // Check for groups with >= 3 particles
    for (_group_id, particles) in groups {
        if particles.len() >= 3 {
            // Compute center of mass
            let center: Vec2 =
                particles.iter().map(|(_, pos, _)| pos).sum::<Vec2>() / particles.len() as f32;

            // Compute convex hull (simple gift wrapping algorithm)
            if let Some(hull) = compute_convex_hull(&particles) {
                // Compute bounding radius
                let bounding_radius = hull
                    .iter()
                    .map(|&p| (p - center).length())
                    .fold(0.0, f32::max);

                // Blend colors
                let avg_color = blend_colors(&particles);

                // Compute moment of inertia
                let _moment_of_inertia: f32 = particles
                    .iter()
                    .map(|(_, pos, _)| (*pos - center).length_squared())
                    .sum();

                // Create composite rigid body (as sphere for simplicity in this version)
                // In a more complete implementation, use a convex polygon
                commands.spawn((
                    RigidBodyGroup,
                    RigidBodyColor(avg_color),
                    RigidBody::Dynamic,
                    Collider::ball(bounding_radius.max(2.0)),
                    Restitution::coefficient(0.7),
                    Transform::from_translation(center.extend(0.0)),
                    GlobalTransform::default(),
                    Velocity::zero(),
                    Damping {
                        linear_damping: 0.0,
                        angular_damping: 0.0,
                    },
                    ExternalForce {
                        force: Vec2::ZERO,
                        torque: 0.0,
                    },
                ));

                // Despawn original particles
                for (entity, _, _) in particles {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

/// Simple convex hull computation using gift wrapping (Graham scan variant)
fn compute_convex_hull(particles: &[(Entity, Vec2, Color)]) -> Option<Vec<Vec2>> {
    if particles.len() < 3 {
        return None;
    }

    let points: Vec<Vec2> = particles.iter().map(|(_, p, _)| *p).collect();

    // Find leftmost point
    let mut min_idx = 0;
    for i in 1..points.len() {
        if points[i].x < points[min_idx].x
            || (points[i].x == points[min_idx].x && points[i].y < points[min_idx].y)
        {
            min_idx = i;
        }
    }

    let mut hull = Vec::new();
    let mut current = min_idx;

    loop {
        hull.push(points[current]);
        let mut next = (current + 1) % points.len();

        for i in 0..points.len() {
            if cross_product(points[current], points[next], points[i]) > 0.0 {
                next = i;
            }
        }

        current = next;
        if current == min_idx {
            break;
        }
    }

    Some(hull)
}

/// Cross product to determine turn direction
fn cross_product(o: Vec2, a: Vec2, b: Vec2) -> f32 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

/// Blend colors by averaging RGB values
fn blend_colors(particles: &[(Entity, Vec2, Color)]) -> Color {
    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;
    let mut _a = 0.0;

    for (_, _, color) in particles {
        r += color.r();
        g += color.g();
        b += color.b();
        _a += color.a();
    }

    let count = particles.len() as f32;
    Color::rgb(r / count, g / count, b / count)
}
