//! Player character: space ship entity, controls, projectile firing, and camera follow.

use crate::asteroid::{
    compute_convex_hull_from_points, spawn_asteroid_with_vertices, AsteroidSize, Vertices,
};
use bevy::input::gamepad::{GamepadAxis, GamepadAxisType, GamepadButton, GamepadButtonType};
use bevy::input::mouse::MouseButton;
use bevy::input::Axis;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::Rng;

// ── Constants ────────────────────────────────────────────────────────────────

/// Forward thrust force applied per frame while W is held.
const THRUST_FORCE: f32 = 120.0;
/// Reverse thrust (S key) is weaker than forward.
const REVERSE_FORCE: f32 = 60.0;
/// Angular velocity (rad/s) applied while A or D is held.
const ROTATION_SPEED: f32 = 3.0;
/// Speed at which projectiles travel (units/s).
const PROJECTILE_SPEED: f32 = 500.0;
/// Seconds between two consecutive shots.
const FIRE_COOLDOWN: f32 = 0.2;
/// Seconds before a projectile is despawned.
const PROJECTILE_LIFETIME: f32 = 3.0;
/// Asteroid cull radius — player gets damped beyond this.
const OOB_RADIUS: f32 = 1000.0;
/// Velocity decay per frame applied when player is outside OOB_RADIUS.
const OOB_DAMPING: f32 = 0.97;
/// Maximum player HP.
const PLAYER_MAX_HP: f32 = 100.0;
/// Relative speed (u/s) above which asteroid impacts deal damage.
const DAMAGE_SPEED_THRESHOLD: f32 = 30.0;
/// Seconds of invincibility after taking a hit.
const INVINCIBILITY_DURATION: f32 = 0.5;

// ── Components / Resources ───────────────────────────────────────────────────

/// Marker component for the player ship entity.
#[derive(Component)]
pub struct Player;

/// Tracks player health and invincibility state.
#[derive(Component)]
pub struct PlayerHealth {
    pub hp: f32,
    pub max_hp: f32,
    /// Remaining seconds of invincibility after a hit.
    pub inv_timer: f32,
}

impl Default for PlayerHealth {
    fn default() -> Self {
        Self {
            hp: PLAYER_MAX_HP,
            max_hp: PLAYER_MAX_HP,
            inv_timer: 0.0,
        }
    }
}

/// Tracks when the player last fired so we can enforce the cooldown.
#[derive(Resource, Default)]
pub struct PlayerFireCooldown {
    pub timer: f32,
}

/// World-space unit vector the player is currently aiming.
/// Updated every frame by `mouse_aim_system` (cursor) or `projectile_fire_system` (gamepad right stick).
/// Falls back to the ship's forward direction when no explicit aim is available.
#[derive(Resource, Clone, Copy)]
pub struct AimDirection(pub Vec2);

impl Default for AimDirection {
    fn default() -> Self {
        Self(Vec2::Y) // ship starts pointing up
    }
}

/// Attached to each projectile; stores its age in seconds.
#[derive(Component)]
pub struct Projectile {
    pub age: f32,
}

// ── Ship geometry helpers ────────────────────────────────────────────────────

/// Local-space vertices of the ship polygon (pointing in +Y / "up" in local space).
/// The shape is a dart: a long nose at the top and two swept-back fins at the bottom.
fn ship_vertices() -> Vec<Vec2> {
    vec![
        Vec2::new(0.0, 12.0),  // nose
        Vec2::new(-8.0, -8.0), // left fin tip
        Vec2::new(-3.0, -4.0), // left fin inner
        Vec2::new(0.0, -10.0), // tail notch
        Vec2::new(3.0, -4.0),  // right fin inner
        Vec2::new(8.0, -8.0),  // right fin tip
    ]
}

// ── Spawn ────────────────────────────────────────────────────────────────────

