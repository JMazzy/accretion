//! Projectile firing, lifetime management, player-asteroid collision damage,
//! and the asteroid splitting / chipping logic triggered on projectile hits.
//!
//! ## Destruction rules by asteroid size
//!
//! | Size (units) | Effect |
//! |---|---|
//! | 0–1 | Immediate destroy |
//! | 2–3 | Scatter into N unit fragments |
//! | 4–8 | Split roughly in half along impact axis |
//! | ≥ 9 | Chip: remove closest vertex, spawn one unit fragment |

use super::state::{
    AimDirection, Player, PlayerFireCooldown, PlayerHealth, PreferredGamepad, Projectile,
};
use crate::asteroid::{
    compute_convex_hull_from_points, spawn_asteroid_with_vertices, Asteroid, AsteroidSize, Vertices,
};
use crate::constants::{
    DAMAGE_SPEED_THRESHOLD, FIRE_COOLDOWN, GAMEPAD_FIRE_THRESHOLD, GAMEPAD_RIGHT_DEADZONE,
    INVINCIBILITY_DURATION, PROJECTILE_COLLIDER_RADIUS, PROJECTILE_LIFETIME, PROJECTILE_MAX_DIST,
    PROJECTILE_SPEED,
};
use bevy::input::gamepad::{GamepadAxis, GamepadAxisType};
use bevy::input::mouse::MouseButton;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::Rng;

// ── Projectile firing ─────────────────────────────────────────────────────────

/// Unified fire system: handles Space / left-click (keyboard+mouse) and the
/// gamepad right stick (twin-stick auto-fire) from a single location.
///
/// The gamepad right stick also *writes* `AimDirection` each frame so the
/// cursor-aim and stick-aim sources stay coherent through one shared resource.
#[allow(clippy::too_many_arguments)]
pub fn projectile_fire_system(
    mut commands: Commands,
    q_player: Query<&Transform, With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    preferred: Res<PreferredGamepad>,
    axes: Res<Axis<GamepadAxis>>,
    mut aim: ResMut<AimDirection>,
    mut cooldown: ResMut<PlayerFireCooldown>,
    time: Res<Time>,
) {
    cooldown.timer = (cooldown.timer - time.delta_seconds()).max(0.0);

    let Ok(transform) = q_player.get_single() else {
        return;
    };

    // ── Gamepad right stick: update aim + auto-fire when pushed past threshold ──
    let mut gamepad_wants_fire = false;
    if let Some(gamepad) = preferred.0 {
        let rx = axes
            .get(GamepadAxis::new(gamepad, GamepadAxisType::RightStickX))
            .unwrap_or(0.0);
        let ry = axes
            .get(GamepadAxis::new(gamepad, GamepadAxisType::RightStickY))
            .unwrap_or(0.0);
        let right_stick = Vec2::new(rx, ry);
        if right_stick.length() > GAMEPAD_RIGHT_DEADZONE {
            aim.0 = right_stick.normalize_or_zero();
            if right_stick.length() > GAMEPAD_FIRE_THRESHOLD {
                gamepad_wants_fire = true;
            }
        }
    }

    let kb_fire = keys.just_pressed(KeyCode::Space);
    let mouse_fire = mouse_buttons.just_pressed(MouseButton::Left);

    if !(kb_fire || mouse_fire || gamepad_wants_fire) || cooldown.timer > 0.0 {
        return;
    }
    cooldown.timer = FIRE_COOLDOWN;

    let fire_dir = if aim.0.length_squared() > 0.01 {
        aim.0.normalize_or_zero()
    } else {
        transform.rotation.mul_vec3(Vec3::Y).truncate()
    };

    let spawn_pos = transform.translation.truncate() + fire_dir * 14.0;

    commands.spawn((
        Projectile { age: 0.0 },
        TransformBundle::from_transform(Transform::from_translation(spawn_pos.extend(0.0))),
        VisibilityBundle::default(),
        RigidBody::KinematicVelocityBased,
        Velocity {
            linvel: fire_dir * PROJECTILE_SPEED,
            angvel: 0.0,
        },
        Collider::ball(PROJECTILE_COLLIDER_RADIUS),
        CollisionGroups::new(
            bevy_rapier2d::geometry::Group::GROUP_3,
            bevy_rapier2d::geometry::Group::GROUP_1,
        ),
        ActiveCollisionTypes::DYNAMIC_KINEMATIC,
        ActiveEvents::COLLISION_EVENTS,
    ));
}

