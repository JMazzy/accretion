//! Unified asteroid component and utilities
//!
//! All simulation entities are asteroids - they're defined by their polygon shape.
//! Any two asteroids can combine if touching and slow, forming a new asteroid with
//! the convex hull of their combined shapes.

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::Rng;

/// Marker component for any asteroid entity
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Asteroid;

/// How many "unit" (single triangle) asteroids this entity represents.
/// Single triangles = 1; composites = sum of constituents.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AsteroidSize(pub u32);

/// Count of nearby asteroids for environmental damping calculation
#[derive(Component, Debug, Clone, Copy)]
pub struct NeighborCount(pub usize);

/// Polygon vertices for wireframe rendering (stored in local space)
#[derive(Component, Debug, Clone)]
pub struct Vertices(pub Vec<Vec2>);

/// Spawns asteroids with random sizes, shapes, and velocities throughout the simulation area
/// Uses grid-based distribution for even spread, with a buffer zone around the player start.
pub fn spawn_initial_asteroids(commands: &mut Commands, count: usize) {
    let mut rng = rand::thread_rng();

    // Extended simulation area (well beyond viewport)
    let sim_width = 3000.0;
    let sim_height = 2000.0;
    let grid_margin = 150.0; // Keep asteroids away from outer edges

    // Buffer zone around player spawn (origin)
    let player_buffer_radius = 400.0;

    // Grid-based distribution for even spread
    let grid_cols = 6;
    let grid_rows = 4;
    let cell_width = (sim_width - 2.0 * grid_margin) / grid_cols as f32;
    let cell_height = (sim_height - 2.0 * grid_margin) / grid_rows as f32;
    let asteroids_per_cell = (count as f32 / (grid_cols * grid_rows) as f32).ceil() as usize;

    let mut spawned = 0;

    for grid_row in 0..grid_rows {
        for grid_col in 0..grid_cols {
            if spawned >= count {
                break;
            }

            // Cell bounds
            let cell_min_x = -sim_width / 2.0 + grid_margin + grid_col as f32 * cell_width;
            let cell_max_x = cell_min_x + cell_width;
            let cell_min_y = -sim_height / 2.0 + grid_margin + grid_row as f32 * cell_height;
            let cell_max_y = cell_min_y + cell_height;

            // Spawn asteroids in this cell
            for _ in 0..asteroids_per_cell {
                if spawned >= count {
                    break;
                }

                // Random position within cell
                let position = Vec2::new(
                    rng.gen_range(cell_min_x..cell_max_x),
                    rng.gen_range(cell_min_y..cell_max_y),
                );

                // Skip if within player buffer zone
                if position.distance(Vec2::ZERO) < player_buffer_radius {
                    continue;
                }

                spawned += 1;

                // Random size scale (0.5 to 1.5x)
                let size_scale = rng.gen_range(0.5..1.5);

                // Random shape (triangle, square, pentagon, hexagon)
                let shape = rng.gen_range(0..4);
                let vertices = match shape {
                    0 => generate_triangle(size_scale), // Triangle
                    1 => generate_square(size_scale),   // Square
                    2 => generate_pentagon(size_scale), // Pentagon
                    _ => generate_hexagon(size_scale),  // Hexagon
                };
                // Unit count: triangle=1, square=2, pentagon=3, hexagon=4
                let unit_size: u32 = match shape {
                    0 => 1,
                    1 => 2,
                    2 => 3,
                    _ => 4,
                };

                // Random velocity (gentle to avoid instant collisions)
                let velocity = Vec2::new(rng.gen_range(-15.0..15.0), rng.gen_range(-15.0..15.0));

                // Spawn the asteroid
                commands.spawn((
                    (
                        Transform::from_translation(position.extend(0.05)),
                        GlobalTransform::default(),
                        Asteroid,
                        AsteroidSize(unit_size),
                        NeighborCount(0),
                        Vertices(vertices.clone()),
                        RigidBody::Dynamic,
                    ),
                    (
                        {
                            if vertices.len() >= 3 {
                                Collider::convex_hull(&vertices)
                                    .unwrap_or_else(|| Collider::ball(5.0))
                            } else if vertices.len() == 2 {
                                let radius = ((vertices[0] - vertices[1]).length() / 2.0).max(2.0);
                                Collider::ball(radius)
                            } else {
                                Collider::ball(2.0)
                            }
                        },
                        Restitution::coefficient(0.0),
                        Friction::coefficient(1.0),
                        Velocity {
                            linvel: velocity,
                            angvel: rng.gen_range(-5.0..5.0), // Random angular velocity
                        },
                        Damping {
                            linear_damping: 0.0,
                            angular_damping: 0.0,
                        },
                        ExternalForce {
                            force: Vec2::ZERO,
                            torque: 0.0,
                        },
                        CollisionGroups::new(
                            bevy_rapier2d::geometry::Group::GROUP_1,
                            bevy_rapier2d::geometry::Group::GROUP_1
                                | bevy_rapier2d::geometry::Group::GROUP_2
                                | bevy_rapier2d::geometry::Group::GROUP_3,
                        ),
                        ActiveEvents::COLLISION_EVENTS,
                        Sleeping::disabled(),
                    ),
                ));
            }
        }
    }
}