/// Spawn the player's ship at the world origin.
pub fn spawn_player(mut commands: Commands) {
    // Ship collider as a small ball (simpler than convex polygon, fine for v1)
    commands.spawn((
        Player,
        PlayerHealth::default(),
        // Physics
        RigidBody::Dynamic,
        Collider::ball(8.0),
        Velocity::zero(),
        ExternalForce::default(),
        Damping {
            linear_damping: 0.5,
            angular_damping: 3.0,
        },
        Restitution::coefficient(0.3),
        // GROUP_2: collides with GROUP_1 (asteroids) only, not projectiles
        CollisionGroups::new(
            bevy_rapier2d::geometry::Group::GROUP_2,
            bevy_rapier2d::geometry::Group::GROUP_1,
        ),
        ActiveEvents::COLLISION_EVENTS,
        // Transform / visibility
        TransformBundle::from_transform(Transform::from_translation(Vec3::ZERO)),
        VisibilityBundle::default(),
    ));

    println!("✓ Player ship spawned at origin");
}

// ── Control system ───────────────────────────────────────────────────────────

/// Resets `ExternalForce` to zero at the start of each frame so keyboard and
/// gamepad systems can independently add their contributions without clobbering each other.
pub fn player_force_reset_system(mut q: Query<&mut ExternalForce, With<Player>>) {
    if let Ok(mut force) = q.get_single_mut() {
        force.force = Vec2::ZERO;
        force.torque = 0.0;
    }
}

/// Applies WASD thrust / rotation to the player ship (keyboard).
pub fn player_control_system(
    mut q: Query<(&Transform, &mut ExternalForce, &mut Velocity), With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let Ok((transform, mut force, mut velocity)) = q.get_single_mut() else {
        return;
    };

    // Local "up" direction is +Y in local space; rotate into world space
    let forward = transform.rotation.mul_vec3(Vec3::Y).truncate();

    // W → thrust forward
    if keys.pressed(KeyCode::KeyW) {
        force.force += forward * THRUST_FORCE;
    }
    // S → thrust backward (gentle reverse)
    if keys.pressed(KeyCode::KeyS) {
        force.force -= forward * REVERSE_FORCE;
    }

    // A/D → direct angular velocity override for snappy rotation
    if keys.pressed(KeyCode::KeyA) {
        velocity.angvel = ROTATION_SPEED;
    } else if keys.pressed(KeyCode::KeyD) {
        velocity.angvel = -ROTATION_SPEED;
    }
    // If neither A nor D, let angular damping handle slow-down naturally
}

// ── Gamepad movement system ───────────────────────────────────────────────────

/// Twin-stick gamepad movement:
/// - **Left stick**: rotates the ship at a fixed angular speed toward the stick direction,
///   then applies forward thrust once roughly aligned (within ~0.5 rad).
/// - **B button (East)**: applies reverse thrust instead while held.
pub fn gamepad_movement_system(
    mut q: Query<(&Transform, &mut ExternalForce, &mut Velocity), With<Player>>,
    gamepads: Res<Gamepads>,
    axes: Res<Axis<GamepadAxis>>,
    buttons: Res<ButtonInput<GamepadButton>>,
) {
    let Ok((transform, mut force, mut velocity)) = q.get_single_mut() else {
        return;
    };
    let Some(gamepad) = gamepads.iter().next() else {
        return;
    };

    const DEADZONE: f32 = 0.15;

    let lx = axes
        .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickX))
        .unwrap_or(0.0);
    let ly = axes
        .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickY))
        .unwrap_or(0.0);
    let left_stick = Vec2::new(lx, ly);

    if left_stick.length() < DEADZONE {
        return;
    }

    // Compute the target world-space angle for the ship's +Y axis to point along left_stick.
    // atan2(-lx, ly) maps stick (0,1)→0°, (1,0)→-90°, (-1,0)→+90° correctly.
    let target_angle = (-lx).atan2(ly);
    let current_angle = transform.rotation.to_euler(EulerRot::ZYX).0;

    // Find the shortest angular path to the target
    let mut angle_diff = target_angle - current_angle;
    while angle_diff > std::f32::consts::PI {
        angle_diff -= std::f32::consts::TAU;
    }
    while angle_diff < -std::f32::consts::PI {
        angle_diff += std::f32::consts::TAU;
    }

    // Rotate at fixed speed toward the target direction
    velocity.angvel = if angle_diff.abs() > 0.05 {
        ROTATION_SPEED * angle_diff.signum()
    } else {
        0.0
    };

    // Apply thrust only when roughly aligned; B (East) reverses
    let forward = transform.rotation.mul_vec3(Vec3::Y).truncate();
    let reverse = buttons.pressed(GamepadButton::new(gamepad, GamepadButtonType::East));

    if reverse {
        force.force -= forward * REVERSE_FORCE * left_stick.length();
    } else if angle_diff.abs() < 0.5 {
        // Only thrust when within ~30 ° of the target to avoid fighting rotation
        force.force += forward * THRUST_FORCE * left_stick.length();
    }
}

