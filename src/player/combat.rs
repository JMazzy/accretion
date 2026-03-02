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
//! | ≥ 9 | Chip: remove closest vertex, spawn a chip fragment |
//!
//! ## Chip fragment size (primary weapon only)
//!
//! A level-L primary weapon can chip off a fragment of size 1 through L on each
//! hit.  The actual size is chosen uniformly at random in `[1, min(L, floor(n/2))]`
//! so a chip can never remove more than half the target's mass.
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
    AimDirection, AimIdleTimer, Missile, MissileAmmo, MissileCooldown, Player, PlayerFireCooldown,
    PlayerHealth, PlayerLives, PlayerScore, PreferredGamepad, PrimaryWeaponLevel, Projectile,
};
use crate::asteroid::{
    apply_crater_deformation, canonical_vertices_for_mass, rescale_vertices_to_area,
    spawn_asteroid_with_vertices, Asteroid, AsteroidSize, BaseVertices, CraterData, Planet,
    Vertices,
};
use crate::config::PhysicsConfig;
use crate::menu::GameState;
use crate::mining::spawn_ore_drop;
use crate::particles::{
    spawn_debris_particles, spawn_impact_particles, spawn_missile_trail_particles,
};
use bevy::input::gamepad::GamepadAxis;
use bevy::input::gamepad::GamepadButton;
use bevy::input::mouse::MouseButton;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::Rng;

#[path = "combat_helpers.rs"]
mod helpers;
use helpers::{
    area_weighted_mass_partition, even_mass_partition, impact_radiating_split_basis,
    normalized_fragment_hull, polygon_area, split_convex_polygon_world,
};

// ── Projectile firing ─────────────────────────────────────────────────────────

/// Unified fire system: handles Space / left-click (keyboard+mouse) and the
/// gamepad South button (A / Cross) from a single location.
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
    config: Res<PhysicsConfig>,
) {
    cooldown.timer = (cooldown.timer - time.delta_secs()).max(0.0);

    let Ok(transform) = q_player.single() else {
        return;
    };

    // ── Gamepad right stick: update aim direction ─────────────────────────────
    if let Some(gamepad_entity) = preferred.0 {
        if let Ok(gamepad) = gamepads.get(gamepad_entity) {
            let rx = gamepad.get(GamepadAxis::RightStickX).unwrap_or(0.0);
            let ry = gamepad.get(GamepadAxis::RightStickY).unwrap_or(0.0);
            let right_stick = Vec2::new(rx, ry);
            if right_stick.length() > config.gamepad_right_deadzone {
                aim.0 = right_stick.normalize_or_zero();
                // Right stick is active — prevent idle aim snap.
                idle.secs = 0.0;
            }
        }
    }

    let gamepad_wants_fire = preferred
        .0
        .and_then(|entity| gamepads.get(entity).ok())
        .is_some_and(|gamepad| gamepad.pressed(GamepadButton::South));

    let kb_fire = keys.pressed(KeyCode::Space);
    let mouse_fire = mouse_buttons.pressed(MouseButton::Left);

    if !(kb_fire || mouse_fire || gamepad_wants_fire) || cooldown.timer > 0.0 {
        return;
    }
    cooldown.timer = config.fire_cooldown;

    let fire_dir = if aim.0.length_squared() > 0.01 {
        aim.0.normalize_or_zero()
    } else {
        transform.rotation.mul_vec3(Vec3::Y).truncate()
    };

    let spawn_pos = transform.translation.truncate() + fire_dir * 14.0;

    commands.spawn((
        Projectile {
            age: 0.0,
            distance_traveled: 0.0,
            was_hit: false,
        },
        Transform::from_translation(spawn_pos.extend(0.0)),
        Visibility::default(),
        RigidBody::KinematicVelocityBased,
        Velocity {
            linvel: fire_dir * config.projectile_speed,
            angvel: 0.0,
        },
        Collider::ball(config.projectile_collider_radius),
        // Sensor: detects collision events for game logic but generates no contact
        // forces.  Without this, Rapier 0.22+ kinematic bodies push dynamic
        // asteroids — transferring significant momentum like a physical projectile
        // rather than the sci-fi "energy blaster" behaviour intended.
        Sensor,
        Ccd { enabled: true },
        CollisionGroups::new(
            bevy_rapier2d::geometry::Group::GROUP_3,
            bevy_rapier2d::geometry::Group::GROUP_1 | bevy_rapier2d::geometry::Group::GROUP_5,
        ),
        ActiveCollisionTypes::DYNAMIC_KINEMATIC,
        ActiveEvents::COLLISION_EVENTS,
    ));
}

// ── Projectile lifetime ───────────────────────────────────────────────────────

/// Age projectiles each frame and despawn them when they expire or leave bounds.
///
/// A projectile that expires without [`Projectile::was_hit`] being set is
/// considered a **miss** and resets the hit streak to zero.
pub fn despawn_old_projectiles_system(
    mut commands: Commands,
    mut q: Query<(Entity, &mut Projectile, &Velocity)>,
    time: Res<Time>,
    config: Res<PhysicsConfig>,
    mut score: ResMut<PlayerScore>,
) {
    let dt = time.delta_secs();
    for (entity, mut proj, velocity) in q.iter_mut() {
        proj.age += dt;
        proj.distance_traveled += velocity.linvel.length() * dt;
        let expired = proj.age >= config.projectile_lifetime
            || proj.distance_traveled > config.projectile_max_dist;
        if expired || proj.was_hit {
            if expired && !proj.was_hit {
                // Projectile ran out of range without hitting anything — break streak.
                score.streak = 0;
            }
            commands.entity(entity).despawn();
        }
    }
}

// ── Missile systems ────────────────────────────────────────────────────────────