// ── Projectile lifetime ───────────────────────────────────────────────────────

/// Age projectiles each frame and despawn them when they expire or leave bounds.
pub fn despawn_old_projectiles_system(
    mut commands: Commands,
    mut q: Query<(Entity, &mut Projectile, &Transform)>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();
    for (entity, mut proj, transform) in q.iter_mut() {
        proj.age += dt;
        let dist = transform.translation.truncate().length();
        if proj.age >= PROJECTILE_LIFETIME || dist > PROJECTILE_MAX_DIST {
            commands.entity(entity).despawn();
        }
    }
}

// ── Player collision damage ────────────────────────────────────────────────────

/// Detect asteroid–player collisions and deal proportional damage.
///
/// Only activates when relative speed exceeds `DAMAGE_SPEED_THRESHOLD`.
/// Grants invincibility frames after each successful damage event to prevent
/// rapid repeated damage from a single sustained contact.
pub fn player_collision_damage_system(
    mut commands: Commands,
    mut q_player: Query<(Entity, &mut PlayerHealth, &Velocity), With<Player>>,
    q_asteroids: Query<&Velocity, With<Asteroid>>,
    rapier_context: Res<RapierContext>,
    time: Res<Time>,
) {
    let Ok((player_entity, mut health, player_vel)) = q_player.get_single_mut() else {
        return;
    };

    health.inv_timer = (health.inv_timer - time.delta_seconds()).max(0.0);
    if health.inv_timer > 0.0 {
        return;
    }

    let mut total_damage = 0.0_f32;

    for contact_pair in rapier_context.contact_pairs() {
        if !contact_pair.has_any_active_contacts() {
            continue;
        }
        let e1 = contact_pair.collider1();
        let e2 = contact_pair.collider2();

        let asteroid_entity = if e1 == player_entity {
            e2
        } else if e2 == player_entity {
            e1
        } else {
            continue;
        };

        if let Ok(ast_vel) = q_asteroids.get(asteroid_entity) {
            let rel_speed = (player_vel.linvel - ast_vel.linvel).length();
            if rel_speed > DAMAGE_SPEED_THRESHOLD {
                total_damage += (rel_speed - DAMAGE_SPEED_THRESHOLD) * 0.5;
            }
        }
    }

    if total_damage > 0.0 {
        health.hp -= total_damage;
        health.inv_timer = INVINCIBILITY_DURATION;
        println!(
            "Player hit! HP: {:.1}/{:.1}",
            health.hp.max(0.0),
            health.max_hp
        );
        if health.hp <= 0.0 {
            commands.entity(player_entity).despawn();
            println!("Player ship destroyed!");
        }
    }
}

// ── Projectile–Asteroid hit system ───────────────────────────────────────────

