use super::state::{AimDirection, IonCannonLevel, Player};
use crate::asteroid_rendering::filled_polygon_mesh;
use crate::enemy::{Enemy, EnemyStun, EnemyTier};
use crate::particles::spawn_ion_particles;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

#[derive(Component, Debug, Clone, Copy)]
pub struct IonCannonShot {
    pub age: f32,
    pub distance_traveled: f32,
}

#[derive(Component)]
pub struct IonCannonShotRenderMarker;

#[derive(Resource, Debug, Clone)]
pub struct IonCannonCooldown {
    pub timer_secs: f32,
}

impl Default for IonCannonCooldown {
    fn default() -> Self {
        Self { timer_secs: 0.0 }
    }
}

fn elongated_projectile_vertices(radius: f32, length: f32, segments: usize) -> Vec<Vec2> {
    let half_segments = segments / 2;
    let mut vertices = Vec::new();

    for i in 0..=half_segments {
        let angle = (i as f32) * std::f32::consts::PI / (half_segments as f32);
        let x = angle.cos() * radius;
        let y = angle.sin() * radius + length / 2.0;
        vertices.push(Vec2::new(x, y));
    }

    for i in 0..=half_segments {
        let angle =
            std::f32::consts::PI + (i as f32) * std::f32::consts::PI / (half_segments as f32);
        let x = angle.cos() * radius;
        let y = angle.sin() * radius - length / 2.0;
        vertices.push(Vec2::new(x, y));
    }

    vertices
}

#[allow(clippy::too_many_arguments)]
pub fn ion_cannon_fire_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    aim: Res<AimDirection>,
    mut cooldown: ResMut<IonCannonCooldown>,
    q_player: Query<&Transform, With<Player>>,
) {
    cooldown.timer_secs = (cooldown.timer_secs - time.delta_secs()).max(0.0);

    if !keys.just_pressed(KeyCode::KeyC) || cooldown.timer_secs > 0.0 {
        return;
    }

    let Ok(player_transform) = q_player.single() else {
        return;
    };

    let ship_forward = player_transform.rotation.mul_vec3(Vec3::Y).truncate();
    let fire_dir = if aim.0.length_squared() > 0.01 {
        aim.0.normalize_or_zero()
    } else {
        ship_forward.normalize_or_zero()
    };
    if fire_dir == Vec2::ZERO {
        return;
    }

    let spawn_pos = player_transform.translation.truncate() + fire_dir * 14.0;

    commands.spawn((
        IonCannonShot {
            age: 0.0,
            distance_traveled: 0.0,
        },
        IonCannonShotRenderMarker,
        Transform::from_translation(spawn_pos.extend(0.2)),
        Visibility::default(),
        RigidBody::KinematicVelocityBased,
        Velocity {
            linvel: fire_dir * crate::constants::ION_CANNON_SHOT_SPEED,
            angvel: 0.0,
        },
        Collider::ball(crate::constants::ION_CANNON_SHOT_COLLIDER_RADIUS),
        Sensor,
        Ccd { enabled: true },
        CollisionGroups::new(
            bevy_rapier2d::geometry::Group::GROUP_3,
            bevy_rapier2d::geometry::Group::GROUP_5,
        ),
        ActiveCollisionTypes::DYNAMIC_KINEMATIC,
        ActiveEvents::COLLISION_EVENTS,
    ));

    cooldown.timer_secs = crate::constants::ION_CANNON_COOLDOWN_SECS;
}

pub fn despawn_old_ion_cannon_shots_system(
    mut commands: Commands,
    mut q_shots: Query<(Entity, &mut IonCannonShot, &Velocity)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (entity, mut shot, velocity) in q_shots.iter_mut() {
        shot.age += dt;
        shot.distance_traveled += velocity.linvel.length() * dt;
        if shot.age >= crate::constants::ION_CANNON_SHOT_LIFETIME
            || shot.distance_traveled > crate::constants::ION_CANNON_SHOT_MAX_DIST
        {
            commands.entity(entity).despawn();
        }
    }
}

