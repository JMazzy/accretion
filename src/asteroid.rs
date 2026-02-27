//! Unified asteroid component and utilities
//!
//! All simulation entities are asteroids - they're defined by their polygon shape.
//! Any two asteroids can combine if touching and slow, forming a new asteroid with
//! the convex hull of their combined shapes.

use std::f32::consts::TAU;

use crate::config::PhysicsConfig;
use crate::constants::{
    FRICTION_ASTEROID, HEPTAGON_BASE_RADIUS, HULL_DEDUP_MIN_DIST, OCTAGON_BASE_RADIUS,
    POLYGON_BASE_RADIUS, RESTITUTION_SMALL, SQUARE_BASE_HALF, TRIANGLE_BASE_SIDE,
};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Marker component for any asteroid entity
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Asteroid;

/// Marker component for a planet body.
///
/// Planets participate in gravity but are fixed in place and excluded from
/// asteroid merge/split weapon-damage logic.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Planet;

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

/// Net gravitational force on this asteroid this physics tick.
///
/// Written exclusively by `nbody_gravity_system` in `FixedUpdate`.
/// Unlike `ExternalForce` (which also accumulates soft-boundary corrections),
/// this component stores the pure N-body gravity vector and is used by the
/// force-vector debug overlay so boundary forces don't contaminate the display.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct GravityForce(pub Vec2);

/// Simple hash-based noise generator for clustering asteroids.
/// Returns a float in [0, 1) that varies smoothly across space.
fn noise_2d(x: f32, y: f32, frequency: f32, offset: Vec2) -> f32 {
    let sample_x = x + offset.x;
    let sample_y = y + offset.y;

    let grid_x = (sample_x * frequency).floor();
    let grid_y = (sample_y * frequency).floor();

    // Hash function: mix bits from grid coordinates
    let h = ((13.0 * grid_x + 31.0 * grid_y).sin() * 13131.0).fract();

    // Smooth transition within cell
    let local_x = (sample_x * frequency).fract();
    let local_y = (sample_y * frequency).fract();
    let smooth_x = local_x * local_x * (3.0 - 2.0 * local_x);
    let smooth_y = local_y * local_y * (3.0 - 2.0 * local_y);

    h * (1.0 - smooth_x * smooth_y) + (1.0 - h) * smooth_x * smooth_y
}

/// Apply random jitter to vertices to make them look more natural.
/// Jitter amount is proportional to the distance from origin to make small and large asteroids both look good.
fn apply_vertex_jitter(vertices: Vec<Vec2>, scale: f32, rng: &mut impl Rng) -> Vec<Vec2> {
    // Jitter amount as fraction of scale
    let jitter_amplitude = scale * 0.8;

    vertices
        .into_iter()
        .map(|v| {
            let jitter_x = rng.gen_range(-jitter_amplitude..jitter_amplitude);
            let jitter_y = rng.gen_range(-jitter_amplitude..jitter_amplitude);
            v + Vec2::new(jitter_x, jitter_y)
        })
        .collect()
}