/// Process projectile-asteroid collision events and apply size-appropriate destruction.
///
/// Matches `CollisionEvent::Started` pairs; ignores `Stopped`.
/// Uses two `HashSet`s to ensure each projectile and each asteroid is processed at
/// most once per frame even if they appear in multiple cascade events.
pub fn projectile_asteroid_hit_system(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    q_asteroids: Query<(&AsteroidSize, &Transform, &Velocity, &Vertices), With<Asteroid>>,
    q_projectiles: Query<Entity, With<Projectile>>,
    q_proj_transforms: Query<&Transform, With<Projectile>>,
) {
    let mut processed_asteroids: std::collections::HashSet<Entity> = Default::default();
    let mut processed_projectiles: std::collections::HashSet<Entity> = Default::default();

    for event in collision_events.read() {
        let (e1, e2) = match event {
            CollisionEvent::Started(e1, e2, _) => (*e1, *e2),
            CollisionEvent::Stopped(..) => continue,
        };

        // Identify which entity is the projectile and which the asteroid
        let (proj_entity, asteroid_entity) =
            if q_projectiles.get(e1).is_ok() && q_asteroids.get(e2).is_ok() {
                (e1, e2)
            } else if q_projectiles.get(e2).is_ok() && q_asteroids.get(e1).is_ok() {
                (e2, e1)
            } else {
                continue;
            };

        if processed_projectiles.contains(&proj_entity)
            || processed_asteroids.contains(&asteroid_entity)
        {
            continue;
        }

        let Ok((size, transform, velocity, vertices)) = q_asteroids.get(asteroid_entity) else {
            continue; // Asteroid may have been despawned already
        };

        processed_projectiles.insert(proj_entity);
        processed_asteroids.insert(asteroid_entity);

        commands.entity(proj_entity).despawn();

        let pos = transform.translation.truncate();
        let rot = transform.rotation;
        let vel = velocity.linvel;
        let ang_vel = velocity.angvel;
        let n = size.0;

        let proj_pos = q_proj_transforms
            .get(proj_entity)
            .map(|t| t.translation.truncate())
            .unwrap_or(pos);

        // World-space hull vertices (used by split and chip paths)
        let world_verts: Vec<Vec2> = vertices
            .0
            .iter()
            .map(|v| pos + rot.mul_vec3(v.extend(0.0)).truncate())
            .collect();

        match n {
            // ── Destroy ───────────────────────────────────────────────────────
            0 | 1 => {
                commands.entity(asteroid_entity).despawn();
            }

            // ── Scatter into unit fragments ───────────────────────────────────
            2..=3 => {
                commands.entity(asteroid_entity).despawn();
                let mut rng = rand::thread_rng();
                for i in 0..n {
                    let angle = std::f32::consts::TAU * (i as f32 / n as f32);
                    let scatter_offset = Vec2::new(angle.cos(), angle.sin()) * 8.0;
                    let scatter_vel =
                        vel + Vec2::new(rng.gen_range(-30.0..30.0), rng.gen_range(-30.0..30.0));
                    spawn_unit_fragment(&mut commands, pos + scatter_offset, scatter_vel, ang_vel);
                }
            }

            // ── Split in half along impact axis ───────────────────────────────
            4..=8 => {
                let impact_dir = (pos - proj_pos).normalize_or_zero();
                let split_axis = impact_dir;
                let (front_world, back_world) = split_convex_polygon(&world_verts, pos, split_axis);

                commands.entity(asteroid_entity).despawn();

                let size_a = n / 2;
                let size_b = n - size_a;
                let mut rng = rand::thread_rng();

                for (half_verts, half_size) in [(&front_world, size_a), (&back_world, size_b)] {
                    if half_verts.len() < 3 {
                        continue;
                    }
                    let hull = match compute_convex_hull_from_points(half_verts) {
                        Some(h) if h.len() >= 3 => h,
                        _ => continue,
                    };
                    let centroid: Vec2 = hull.iter().copied().sum::<Vec2>() / hull.len() as f32;
                    let local: Vec<Vec2> = hull.iter().map(|v| *v - centroid).collect();
                    let grey = 0.4 + rand::random::<f32>() * 0.3;
                    let push_sign = if (centroid - pos).dot(impact_dir) >= 0.0 {
                        1.0
                    } else {
                        -1.0
                    };
                    let split_vel = vel
                        + impact_dir * push_sign * 25.0
                        + Vec2::new(rng.gen_range(-10.0..10.0), rng.gen_range(-10.0..10.0));
                    let new_ent = spawn_asteroid_with_vertices(
                        &mut commands,
                        centroid,
                        &local,
                        Color::rgb(grey, grey, grey),
                        half_size.max(1),
                    );
                    commands.entity(new_ent).insert((
                        Velocity {
                            linvel: split_vel,
                            angvel: ang_vel,
                        },
                        ActiveCollisionTypes::DYNAMIC_KINEMATIC,
                    ));
                }
            }

            // ── Chip: remove closest vertex, spawn one unit fragment ───────────
            _ => {
                let closest_idx = world_verts
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        a.distance(proj_pos)
                            .partial_cmp(&b.distance(proj_pos))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(0);

                let chip_pos = world_verts[closest_idx];
                let chip_dir = (chip_pos - pos).normalize_or_zero();
                let mut rng = rand::thread_rng();
                let chip_vel = vel
                    + chip_dir * 40.0
                    + Vec2::new(rng.gen_range(-15.0..15.0), rng.gen_range(-15.0..15.0));
                spawn_unit_fragment(&mut commands, chip_pos, chip_vel, 0.0);

                // Reduce original asteroid: remove impact vertex, recompute hull
                let mut new_world_verts = world_verts;
                if new_world_verts.len() > 3 {
                    new_world_verts.remove(closest_idx);
                }

                let hull_world = compute_convex_hull_from_points(&new_world_verts)
                    .unwrap_or(new_world_verts.clone());
                let hull_centroid: Vec2 =
                    hull_world.iter().copied().sum::<Vec2>() / hull_world.len() as f32;
                let new_local: Vec<Vec2> = hull_world.iter().map(|v| *v - hull_centroid).collect();

                commands.entity(asteroid_entity).despawn();

                let grey = 0.4 + rand::random::<f32>() * 0.3;
                let new_ent = spawn_asteroid_with_vertices(
                    &mut commands,
                    hull_centroid,
                    &new_local,
                    Color::rgb(grey, grey, grey),
                    n - 1,
                );
                commands.entity(new_ent).insert(Velocity {
                    linvel: vel,
                    angvel: ang_vel,
                });
            }
        }
    }
}

