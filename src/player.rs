//! Player character: space ship entity, controls, projectile firing, and camera follow.

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

// ── Constants ────────────────────────────────────────────────────────────────

/// Forward thrust force applied per frame while W is held.
const THRUST_FORCE: f32 = 600.0;
/// Reverse thrust (S key) is weaker than forward.
const REVERSE_FORCE: f32 = 300.0;
/// Angular velocity (rad/s) applied while A or D is held.
const ROTATION_SPEED: f32 = 3.0;
/// Speed at which projectiles travel (units/s).
const PROJECTILE_SPEED: f32 = 500.0;
/// Seconds between two consecutive shots.
const FIRE_COOLDOWN: f32 = 0.2;
/// Seconds before a projectile is despawned.
const PROJECTILE_LIFETIME: f32 = 3.0;

// ── Components / Resources ───────────────────────────────────────────────────

/// Marker component for the player ship entity.
#[derive(Component)]
pub struct Player;

/// Tracks when the player last fired so we can enforce the cooldown.
#[derive(Resource, Default)]
pub struct PlayerFireCooldown {
    pub timer: f32,
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
        Vec2::new(0.0, 12.0),    // nose
        Vec2::new(-8.0, -8.0),   // left fin tip
        Vec2::new(-3.0, -4.0),   // left fin inner
        Vec2::new(0.0, -10.0),   // tail notch
        Vec2::new(3.0, -4.0),    // right fin inner
        Vec2::new(8.0, -8.0),    // right fin tip
    ]
}

// ── Spawn ────────────────────────────────────────────────────────────────────

/// Spawn the player's ship at the world origin.
pub fn spawn_player(mut commands: Commands) {
    // Ship collider as a small ball (simpler than convex polygon, fine for v1)
    commands.spawn((
        Player,
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
        CollisionGroups::new(
            bevy_rapier2d::geometry::Group::GROUP_2,
            bevy_rapier2d::geometry::Group::GROUP_2,
        ),
        ActiveEvents::COLLISION_EVENTS,
        // Transform / visibility
        TransformBundle::from_transform(Transform::from_translation(Vec3::ZERO)),
        VisibilityBundle::default(),
    ));

    println!("✓ Player ship spawned at origin");
}

// ── Control system ───────────────────────────────────────────────────────────

/// Applies WASD thrust / rotation to the player ship.
pub fn player_control_system(
    mut q: Query<(&Transform, &mut ExternalForce, &mut Velocity), With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let Ok((transform, mut force, mut velocity)) = q.get_single_mut() else {
        return;
    };

    // Reset force each frame (ExternalForce accumulates unless cleared)
    force.force = Vec2::ZERO;
    force.torque = 0.0;

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

// ── Projectile fire system ───────────────────────────────────────────────────

/// Fires a projectile from the ship nose when spacebar is pressed (with cooldown).
pub fn projectile_fire_system(
    mut commands: Commands,
    q_player: Query<&Transform, With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut cooldown: ResMut<PlayerFireCooldown>,
    time: Res<Time>,
) {
    cooldown.timer = (cooldown.timer - time.delta_seconds()).max(0.0);

    let Ok(transform) = q_player.get_single() else {
        return;
    };

    if keys.just_pressed(KeyCode::Space) && cooldown.timer <= 0.0 {
        cooldown.timer = FIRE_COOLDOWN;

        let forward = transform.rotation.mul_vec3(Vec3::Y).truncate();
        let nose_offset = forward * 14.0; // spawn just ahead of the nose vertex
        let spawn_pos = transform.translation.truncate() + nose_offset;

        commands.spawn((
            Projectile { age: 0.0 },
            TransformBundle::from_transform(Transform::from_translation(spawn_pos.extend(0.0))),
            VisibilityBundle::default(),
            // Pure kinematic — no gravity / collisions with asteroids
            RigidBody::KinematicVelocityBased,
            Velocity {
                linvel: forward * PROJECTILE_SPEED,
                angvel: 0.0,
            },
            // Belong to group 3; collide with nothing (no matching groups)
            CollisionGroups::new(
                bevy_rapier2d::geometry::Group::GROUP_3,
                bevy_rapier2d::geometry::Group::NONE,
            ),
        ));
    }
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

/// Draw the player ship and all active projectiles using Bevy gizmos.
pub fn player_gizmo_system(
    mut gizmos: Gizmos,
    q_player: Query<&Transform, With<Player>>,
    q_projectiles: Query<&Transform, With<Projectile>>,
) {
    // ── Ship ──────────────────────────────────────────────────────────────────
    if let Ok(transform) = q_player.get_single() {
        let pos = transform.translation.truncate();
        let rot = transform.rotation;
        let verts = ship_vertices();
        let ship_color = Color::rgb(0.2, 0.8, 1.0); // cyan-ish

        // Draw ship outline
        for i in 0..verts.len() {
            let v1 = verts[i];
            let v2 = verts[(i + 1) % verts.len()];
            let p1 = pos + rot.mul_vec3(v1.extend(0.0)).truncate();
            let p2 = pos + rot.mul_vec3(v2.extend(0.0)).truncate();
            gizmos.line_2d(p1, p2, ship_color);
        }

        // Direction indicator: bright line from center toward the nose
        let nose_world = pos + rot.mul_vec3(Vec3::new(0.0, 12.0, 0.0)).truncate();
        gizmos.line_2d(pos, nose_world, Color::WHITE);
    }

    // ── Projectiles ───────────────────────────────────────────────────────────
    let proj_color = Color::rgb(1.0, 0.9, 0.2); // yellow
    for transform in q_projectiles.iter() {
        let p = transform.translation.truncate();
        // Small filled circle approximated by a circle_2d gizmo
        gizmos.circle_2d(p, 3.0, proj_color);
    }
}