/// Fire a missile when the player presses `X` / right-click / gamepad East button.
///
/// Missiles are heavier, slower, and more destructive than regular projectiles.
/// A missile uses one `[MissileAmmo]` count; silently does nothing when empty.
#[allow(clippy::too_many_arguments)]
pub fn missile_fire_system(
    mut commands: Commands,
    q_player: Query<&Transform, With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    preferred: Res<PreferredGamepad>,
    gamepads: Query<&Gamepad>,
    aim: Res<AimDirection>,
    mut cooldown: ResMut<MissileCooldown>,
    mut ammo: ResMut<MissileAmmo>,
    mut missile_telemetry: ResMut<crate::simulation::MissileTelemetry>,
    time: Res<Time>,
    config: Res<PhysicsConfig>,
) {
    cooldown.timer = (cooldown.timer - time.delta_secs()).max(0.0);

    let Ok(transform) = q_player.single() else {
        return;
    };

    // Gamepad East button (B on Xbox, Circle on PS)
    let mut gamepad_wants_fire = false;
    if let Some(gamepad_entity) = preferred.0 {
        if let Ok(gamepad) = gamepads.get(gamepad_entity) {
            if gamepad.just_pressed(GamepadButton::East) {
                gamepad_wants_fire = true;
            }
        }
    }

    let kb_fire = keys.just_pressed(KeyCode::KeyX);
    let mouse_fire = mouse_buttons.just_pressed(MouseButton::Right);

    if !(kb_fire || mouse_fire || gamepad_wants_fire) || cooldown.timer > 0.0 {
        return;
    }
    if ammo.count == 0 {
        return; // no ammo — ignore silently
    }

    cooldown.timer = config.missile_cooldown;
    ammo.count -= 1;
    missile_telemetry.shots_fired += 1;

    let fire_dir = if aim.0.length_squared() > 0.01 {
        aim.0.normalize_or_zero()
    } else {
        transform.rotation.mul_vec3(Vec3::Y).truncate()
    };

    let spawn_pos = transform.translation.truncate() + fire_dir * 16.0;

    commands.spawn((
        Missile {
            age: 0.0,
            distance_traveled: 0.0,
            trail_emit_timer: 0.0,
        },
        Transform::from_translation(spawn_pos.extend(0.0)),
        Visibility::default(),
        RigidBody::KinematicVelocityBased,
        Velocity {
            linvel: fire_dir * config.missile_initial_speed,
            angvel: 0.0,
        },
        Collider::ball(config.missile_collider_radius),
        Sensor,
        Ccd { enabled: true },
        CollisionGroups::new(
            bevy_rapier2d::geometry::Group::GROUP_3,
            bevy_rapier2d::geometry::Group::GROUP_1 | bevy_rapier2d::geometry::Group::GROUP_5,
        ),
        ActiveCollisionTypes::DYNAMIC_KINEMATIC,
        ActiveEvents::COLLISION_EVENTS,
    ));
}

/// Accelerate missiles in-flight until they reach configured max speed.
pub fn missile_acceleration_system(
    time: Res<Time>,
    config: Res<PhysicsConfig>,
    mut q: Query<&mut Velocity, With<Missile>>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 || config.missile_acceleration <= 0.0 {
        return;
    }

    for mut velocity in q.iter_mut() {
        let speed = velocity.linvel.length();
        if speed <= 1e-4 || speed >= config.missile_speed {
            continue;
        }

        let next_speed = (speed + config.missile_acceleration * dt).min(config.missile_speed);
        velocity.linvel = velocity.linvel.normalize() * next_speed;
    }
}

/// Age missiles each frame; despawn when they expire or go out of range.
pub fn despawn_old_missiles_system(
    mut commands: Commands,
    mut q: Query<(Entity, &mut Missile, &Velocity)>,
    time: Res<Time>,
    config: Res<PhysicsConfig>,
) {
    let dt = time.delta_secs();
    for (entity, mut missile, velocity) in q.iter_mut() {
        missile.age += dt;
        missile.distance_traveled += velocity.linvel.length() * dt;
        if missile.age >= config.missile_lifetime
            || missile.distance_traveled > config.missile_max_dist
        {
            commands.entity(entity).despawn();
        }
    }
}

/// Emit short-lived exhaust particles behind moving missiles.
///
/// The trail is emitted at a fixed cadence per missile so visuals remain
/// consistent regardless of frame rate.
pub fn missile_trail_particles_system(
    mut commands: Commands,
    mut q: Query<(&Transform, &Velocity, &mut Missile)>,
    time: Res<Time>,
) {
    const TRAIL_INTERVAL_SECS: f32 = 0.035;

    let dt = time.delta_secs();
    for (transform, velocity, mut missile) in q.iter_mut() {
        let speed_sq = velocity.linvel.length_squared();
        if speed_sq < 1.0 {
            continue;
        }

        missile.trail_emit_timer += dt;
        let dir = velocity.linvel.normalize();
        let nozzle_pos = transform.translation.truncate() - dir * 8.0;

        while missile.trail_emit_timer >= TRAIL_INTERVAL_SECS {
            missile.trail_emit_timer -= TRAIL_INTERVAL_SECS;
            spawn_missile_trail_particles(&mut commands, nozzle_pos, -dir, velocity.linvel);
        }
    }
}