// ── Geometry helpers ──────────────────────────────────────────────────────────

/// Split a convex polygon (world-space vertices) with a plane through `origin`
/// whose normal is `axis`.
///
/// Returns `(front, back)` where:
/// - `front` contains vertices where `dot(v − origin, axis) ≥ 0`
/// - `back` contains the rest
///
/// Both halves include the two edge-intersection points so each remains a closed polygon.
fn split_convex_polygon(verts: &[Vec2], origin: Vec2, axis: Vec2) -> (Vec<Vec2>, Vec<Vec2>) {
    let mut front: Vec<Vec2> = Vec::new();
    let mut back: Vec<Vec2> = Vec::new();
    let n = verts.len();

    for i in 0..n {
        let a = verts[i];
        let b = verts[(i + 1) % n];
        let da = (a - origin).dot(axis);
        let db = (b - origin).dot(axis);

        if da >= 0.0 {
            front.push(a);
        } else {
            back.push(a);
        }

        if (da > 0.0 && db < 0.0) || (da < 0.0 && db > 0.0) {
            let t = da / (da - db);
            let intersect = a + (b - a) * t;
            front.push(intersect);
            back.push(intersect);
        }
    }

    (front, back)
}

/// Spawn a single unit-size (triangle) asteroid fragment at `pos` with the given velocity.
fn spawn_unit_fragment(commands: &mut Commands, pos: Vec2, velocity: Vec2, angvel: f32) {
    let grey = 0.4 + rand::random::<f32>() * 0.4;
    let side = 6.0_f32;
    let h = side * 3.0_f32.sqrt() / 2.0;
    let verts = vec![
        Vec2::new(0.0, h / 2.0),
        Vec2::new(-side / 2.0, -h / 2.0),
        Vec2::new(side / 2.0, -h / 2.0),
    ];
    let ent = spawn_asteroid_with_vertices(commands, pos, &verts, Color::rgb(grey, grey, grey), 1);
    commands.entity(ent).insert(Velocity {
        linvel: velocity,
        angvel,
    });
}
