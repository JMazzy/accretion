//! Projectile firing, lifetime management, player-asteroid collision damage,
//! and the asteroid splitting / chipping logic triggered on projectile hits.
//!
//! ## Destruction rules by asteroid size
//!
//! | Size (units) | Effect |
//! |---|---|
//! | 0–1 | Immediate destroy |
//! | 2–3 | Scatter into N unit fragments (each mass 1 = triangle) |
//! | 4–8 | Split roughly in half along impact axis |
//! | ≥ 9 | Chip: remove closest vertex, spawn one unit fragment |
//!
//! ## Mass → shape rules for split/chip fragments
//!
//! Fragment shapes are regulated so they never have *fewer* sides than their
//! mass warrants (merging is exempt):
//!
//! | Fragment mass | Min shape  | Min vertices |
//! |---------------|------------|--------------|
//! | 1             | triangle   | 3            |
//! | 2–4           | square     | 4            |
//! | 5             | pentagon   | 5            |
//! | ≥ 6           | hexagon    | 6            |

use super::state::{
    AimDirection, AimIdleTimer, Player, PlayerFireCooldown, PlayerHealth, PreferredGamepad,
    Projectile,
};
use crate::asteroid::{
    canonical_vertices_for_mass, compute_convex_hull_from_points, min_vertices_for_mass,
    spawn_asteroid_with_vertices, Asteroid, AsteroidSize, Vertices,
};
use crate::constants::{
    DAMAGE_SPEED_THRESHOLD, FIRE_COOLDOWN, GAMEPAD_FIRE_THRESHOLD, GAMEPAD_RIGHT_DEADZONE,
    INVINCIBILITY_DURATION, PROJECTILE_COLLIDER_RADIUS, PROJECTILE_LIFETIME, PROJECTILE_MAX_DIST,
    PROJECTILE_SPEED,
};
use bevy::input::gamepad::GamepadAxis;
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
    gamepads: Query<&Gamepad>,
    mut aim: ResMut<AimDirection>,
    mut cooldown: ResMut<PlayerFireCooldown>,
    mut idle: ResMut<AimIdleTimer>,
    time: Res<Time>,
) {
    cooldown.timer = (cooldown.timer - time.delta_secs()).max(0.0);

    let Ok(transform) = q_player.single() else {
        return;
    };

    // ── Gamepad right stick: update aim + auto-fire when pushed past threshold ──
    let mut gamepad_wants_fire = false;
    if let Some(gamepad_entity) = preferred.0 {
        if let Ok(gamepad) = gamepads.get(gamepad_entity) {
            let rx = gamepad.get(GamepadAxis::RightStickX).unwrap_or(0.0);
            let ry = gamepad.get(GamepadAxis::RightStickY).unwrap_or(0.0);
            let right_stick = Vec2::new(rx, ry);
            if right_stick.length() > GAMEPAD_RIGHT_DEADZONE {
                aim.0 = right_stick.normalize_or_zero();
                // Right stick is active — prevent idle aim snap.
                idle.secs = 0.0;
                if right_stick.length() > GAMEPAD_FIRE_THRESHOLD {
                    gamepad_wants_fire = true;
                }
            }
        }
    }

    let kb_fire = keys.pressed(KeyCode::Space);
    let mouse_fire = mouse_buttons.pressed(MouseButton::Left);

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
        Transform::from_translation(spawn_pos.extend(0.0)),
        Visibility::default(),
        RigidBody::KinematicVelocityBased,
        Velocity {
            linvel: fire_dir * PROJECTILE_SPEED,
            angvel: 0.0,
        },
        Collider::ball(PROJECTILE_COLLIDER_RADIUS),
        // Sensor: detects collision events for game logic but generates no contact
        // forces.  Without this, Rapier 0.22+ kinematic bodies push dynamic
        // asteroids — transferring significant momentum like a physical projectile
        // rather than the sci-fi "energy blaster" behaviour intended.
        Sensor,
        Ccd { enabled: true },
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
    let dt = time.delta_secs();
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
    rapier_context: ReadRapierContext,
    time: Res<Time>,
) {
    let Ok((player_entity, mut health, player_vel)) = q_player.single_mut() else {
        return;
    };

    health.inv_timer = (health.inv_timer - time.delta_secs()).max(0.0);
    if health.inv_timer > 0.0 {
        return;
    }

    let mut total_damage = 0.0_f32;

    let Ok(rapier) = rapier_context.single() else {
        return;
    };
    for contact_pair in rapier.contact_pairs_with(player_entity) {
        if !contact_pair.has_any_active_contact() {
            continue;
        }
        let Some(e1) = contact_pair.collider1() else {
            continue;
        };
        let Some(e2) = contact_pair.collider2() else {
            continue;
        };

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
    mut collision_events: MessageReader<CollisionEvent>,
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
                    let final_mass = half_size.max(1);
                    // Enforce mass→shape: if the hull has fewer vertices than the
                    // mass requires, substitute the canonical regular polygon.
                    let local: Vec<Vec2> = if hull.len() < min_vertices_for_mass(final_mass) {
                        canonical_vertices_for_mass(final_mass)
                    } else {
                        hull.iter().map(|v| *v - centroid).collect()
                    };
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
                        Color::srgb(grey, grey, grey),
                        final_mass,
                    );
                    commands.entity(new_ent).insert(Velocity {
                        linvel: split_vel,
                        angvel: ang_vel,
                    });
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
                    .unwrap_or_else(|| new_world_verts.clone());
                let hull_centroid: Vec2 =
                    hull_world.iter().copied().sum::<Vec2>() / hull_world.len() as f32;
                let new_mass = (n - 1).max(1);
                // Enforce mass→shape: if the hull has fewer vertices than the
                // mass requires, substitute the canonical regular polygon.
                let new_local: Vec<Vec2> = if hull_world.len() < min_vertices_for_mass(new_mass) {
                    canonical_vertices_for_mass(new_mass)
                } else {
                    hull_world.iter().map(|v| *v - hull_centroid).collect()
                };

                commands.entity(asteroid_entity).despawn();

                let grey = 0.4 + rand::random::<f32>() * 0.3;
                let new_ent = spawn_asteroid_with_vertices(
                    &mut commands,
                    hull_centroid,
                    &new_local,
                    Color::srgb(grey, grey, grey),
                    new_mass,
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── split_convex_polygon ──────────────────────────────────────────────────

    #[test]
    fn split_square_along_vertical_axis_both_halves_have_correct_signs() {
        // Unit square split along X axis: front = x >= 0, back = x < 0
        let square = vec![
            Vec2::new(-10.0, -10.0),
            Vec2::new(10.0, -10.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(-10.0, 10.0),
        ];
        let (front, back) = split_convex_polygon(&square, Vec2::ZERO, Vec2::X);
        assert!(
            front.len() >= 3,
            "front half needs ≥3 points for a valid polygon, got {}",
            front.len()
        );
        assert!(
            back.len() >= 3,
            "back half needs ≥3 points for a valid polygon, got {}",
            back.len()
        );
        for v in &front {
            assert!(v.x >= -1e-5, "front vertex has x={} (should be ≥ 0)", v.x);
        }
        for v in &back {
            assert!(v.x <= 1e-5, "back vertex has x={} (should be ≤ 0)", v.x);
        }
    }

    #[test]
    fn split_all_points_on_front_side_back_is_empty() {
        // Polygon entirely to the right of origin → all vertices go to front
        let rect = vec![
            Vec2::new(5.0, -5.0),
            Vec2::new(15.0, -5.0),
            Vec2::new(15.0, 5.0),
            Vec2::new(5.0, 5.0),
        ];
        let (front, back) = split_convex_polygon(&rect, Vec2::ZERO, Vec2::X);
        assert_eq!(front.len(), 4, "all 4 vertices should be in front");
        assert!(back.is_empty(), "nothing should be in back");
    }

    #[test]
    fn split_intersection_points_shared_between_halves() {
        // Axis-aligned rectangle crossing origin: intersection at x=0, y=±5
        let rect = vec![
            Vec2::new(-10.0, -5.0),
            Vec2::new(10.0, -5.0),
            Vec2::new(10.0, 5.0),
            Vec2::new(-10.0, 5.0),
        ];
        let (front, back) = split_convex_polygon(&rect, Vec2::ZERO, Vec2::X);

        let has_pt = |verts: &[Vec2], x: f32, y: f32| {
            verts
                .iter()
                .any(|v| (v.x - x).abs() < 1e-4 && (v.y - y).abs() < 1e-4)
        };

        assert!(
            has_pt(&front, 0.0, 5.0) || has_pt(&front, 0.0, -5.0),
            "front half should contain at least one intersection point"
        );
        assert!(
            has_pt(&back, 0.0, 5.0) || has_pt(&back, 0.0, -5.0),
            "back half should contain at least one intersection point"
        );
    }

    #[test]
    fn split_triangle_does_not_panic() {
        // Equilateral triangle split along Y → apex on boundary, two base corners split
        let tri = vec![
            Vec2::new(0.0, 10.0),
            Vec2::new(-10.0, -5.0),
            Vec2::new(10.0, -5.0),
        ];
        let (front, back) = split_convex_polygon(&tri, Vec2::ZERO, Vec2::X);
        assert!(
            !front.is_empty() || !back.is_empty(),
            "at least one side should have vertices"
        );
    }

    #[test]
    fn split_preserves_all_original_vertices_in_union() {
        // Every original vertex must appear in either front or back (not lost)
        let square = vec![
            Vec2::new(-10.0, -10.0),
            Vec2::new(10.0, -10.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(-10.0, 10.0),
        ];
        let (front, back) = split_convex_polygon(&square, Vec2::ZERO, Vec2::X);
        let union: Vec<Vec2> = front.iter().chain(back.iter()).copied().collect();
        for v in &square {
            assert!(
                union.iter().any(|u| (*u - *v).length() < 1e-4),
                "original vertex {v:?} missing from split output"
            );
        }
    }

    // ── spawn_unit_fragment geometry ─────────────────────────────────────────

    #[test]
    fn unit_fragment_triangle_has_positive_area() {
        let side = 6.0_f32;
        let h = side * 3.0_f32.sqrt() / 2.0;
        let v = [
            Vec2::new(0.0, h / 2.0),
            Vec2::new(-side / 2.0, -h / 2.0),
            Vec2::new(side / 2.0, -h / 2.0),
        ];
        let a = v[1] - v[0];
        let b = v[2] - v[0];
        let area = (a.x * b.y - a.y * b.x).abs() / 2.0;
        assert!(
            area > 1.0,
            "unit fragment triangle must have positive area, got {area}"
        );
    }

    #[test]
    fn unit_fragment_triangle_accepted_by_rapier_convex_hull() {
        let side = 6.0_f32;
        let h = side * 3.0_f32.sqrt() / 2.0;
        let verts = vec![
            Vec2::new(0.0, h / 2.0),
            Vec2::new(-side / 2.0, -h / 2.0),
            Vec2::new(side / 2.0, -h / 2.0),
        ];
        let collider = bevy_rapier2d::prelude::Collider::convex_hull(&verts);
        assert!(
            collider.is_some(),
            "unit fragment vertices must produce a valid Rapier convex hull collider"
        );
    }

    // ── Full split pipeline: split → hull → local → Rapier collider ──────────
    //
    // These tests exercise the same code path as `projectile_asteroid_hit_system`
    // for size-4..=8 asteroids.  A `None` return from `Collider::convex_hull`
    // means the fragment silently falls back to a ball and can look like it has
    // no collision shape.

    /// Returns the polygon area (shoelace formula).
    fn poly_area(v: &[Vec2]) -> f32 {
        let n = v.len();
        let mut a = 0.0f32;
        for i in 0..n {
            let j = (i + 1) % n;
            a += v[i].x * v[j].y - v[j].x * v[i].y;
        }
        a.abs() / 2.0
    }

    /// Simulate the exact pipeline from `projectile_asteroid_hit_system`
    /// (size 4..=8 path) and assert both halves produce a valid Rapier collider.
    fn assert_split_produces_valid_colliders(shape_name: &str, verts: &[Vec2], axis: Vec2) {
        let origin = Vec2::ZERO;
        let (front_raw, back_raw) = split_convex_polygon(verts, origin, axis);

        for (side, raw) in [("front", &front_raw), ("back", &back_raw)] {
            if raw.len() < 3 {
                // Empty half is fine — the code skips it with `continue`
                continue;
            }
            let hull = crate::asteroid::compute_convex_hull_from_points(raw);
            let hull = match hull {
                Some(ref h) if h.len() >= 3 => h.clone(),
                _ => {
                    panic!(
                        "{shape_name} axis=({ax:.2},{ay:.2}) {side}: \
                         compute_convex_hull_from_points returned < 3 pts from {n} raw pts",
                        ax = axis.x,
                        ay = axis.y,
                        n = raw.len()
                    );
                }
            };
            let centroid: Vec2 = hull.iter().copied().sum::<Vec2>() / hull.len() as f32;
            let local: Vec<Vec2> = hull.iter().map(|v| *v - centroid).collect();
            let area = poly_area(&local);
            let collider = bevy_rapier2d::prelude::Collider::convex_hull(&local);

            assert!(
                collider.is_some(),
                "{shape_name} axis=({ax:.2},{ay:.2}) {side}: \
                 Collider::convex_hull returned None for {n} local pts (area={area:.4})\n\
                 local: {local:?}",
                ax = axis.x,
                ay = axis.y,
                n = local.len()
            );

            // Sanity: the resulting shape must have positive area
            assert!(
                area > 0.1,
                "{shape_name} axis=({ax:.2},{ay:.2}) {side}: \
                 polygon area {area:.4} is suspiciously small (near-degenerate)",
                ax = axis.x,
                ay = axis.y
            );
        }
    }

    fn impact_axes() -> [Vec2; 6] {
        use std::f32::consts::FRAC_PI_4;
        [
            Vec2::X,
            Vec2::NEG_X,
            Vec2::Y,
            Vec2::NEG_Y,
            Vec2::new(FRAC_PI_4.cos(), FRAC_PI_4.sin()), // 45°
            Vec2::new(-FRAC_PI_4.cos(), FRAC_PI_4.sin()), // 135°
        ]
    }

    #[test]
    fn split_pipeline_square_asteroid_all_axes() {
        use crate::constants::SQUARE_BASE_HALF;
        let h = SQUARE_BASE_HALF;
        let square = vec![
            Vec2::new(-h, -h),
            Vec2::new(h, -h),
            Vec2::new(h, h),
            Vec2::new(-h, h),
        ];
        for axis in impact_axes() {
            assert_split_produces_valid_colliders("square", &square, axis);
        }
    }

    #[test]
    fn split_pipeline_triangle_asteroid_all_axes() {
        use crate::constants::TRIANGLE_BASE_SIDE;
        let side = TRIANGLE_BASE_SIDE;
        let height = side * 3.0_f32.sqrt() / 2.0;
        let tri = vec![
            Vec2::new(0.0, height / 2.0),
            Vec2::new(-side / 2.0, -height / 2.0),
            Vec2::new(side / 2.0, -height / 2.0),
        ];
        for axis in impact_axes() {
            assert_split_produces_valid_colliders("triangle", &tri, axis);
        }
    }

    #[test]
    fn split_pipeline_pentagon_asteroid_all_axes() {
        use crate::constants::POLYGON_BASE_RADIUS;
        let r = POLYGON_BASE_RADIUS;
        let pent: Vec<Vec2> = (0..5)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / 5.0;
                Vec2::new(r * a.cos(), r * a.sin())
            })
            .collect();
        for axis in impact_axes() {
            assert_split_produces_valid_colliders("pentagon", &pent, axis);
        }
    }

    #[test]
    fn split_pipeline_hexagon_asteroid_all_axes() {
        use crate::constants::POLYGON_BASE_RADIUS;
        let r = POLYGON_BASE_RADIUS;
        let hex: Vec<Vec2> = (0..6)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / 6.0;
                Vec2::new(r * a.cos(), r * a.sin())
            })
            .collect();
        for axis in impact_axes() {
            assert_split_produces_valid_colliders("hexagon", &hex, axis);
        }
    }

    #[test]
    fn split_pipeline_large_composite_hull_all_axes() {
        // A large octagon simulating a merged composite (e.g. 8 triangles merged)
        let r = 30.0_f32;
        let oct: Vec<Vec2> = (0..8)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / 8.0;
                Vec2::new(r * a.cos(), r * a.sin())
            })
            .collect();
        for axis in impact_axes() {
            assert_split_produces_valid_colliders("large_octagon", &oct, axis);
        }
    }

    #[test]
    fn split_pipeline_vertex_exactly_on_split_plane() {
        // Diamond where two vertices lie exactly on the split plane (y-axis).
        // This is a tricky edge case that can produce near-duplicate intersection points.
        let diamond = vec![
            Vec2::new(-10.0, -5.0),
            Vec2::new(0.0, 8.0), // ON y-axis
            Vec2::new(10.0, -5.0),
            Vec2::new(0.0, -8.0), // ON y-axis
        ];
        // Split along Y (axis = X), so the two vertices on the y-axis are on the plane
        assert_split_produces_valid_colliders("diamond_on_axis", &diamond, Vec2::X);
        assert_split_produces_valid_colliders("diamond_on_axis", &diamond, Vec2::NEG_X);
    }

    // ── Chip path: vertex-removal pipeline ───────────────────────────────────
    //
    // These mirror the `_ =>` (size ≥ 9) path in `projectile_asteroid_hit_system`.

    fn assert_chip_produces_valid_collider(shape_name: &str, verts: &[Vec2], remove_idx: usize) {
        // Simulate the chip path
        let mut new_world = verts.to_vec();
        if new_world.len() > 3 {
            new_world.remove(remove_idx);
        }
        let hull_world = crate::asteroid::compute_convex_hull_from_points(&new_world)
            .unwrap_or_else(|| new_world.clone());
        let centroid: Vec2 = hull_world.iter().copied().sum::<Vec2>() / hull_world.len() as f32;
        let new_local: Vec<Vec2> = hull_world.iter().map(|v| *v - centroid).collect();

        let collider = bevy_rapier2d::prelude::Collider::convex_hull(&new_local);
        let area = poly_area(&new_local);

        assert!(
            collider.is_some(),
            "{shape_name} chip[{remove_idx}]: Collider::convex_hull returned None \
             for {n} pts (area={area:.4})\n  local: {new_local:?}",
            n = new_local.len()
        );
        assert!(
            area > 0.1,
            "{shape_name} chip[{remove_idx}]: area {area:.4} is suspiciously small"
        );
    }

    #[test]
    fn chip_path_pentagon_valid_collider_after_each_vertex_removed() {
        use crate::constants::POLYGON_BASE_RADIUS;
        let r = POLYGON_BASE_RADIUS;
        let pent: Vec<Vec2> = (0..5)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / 5.0;
                Vec2::new(r * a.cos(), r * a.sin())
            })
            .collect();
        for idx in 0..5 {
            assert_chip_produces_valid_collider("pentagon", &pent, idx);
        }
    }

    #[test]
    fn chip_path_octagon_valid_collider_after_each_vertex_removed() {
        let r = 20.0_f32;
        let oct: Vec<Vec2> = (0..8)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / 8.0;
                Vec2::new(r * a.cos(), r * a.sin())
            })
            .collect();
        for idx in 0..8 {
            assert_chip_produces_valid_collider("octagon", &oct, idx);
        }
    }

    #[test]
    fn chip_path_triangle_leaves_three_vertices_unchanged() {
        // Triangles (len=3) must NOT have a vertex removed (would leave 2, degenerate)
        use crate::constants::TRIANGLE_BASE_SIDE;
        let side = TRIANGLE_BASE_SIDE;
        let h = side * 3.0_f32.sqrt() / 2.0;
        let tri = vec![
            Vec2::new(0.0, h / 2.0),
            Vec2::new(-side / 2.0, -h / 2.0),
            Vec2::new(side / 2.0, -h / 2.0),
        ];
        // When len == 3 the remove is skipped; hull computation on 3 pts must still produce a valid collider
        assert_chip_produces_valid_collider("triangle_no_remove", &tri, 0);
    }

    // ── Mass→shape correction ─────────────────────────────────────────────────

    /// Simulate the full split pipeline with mass→shape correction and assert the result
    /// meets the minimum vertex count for the given mass.
    fn split_with_shape_correction(
        shape_name: &str,
        verts: &[Vec2],
        axis: Vec2,
        mass_a: u32,
        mass_b: u32,
    ) {
        let origin = Vec2::ZERO;
        let (front_raw, back_raw) = split_convex_polygon(verts, origin, axis);
        for (side, raw, mass) in [("front", &front_raw, mass_a), ("back", &back_raw, mass_b)] {
            if raw.len() < 3 {
                continue; // empty half is fine
            }
            let hull = match crate::asteroid::compute_convex_hull_from_points(raw) {
                Some(h) if h.len() >= 3 => h,
                _ => continue, // hull failed → production code skips this half
            };
            let final_mass = mass.max(1);
            let centroid: Vec2 = hull.iter().copied().sum::<Vec2>() / hull.len() as f32;
            let local: Vec<Vec2> = if hull.len() < min_vertices_for_mass(final_mass) {
                canonical_vertices_for_mass(final_mass)
            } else {
                hull.iter().map(|v| *v - centroid).collect()
            };

            let min_v = min_vertices_for_mass(final_mass);
            assert!(
                local.len() >= min_v,
                "{shape_name} {side} mass={final_mass}: got {} verts, expected ≥{min_v}",
                local.len()
            );
            let collider = bevy_rapier2d::prelude::Collider::convex_hull(&local);
            assert!(
                collider.is_some(),
                "{shape_name} {side} mass={final_mass}: Collider::convex_hull returned None \
                 for {} verts",
                local.len()
            );
        }
    }

    #[test]
    fn mass_shape_correction_size4_square_split_gives_at_least_4_vertices() {
        // A size-4 square (split → two halves each of mass 2).
        // Mass 2 requires ≥4 vertices. The raw split half may only have 3.
        use crate::constants::SQUARE_BASE_HALF;
        let h = SQUARE_BASE_HALF;
        let square = vec![
            Vec2::new(-h, -h),
            Vec2::new(h, -h),
            Vec2::new(h, h),
            Vec2::new(-h, h),
        ];
        for axis in [Vec2::X, Vec2::Y, Vec2::new(1.0, 1.0).normalize()] {
            split_with_shape_correction("square_size4", &square, axis, 2, 2);
        }
    }

    #[test]
    fn mass_shape_correction_size5_pentagon_split_both_halves_valid() {
        // Size-5 split → mass 2 + mass 3 (both need ≥4 vertices).
        use crate::constants::POLYGON_BASE_RADIUS;
        let r = POLYGON_BASE_RADIUS;
        let pent: Vec<Vec2> = (0..5)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / 5.0;
                Vec2::new(r * a.cos(), r * a.sin())
            })
            .collect();
        for axis in [Vec2::X, Vec2::Y] {
            split_with_shape_correction("pentagon_size5", &pent, axis, 2, 3);
        }
    }

    #[test]
    fn mass_shape_correction_size6_hexagon_split_both_halves_valid() {
        // Size-6 hexagon split → mass 3 + mass 3 (each need ≥4 vertices).
        use crate::constants::POLYGON_BASE_RADIUS;
        let r = POLYGON_BASE_RADIUS;
        let hex: Vec<Vec2> = (0..6)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / 6.0;
                Vec2::new(r * a.cos(), r * a.sin())
            })
            .collect();
        for axis in [Vec2::X, Vec2::Y, Vec2::new(1.0, 1.0).normalize()] {
            split_with_shape_correction("hexagon_size6", &hex, axis, 3, 3);
        }
    }

    #[test]
    fn mass_shape_correction_degenerate_input_uses_canonical() {
        // Force a below-minimum scenario by directly calling the correction logic.
        // A 3-vertex hull for mass=4 must be replaced with the 4-vertex canonical square.
        let three_vert_hull = vec![
            Vec2::new(-5.0, -5.0),
            Vec2::new(5.0, -5.0),
            Vec2::new(0.0, 5.0),
        ];
        let mass = 4u32;
        let centroid = Vec2::ZERO;
        let result: Vec<Vec2> = if three_vert_hull.len() < min_vertices_for_mass(mass) {
            canonical_vertices_for_mass(mass)
        } else {
            three_vert_hull.iter().map(|v| *v - centroid).collect()
        };
        let min_v = min_vertices_for_mass(mass);
        assert!(
            result.len() >= min_v,
            "mass {mass}: expected ≥{min_v} vertices but got {}",
            result.len()
        );
    }
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
    let ent = spawn_asteroid_with_vertices(commands, pos, &verts, Color::srgb(grey, grey, grey), 1);
    commands.entity(ent).insert(Velocity {
        linvel: velocity,
        angvel,
    });
}