/// Spawns asteroids with clustered distributions using noise-based seeding.
/// Creates natural asteroid field patterns rather than even distribution.
pub fn spawn_initial_asteroids(commands: &mut Commands, count: usize, config: &PhysicsConfig) {
    let seed = rand::random::<u64>();
    let mut rng = StdRng::seed_from_u64(seed);
    info!("Field scenario seed: {}", seed);

    // Extended simulation area (well beyond viewport)
    let sim_width = config.sim_width;
    let sim_height = config.sim_height;
    let grid_margin = config.spawn_grid_margin;

    // Buffer zone around player spawn (origin)
    let player_buffer_radius = config.player_buffer_radius;

    // Sample grid: coarse grid for noise-based clustering
    // We'll evaluate noise at candidate positions and spawn asteroids probabilistically
    let sample_grid_size = 56; // Number of samples per dimension
    let sample_step_x = (sim_width - 2.0 * grid_margin) / sample_grid_size as f32;
    let sample_step_y = (sim_height - 2.0 * grid_margin) / sample_grid_size as f32;

    // Multi-scale seeded noise produces richer nearby cluster patches.
    let coarse_noise_frequency = 0.0045;
    let fine_noise_frequency = 0.018;
    let coarse_noise_offset = Vec2::new(
        rng.gen_range(-40_000.0..40_000.0),
        rng.gen_range(-40_000.0..40_000.0),
    );
    let fine_noise_offset = Vec2::new(
        rng.gen_range(-40_000.0..40_000.0),
        rng.gen_range(-40_000.0..40_000.0),
    );

    let size_scale_min = (config.asteroid_size_scale_min * 0.7).max(0.2);
    let size_scale_max = (config.asteroid_size_scale_max * 1.35).max(size_scale_min + 0.05);

    let mut spawned = 0;

    for sample_y in 0..sample_grid_size {
        for sample_x in 0..sample_grid_size {
            if spawned >= count {
                break;
            }

            let base_x = -sim_width / 2.0 + grid_margin + sample_x as f32 * sample_step_x;
            let base_y = -sim_height / 2.0 + grid_margin + sample_y as f32 * sample_step_y;

            // Evaluate seeded multi-scale noise at this grid position.
            let coarse_noise =
                noise_2d(base_x, base_y, coarse_noise_frequency, coarse_noise_offset);
            let fine_noise = noise_2d(base_x, base_y, fine_noise_frequency, fine_noise_offset);

            // Ridge term increases patch boundaries so nearby dense pockets form.
            let ridge = (1.0 - (2.0 * coarse_noise - 1.0).abs()).powf(1.7);
            let cluster_weight = (0.65 * coarse_noise + 0.35 * ridge).clamp(0.0, 1.0);

            // Probability is heavily cluster-weighted with fine local modulation.
            let spawn_prob =
                (0.08 + cluster_weight * 0.34 + fine_noise.powf(2.0) * 0.22).clamp(0.0, 0.72);

            if rng.gen::<f32>() > spawn_prob {
                continue;
            }

            // Position within the cell with some randomness
            let position = Vec2::new(
                base_x + rng.gen_range(-sample_step_x * 0.4..sample_step_x * 0.4),
                base_y + rng.gen_range(-sample_step_y * 0.4..sample_step_y * 0.4),
            );

            // Skip if within player buffer zone
            if position.distance(Vec2::ZERO) < player_buffer_radius {
                continue;
            }

            spawned += 1;

            // Random size scale
            let size_scale = rng.gen_range(size_scale_min..size_scale_max);

            // Random shape (triangle, square, pentagon, hexagon, heptagon, octagon)
            let shape = rng.gen_range(0..6);
            let mut vertices = match shape {
                0 => generate_triangle(size_scale, config.triangle_base_side),
                1 => generate_square(size_scale, config.square_base_half),
                2 => generate_pentagon(size_scale, config.polygon_base_radius),
                3 => generate_hexagon(size_scale, config.polygon_base_radius),
                4 => generate_heptagon(size_scale, config.heptagon_base_radius),
                _ => generate_octagon(size_scale, config.octagon_base_radius),
            };

            // Apply vertex jitter for natural-looking shapes
            vertices = apply_vertex_jitter(vertices, size_scale, &mut rng);

            // Derive AsteroidSize from actual polygon area so the density invariant
            // (vertices.area == AsteroidSize / density) holds from the very first frame.
            // Accounts for the randomised scale and jitter applied above.
            let tri_area = 3.0_f32.sqrt() / 4.0 * config.triangle_base_side.powi(2);
            let unit_size = ((polygon_area(&vertices) / tri_area).round() as u32).max(1);
            vertices =
                rescale_vertices_to_area(&vertices, unit_size as f32 / config.asteroid_density);

            // Random velocity (gentle to avoid instant collisions)
            let speed_scale = rng.gen_range(0.35..1.55);
            let velocity_range = config.asteroid_initial_velocity_range * speed_scale;
            let velocity = Vec2::new(
                rng.gen_range(-velocity_range..velocity_range),
                rng.gen_range(-velocity_range..velocity_range),
            );
            let initial_rotation = Quat::from_rotation_z(rng.gen_range(0.0..TAU));

            // Spawn the asteroid
            commands.spawn((
                (
                    Transform::from_translation(position.extend(0.05))
                        .with_rotation(initial_rotation),
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
                            Collider::convex_hull(&vertices).unwrap_or_else(|| Collider::ball(5.0))
                        } else if vertices.len() == 2 {
                            let radius = ((vertices[0] - vertices[1]).length() / 2.0).max(2.0);
                            Collider::ball(radius)
                        } else {
                            Collider::ball(2.0)
                        }
                    },
                    Restitution::coefficient(RESTITUTION_SMALL),
                    Friction::coefficient(FRICTION_ASTEROID),
                    Velocity {
                        linvel: velocity,
                        angvel: rng.gen_range(
                            -config.asteroid_initial_angvel_range * 1.6
                                ..config.asteroid_initial_angvel_range * 1.6,
                        ),
                    },
                    Damping {
                        linear_damping: 0.0,
                        angular_damping: 0.0,
                    },
                    ExternalForce {
                        force: Vec2::ZERO,
                        torque: 0.0,
                    },
                    GravityForce::default(),
                    CollisionGroups::new(
                        bevy_rapier2d::geometry::Group::GROUP_1,
                        bevy_rapier2d::geometry::Group::GROUP_1
                            | bevy_rapier2d::geometry::Group::GROUP_2
                            | bevy_rapier2d::geometry::Group::GROUP_3
                            | bevy_rapier2d::geometry::Group::GROUP_5
                            | bevy_rapier2d::geometry::Group::GROUP_6,
                    ),
                    ActiveEvents::COLLISION_EVENTS,
                    Sleeping::disabled(),
                ),
            ));
        }
    }
}

/// Spawns a single fixed planet at the given position.
///
/// The planet is a near-circular high-mass body that participates in gravity
/// while remaining anchored in world-space (`RigidBody::Fixed`).
///
/// It carries both [`Asteroid`] and [`Planet`] markers so gravity systems can
/// include it, while merge/split and projectile-damage systems can explicitly
/// exclude it via `Without<Planet>` filters.
///
/// # Example
/// ```ignore
/// spawn_planet(&mut commands, Vec2::new(500.0, 300.0), &config);
/// ```
pub fn spawn_planet(commands: &mut Commands, position: Vec2, config: &PhysicsConfig) {
    let vertices = rescale_vertices_to_area(
        &generate_regular_polygon(16, 1.0, config.planetoid_base_radius),
        config.planetoid_unit_size as f32 / config.asteroid_density,
    );
    commands.spawn((
        (
            Transform::from_translation(position.extend(0.05)),
            GlobalTransform::default(),
            Asteroid,
            Planet,
            AsteroidSize(config.planetoid_unit_size),
            NeighborCount(0),
            Vertices(vertices.clone()),
            RigidBody::Fixed,
        ),
        (
            {
                Collider::convex_hull(&vertices)
                    .unwrap_or_else(|| Collider::ball(config.planetoid_base_radius))
            },
            Restitution::coefficient(RESTITUTION_SMALL),
            Friction::coefficient(FRICTION_ASTEROID),
            Velocity::zero(),
            Damping {
                linear_damping: 0.0,
                angular_damping: 0.0,
            },
            ExternalForce {
                force: Vec2::ZERO,
                torque: 0.0,
            },
            GravityForce::default(),
            CollisionGroups::new(
                bevy_rapier2d::geometry::Group::GROUP_1,
                bevy_rapier2d::geometry::Group::GROUP_1
                    | bevy_rapier2d::geometry::Group::GROUP_2
                    | bevy_rapier2d::geometry::Group::GROUP_3
                    | bevy_rapier2d::geometry::Group::GROUP_5
                    | bevy_rapier2d::geometry::Group::GROUP_6,
            ),
            ActiveEvents::COLLISION_EVENTS,
            Sleeping::disabled(),
        ),
    ));
}

/// Backward-compatible wrapper retained for existing call-sites.
#[allow(dead_code)]
pub fn spawn_planetoid(commands: &mut Commands, position: Vec2, config: &PhysicsConfig) {
    spawn_planet(commands, position, config);
}