// ── Projectile fire system ───────────────────────────────────────────────────

/// Unified fire system: handles Space / left-click (keyboard+mouse) and the gamepad right stick
/// (twin-stick auto-fire).  In all cases the projectile travels toward `AimDirection`.
///
/// The gamepad right stick also *updates* `AimDirection` each frame so the cursor-aim and
/// stick-aim sources are kept separate yet coherent.
#[allow(clippy::too_many_arguments)]
pub fn projectile_fire_system(
    mut commands: Commands,
    q_player: Query<&Transform, With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    gamepads: Res<Gamepads>,
    axes: Res<Axis<GamepadAxis>>,
    mut aim: ResMut<AimDirection>,
    mut cooldown: ResMut<PlayerFireCooldown>,
    time: Res<Time>,
) {
    cooldown.timer = (cooldown.timer - time.delta_seconds()).max(0.0);

    let Ok(transform) = q_player.get_single() else {
        return;
    };

    // ── Gamepad right stick: update aim + auto-fire when pushed past the threshold ──
    const GP_DEADZONE: f32 = 0.2;
    const GP_FIRE_THRESHOLD: f32 = 0.5;
    let mut gamepad_wants_fire = false;

    if let Some(gamepad) = gamepads.iter().next() {
        let rx = axes
            .get(GamepadAxis::new(gamepad, GamepadAxisType::RightStickX))
            .unwrap_or(0.0);
        let ry = axes
            .get(GamepadAxis::new(gamepad, GamepadAxisType::RightStickY))
            .unwrap_or(0.0);
        let right_stick = Vec2::new(rx, ry);

        if right_stick.length() > GP_DEADZONE {
            aim.0 = right_stick.normalize_or_zero();
            if right_stick.length() > GP_FIRE_THRESHOLD {
                gamepad_wants_fire = true;
            }
        }
    }

    // ── Determine if any source wants to fire this frame ──
    let kb_fire = keys.just_pressed(KeyCode::Space);
    let mouse_fire = mouse_buttons.just_pressed(MouseButton::Left);

    if !(kb_fire || mouse_fire || gamepad_wants_fire) || cooldown.timer > 0.0 {
        return;
    }
    cooldown.timer = FIRE_COOLDOWN;

    // Use current aim direction; fall back to ship forward if aim is zero
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
        Collider::ball(2.0),
        CollisionGroups::new(
            bevy_rapier2d::geometry::Group::GROUP_3,
            bevy_rapier2d::geometry::Group::GROUP_1,
        ),
        ActiveCollisionTypes::DYNAMIC_KINEMATIC,
        ActiveEvents::COLLISION_EVENTS,
    ));
}

// ── Projectile lifetime / despawn ────────────────────────────────────────────

/// Ages projectiles each frame and despawns them when they expire.
pub fn despawn_old_projectiles_system(
    mut commands: Commands,
    mut q: Query<(Entity, &mut Projectile, &Transform)>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();
    for (entity, mut proj, transform) in q.iter_mut() {
        proj.age += dt;
        let dist = transform.translation.truncate().length();
        if proj.age >= PROJECTILE_LIFETIME || dist > 1000.0 {
            commands.entity(entity).despawn();
        }
    }
}

// ── Camera follow system ──────────────────────────────────────────────────────

/// Keeps the camera centred on the player; handles zoom via mouse wheel.
pub fn camera_follow_system(
    q_player: Query<&Transform, With<Player>>,
    mut q_camera: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    let Ok(player_transform) = q_player.get_single() else {
        return;
    };
    let Ok(mut cam) = q_camera.get_single_mut() else {
        return;
    };

    // Keep camera at same XY as player; preserve Z (depth / scale unaffected here)
    cam.translation.x = player_transform.translation.x;
    cam.translation.y = player_transform.translation.y;
}