pub fn ion_shot_particles_system(
    mut commands: Commands,
    time: Res<Time>,
    mut emit_timer: Local<f32>,
    q_shots: Query<(&Transform, &Velocity), With<IonCannonShot>>,
) {
    const ION_EMIT_INTERVAL: f32 = 0.03;

    *emit_timer -= time.delta_secs();
    if *emit_timer > 0.0 {
        return;
    }
    *emit_timer = ION_EMIT_INTERVAL;

    for (transform, velocity) in q_shots.iter() {
        let pos = transform.translation.truncate();
        let dir = velocity.linvel.normalize_or_zero();
        spawn_ion_particles(&mut commands, pos, dir, velocity.linvel);
    }
}

pub fn stunned_enemy_particles_system(
    mut commands: Commands,
    time: Res<Time>,
    mut emit_timer: Local<f32>,
    q_enemy: Query<(&Transform, &Velocity, &EnemyStun), With<Enemy>>,
) {
    const STUN_EMIT_INTERVAL: f32 = 0.08;

    *emit_timer -= time.delta_secs();
    if *emit_timer > 0.0 {
        return;
    }
    *emit_timer = STUN_EMIT_INTERVAL;

    for (transform, velocity, stun) in q_enemy.iter() {
        if stun.remaining_secs <= 0.0 {
            continue;
        }
        let pos = transform.translation.truncate();
        spawn_ion_particles(&mut commands, pos, Vec2::ZERO, velocity.linvel);
    }
}

pub fn ion_cannon_hit_enemy_system(
    mut commands: Commands,
    mut collision_events: MessageReader<CollisionEvent>,
    q_shots: Query<&Transform, With<IonCannonShot>>,
    mut q_enemy: Query<(&EnemyTier, &mut EnemyStun), With<Enemy>>,
    ion_level: Res<IonCannonLevel>,
) {
    let max_tier = ion_level.max_enemy_tier_affected();
    let stun_secs = ion_level.stun_duration_secs();
    let mut processed_shots: std::collections::HashSet<Entity> = Default::default();

    for event in collision_events.read() {
        let (e1, e2) = match event {
            CollisionEvent::Started(e1, e2, _) => (*e1, *e2),
            CollisionEvent::Stopped(..) => continue,
        };

        let shot_entity = if q_shots.contains(e1) {
            e1
        } else if q_shots.contains(e2) {
            e2
        } else {
            continue;
        };

        if processed_shots.contains(&shot_entity) {
            continue;
        }
        processed_shots.insert(shot_entity);

        let enemy_entity = if shot_entity == e1 { e2 } else { e1 };
        let shot_pos = q_shots
            .get(shot_entity)
            .map(|t| t.translation.truncate())
            .unwrap_or(Vec2::ZERO);

        commands.entity(shot_entity).despawn();

        let Ok((tier, mut stun)) = q_enemy.get_mut(enemy_entity) else {
            continue;
        };
        let applied_stun = if tier.level <= max_tier {
            stun_secs
        } else {
            // Over-cap tiers still receive a shorter stun so ion remains useful
            // as a projectile weapon while upgrades preserve stronger control.
            (stun_secs * 0.45).max(0.75)
        };
        stun.remaining_secs = stun.remaining_secs.max(applied_stun);
        spawn_ion_particles(&mut commands, shot_pos, Vec2::ZERO, Vec2::ZERO);
    }
}

pub fn attach_ion_cannon_shot_mesh_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Velocity, &mut Transform), Added<IonCannonShotRenderMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    const ION_RADIUS: f32 = 4.5;
    const ION_LENGTH: f32 = 18.0;

    let vertices = elongated_projectile_vertices(ION_RADIUS, ION_LENGTH, 16);
    let mesh = meshes.add(filled_polygon_mesh(&vertices));
    let mat = materials.add(ColorMaterial::from_color(Color::srgb(0.52, 0.94, 1.0)));

    for (entity, velocity, mut transform) in query.iter_mut() {
        let direction = velocity.linvel.normalize_or_zero();
        let angle = direction.y.atan2(direction.x) - std::f32::consts::FRAC_PI_2;
        transform.rotation = Quat::from_rotation_z(angle);

        commands
            .entity(entity)
            .insert((Mesh2d(mesh.clone()), MeshMaterial2d(mat.clone())));
    }
}