/// Spawns the "orbit" pre-built scenario.
///
/// The scenario consists of:
/// - One very large 16-gon central planet at `(800, 0)`.
/// - Three concentric rings of small triangle asteroids in near-circular orbits
///   around that body (CCW rotation).
///
/// Orbital velocities are derived from the calibrated unit-triangle Rapier mass so
/// that each ring is approximately in balance against the N-body gravity force.
/// In practice the orbits will slowly precess and exchange energy — this is expected
/// and adds visual interest.
/// Gravitational mass used for the Orbit scenario central body.
/// Must match `AsteroidSize` set on the spawned entity; used in the orbital
/// velocity formula below.
const ORBIT_CENTRAL_MASS: u32 = 2800;

pub fn spawn_orbit_scenario(commands: &mut Commands, config: &PhysicsConfig) {
    // ── Central anchored planet ──────────────────────────────────────────────
    //
    // 5.2× the normal planetoid radius gives a visually dominant body.  It is placed
    // at (800, 0) so the player (always at origin) starts outside the ring system
    // and can fly in.
    //
    // AsteroidSize(ORBIT_CENTRAL_MASS) makes this body gravitationally dominant:
    // with mass-scaled gravity (F = G·m_i·m_j/r²), it attracts ring triangles
    // with 2800× the force that a single triangle would.  The 66 ring asteroids
    // combined exert only 66 units of perturbation, giving a dominant central well
    // that keeps orbits stable for many revolutions.
    let central_radius = config.planetoid_base_radius * 5.2;
    let central_pos = Vec2::new(800.0, 0.0);
    let central_vertices = rescale_vertices_to_area(
        &generate_regular_polygon(16, 1.0, central_radius),
        ORBIT_CENTRAL_MASS as f32 / config.asteroid_density,
    );

    commands.spawn((
        (
            Transform::from_translation(central_pos.extend(0.05)),
            GlobalTransform::default(),
            Asteroid,
            Planet,
            AsteroidSize(ORBIT_CENTRAL_MASS),
            NeighborCount(0),
            Vertices(central_vertices.clone()),
            RigidBody::Fixed,
        ),
        (
            Collider::convex_hull(&central_vertices)
                .unwrap_or_else(|| Collider::ball(central_radius)),
            Restitution::coefficient(RESTITUTION_SMALL),
            Friction::coefficient(FRICTION_ASTEROID),
            Velocity::zero(),
            Damping {
                linear_damping: 0.0,
                angular_damping: 0.0,
            },
            ExternalForce {
                force: Vec2::ZERO,
                torque: 0.0,
            },
            GravityForce::default(),
            CollisionGroups::new(
                bevy_rapier2d::geometry::Group::GROUP_1,
                bevy_rapier2d::geometry::Group::GROUP_1
                    | bevy_rapier2d::geometry::Group::GROUP_2
                    | bevy_rapier2d::geometry::Group::GROUP_3
                    | bevy_rapier2d::geometry::Group::GROUP_5
                    | bevy_rapier2d::geometry::Group::GROUP_6,
            ),
            ActiveEvents::COLLISION_EVENTS,
            Sleeping::disabled(),
        ),
    ));

    // ── Orbital debris rings ─────────────────────────────────────────────────
    //
    // Orbital velocity formula per body (centripetal condition):
    //   v = sqrt(G · AsteroidSize_i · M_central / (r · m_rapier_i))
    //
    // m_rapier_i is the Rapier mass (= polygon area in world units, since
    // density=1 and pixels_per_meter=1).  Each shape has a different area so
    // each body's orbital speed is computed individually.
    //
    // Ring layout — centred on `central_pos` (all within 1800u soft boundary):
    //   Ring 1: r=280, 14 unit triangles         — fine, fast inner ring
    //   Ring 2: r=480, 22 triangles + squares    — medium ring, mixed sizes
    //   Ring 3: r=680, 30 pentagons/hexs/hepts   — sparse outer ring, larger bodies
    let g = config.gravity_const;
    let cm = ORBIT_CENTRAL_MASS as f32;
    let mut rng = rand::thread_rng();

    // ── Ring 1: unit triangles (inner, unchanged) ────────────────────────────
    let (r1, n1) = (260.0_f32, 16u32);
    let m_tri = 3.0_f32.sqrt() / 4.0 * config.triangle_base_side.powi(2);
    // v = sqrt(G·CM·density/r) — after density rescaling all bodies have
    // m_rapier_i = AsteroidSize_i/density, so centripetal balance gives this.
    let v_orbit = |r: f32| -> f32 { (g * cm * config.asteroid_density / r).sqrt() };
    for i in 0..n1 {
        let base_angle = i as f32 * TAU / n1 as f32;
        let angle = base_angle + rng.gen_range(-0.09..0.09);
        let radius = (r1 + rng.gen_range(-18.0..18.0)).max(120.0);
        let pos = central_pos + Vec2::new(angle.cos(), angle.sin()) * radius;
        let tangent = Vec2::new(-angle.sin(), angle.cos());
        let initial_rotation = Quat::from_rotation_z(rng.gen_range(0.0..TAU));
        let speed_boost = rng.gen_range(1.08..1.20);
        let vertices = rescale_vertices_to_area(
            &generate_triangle(1.0, config.triangle_base_side),
            1.0 / config.asteroid_density,
        );
        commands.spawn((
            (
                Transform::from_translation(pos.extend(0.05)).with_rotation(initial_rotation),
                GlobalTransform::default(),
                Asteroid,
                AsteroidSize(1),
                NeighborCount(0),
                Vertices(vertices.clone()),
                RigidBody::Dynamic,
            ),
            (
                Collider::convex_hull(&vertices)
                    .unwrap_or_else(|| Collider::ball(config.triangle_base_side / 2.0)),
                Restitution::coefficient(RESTITUTION_SMALL),
                Friction::coefficient(FRICTION_ASTEROID),
                Velocity {
                    linvel: tangent * v_orbit(radius) * speed_boost,
                    angvel: rng.gen_range(-0.35..0.35),
                },
                Damping {
                    linear_damping: 0.0,
                    angular_damping: 0.0,
                },
                ExternalForce {
                    force: Vec2::ZERO,
                    torque: 0.0,
                },
                GravityForce::default(),
                CollisionGroups::new(
                    bevy_rapier2d::geometry::Group::GROUP_1,
                    bevy_rapier2d::geometry::Group::GROUP_1
                        | bevy_rapier2d::geometry::Group::GROUP_2
                        | bevy_rapier2d::geometry::Group::GROUP_3
                        | bevy_rapier2d::geometry::Group::GROUP_5
                        | bevy_rapier2d::geometry::Group::GROUP_6,
                ),
                ActiveEvents::COLLISION_EVENTS,
                Sleeping::disabled(),
            ),
        ));
    }

    // ── Ring 2: triangles and squares (mid ring, varied sizes) ───────────────
    let (r2, n2) = (450.0_f32, 24u32);
    for i in 0..n2 {
        let base_angle = i as f32 * TAU / n2 as f32;
        let angle = base_angle + rng.gen_range(-0.10..0.10);
        let radius = (r2 + rng.gen_range(-35.0..35.0)).max(180.0);
        let pos = central_pos + Vec2::new(angle.cos(), angle.sin()) * radius;
        let tangent = Vec2::new(-angle.sin(), angle.cos());

        // Random scale in 0.9–2.1 for visual size variety.
        let scale: f32 = rng.gen_range(0.9..2.1);
        let initial_rotation = Quat::from_rotation_z(rng.gen_range(0.0..TAU));
        let speed_boost = rng.gen_range(1.02..1.16);

        let (raw_verts, pre_area) = if i % 2 == 0 {
            // Triangle
            let scaled_side = config.triangle_base_side * scale;
            let a = 3.0_f32.sqrt() / 4.0 * scaled_side.powi(2);
            (generate_triangle(scale, config.triangle_base_side), a)
        } else {
            // Square
            let half = config.square_base_half * scale;
            let a = (2.0 * half).powi(2);
            (generate_square(scale, config.square_base_half), a)
        };
        // Derive AsteroidSize from polygon area, then rescale to the density-consistent
        // area so the invariant vertices.area == AsteroidSize / density holds at spawn.
        let asteroid_size = ((pre_area / m_tri).round() as u32).max(4);
        let vertices =
            rescale_vertices_to_area(&raw_verts, asteroid_size as f32 / config.asteroid_density);

        commands.spawn((
            (
                Transform::from_translation(pos.extend(0.05)).with_rotation(initial_rotation),
                GlobalTransform::default(),
                Asteroid,
                AsteroidSize(asteroid_size),
                NeighborCount(0),
                Vertices(vertices.clone()),
                RigidBody::Dynamic,
            ),
            (
                Collider::convex_hull(&vertices)
                    .unwrap_or_else(|| Collider::ball(config.polygon_base_radius * scale)),
                Restitution::coefficient(RESTITUTION_SMALL),
                Friction::coefficient(FRICTION_ASTEROID),
                Velocity {
                    linvel: tangent * v_orbit(radius) * speed_boost,
                    angvel: rng.gen_range(-0.25..0.25),
                },
                Damping {
                    linear_damping: 0.0,
                    angular_damping: 0.0,
                },
                ExternalForce {
                    force: Vec2::ZERO,
                    torque: 0.0,
                },
                GravityForce::default(),
                CollisionGroups::new(
                    bevy_rapier2d::geometry::Group::GROUP_1,
                    bevy_rapier2d::geometry::Group::GROUP_1
                        | bevy_rapier2d::geometry::Group::GROUP_2
                        | bevy_rapier2d::geometry::Group::GROUP_3
                        | bevy_rapier2d::geometry::Group::GROUP_5
                        | bevy_rapier2d::geometry::Group::GROUP_6,
                ),
                ActiveEvents::COLLISION_EVENTS,
                Sleeping::disabled(),
            ),
        ));
    }

    // ── Ring 3: pentagons, hexagons, heptagons (outer, larger) ───────────────
    let (r3, n3) = (640.0_f32, 32u32);
    for i in 0..n3 {
        let base_angle = i as f32 * TAU / n3 as f32;
        let angle = base_angle + rng.gen_range(-0.12..0.12);
        let radius = (r3 + rng.gen_range(-55.0..55.0)).max(260.0);
        let pos = central_pos + Vec2::new(angle.cos(), angle.sin()) * radius;
        let tangent = Vec2::new(-angle.sin(), angle.cos());

        // Random scale in 0.9–2.6 for bigger visual spread.
        let scale: f32 = rng.gen_range(0.9..2.6);
        let initial_rotation = Quat::from_rotation_z(rng.gen_range(0.0..TAU));
        let speed_boost = rng.gen_range(0.98..1.12);

        let (raw_verts, pre_area) = match i % 3 {
            0 => {
                // Pentagon (5 sides)
                let r = config.polygon_base_radius * scale;
                let a = 5.0 / 2.0 * r.powi(2) * (TAU / 5.0).sin();
                (generate_pentagon(scale, config.polygon_base_radius), a)
            }
            1 => {
                // Hexagon (6 sides)
                let r = config.polygon_base_radius * scale;
                let a = 6.0 / 2.0 * r.powi(2) * (TAU / 6.0).sin();
                (generate_hexagon(scale, config.polygon_base_radius), a)
            }
            _ => {
                // Heptagon (7 sides)
                let r = config.heptagon_base_radius * scale;
                let a = 7.0 / 2.0 * r.powi(2) * (TAU / 7.0).sin();
                (generate_heptagon(scale, config.heptagon_base_radius), a)
            }
        };
        let asteroid_size = ((pre_area / m_tri).round() as u32).max(4);
        let vertices =
            rescale_vertices_to_area(&raw_verts, asteroid_size as f32 / config.asteroid_density);

        commands.spawn((
            (
                Transform::from_translation(pos.extend(0.05)).with_rotation(initial_rotation),
                GlobalTransform::default(),
                Asteroid,
                AsteroidSize(asteroid_size),
                NeighborCount(0),
                Vertices(vertices.clone()),
                RigidBody::Dynamic,
            ),
            (
                Collider::convex_hull(&vertices)
                    .unwrap_or_else(|| Collider::ball(config.polygon_base_radius * scale)),
                Restitution::coefficient(RESTITUTION_SMALL),
                Friction::coefficient(FRICTION_ASTEROID),
                Velocity {
                    linvel: tangent * v_orbit(radius) * speed_boost,
                    angvel: rng.gen_range(-0.20..0.20),
                },
                Damping {
                    linear_damping: 0.0,
                    angular_damping: 0.0,
                },
                ExternalForce {
                    force: Vec2::ZERO,
                    torque: 0.0,
                },
                GravityForce::default(),
                CollisionGroups::new(
                    bevy_rapier2d::geometry::Group::GROUP_1,
                    bevy_rapier2d::geometry::Group::GROUP_1
                        | bevy_rapier2d::geometry::Group::GROUP_2
                        | bevy_rapier2d::geometry::Group::GROUP_3
                        | bevy_rapier2d::geometry::Group::GROUP_5
                        | bevy_rapier2d::geometry::Group::GROUP_6,
                ),
                ActiveEvents::COLLISION_EVENTS,
                Sleeping::disabled(),
            ),
        ));
    }
}