/// Missile hit rules (different from regular projectiles):
///
/// | Asteroid size | Effect |
/// |---|---|
/// | `display_level >= size` | Full decomposition into unit fragments |
/// | `<= destroy_threshold()` | Immediate destroy + double destroy bonus |
/// | `> destroy_threshold()` | Split into level-scaled convex fragments (no chip path) |
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn missile_asteroid_hit_system(
    mut commands: Commands,
    mut collision_events: MessageReader<CollisionEvent>,
    q_asteroids: Query<
        (&AsteroidSize, &Transform, &Velocity, &Vertices),
        (With<Asteroid>, Without<Planet>),
    >,
    q_missiles: Query<&Transform, With<Missile>>,
    mut stats: ResMut<crate::simulation::SimulationStats>,
    mut score: ResMut<PlayerScore>,
    mut missile_telemetry: ResMut<crate::simulation::MissileTelemetry>,
    config: Res<PhysicsConfig>,
    missile_level: Res<super::SecondaryWeaponLevel>,
) {
    let mut processed_asteroids: std::collections::HashSet<Entity> = Default::default();
    let mut processed_missiles: std::collections::HashSet<Entity> = Default::default();

    for event in collision_events.read() {
        let (e1, e2) = match event {
            CollisionEvent::Started(e1, e2, _) => (*e1, *e2),
            CollisionEvent::Stopped(..) => continue,
        };

        let (missile_entity, asteroid_entity) =
            if q_missiles.contains(e1) && q_asteroids.contains(e2) {
                (e1, e2)
            } else if q_missiles.contains(e2) && q_asteroids.contains(e1) {
                (e2, e1)
            } else {
                continue;
            };

        if processed_missiles.contains(&missile_entity)
            || processed_asteroids.contains(&asteroid_entity)
        {
            continue;
        }

        let Ok((size, transform, velocity, vertices)) = q_asteroids.get(asteroid_entity) else {
            continue;
        };

        processed_missiles.insert(missile_entity);
        processed_asteroids.insert(asteroid_entity);

        // Capture missile world position before the deferred despawn flushes it.
        let missile_pos = q_missiles
            .get(missile_entity)
            .map(|t| t.translation.truncate())
            .unwrap_or_else(|_| transform.translation.truncate());

        commands.entity(missile_entity).despawn();

        let pos = transform.translation.truncate();
        let vel = velocity.linvel;
        let ang_vel = velocity.angvel;
        let n = size.0;

        // Missiles grant streak + multiplier like bullets.
        score.hits += 1;
        score.streak += 1;
        let multiplier = score.multiplier();
        missile_telemetry.hits += 1;

        let destroy_threshold = missile_level.destroy_threshold();

        if n <= destroy_threshold {
            // ── Instant destroy (small asteroids) ─────────────────────────────
            commands.entity(asteroid_entity).despawn();
            stats.destroyed_total += 1;
            score.destroyed += 1;
            missile_telemetry.instant_destroy_events += 1;
            missile_telemetry.destroyed_mass_total += n;
            // Missiles award double the destroy bonus for small targets.
            score.points += multiplier + 10 * multiplier;

            // Spawn ore drops (one per unit mass destroyed).
            let drop_count = n.max(1);
            for i in 0..drop_count {
                let angle = std::f32::consts::TAU * (i as f32 / drop_count as f32);
                let offset = Vec2::new(angle.cos(), angle.sin()) * 6.0;
                spawn_ore_drop(&mut commands, pos + offset, vel);
            }
            spawn_debris_particles(&mut commands, pos, vel, n + 2);
        } else if missile_level.can_fully_decompose_size(n) {
            // ── Full decomposition into unit asteroids ───────────────────────
            commands.entity(asteroid_entity).despawn();
            stats.split_total += 1;
            score.points += multiplier;
            missile_telemetry.full_decompose_events += 1;
            missile_telemetry.decomposed_mass_total += n;

            let impact_dir = (missile_pos - pos).normalize_or_zero();
            let impact_dir = if impact_dir == Vec2::ZERO {
                Vec2::X
            } else {
                impact_dir
            };
            let base_angle = impact_dir.y.atan2(impact_dir.x);
            for i in 0..n {
                let angle = base_angle + std::f32::consts::TAU * (i as f32 / n as f32);
                let dir = Vec2::new(angle.cos(), angle.sin());
                let spawn_pos = pos + dir * 9.0;
                let spawn_vel = vel + dir * 30.0;
                spawn_fragment_of_mass(
                    &mut commands,
                    spawn_pos,
                    spawn_vel,
                    ang_vel,
                    config.asteroid_density,
                    1,
                );
            }
            spawn_debris_particles(&mut commands, pos, vel, n.min(10));
        } else {
            // ── Split large asteroid into level-scaled convex fragments ───────
            score.points += multiplier;
            missile_telemetry.split_events += 1;
            let rot = transform.rotation;
            let world_verts: Vec<Vec2> = vertices
                .0
                .iter()
                .map(|v| pos + rot.mul_vec3(v.extend(0.0)).truncate())
                .collect();

            let split_axis = (missile_pos - pos).normalize_or_zero();
            let split_axis = if split_axis == Vec2::ZERO {
                Vec2::X
            } else {
                split_axis
            };

            let target_pieces = missile_level.split_piece_count(&config).min(n).max(2);
            let mut fragment_hulls: Vec<Vec<Vec2>> = vec![world_verts.clone()];
            let mut split_attempt = 0_u32;

            while fragment_hulls.len() < target_pieces as usize {
                let Some((largest_idx, largest_hull)) = fragment_hulls
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| polygon_area(a).total_cmp(&polygon_area(b)))
                    .map(|(idx, hull)| (idx, hull.clone()))
                else {
                    break;
                };

                let Some((split_origin, base_normal)) =
                    impact_radiating_split_basis(&largest_hull, missile_pos, split_axis)
                else {
                    break;
                };

                let spread = 0.42 * (split_attempt as f32 + 1.0);
                let base_angle = base_normal.to_angle();
                let candidate_axes = [
                    base_normal,
                    Vec2::from_angle(base_angle + spread),
                    Vec2::from_angle(base_angle - spread),
                    Vec2::new(-base_normal.y, base_normal.x).normalize_or_zero(),
                ];

                let mut split_result: Option<(Vec<Vec2>, Vec<Vec2>)> = None;
                for axis in candidate_axes {
                    if axis.length_squared() < 1e-5 {
                        continue;
                    }
                    let (front_raw, back_raw) =
                        split_convex_polygon_world(&largest_hull, split_origin, axis);
                    let Some(front_hull) = normalized_fragment_hull(&front_raw) else {
                        continue;
                    };
                    let Some(back_hull) = normalized_fragment_hull(&back_raw) else {
                        continue;
                    };
                    split_result = Some((front_hull, back_hull));
                    break;
                }

                let Some((front_hull, back_hull)) = split_result else {
                    break;
                };

                fragment_hulls.swap_remove(largest_idx);
                fragment_hulls.push(front_hull);
                fragment_hulls.push(back_hull);
                split_attempt += 1;
            }

            commands.entity(asteroid_entity).despawn();
            stats.split_total += 1;

            let areas: Vec<f32> = fragment_hulls
                .iter()
                .map(|hull| polygon_area(hull))
                .collect();
            let masses = area_weighted_mass_partition(&areas, n, target_pieces as usize);

            if fragment_hulls.len() == target_pieces as usize {
                for (hull_world, mass) in fragment_hulls.into_iter().zip(masses.into_iter()) {
                    let centroid =
                        hull_world.iter().copied().sum::<Vec2>() / hull_world.len() as f32;
                    let local: Vec<Vec2> = hull_world
                        .iter()
                        .map(|v| {
                            rot.inverse()
                                .mul_vec3((*v - centroid).extend(0.0))
                                .truncate()
                        })
                        .collect();
                    let target_area = mass as f32 / config.asteroid_density;
                    let local = rescale_vertices_to_area(&local, target_area);
                    let grey = 0.4 + rand::random::<f32>() * 0.3;
                    let frag_ent = spawn_asteroid_with_vertices(
                        &mut commands,
                        centroid,
                        &local,
                        Color::srgb(grey, grey, grey),
                        mass,
                    );

                    let kick_dir = (centroid - pos).normalize_or_zero();
                    let kick_dir = if kick_dir == Vec2::ZERO {
                        split_axis
                    } else {
                        kick_dir
                    };
                    commands.entity(frag_ent).insert(Velocity {
                        linvel: vel + kick_dir * 25.0,
                        angvel: ang_vel,
                    });
                    let preserved_transform =
                        Transform::from_translation(centroid.extend(0.05)).with_rotation(rot);
                    commands.entity(frag_ent).insert((
                        preserved_transform,
                        GlobalTransform::from(preserved_transform),
                    ));
                }
            } else {
                // Geometry fallback: still keep split-only semantics and target piece count.
                let fallback_masses = even_mass_partition(n, target_pieces as usize);
                for (idx, mass) in fallback_masses.into_iter().enumerate() {
                    let angle = std::f32::consts::TAU * idx as f32 / target_pieces as f32;
                    let dir = (split_axis + Vec2::from_angle(angle)).normalize_or_zero();
                    let dir = if dir == Vec2::ZERO { split_axis } else { dir };
                    let spawn_pos = pos + dir * 10.0;
                    spawn_fragment_of_mass(
                        &mut commands,
                        spawn_pos,
                        vel + dir * 28.0,
                        ang_vel,
                        config.asteroid_density,
                        mass,
                    );
                }
            }

            spawn_debris_particles(&mut commands, pos, vel, n.min(8));
        }
    }
}