/// Generate an equilateral triangle with configurable size
fn generate_triangle(scale: f32) -> Vec<Vec2> {
    let side = 6.0 * scale;
    let height = side * 3.0_f32.sqrt() / 2.0;
    vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ]
}

/// Generate a square with configurable size
fn generate_square(scale: f32) -> Vec<Vec2> {
    let half = 4.0 * scale;
    vec![
        Vec2::new(-half, half),
        Vec2::new(half, half),
        Vec2::new(half, -half),
        Vec2::new(-half, -half),
    ]
}

/// Generate a regular pentagon with configurable size
fn generate_pentagon(scale: f32) -> Vec<Vec2> {
    let radius = 5.0 * scale;
    let mut vertices = Vec::new();
    for i in 0..5 {
        let angle = 2.0 * std::f32::consts::PI * i as f32 / 5.0;
        vertices.push(Vec2::new(radius * angle.cos(), radius * angle.sin()));
    }
    vertices
}

/// Generate a regular hexagon with configurable size
fn generate_hexagon(scale: f32) -> Vec<Vec2> {
    let radius = 5.0 * scale;
    let mut vertices = Vec::new();
    for i in 0..6 {
        let angle = 2.0 * std::f32::consts::PI * i as f32 / 6.0;
        vertices.push(Vec2::new(radius * angle.cos(), radius * angle.sin()));
    }
    vertices
}

/// Spawns an asteroid with arbitrary polygon vertices and an explicit unit-size count.
/// `size` is how many unit triangles this asteroid represents (use 1 for fresh spawns).
pub fn spawn_asteroid_with_vertices(
    commands: &mut Commands,
    center: Vec2,
    hull: &[Vec2],
    _color: Color,
    size: u32,
) -> Entity {
    // Ensure we have valid vertices (need at least 3 for a polygon, minimum 2 for safety)
    if hull.is_empty() {
        panic!("Cannot spawn asteroid with no vertices");
    }

    // Create polygon collider from convex hull (vertices are already local-space)
    // For 2 vertices, use a capsule-like shape; for 3+, use polygon
    let collider = if hull.len() >= 3 {
        Collider::convex_hull(hull).unwrap_or_else(|| Collider::ball(5.0))
    } else if hull.len() == 2 {
        // For 2 vertices, estimate a bounding ball
        let radius = ((hull[0] - hull[1]).length() / 2.0).max(2.0);
        Collider::ball(radius)
    } else {
        // Single vertex, use ball
        Collider::ball(2.0)
    };

    // Spawn asteroid with just transform and physics - wireframe rendering via gizmos
    let entity = commands
        .spawn((
            (
                Transform::from_translation(center.extend(0.05)),
                GlobalTransform::default(),
                Asteroid,
                AsteroidSize(size),
                NeighborCount(0),
                Vertices(hull.to_vec()), // Store as LOCAL-SPACE vertices
                RigidBody::Dynamic,
                collider,
            ),
            (
                Restitution::coefficient(0.0),
                Friction::coefficient(1.0),
                Velocity::zero(),
                Damping {
                    linear_damping: 0.0,
                    angular_damping: 0.0,
                },
                ExternalForce {
                    force: Vec2::ZERO,
                    torque: 0.0,
                },
                CollisionGroups::new(
                    bevy_rapier2d::geometry::Group::GROUP_1,
                    bevy_rapier2d::geometry::Group::GROUP_1
                        | bevy_rapier2d::geometry::Group::GROUP_2
                        | bevy_rapier2d::geometry::Group::GROUP_3,
                ),
                ActiveEvents::COLLISION_EVENTS,
                Sleeping::disabled(),
            ),
        ))
        .id();

    entity
}

/// Compute convex hull using gift wrapping algorithm
#[allow(dead_code)]
pub fn compute_convex_hull(particles: &[(Entity, Vec2, Color)]) -> Option<Vec<Vec2>> {
    if particles.len() < 2 {
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

/// Compute convex hull from a list of points using gift wrapping algorithm.
/// Near-duplicate points (within 0.5 units) are deduplicated first so that
/// Rapier's `Collider::convex_hull` never silently falls back to a ball.
pub fn compute_convex_hull_from_points(points: &[Vec2]) -> Option<Vec<Vec2>> {
    if points.len() < 2 {
        return None;
    }

    // Deduplicate points that are too close together (prevents degenerate hulls)
    const MIN_DIST: f32 = 0.5;
    let mut deduped: Vec<Vec2> = Vec::with_capacity(points.len());
    for &p in points {
        if !deduped.iter().any(|q| q.distance(p) < MIN_DIST) {
            deduped.push(p);
        }
    }
    if deduped.len() < 2 {
        return None;
    }
    let points = deduped.as_slice();

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
#[allow(dead_code)]
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