/// Spawns the "comets" scenario.
///
/// Twenty large polygonal asteroids (9–12 sides, scale 2.5–4.5) are launched
/// on fast inward crossing trajectories.  Their high relative speed means they
/// fragment rather than merge on contact, providing a high-action
/// dodge-and-shoot challenge.
///
/// `AsteroidSize` is derived from the ratio of each polygon's area to the
/// unit-triangle area so that the gravity system weights them correctly.
pub fn spawn_comets_scenario(commands: &mut Commands, config: &PhysicsConfig) {
    let mut rng = rand::thread_rng();
    let tri_area = 3.0_f32.sqrt() / 4.0 * config.triangle_base_side.powi(2);

    for _ in 0..20u32 {
        // Spawn position: 400–1500 units from origin at a random angle.
        let spawn_angle: f32 = rng.gen_range(0.0..TAU);
        let dist: f32 = rng.gen_range(400.0..1500.0);
        let position = Vec2::new(dist * spawn_angle.cos(), dist * spawn_angle.sin());

        // Scale: visually large.
        let scale: f32 = rng.gen_range(2.5..4.5);

        // Pick a high-sided polygon (9–12 sides).
        let sides: usize = rng.gen_range(9usize..=12);
        let base_radius = config.polygon_base_radius;
        let vertices = generate_regular_polygon(sides, scale, base_radius);

        // Inward velocity with angular spread so comets cross the arena.
        let inward_dir = std::f32::consts::PI + spawn_angle;
        let spread: f32 = rng.gen_range(-0.7..0.7);
        let vel_angle = inward_dir + spread;
        let speed: f32 = rng.gen_range(35.0..60.0);
        let velocity = Vec2::new(vel_angle.cos() * speed, vel_angle.sin() * speed);

        // Derive AsteroidSize from polygon area, then rescale vertices to satisfy
        // the density invariant: vertices.area == AsteroidSize / density.
        let pre_area =
            (sides as f32 / 2.0) * (base_radius * scale).powi(2) * (TAU / sides as f32).sin();
        let unit_size = ((pre_area / tri_area).round() as u32).max(1);
        let vertices =
            rescale_vertices_to_area(&vertices, unit_size as f32 / config.asteroid_density);

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
                Collider::convex_hull(&vertices)
                    .unwrap_or_else(|| Collider::ball(base_radius * scale)),
                Restitution::coefficient(RESTITUTION_SMALL),
                Friction::coefficient(FRICTION_ASTEROID),
                Velocity {
                    linvel: velocity,
                    angvel: rng.gen_range(-0.2..0.2),
                },
                Damping {
                    linear_damping: 0.0,
                    angular_damping: 0.0,
                },
                ExternalForce {
                    force: Vec2::ZERO,
                    torque: 0.0,
                },
                GravityForce::default(),
                CollisionGroups::new(
                    bevy_rapier2d::geometry::Group::GROUP_1,
                    bevy_rapier2d::geometry::Group::GROUP_1
                        | bevy_rapier2d::geometry::Group::GROUP_2
                        | bevy_rapier2d::geometry::Group::GROUP_3
                        | bevy_rapier2d::geometry::Group::GROUP_5
                        | bevy_rapier2d::geometry::Group::GROUP_6,
                ),
                ActiveEvents::COLLISION_EVENTS,
                Sleeping::disabled(),
            ),
        ));
    }
}