// ── Player collision damage ────────────────────────────────────────────────────

/// Detect asteroid–player collisions and deal proportional damage.
///
/// Only activates when relative speed exceeds `DAMAGE_SPEED_THRESHOLD`.
/// Grants invincibility frames after each successful damage event.
///
/// On death: decrements [`PlayerLives`] and starts a respawn countdown.
/// When no lives remain, transitions to [`GameState::GameOver`].
#[allow(clippy::too_many_arguments)]
pub fn player_collision_damage_system(
    mut commands: Commands,
    mut q_player: Query<(Entity, &mut PlayerHealth, &Velocity), With<Player>>,
    q_asteroids: Query<&Velocity, With<Asteroid>>,
    rapier_context: ReadRapierContext,
    time: Res<Time>,
    config: Res<PhysicsConfig>,
    mut lives: ResMut<PlayerLives>,
    mut score: ResMut<PlayerScore>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok((player_entity, mut health, player_vel)) = q_player.single_mut() else {
        return;
    };

    // Tick down invincibility; tick up time since last damage.
    let dt = time.delta_secs();
    health.inv_timer = (health.inv_timer - dt).max(0.0);
    health.time_since_damage += dt;

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
            if rel_speed > config.damage_speed_threshold {
                total_damage += (rel_speed - config.damage_speed_threshold) * 0.5;
            }
        }
    }

    if total_damage > 0.0 {
        health.hp -= total_damage;
        health.inv_timer = config.invincibility_duration;
        health.time_since_damage = 0.0;
        if health.hp <= 0.0 {
            // Ship destroyed — consume one life.
            commands.entity(player_entity).despawn();
            lives.remaining -= 1;
            score.streak = 0; // death breaks the hit streak
            if lives.remaining <= 0 {
                // No lives left → game over.
                lives.remaining = 0;
                next_state.set(GameState::GameOver);
            } else {
                // Still have lives → schedule respawn.
                lives.respawn_timer = Some(config.respawn_delay_secs);
                println!(
                    "Player ship destroyed! Lives remaining: {}  Respawning in {:.1}s…",
                    lives.remaining, config.respawn_delay_secs
                );
            }
        }
    }
}

// ── Player respawn ────────────────────────────────────────────────────────────