// ── Rendering ────────────────────────────────────────────────────────────────

/// Draw the player ship, health bar, aim indicator, and all active projectiles using Bevy gizmos.
pub fn player_gizmo_system(
    mut gizmos: Gizmos,
    q_player: Query<(&Transform, &PlayerHealth), With<Player>>,
    q_projectiles: Query<&Transform, With<Projectile>>,
    aim: Res<AimDirection>,
) {
    // ── Ship ──────────────────────────────────────────────────────────────────
    if let Ok((transform, health)) = q_player.get_single() {
        let pos = transform.translation.truncate();
        let rot = transform.rotation;
        let verts = ship_vertices();

        // Tint ship cyan when healthy; shift toward red as health drops
        let hp_frac = (health.hp / health.max_hp).clamp(0.0, 1.0);
        let ship_color = Color::rgb(1.0 - hp_frac * 0.8, hp_frac * 0.6 + 0.2, hp_frac);

        // Draw ship outline
        for i in 0..verts.len() {
            let v1 = verts[i];
            let v2 = verts[(i + 1) % verts.len()];
            let p1 = pos + rot.mul_vec3(v1.extend(0.0)).truncate();
            let p2 = pos + rot.mul_vec3(v2.extend(0.0)).truncate();
            gizmos.line_2d(p1, p2, ship_color);
        }

        // Direction indicator: white line from center toward the nose
        let nose_world = pos + rot.mul_vec3(Vec3::new(0.0, 12.0, 0.0)).truncate();
        gizmos.line_2d(pos, nose_world, Color::WHITE);

        // Aim indicator: orange line + dot showing current fire direction
        if aim.0.length_squared() > 0.01 {
            let aim_tip = pos + aim.0.normalize_or_zero() * 35.0;
            gizmos.line_2d(pos, aim_tip, Color::rgb(1.0, 0.5, 0.0));
            gizmos.circle_2d(aim_tip, 3.0, Color::rgb(1.0, 0.5, 0.0));
        }

        // ── Health bar (above the ship, always axis-aligned) ──────────────────
        let bar_half = 20.0;
        let bar_y_offset = 18.0;
        let bar_start = pos + Vec2::new(-bar_half, bar_y_offset);
        let bar_end_full = pos + Vec2::new(bar_half, bar_y_offset);
        let bar_end_hp = bar_start + Vec2::new(bar_half * 2.0 * hp_frac, 0.0);
        // Background (dark red)
        gizmos.line_2d(bar_start, bar_end_full, Color::rgba(0.4, 0.0, 0.0, 0.8));
        // Fill (green → red based on hp)
        if hp_frac > 0.0 {
            let fill_color = Color::rgb(1.0 - hp_frac, hp_frac, 0.0);
            gizmos.line_2d(bar_start, bar_end_hp, fill_color);
        }
    }

    // ── Projectiles ───────────────────────────────────────────────────────────
    let proj_color = Color::rgb(1.0, 0.9, 0.2); // yellow
    for transform in q_projectiles.iter() {
        let p = transform.translation.truncate();
        gizmos.circle_2d(p, 3.0, proj_color);
    }
}

// ── Out-of-bounds damping ─────────────────────────────────────────────────────

/// Applies extra velocity damping to the player when they drift outside the cull radius.
/// This prevents the player from escaping the simulation area easily.
pub fn player_oob_damping_system(mut q: Query<(&Transform, &mut Velocity), With<Player>>) {
    let Ok((transform, mut velocity)) = q.get_single_mut() else {
        return;
    };

    let dist = transform.translation.truncate().length();
    if dist > OOB_RADIUS {
        // Ramp damping smoothly: 0% at OOB_RADIUS, 3% per frame at OOB_RADIUS + 200
        let exceed = (dist - OOB_RADIUS).min(200.0) / 200.0;
        let factor = 1.0 - exceed * (1.0 - OOB_DAMPING);
        velocity.linvel *= factor;
        velocity.angvel *= factor;
    }
}

// ── Player collision damage ───────────────────────────────────────────────────