/// Spawns the "shower" scenario.
///
/// 250 unit-triangle asteroids are scattered uniformly across a 1600-unit
/// radius disk with near-zero initial velocity.  Mutual N-body gravity quickly
/// collapses them into growing clusters — watch the field accrete in real time.
pub fn spawn_shower_scenario(commands: &mut Commands, config: &PhysicsConfig) {
    let mut rng = rand::thread_rng();
    let sim_radius: f32 = 1600.0;
    let player_buffer = config.player_buffer_radius;
    let mut spawned = 0u32;

    while spawned < 120 {
        // Uniform-area random point within sim_radius (sqrt gives uniform disk distribution).
        let angle: f32 = rng.gen_range(0.0..TAU);
        let dist: f32 = rng.gen_range(0.0_f32..1.0).sqrt() * sim_radius;
        let position = Vec2::new(dist * angle.cos(), dist * angle.sin());

        if position.length() < player_buffer {
            continue;
        }

        let vertices = rescale_vertices_to_area(
            &generate_triangle(1.0, config.triangle_base_side),
            1.0 / config.asteroid_density,
        );
        let velocity = Vec2::new(rng.gen_range(-3.0..3.0), rng.gen_range(-3.0..3.0));

        commands.spawn((
            (
                Transform::from_translation(position.extend(0.05)),
                GlobalTransform::default(),
                Asteroid,
                AsteroidSize(1),
                NeighborCount(0),
                Vertices(vertices.clone()),
                RigidBody::Dynamic,
            ),
            (
                Collider::convex_hull(&vertices)
                    .unwrap_or_else(|| Collider::ball(config.triangle_base_side * 0.5)),
                Restitution::coefficient(RESTITUTION_SMALL),
                Friction::coefficient(FRICTION_ASTEROID),
                Velocity {
                    linvel: velocity,
                    angvel: rng.gen_range(-0.5..0.5),
                },
                Damping {
                    linear_damping: 0.0,
                    angular_damping: 0.0,
                },
                ExternalForce {
                    force: Vec2::ZERO,
                    torque: 0.0,
                },
                GravityForce::default(),
                CollisionGroups::new(
                    bevy_rapier2d::geometry::Group::GROUP_1,
                    bevy_rapier2d::geometry::Group::GROUP_1
                        | bevy_rapier2d::geometry::Group::GROUP_2
                        | bevy_rapier2d::geometry::Group::GROUP_3
                        | bevy_rapier2d::geometry::Group::GROUP_5
                        | bevy_rapier2d::geometry::Group::GROUP_6,
                ),
                ActiveEvents::COLLISION_EVENTS,
                Sleeping::disabled(),
            ),
        ));

        spawned += 1;
    }
}