/// Countdown the respawn timer and re-spawn the player ship when it reaches zero.
///
/// Only runs while no `Player` entity exists and `respawn_timer.is_some()`.
/// The freshly-spawned ship is given a long invincibility window so the player
/// can orient themselves before taking damage again.
pub fn player_respawn_system(
    mut commands: Commands,
    q_player: Query<(), With<Player>>,
    mut lives: ResMut<PlayerLives>,
    time: Res<Time>,
    config: Res<PhysicsConfig>,
) {
    // Only tick when the ship is absent and we have a pending respawn.
    if q_player.single().is_ok() {
        return;
    }
    let Some(ref mut timer) = lives.respawn_timer else {
        return;
    };
    *timer -= time.delta_secs();
    if *timer > 0.0 {
        return;
    }
    lives.respawn_timer = None;

    // Spawn with full HP and extended invincibility.
    let health = PlayerHealth {
        inv_timer: config.respawn_invincibility_secs,
        ..Default::default()
    };

    commands.spawn((
        Player,
        health,
        bevy_rapier2d::prelude::RigidBody::Dynamic,
        bevy_rapier2d::prelude::Collider::ball(config.player_collider_radius),
        bevy_rapier2d::prelude::Velocity::zero(),
        bevy_rapier2d::prelude::ExternalForce::default(),
        bevy_rapier2d::prelude::Damping {
            linear_damping: config.player_linear_damping,
            angular_damping: config.player_angular_damping,
        },
        bevy_rapier2d::prelude::Restitution::coefficient(config.player_restitution),
        bevy_rapier2d::prelude::CollisionGroups::new(
            bevy_rapier2d::geometry::Group::GROUP_2,
            bevy_rapier2d::geometry::Group::GROUP_1
                | bevy_rapier2d::geometry::Group::GROUP_4
                | bevy_rapier2d::geometry::Group::GROUP_5
                | bevy_rapier2d::geometry::Group::GROUP_6,
        ),
        bevy_rapier2d::prelude::ActiveEvents::COLLISION_EVENTS,
        Transform::from_translation(Vec3::ZERO),
        Visibility::default(),
    ));
}

// ── Projectile–Asteroid hit system ───────────────────────────────────────────

/// Process projectile-asteroid collision events and apply size-appropriate destruction.
///
/// Matches `CollisionEvent::Started` pairs; ignores `Stopped`.
/// Uses two `HashSet`s to ensure each projectile and each asteroid is processed at
/// most once per frame even if they appear in multiple cascade events.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn projectile_asteroid_hit_system(
    mut commands: Commands,
    mut collision_events: MessageReader<CollisionEvent>,
    q_asteroids: Query<
        (
            &AsteroidSize,
            &Transform,
            Option<&Velocity>,
            &Vertices,
            Option<&BaseVertices>,
            Option<&CraterData>,
        ),
        (With<Asteroid>, Without<Planet>),
    >,
    mut q_proj: Query<(&Transform, &mut Projectile)>,
    mut stats: ResMut<crate::simulation::SimulationStats>,
    mut score: ResMut<PlayerScore>,
    config: Res<PhysicsConfig>,
    weapon_level: Res<PrimaryWeaponLevel>,
) {
    let mut processed_asteroids: std::collections::HashSet<Entity> = Default::default();
    let mut processed_projectiles: std::collections::HashSet<Entity> = Default::default();

    for event in collision_events.read() {
        let (e1, e2) = match event {
            CollisionEvent::Started(e1, e2, _) => (*e1, *e2),
            CollisionEvent::Stopped(..) => continue,
        };

        // Identify which entity is the projectile and which the asteroid
        let (proj_entity, asteroid_entity) = if q_proj.contains(e1) && q_asteroids.contains(e2) {
            (e1, e2)
        } else if q_proj.contains(e2) && q_asteroids.contains(e1) {
            (e2, e1)
        } else {
            continue;
        };

        if processed_projectiles.contains(&proj_entity)
            || processed_asteroids.contains(&asteroid_entity)
        {
            continue;
        }

        let Ok((size, transform, velocity, vertices, base_vertices, crater_data)) =
            q_asteroids.get(asteroid_entity)
        else {
            continue; // Asteroid may have been despawned already
        };

        processed_projectiles.insert(proj_entity);
        processed_asteroids.insert(asteroid_entity);

        // Mark the projectile as hit so the lifetime system knows to despawn it
        // without counting it as a missed shot.  We do NOT despawn immediately so
        // that split/chip paths can still read its world-space position this frame.
        let proj_pos = q_proj
            .get(proj_entity)
            .map(|(t, _)| t.translation.truncate())
            .unwrap_or_else(|_| transform.translation.truncate());
        if let Ok((_, mut proj)) = q_proj.get_mut(proj_entity) {
            proj.was_hit = true;
        }

        let pos = transform.translation.truncate();
        let rot = transform.rotation;
        let vel = velocity.map_or(Vec2::ZERO, |v| v.linvel);
        let ang_vel = velocity.map_or(0.0, |v| v.angvel);
        let n = size.0;

        // World-space hull vertices (used by split and chip paths)
        let world_verts: Vec<Vec2> = vertices
            .0
            .iter()
            .map(|v| pos + rot.mul_vec3(v.extend(0.0)).truncate())
            .collect();

        // Increment streak and compute multiplier BEFORE accumulating points so
        // the threshold hit itself immediately benefits from the new tier.
        score.hits += 1;
        score.streak += 1;
        let multiplier = score.multiplier();
        score.points += multiplier; // 1 × multiplier for the hit itself

        // Unified impact direction for particle effects (projectile → asteroid).
        let impact_dir = (pos - proj_pos).normalize_or_zero();

        let destroy_threshold = weapon_level.max_destroy_size();

        // ── Level-gated full destroy ──────────────────────────────────────────
        // The primary weapon fully eliminates asteroids up to `max_destroy_size`.
        // Anything larger is always chipped (1 vertex removed, 1-unit fragment
        // ejected) regardless of level, so no hit ever removes more than half the
        // target.
        if n <= destroy_threshold {
            commands.entity(asteroid_entity).despawn();
            stats.destroyed_total += 1;
            score.destroyed += 1;
            score.points += 5 * multiplier; // bonus for full destroy
                                            // Scatter one ore drop per mass unit so larger destroys yield more ore.
            let drop_count = n.max(1);
            for i in 0..drop_count {
                let angle = std::f32::consts::TAU * (i as f32 / drop_count as f32);
                let offset = Vec2::new(angle.cos(), angle.sin()) * 6.0;
                spawn_ore_drop(&mut commands, pos + offset, vel);
            }
            spawn_impact_particles(&mut commands, proj_pos, impact_dir, vel);
            spawn_debris_particles(&mut commands, pos, vel, n.max(1));
        } else {
            // ── Chip: spawn fragment + inward local deformation ───────────────
            spawn_impact_particles(&mut commands, proj_pos, impact_dir, vel);
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

            // Chip size scales with weapon level: level L can chip 1..=min(L, n/2).
            let max_chip_size = weapon_level.display_level().min(n / 2).max(1);
            let chip_size = if max_chip_size <= 1 {
                1u32
            } else {
                rng.gen_range(1u32..=max_chip_size)
            };

            let chip_vel = vel
                + chip_dir * 40.0
                + Vec2::new(rng.gen_range(-15.0..15.0), rng.gen_range(-15.0..15.0));
            spawn_fragment_of_mass(
                &mut commands,
                chip_pos,
                chip_vel,
                0.0,
                config.asteroid_density,
                chip_size,
            );
            let new_mass = (n - chip_size).max(1);

            // Add new crater at impact point
            let impact_local = rot
                .inverse()
                .mul_vec3((proj_pos - pos).extend(0.0))
                .truncate();
            let base_vertices_local = base_vertices
                .map(|base| base.0.clone())
                .unwrap_or_else(|| vertices.0.clone());
            let bounding_radius = base_vertices_local
                .iter()
                .map(|v| v.length())
                .fold(0.0f32, f32::max);
            let crater_radius = bounding_radius * config.crater_radius_ratio;

            let mut new_crater_data = crater_data.cloned().unwrap_or_default();
            new_crater_data.craters.push((
                impact_local,
                config.crater_depth_per_hit,
                crater_radius,
            ));

            // Limit crater count
            if new_crater_data.craters.len() > config.max_craters_per_asteroid {
                new_crater_data.craters.remove(0);
            }

            // Apply all craters to base vertices to get deformed shape
            let deformed_local =
                apply_crater_deformation(&base_vertices_local, &new_crater_data.craters, &config)
                    .unwrap_or_else(|| base_vertices_local.clone());

            // Rescale to match new mass after chip removal
            let target_area = new_mass as f32 / config.asteroid_density;
            let new_local = rescale_vertices_to_area(&deformed_local, target_area);

            // Also rescale base vertices to match new mass
            let new_base = rescale_vertices_to_area(&base_vertices_local, target_area);
            let hull_centroid = pos;

            commands.entity(asteroid_entity).despawn();

            let grey = 0.4 + rand::random::<f32>() * 0.3;
            let new_ent = spawn_asteroid_with_vertices(
                &mut commands,
                hull_centroid,
                &new_local,
                Color::srgb(grey, grey, grey),
                new_mass,
            );
            let preserved_transform =
                Transform::from_translation(hull_centroid.extend(0.05)).with_rotation(rot);
            commands.entity(new_ent).insert(Velocity {
                linvel: vel,
                angvel: ang_vel,
            });
            commands.entity(new_ent).insert((
                preserved_transform,
                GlobalTransform::from(preserved_transform),
            ));
            commands
                .entity(new_ent)
                .insert((new_crater_data, BaseVertices(new_base)));
        }
    }
}

