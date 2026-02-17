//! Unified asteroid component and utilities
//!
//! All simulation entities are asteroids. Smaller ones are triangles,
//! larger ones are convex polygons formed from groups.

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::Rng;

/// Marker component for any asteroid entity
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Asteroid;

/// Size classification: Small = spawned triangles, Large = formed from groups
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsteroidSize {
    Small,
    Large,
}

/// Optional grouping for asteroids that are locked together
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupId(pub u32);

/// Whether this asteroid is locked to others in its group
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Locked(pub bool);

/// Count of nearby asteroids for environmental damping calculation
#[derive(Component, Debug, Clone, Copy)]
pub struct NeighborCount(pub usize);

/// Polygon vertices for wireframe rendering
#[derive(Component, Debug, Clone)]
pub struct Vertices(pub Vec<Vec2>);

/// Spawns a small asteroid as an equilateral triangle at the given position
pub fn spawn_asteroid(commands: &mut Commands, position: Vec2, _color: Color, group_id: u32) {
    // Generate a random grey shade for variety
    let grey = rand::random::<f32>() * 0.6 + 0.3;
    let color = Color::rgb(grey, grey, grey);

    // Create equilateral triangle (6 unit side length)
    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),                       // Top
        Vec2::new(-side / 2.0, -height / 2.0),              // Bottom-left
        Vec2::new(side / 2.0, -height / 2.0),               // Bottom-right
    ];

    commands.spawn((
        // Rendering - small grey triangle (stored as vertices for wireframe)
        SpriteBundle {
            sprite: Sprite {
                color,
                custom_size: Some(Vec2::splat(3.0)),
                ..Default::default()
            },
            transform: Transform::from_translation(position.extend(0.1)),
            ..Default::default()
        },
        // Marker and classification
        Asteroid,
        AsteroidSize::Small,
        GroupId(group_id),
        Locked(false),
        NeighborCount(0),
        Vertices(vertices),
        // Physics
        RigidBody::Dynamic,
        Collider::ball(2.0),
        Restitution::coefficient(0.5),
        Friction::coefficient(0.3),
        Velocity::zero(),
        Damping {
            linear_damping: 0.0,
            angular_damping: 0.0,
        },
        ExternalForce {
            force: Vec2::ZERO,
            torque: 0.0,
        },
        Sleeping::disabled(),
    ));
}

/// Spawns a large asteroid from locked group (polygon formed from combined asteroids)
pub fn spawn_large_asteroid(
    commands: &mut Commands,
    center: Vec2,
    hull: &[Vec2],
    _avg_color: Color,
) -> Entity {
    // Create polygon collider from convex hull
    let hull_relative: Vec<Vec2> = hull.iter().map(|p| *p - center).collect();
    let collider = if let Some(col) = Collider::convex_hull(&hull_relative) {
        col
    } else {
        Collider::ball(5.0)
    };

    // Generate grey color
    let mut rng = rand::thread_rng();
    let grey = rng.gen_range(0.3..0.9);
    let grey_color = Color::rgb(grey, grey, grey);

    // Spawn large asteroid as polygon with vertices for wireframe rendering
    let entity = commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: grey_color,
                custom_size: Some(Vec2::splat(10.0)), // Placeholder visual size
                ..Default::default()
            },
            transform: Transform::from_translation(center.extend(0.05)),
            ..Default::default()
        },
        Asteroid,
        AsteroidSize::Large,
        Vertices(hull.to_vec()),
        RigidBody::Dynamic,
        collider,
        Restitution::coefficient(0.7),
        Friction::coefficient(0.4),
        Velocity::zero(),
        Damping {
            linear_damping: 0.0,
            angular_damping: 0.0,
        },
        ExternalForce {
            force: Vec2::ZERO,
            torque: 0.0,
        },
        Sleeping::disabled(),
    )).id();
    
    entity
}

/// Compute convex hull using gift wrapping algorithm
pub fn compute_convex_hull(particles: &[(Entity, Vec2, Color)]) -> Option<Vec<Vec2>> {
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
pub fn blend_colors(particles: &[(Entity, Vec2, Color)]) -> Color {
    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;

    for (_, _, color) in particles {
        r += color.r();
        g += color.g();
        b += color.b();
    }

    let count = particles.len() as f32;
    Color::rgb(r / count, g / count, b / count)
}