/// Detects asteroid-player collisions and applies proportional damage.
/// Uses invincibility frames to prevent damage spam on sustained contact.
pub fn player_collision_damage_system(
    mut commands: Commands,
    mut q_player: Query<(Entity, &mut PlayerHealth, &Velocity), With<Player>>,
    q_asteroids: Query<&Velocity, With<crate::asteroid::Asteroid>>,
    rapier_context: Res<RapierContext>,
    time: Res<Time>,
) {
    let Ok((player_entity, mut health, player_vel)) = q_player.get_single_mut() else {
        return;
    };

    // Tick down invincibility
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

        // Identify which entity is the player and which is the asteroid
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

/// Processes projectile-asteroid collision events.
///
/// Size rules:
///  - Size 1     → asteroid destroyed.
///  - Size 2–3   → splits into N unit triangles.
///  - Size 4–8   → splits roughly in half along the impact axis.
///  - Size ≥9    → chip off 1 unit from the impact vertex; asteroid shrinks by 1.
pub fn projectile_asteroid_hit_system(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    q_asteroids: Query<
        (&AsteroidSize, &Transform, &Velocity, &Vertices),
        With<crate::asteroid::Asteroid>,
    >,
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

        // Identify projectile and asteroid from the pair
        let (proj_entity, asteroid_entity) =
            if q_projectiles.get(e1).is_ok() && q_asteroids.get(e2).is_ok() {
                (e1, e2)
            } else if q_projectiles.get(e2).is_ok() && q_asteroids.get(e1).is_ok() {
                (e2, e1)
            } else {
                continue;
            };

        // Skip already-processed entities this frame
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

        // Always despawn the projectile
        commands.entity(proj_entity).despawn();

        let pos = transform.translation.truncate();
        let rot = transform.rotation;
        let vel = velocity.linvel;
        let ang_vel = velocity.angvel;
        let n = size.0;

        // Projectile world position — used to find the true impact vertex.
        // Falls back to the asteroid center if the transform is already gone.
        let proj_pos = q_proj_transforms
            .get(proj_entity)
            .map(|t| t.translation.truncate())
            .unwrap_or(pos);

        // World-space hull vertices (needed by split and chip paths)
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

            // ── Split in half ─────────────────────────────────────────────────
            4..=8 => {
                // Split along the impact direction line so chunks separate
                // cleanly along the projectile trajectory.
                let impact_dir = (pos - proj_pos).normalize_or_zero();
                // Splitting plane passes through the asteroid centroid;
                // axis aligns with the impact direction for trajectory-aligned splits.
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
                    // Push each half outward from the split line along the impact direction
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
                    // Ensure collision detection is active immediately after spawn
                    commands.entity(new_ent).insert((
                        Velocity {
                            linvel: split_vel,
                            angvel: ang_vel,
                        },
                        ActiveCollisionTypes::DYNAMIC_KINEMATIC,
                    ));
                }
            }

            // ── Chip ──────────────────────────────────────────────────────────
            _ => {
                // Find the hull vertex closest to the projectile — the true impact point.
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

                // Spawn the chip at the impact vertex, flying outward from the surface
                let chip_pos = world_verts[closest_idx];
                let chip_dir = (chip_pos - pos).normalize_or_zero();
                let mut rng = rand::thread_rng();
                let chip_vel = vel
                    + chip_dir * 40.0
                    + Vec2::new(rng.gen_range(-15.0..15.0), rng.gen_range(-15.0..15.0));
                spawn_unit_fragment(&mut commands, chip_pos, chip_vel, 0.0);

                // Reduce the original asteroid: remove the impact vertex, recompute hull
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

// ── Helper ────────────────────────────────────────────────────────────────────

/// Split a convex polygon (world-space vertices) with a plane that passes through
/// `origin` and whose normal is `axis`.
///
/// Returns `(front, back)` where `front` contains vertices with
/// `dot(v - origin, axis) >= 0` and `back` the rest.
/// Both halves include the two edge-intersection points so each remains closed.
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

        // Edge straddles the split plane — compute and share the intersection point
        if (da > 0.0 && db < 0.0) || (da < 0.0 && db > 0.0) {
            let t = da / (da - db);
            let intersect = a + (b - a) * t;
            front.push(intersect);
            back.push(intersect);
        }
    }

    (front, back)
}

/// Spawn a single unit-size (triangle) asteroid fragment with the given velocity.
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