/// Consume projectile/missile hits against planets without awarding score.
///
/// - Projectiles are marked as hit so lifetime cleanup despawns them.
/// - Missiles are despawned immediately on contact.
pub fn projectile_missile_planet_hit_system(
    mut commands: Commands,
    mut collision_events: MessageReader<CollisionEvent>,
    q_planets: Query<Entity, With<Planet>>,
    mut q_proj: Query<&mut Projectile>,
    q_missiles: Query<(), With<Missile>>,
) {
    for event in collision_events.read() {
        let (e1, e2) = match event {
            CollisionEvent::Started(e1, e2, _) => (*e1, *e2),
            CollisionEvent::Stopped(..) => continue,
        };

        let (planet_entity, other_entity) = if q_planets.contains(e1) {
            (e1, e2)
        } else if q_planets.contains(e2) {
            (e2, e1)
        } else {
            continue;
        };

        let _ = planet_entity;

        if let Ok(mut projectile) = q_proj.get_mut(other_entity) {
            projectile.was_hit = true;
            continue;
        }

        if q_missiles.contains(other_entity) {
            commands.entity(other_entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::{App, MinimalPlugins};

    fn setup_projectile_lifetime_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(crate::config::PhysicsConfig::default())
            .insert_resource(PlayerScore::default())
            .add_systems(Update, despawn_old_projectiles_system);
        app
    }

    fn setup_missile_lifetime_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(crate::config::PhysicsConfig::default())
            .add_systems(Update, despawn_old_missiles_system);
        app
    }

    fn setup_missile_hit_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<CollisionEvent>()
            .insert_resource(crate::config::PhysicsConfig::default())
            .insert_resource(crate::simulation::SimulationStats::default())
            .insert_resource(PlayerScore::default())
            .insert_resource(crate::simulation::MissileTelemetry::default())
            .insert_resource(crate::player::state::SecondaryWeaponLevel::default())
            .add_systems(PostUpdate, missile_asteroid_hit_system);
        app
    }

    #[test]
    fn projectile_not_despawned_just_for_being_far_from_origin() {
        let mut app = setup_projectile_lifetime_test_app();
        let cfg = app
            .world()
            .resource::<crate::config::PhysicsConfig>()
            .clone();

        let entity = app
            .world_mut()
            .spawn((
                Projectile {
                    age: 0.0,
                    distance_traveled: 0.0,
                    was_hit: false,
                },
                Velocity {
                    linvel: Vec2::ZERO,
                    angvel: 0.0,
                },
                Transform::from_translation(Vec3::new(cfg.hard_cull_distance * 2.0, 0.0, 0.0)),
            ))
            .id();

        app.update();

        assert!(
            app.world().get_entity(entity).is_ok(),
            "projectile should not expire merely from being beyond world border"
        );
    }

    #[test]
    fn projectile_despawns_when_traveled_distance_exceeds_max() {
        let mut app = setup_projectile_lifetime_test_app();
        let cfg = app
            .world()
            .resource::<crate::config::PhysicsConfig>()
            .clone();

        let entity = app
            .world_mut()
            .spawn((
                Projectile {
                    age: 0.0,
                    distance_traveled: cfg.projectile_max_dist + 1.0,
                    was_hit: false,
                },
                Velocity {
                    linvel: Vec2::ZERO,
                    angvel: 0.0,
                },
                Transform::from_translation(Vec3::ZERO),
            ))
            .id();

        app.update();

        assert!(
            app.world().get_entity(entity).is_err(),
            "projectile should expire when traveled distance exceeds projectile_max_dist"
        );
    }

    #[test]
    fn missile_not_despawned_just_for_being_far_from_origin() {
        let mut app = setup_missile_lifetime_test_app();
        let cfg = app
            .world()
            .resource::<crate::config::PhysicsConfig>()
            .clone();

        let entity = app
            .world_mut()
            .spawn((
                Missile {
                    age: 0.0,
                    distance_traveled: 0.0,
                    trail_emit_timer: 0.0,
                },
                Velocity {
                    linvel: Vec2::ZERO,
                    angvel: 0.0,
                },
                Transform::from_translation(Vec3::new(cfg.hard_cull_distance * 2.0, 0.0, 0.0)),
            ))
            .id();

        app.update();

        assert!(
            app.world().get_entity(entity).is_ok(),
            "missile should not expire merely from being beyond world border"
        );
    }

    #[test]
    fn missile_split_replacement_preserves_rotation() {
        let mut app = setup_missile_hit_test_app();

        let original_rotation = Quat::from_rotation_z(1.17);
        let asteroid = app
            .world_mut()
            .spawn((
                Asteroid,
                AsteroidSize(6),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0))
                    .with_rotation(original_rotation),
                Velocity {
                    linvel: Vec2::new(3.0, -2.0),
                    angvel: 0.35,
                },
                Vertices(vec![
                    Vec2::new(0.0, 10.0),
                    Vec2::new(-8.0, 5.0),
                    Vec2::new(-9.0, -4.0),
                    Vec2::new(0.0, -10.0),
                    Vec2::new(9.0, -4.0),
                    Vec2::new(8.0, 5.0),
                ]),
            ))
            .id();

        let missile = app
            .world_mut()
            .spawn((
                Missile::default(),
                Transform::from_translation(Vec3::new(20.0, 0.0, 0.0)),
            ))
            .id();

        app.world_mut().write_message(CollisionEvent::Started(
            missile,
            asteroid,
            bevy_rapier2d::rapier::geometry::CollisionEventFlags::empty(),
        ));

        app.update();

        assert!(app.world().get_entity(missile).is_err());
        assert!(app.world().get_entity(asteroid).is_err());

        let mut q_fragments = app.world_mut().query::<(&AsteroidSize, &Transform)>();
        let fragment_rotations: Vec<Quat> = q_fragments
            .iter(app.world())
            .map(|(_, transform)| transform.rotation)
            .collect();

        assert_eq!(
            fragment_rotations.len(),
            2,
            "default missile split should create two replacement fragments"
        );

        for (i, rotation) in fragment_rotations.iter().enumerate() {
            assert!(
                rotation.dot(original_rotation).abs() > 0.9999,
                "fragment {i} should preserve source asteroid orientation"
            );
        }
    }

    #[test]
    fn impact_radiating_split_basis_anchors_near_impact_edge() {
        let square = vec![
            Vec2::new(-10.0, -10.0),
            Vec2::new(10.0, -10.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(-10.0, 10.0),
        ];
        let impact = Vec2::new(20.0, 2.0);
        let (origin, _) = impact_radiating_split_basis(&square, impact, Vec2::X)
            .expect("split basis should exist for convex hull");

        assert!(
            origin.x > 7.0,
            "origin should stay near impact-side edge, got x={}",
            origin.x
        );
        assert!(
            origin.y > -9.5 && origin.y < 9.5,
            "origin should stay inside hull y-bounds, got y={}",
            origin.y
        );
    }

    #[test]
    fn impact_radiating_split_basis_aligns_cut_with_impact_ray() {
        let square = vec![
            Vec2::new(-10.0, -10.0),
            Vec2::new(10.0, -10.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(-10.0, 10.0),
        ];

        let impact = Vec2::new(20.0, 0.0);
        let (origin, normal) = impact_radiating_split_basis(&square, impact, Vec2::X)
            .expect("split basis should exist for convex hull");

        let centroid = square.iter().copied().sum::<Vec2>() / square.len() as f32;
        let impact_ray = (centroid - origin).normalize_or_zero();
        let cut_direction = Vec2::new(-normal.y, normal.x).normalize_or_zero();
        let alignment = impact_ray.dot(cut_direction).abs();

        assert!(
            alignment > 0.9,
            "cut direction should align with impact ray (alignment={alignment})"
        );
    }

    #[test]
    fn missile_split_helper_preserves_square_area() {
        let square = vec![
            Vec2::new(-10.0, -10.0),
            Vec2::new(10.0, -10.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(-10.0, 10.0),
        ];

        let (front_raw, back_raw) = split_convex_polygon_world(&square, Vec2::ZERO, Vec2::X);
        let front =
            crate::asteroid::compute_convex_hull_from_points(&front_raw).expect("front split hull");
        let back =
            crate::asteroid::compute_convex_hull_from_points(&back_raw).expect("back split hull");

        let total = polygon_area(&square);
        let split_total = polygon_area(&front) + polygon_area(&back);

        assert!(
            (total - split_total).abs() < 1e-3,
            "split area should be preserved (total={total}, split_total={split_total})"
        );
    }

    // ── split_convex_polygon ──────────────────────────────────────────────────

    /// Split a convex polygon (world-space vertices) with a plane through `origin`
    /// whose normal is `axis`.
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
                front.push(a + (b - a) * t);
                back.push(a + (b - a) * t);
            }
        }
        (front, back)
    }

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

    // ── Split geometry ────────────────────────────────────────────────────────

    /// Run the full split pipeline on a polygon and assert both halves produce
    /// valid, spawn-able asteroids.  Vertex-count requirements are no longer
    /// enforced; the only hard constraint is ≥3 vertices and a valid collider.
    fn assert_split_halves_valid(shape_name: &str, verts: &[Vec2], axis: Vec2, density: f32) {
        let origin = Vec2::ZERO;
        let (front_raw, back_raw) = split_convex_polygon(verts, origin, axis);
        for (side, raw) in [("front", &front_raw), ("back", &back_raw)] {
            if raw.len() < 3 {
                continue; // empty half is acceptable
            }
            let hull = match crate::asteroid::compute_convex_hull_from_points(raw) {
                Some(h) if h.len() >= 3 => h,
                _ => continue,
            };
            let centroid: Vec2 = hull.iter().copied().sum::<Vec2>() / hull.len() as f32;
            let local: Vec<Vec2> = hull.iter().map(|v| *v - centroid).collect();
            // Density-rescaled area must produce a valid collider.
            let mass = 2u32; // minimum representative mass
            let rescaled = crate::asteroid::rescale_vertices_to_area(&local, mass as f32 / density);
            let collider = bevy_rapier2d::prelude::Collider::convex_hull(&rescaled);
            assert!(
                collider.is_some(),
                "{shape_name} {side}: Collider::convex_hull returned None for {} verts",
                rescaled.len()
            );
            assert!(
                rescaled.len() >= 3,
                "{shape_name} {side}: need ≥3 verts, got {}",
                rescaled.len()
            );
        }
    }

    #[test]
    fn split_size4_square_both_halves_valid() {
        use crate::constants::SQUARE_BASE_HALF;
        let h = SQUARE_BASE_HALF;
        let square = vec![
            Vec2::new(-h, -h),
            Vec2::new(h, -h),
            Vec2::new(h, h),
            Vec2::new(-h, h),
        ];
        for axis in [Vec2::X, Vec2::Y, Vec2::new(1.0, 1.0).normalize()] {
            assert_split_halves_valid("square_size4", &square, axis, 0.1);
        }
    }

    #[test]
    fn split_size5_pentagon_both_halves_valid() {
        use crate::constants::POLYGON_BASE_RADIUS;
        let r = POLYGON_BASE_RADIUS;
        let pent: Vec<Vec2> = (0..5)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / 5.0;
                Vec2::new(r * a.cos(), r * a.sin())
            })
            .collect();
        for axis in [Vec2::X, Vec2::Y] {
            assert_split_halves_valid("pentagon_size5", &pent, axis, 0.1);
        }
    }

    #[test]
    fn split_size6_hexagon_both_halves_valid() {
        use crate::constants::POLYGON_BASE_RADIUS;
        let r = POLYGON_BASE_RADIUS;
        let hex: Vec<Vec2> = (0..6)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / 6.0;
                Vec2::new(r * a.cos(), r * a.sin())
            })
            .collect();
        for axis in [Vec2::X, Vec2::Y, Vec2::new(1.0, 1.0).normalize()] {
            assert_split_halves_valid("hexagon_size6", &hex, axis, 0.1);
        }
    }

    #[test]
    fn chip_bevel_adds_vertex_and_produces_valid_collider() {
        // Chip a triangle at its first vertex: expect a quadrilateral (4 verts).
        let tri = vec![
            Vec2::new(0.0, 10.0),
            Vec2::new(-8.0, -5.0),
            Vec2::new(8.0, -5.0),
        ];
        let tip_idx = 0usize;
        let n_verts = tri.len();
        let prev_idx = (tip_idx + n_verts - 1) % n_verts;
        let next_idx = (tip_idx + 1) % n_verts;
        let tip = tri[tip_idx];
        let cut_a = tip + (tri[prev_idx] - tip) * 0.30;
        let cut_b = tip + (tri[next_idx] - tip) * 0.30;
        let mut bevelled: Vec<Vec2> = Vec::new();
        for (i, &v) in tri.iter().enumerate() {
            if i == tip_idx {
                bevelled.push(cut_a);
                bevelled.push(cut_b);
            } else {
                bevelled.push(v);
            }
        }
        let hull = crate::asteroid::compute_convex_hull_from_points(&bevelled).unwrap_or(bevelled);
        assert_eq!(
            hull.len(),
            4,
            "triangle chipped at corner should become quadrilateral"
        );
        let collider = bevy_rapier2d::prelude::Collider::convex_hull(&hull);
        assert!(
            collider.is_some(),
            "bevelled hull must produce a valid collider"
        );
    }
}

/// Spawn an asteroid fragment of arbitrary `mass` at `pos` with the given velocity.
///
/// Fragment shape is determined by [`canonical_vertices_for_mass`] and scaled to
/// the correct area for the requested mass.  Used by the chip path when a higher
/// weapon level chips off more than one mass unit.
fn spawn_fragment_of_mass(
    commands: &mut Commands,
    pos: Vec2,
    velocity: Vec2,
    angvel: f32,
    density: f32,
    mass: u32,
) {
    let grey = 0.4 + rand::random::<f32>() * 0.4;
    let verts = rescale_vertices_to_area(&canonical_vertices_for_mass(mass), mass as f32 / density);
    let ent =
        spawn_asteroid_with_vertices(commands, pos, &verts, Color::srgb(grey, grey, grey), mass);
    commands.entity(ent).insert(Velocity {
        linvel: velocity,
        angvel,
    });
}