/// Generate an equilateral triangle with configurable size
fn generate_triangle(scale: f32, base_side: f32) -> Vec<Vec2> {
    let side = base_side * scale;
    let height = side * 3.0_f32.sqrt() / 2.0;
    vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ]
}

/// Generate a square with configurable size
fn generate_square(scale: f32, base_half: f32) -> Vec<Vec2> {
    let half = base_half * scale;
    vec![
        Vec2::new(-half, half),
        Vec2::new(half, half),
        Vec2::new(half, -half),
        Vec2::new(-half, -half),
    ]
}

/// Generate a regular pentagon with configurable size
fn generate_pentagon(scale: f32, base_radius: f32) -> Vec<Vec2> {
    generate_regular_polygon(5, scale, base_radius)
}

/// Generate a regular hexagon with configurable size
fn generate_hexagon(scale: f32, base_radius: f32) -> Vec<Vec2> {
    generate_regular_polygon(6, scale, base_radius)
}

/// Generate a regular heptagon (7-sided polygon) with configurable size
fn generate_heptagon(scale: f32, base_radius: f32) -> Vec<Vec2> {
    generate_regular_polygon(7, scale, base_radius)
}

/// Generate a regular octagon (8-sided polygon) with configurable size
fn generate_octagon(scale: f32, base_radius: f32) -> Vec<Vec2> {
    generate_regular_polygon(8, scale, base_radius)
}

/// Generic regular polygon generator — used for all n-gon shapes.
fn generate_regular_polygon(sides: usize, scale: f32, base_radius: f32) -> Vec<Vec2> {
    let radius = base_radius * scale;
    (0..sides)
        .map(|i| {
            let angle = 2.0 * std::f32::consts::PI * i as f32 / sides as f32;
            Vec2::new(radius * angle.cos(), radius * angle.sin())
        })
        .collect()
}

/// Returns the minimum number of polygon vertices a split/chip fragment of the
/// given mass should have.  Follows the mass→shape table:
///
/// | Mass  | Min shape  | Min vertices |
/// |-------|------------|--------------|
/// | 1     | triangle   | 3            |
/// | 2–4   | square     | 4            |
/// | 5     | pentagon   | 5            |
/// | 6–7   | hexagon    | 6            |
/// | 8–9   | heptagon   | 7            |
/// | ≥ 10  | octagon    | 8            |
///
/// Merging is exempt — merged composites keep however many hull vertices they produce.
/// Returns canonical centred (local-space) polygon vertices at base scale for
/// the given mass.  Used for unit-fragment spawns (mass 1) and as a
/// last-resort fallback when no hull vertices are available.
///
/// Vertices are always centred at the origin, so placing the entity at the
/// desired position produces a correctly-positioned shape.
pub fn canonical_vertices_for_mass(mass: u32) -> Vec<Vec2> {
    let raw = match mass {
        0 | 1 => generate_triangle(1.0, TRIANGLE_BASE_SIDE),
        2..=4 => generate_square(1.0, SQUARE_BASE_HALF),
        5 => generate_pentagon(1.0, POLYGON_BASE_RADIUS),
        6..=7 => generate_hexagon(1.0, POLYGON_BASE_RADIUS),
        8..=9 => generate_heptagon(1.0, HEPTAGON_BASE_RADIUS),
        _ => generate_octagon(1.0, OCTAGON_BASE_RADIUS),
    };
    // Centre the vertices at origin (centroid subtraction).
    // Square / pentagon / hexagon generators already produce centred vertices, but
    // triangle does not — subtracting the centroid makes all cases consistent.
    let c = raw.iter().copied().sum::<Vec2>() / raw.len() as f32;
    if c.length() > 1e-4 {
        raw.iter().map(|v| *v - c).collect()
    } else {
        raw
    }
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
        if let Some(c) = Collider::convex_hull(hull) {
            c
        } else {
            // This should rarely happen — log it so we can diagnose in-game failures.
            // Common causes: degenerate/collinear vertices, or near-duplicate points.
            eprintln!(
                "WARNING: Collider::convex_hull failed for {} vertices (falling back to ball=5.0). \
                 Vertices: {:?}",
                hull.len(),
                hull
            );
            Collider::ball(5.0)
        }
    } else if hull.len() == 2 {
        // For 2 vertices, estimate a bounding ball
        let radius = ((hull[0] - hull[1]).length() / 2.0).max(2.0);
        Collider::ball(radius)
    } else {
        // Single vertex, use ball
        Collider::ball(2.0)
    };

    // Spawn asteroid with just transform and physics - wireframe rendering via gizmos
    //
    // IMPORTANT: GlobalTransform must be derived from Transform at spawn time.
    // Rapier's `init_rigid_bodies` (in PhysicsSet::SyncBackend, which runs BEFORE
    // TransformSystems::Propagate in PostUpdate) reads GlobalTransform to set the
    // initial physics body position.  If we leave GlobalTransform as identity/default,
    // the body is placed at the world origin and Writeback will then move Transform
    // to origin as well — permanently displacing the asteroid regardless of center.
    let transform = Transform::from_translation(center.extend(0.05));
    let entity = commands
        .spawn((
            (
                transform,
                GlobalTransform::from(transform),
                Asteroid,
                AsteroidSize(size),
                NeighborCount(0),
                Vertices(hull.to_vec()), // Store as LOCAL-SPACE vertices
                RigidBody::Dynamic,
                collider,
            ),
            (
                Restitution::coefficient(RESTITUTION_SMALL),
                Friction::coefficient(FRICTION_ASTEROID),
                Velocity::zero(),
                Damping {
                    linear_damping: 0.0,
                    angular_damping: 0.0,
                },
                ExternalForce {
                    force: Vec2::ZERO,
                    torque: 0.0,
                },
                GravityForce::default(),
                CollisionGroups::new(
                    bevy_rapier2d::geometry::Group::GROUP_1,
                    bevy_rapier2d::geometry::Group::GROUP_1
                        | bevy_rapier2d::geometry::Group::GROUP_2
                        | bevy_rapier2d::geometry::Group::GROUP_3
                        | bevy_rapier2d::geometry::Group::GROUP_5
                        | bevy_rapier2d::geometry::Group::GROUP_6,
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
    const MIN_DIST: f32 = HULL_DEDUP_MIN_DIST;
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

/// Compute the area of a polygon (in world-unit²) using the shoelace formula.
///
/// Vertices should be given in order (either CW or CCW); the absolute value of the
/// signed area is returned so winding direction does not matter.
/// Returns `0.0` for degenerate inputs (fewer than 3 vertices).
pub fn polygon_area(vertices: &[Vec2]) -> f32 {
    if vertices.len() < 3 {
        return 0.0;
    }
    let n = vertices.len();
    let mut area = 0.0_f32;
    for i in 0..n {
        let j = (i + 1) % n;
        area += vertices[i].x * vertices[j].y;
        area -= vertices[j].x * vertices[i].y;
    }
    (area / 2.0).abs()
}

/// Rescale a polygon's vertices (in local space) so its enclosed area equals
/// `target_area`.
///
/// The polygon centroid is preserved; each vertex is moved radially so the area
/// changes by the scaling factor `sqrt(target_area / current_area)`.
///
/// If the current area is near zero (degenerate polygon) or `target_area ≤ 0`,
/// the vertices are returned unchanged.
pub fn rescale_vertices_to_area(vertices: &[Vec2], target_area: f32) -> Vec<Vec2> {
    let current_area = polygon_area(vertices);
    if current_area < 1e-6 || target_area <= 0.0 {
        return vertices.to_vec();
    }
    let scale = (target_area / current_area).sqrt();
    // Local-space vertices should already be centred at the origin, but compute
    // the centroid defensively to handle any residual offset.
    let centroid = vertices.iter().copied().sum::<Vec2>() / vertices.len() as f32;
    vertices
        .iter()
        .map(|v| centroid + (*v - centroid) * scale)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── polygon_area ──────────────────────────────────────────────────────────

    #[test]
    fn polygon_area_unit_square() {
        // A 2×2 square centred at origin should have area 4.
        let sq = vec![
            Vec2::new(-1.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, 1.0),
        ];
        let area = polygon_area(&sq);
        assert!((area - 4.0).abs() < 1e-4, "expected area ≈ 4.0, got {area}");
    }

    #[test]
    fn polygon_area_equilateral_triangle() {
        // Equilateral triangle with side 4: area = (√3/4)×16 ≈ 6.928.
        let side = 4.0_f32;
        let h = side * 3.0_f32.sqrt() / 2.0;
        let tri = vec![
            Vec2::new(0.0, h * 2.0 / 3.0),
            Vec2::new(-side / 2.0, -h / 3.0),
            Vec2::new(side / 2.0, -h / 3.0),
        ];
        let expected = (3.0_f32.sqrt() / 4.0) * side * side;
        let area = polygon_area(&tri);
        assert!(
            (area - expected).abs() < 0.05,
            "expected area ≈ {expected:.3}, got {area:.3}"
        );
    }

    #[test]
    fn polygon_area_degenerate_returns_zero() {
        assert_eq!(polygon_area(&[]), 0.0);
        assert_eq!(polygon_area(&[Vec2::ZERO, Vec2::ONE]), 0.0);
    }

    // ── rescale_vertices_to_area ──────────────────────────────────────────────

    #[test]
    fn rescale_vertices_doubles_area() {
        let sq = vec![
            Vec2::new(-1.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, 1.0),
        ];
        let rescaled = rescale_vertices_to_area(&sq, 8.0); // original area = 4, target = 8
        let new_area = polygon_area(&rescaled);
        assert!(
            (new_area - 8.0).abs() < 0.01,
            "expected rescaled area ≈ 8.0, got {new_area}"
        );
    }

    #[test]
    fn rescale_vertices_preserves_centroid() {
        let sq = vec![
            Vec2::new(-1.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, 1.0),
        ];
        let rescaled = rescale_vertices_to_area(&sq, 16.0);
        let centroid = rescaled.iter().copied().sum::<Vec2>() / rescaled.len() as f32;
        assert!(
            centroid.length() < 1e-4,
            "centroid should remain near origin after rescaling, got {centroid:?}"
        );
    }

    #[test]
    fn rescale_vertices_zero_target_returns_unchanged() {
        let sq = vec![
            Vec2::new(-1.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, 1.0),
        ];
        let unchanged = rescale_vertices_to_area(&sq, 0.0);
        assert_eq!(unchanged, sq);
    }

    // ── compute_convex_hull_from_points ───────────────────────────────────────

    #[test]
    fn hull_empty_input_returns_none() {
        assert!(compute_convex_hull_from_points(&[]).is_none());
    }

    #[test]
    fn hull_single_point_returns_none() {
        assert!(compute_convex_hull_from_points(&[Vec2::ZERO]).is_none());
    }

    #[test]
    fn hull_three_non_collinear_returns_three_points() {
        let pts = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(5.0, 10.0),
        ];
        let hull = compute_convex_hull_from_points(&pts).expect("should produce hull");
        assert_eq!(hull.len(), 3);
    }

    #[test]
    fn hull_square_four_corners_returns_four_points() {
        let pts = vec![
            Vec2::new(-10.0, -10.0),
            Vec2::new(10.0, -10.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(-10.0, 10.0),
        ];
        let hull = compute_convex_hull_from_points(&pts).expect("should produce hull");
        assert_eq!(hull.len(), 4, "square should yield 4 hull vertices");
    }

    #[test]
    fn hull_interior_point_is_excluded() {
        let pts = vec![
            Vec2::new(-10.0, -10.0),
            Vec2::new(10.0, -10.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(-10.0, 10.0),
            Vec2::new(0.0, 0.0), // interior
        ];
        let hull = compute_convex_hull_from_points(&pts).expect("should produce hull");
        assert_eq!(hull.len(), 4, "interior point should be excluded from hull");
        assert!(
            !hull.contains(&Vec2::new(0.0, 0.0)),
            "origin should not be in hull"
        );
    }

    #[test]
    fn hull_deduplicates_near_identical_points() {
        // Two near-duplicate pairs (within HULL_DEDUP_MIN_DIST = 0.5), one unique apex
        let pts = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(0.1, 0.0), // dup of first
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 0.1), // dup of third
            Vec2::new(5.0, 10.0),
        ];
        let hull = compute_convex_hull_from_points(&pts).expect("should produce hull");
        // After dedup: 3 unique groups → triangle hull
        assert_eq!(
            hull.len(),
            3,
            "near-duplicate points should be merged before hull"
        );
    }

    #[test]
    fn hull_all_points_within_bounding_box_of_inputs() {
        let pts = vec![
            Vec2::new(-20.0, -15.0),
            Vec2::new(20.0, -15.0),
            Vec2::new(20.0, 15.0),
            Vec2::new(-20.0, 15.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(5.0, 5.0),
        ];
        let hull = compute_convex_hull_from_points(&pts).unwrap();
        for v in &hull {
            assert!(
                v.x >= -20.0 && v.x <= 20.0,
                "hull x={} out of input range",
                v.x
            );
            assert!(
                v.y >= -15.0 && v.y <= 15.0,
                "hull y={} out of input range",
                v.y
            );
        }
    }

    #[test]
    fn hull_collinear_points_does_not_panic() {
        // All on x-axis — gift wrapping degenerates but must not panic
        let pts = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(5.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(15.0, 0.0),
        ];
        let _ = compute_convex_hull_from_points(&pts);
    }

    // ── Shape generators ──────────────────────────────────────────────────────

    #[test]
    fn generate_triangle_has_three_vertices() {
        assert_eq!(generate_triangle(1.0, TRIANGLE_BASE_SIDE).len(), 3);
    }

    #[test]
    fn generate_square_has_four_vertices() {
        assert_eq!(generate_square(1.0, SQUARE_BASE_HALF).len(), 4);
    }

    #[test]
    fn generate_pentagon_has_five_vertices() {
        assert_eq!(generate_pentagon(1.0, POLYGON_BASE_RADIUS).len(), 5);
    }

    #[test]
    fn generate_hexagon_has_six_vertices() {
        assert_eq!(generate_hexagon(1.0, POLYGON_BASE_RADIUS).len(), 6);
    }

    #[test]
    fn generated_triangle_centroid_is_symmetric_and_inside() {
        // The triangle has its apex at top, base at bottom: centroid x must be 0 (symmetric)
        // and centroid y must lie within the vertex y-range.
        let verts = generate_triangle(1.0, TRIANGLE_BASE_SIDE);
        let c = verts.iter().copied().sum::<Vec2>() / verts.len() as f32;
        assert!(
            c.x.abs() < 1e-5,
            "centroid x should be 0 by symmetry, got {}",
            c.x
        );
        let min_y = verts.iter().map(|v| v.y).fold(f32::INFINITY, f32::min);
        let max_y = verts.iter().map(|v| v.y).fold(f32::NEG_INFINITY, f32::max);
        assert!(
            c.y >= min_y && c.y <= max_y,
            "centroid y={} should be within [{min_y}, {max_y}]",
            c.y
        );
    }

    #[test]
    fn shape_scale_doubles_size() {
        // At 2× scale all vertex distances from origin should double
        let t1 = generate_triangle(1.0, TRIANGLE_BASE_SIDE);
        let t2 = generate_triangle(2.0, TRIANGLE_BASE_SIDE);
        let max1 = t1.iter().map(|v| v.length()).fold(0.0_f32, f32::max);
        let max2 = t2.iter().map(|v| v.length()).fold(0.0_f32, f32::max);
        assert!(
            (max2 / max1 - 2.0).abs() < 1e-5,
            "scale 2× should double vertex extent (got ratio {})",
            max2 / max1
        );
    }

    #[test]
    fn generate_triangle_has_positive_area() {
        let v = generate_triangle(1.0, TRIANGLE_BASE_SIDE);
        let a = v[1] - v[0];
        let b = v[2] - v[0];
        let area = (a.x * b.y - a.y * b.x).abs() / 2.0;
        assert!(area > 1.0, "triangle area should be > 1, got {area}");
    }

    #[test]
    fn spawn_asteroid_with_vertices_returns_entity() {
        // Smoke test: verify that the triangle vertices accepted by spawn_asteroid_with_vertices
        // produce a valid Rapier convex hull (not a ball fallback).
        let verts = generate_triangle(1.0, TRIANGLE_BASE_SIDE);
        let collider = bevy_rapier2d::prelude::Collider::convex_hull(&verts);
        assert!(
            collider.is_some(),
            "valid triangle should produce a convex hull collider"
        );
    }

    // ── canonical_vertices_for_mass ────────────────────────────────────────────

    #[test]
    fn canonical_vertices_for_mass_shapes_are_valid() {
        // Each canonical shape must produce a valid Rapier convex hull.
        for mass in [1u32, 2, 3, 4, 5, 6, 7, 8] {
            let verts = canonical_vertices_for_mass(mass);
            assert!(
                verts.len() >= 3,
                "mass {mass}: canonical shape has {} verts (need ≥3)",
                verts.len(),
            );
            let collider = bevy_rapier2d::prelude::Collider::convex_hull(&verts);
            assert!(
                collider.is_some(),
                "mass {mass}: canonical shape must produce a valid convex hull collider"
            );
        }
    }

    #[test]
    fn canonical_vertices_centroid_is_near_origin() {
        // Canonical shapes should be centred; centroid should be very close to (0, 0).
        for mass in [1u32, 2, 3, 4, 5, 6] {
            let verts = canonical_vertices_for_mass(mass);
            let c = verts.iter().copied().sum::<Vec2>() / verts.len() as f32;
            assert!(
                c.length() < 0.5,
                "mass {mass}: canonical centroid ({:.3}, {:.3}) is not near origin",
                c.x,
                c.y
            );
        }
    }
}

/// Blend colors by averaging RGB values
#[allow(dead_code)]
pub fn blend_colors(particles: &[(Entity, Vec2, Color)]) -> Color {
    let mut r = 0.0;
    let mut g = 0.0;
    let mut b = 0.0;

    for (_, _, color) in particles {
        let c = Srgba::from(*color);
        r += c.red;
        g += c.green;
        b += c.blue;
    }

    let count = particles.len() as f32;
    Color::srgb(r / count, g / count, b / count)
}
