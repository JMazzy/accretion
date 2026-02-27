//! Enemy ship foundation: deterministic spawning, basic seek movement, and rendering.

use crate::asteroid::{
    canonical_vertices_for_mass, compute_convex_hull_from_points, rescale_vertices_to_area,
    spawn_asteroid_with_vertices, Asteroid, AsteroidSize, Planet, Vertices,
};
use crate::asteroid_rendering::filled_polygon_mesh;
use crate::config::PhysicsConfig;
use crate::menu::GameState;
use crate::mining::spawn_ore_drop;
use crate::particles::{spawn_debris_particles, spawn_impact_particles, spawn_ion_particles};
use crate::player::state::{Missile, Projectile};
use crate::player::{
    IonCannonLevel, Player, PlayerHealth, PlayerLives, PlayerScore, PrimaryWeaponLevel,
    SecondaryWeaponLevel,
};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use rand::Rng;
use std::collections::HashMap;

const ENEMY_HARD_CAP: u32 = 1;
const ENEMY_PROJECTILE_HARD_CAP: usize = 64;
const ENEMY_TIER_CAP: u32 = 4;

#[derive(Component, Debug, Clone, Copy)]
pub struct Enemy;

#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyHealth {
    pub hp: f32,
    pub max_hp: f32,
}

#[derive(Component)]
pub struct EnemyRenderMarker;

#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyFireCooldown {
    pub timer: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyProjectile {
    pub age: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct IonCannonShot {
    pub age: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyTier {
    pub level: u32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct EnemyStun {
    pub remaining_secs: f32,
}

#[derive(Component)]
pub struct EnemyProjectileRenderMarker;

#[derive(Component)]
pub struct IonCannonShotRenderMarker;

#[derive(Resource, Debug, Clone)]
pub struct EnemySpawnState {
    pub timer_secs: f32,
    pub session_elapsed_secs: f32,
    pub total_spawned: u64,
}

#[derive(Resource, Debug, Clone)]
pub struct IonCannonCooldown {
    pub timer_secs: f32,
}

impl Default for IonCannonCooldown {
    fn default() -> Self {
        Self { timer_secs: 0.0 }
    }
}

impl Default for EnemySpawnState {
    fn default() -> Self {
        Self {
            timer_secs: 0.0,
            session_elapsed_secs: 0.0,
            total_spawned: 0,
        }
    }
}

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EnemySpawnState::default())
            .insert_resource(IonCannonCooldown::default())
            .add_systems(
                Update,
                (
                    enemy_session_clock_system,
                    enemy_spawn_system,
                    enemy_stun_tick_system,
                    ion_cannon_fire_system,
                    despawn_old_ion_cannon_shots_system,
                    ion_shot_particles_system,
                    stunned_enemy_particles_system,
                    enemy_seek_player_system,
                    enemy_fire_system,
                    despawn_old_enemy_projectiles_system,
                    attach_enemy_mesh_system,
                    attach_enemy_projectile_mesh_system,
                    attach_ion_cannon_shot_mesh_system,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                PostUpdate,
                (
                    ion_cannon_hit_enemy_system,
                    enemy_damage_from_player_weapons_system,
                    enemy_collision_damage_system,
                    enemy_player_collision_damage_system,
                    enemy_projectile_hit_system,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn enemy_session_clock_system(time: Res<Time>, mut state: ResMut<EnemySpawnState>) {
    state.session_elapsed_secs += time.delta_secs();
}

fn enemy_spawn_profile(config: &PhysicsConfig, score_points: u32, elapsed_secs: f32) -> (u32, f32) {
    let time_stage = (elapsed_secs / config.enemy_stage_time_secs.max(1.0)).floor() as u32;
    let score_stage = score_points / config.enemy_stage_score_points.max(1);
    let stage = time_stage + score_stage;

    let max_count = (config.enemy_max_count_base + stage * config.enemy_max_count_per_stage)
        .min(config.enemy_max_count_cap.max(1))
        .min(ENEMY_HARD_CAP);
    let cooldown = (config.enemy_spawn_base_cooldown
        - stage as f32 * config.enemy_spawn_cooldown_per_stage)
        .max(config.enemy_spawn_cooldown_min.max(0.5));

    (max_count.max(1), cooldown)
}

fn enemy_tier_for_stage(stage: u32) -> u32 {
    (1 + stage / 2).min(ENEMY_TIER_CAP)
}

fn deterministic_spawn_offset(index: u64, radius: f32) -> Vec2 {
    const GOLDEN_ANGLE: f32 = 2.3999631;
    let a = index as f32 * GOLDEN_ANGLE;
    Vec2::new(a.cos(), a.sin()) * radius
}

fn initial_enemy_fire_timer(spawn_index: u64, base_cooldown: f32) -> f32 {
    let phase =
        ((spawn_index.wrapping_mul(1_103_515_245).wrapping_add(12_345)) % 10_000) as f32 / 10_000.0;
    base_cooldown * (0.4 + 0.6 * phase)
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
fn enemy_spawn_system(
    mut commands: Commands,
    time: Res<Time>,
    mut state: ResMut<EnemySpawnState>,
    config: Res<PhysicsConfig>,
    score: Res<PlayerScore>,
    q_player: Query<&Transform, With<Player>>,
    q_enemies: Query<&Transform, With<Enemy>>,
) {
    let Ok(player_transform) = q_player.single() else {
        return;
    };

    state.timer_secs -= time.delta_secs();
    let score_stage = score.points / config.enemy_stage_score_points.max(1);
    let time_stage =
        (state.session_elapsed_secs / config.enemy_stage_time_secs.max(1.0)).floor() as u32;
    let stage = time_stage + score_stage;
    let (_profile_max_count, next_cooldown) =
        enemy_spawn_profile(&config, score.points, state.session_elapsed_secs);
    let max_count = 1;

    if q_enemies.iter().count() as u32 >= max_count {
        state.timer_secs = state.timer_secs.max(0.25);
        return;
    }
    if state.timer_secs > 0.0 {
        return;
    }

    let player_pos = player_transform.translation.truncate();
    let min_player_dist = config.enemy_min_player_spawn_distance.max(1.0);

    let edge_radius = (config.cull_distance * 0.92)
        .min((config.hard_cull_distance - 24.0).max(1.0))
        .max(min_player_dist + 8.0);

    let mut spawn_pos = None;
    for attempt in 0..18_u64 {
        let offset = deterministic_spawn_offset(state.total_spawned + attempt, edge_radius);
        if offset.length() < min_player_dist {
            continue;
        }

        let candidate = offset;
        if candidate.distance_squared(player_pos) < min_player_dist * min_player_dist {
            continue;
        }
        let too_close = q_enemies.iter().any(|t| {
            t.translation.truncate().distance_squared(candidate)
                < config.enemy_min_enemy_spacing * config.enemy_min_enemy_spacing
        });

        if !too_close {
            spawn_pos = Some(candidate);
            state.total_spawned += attempt + 1;
            break;
        }
    }

    let Some(pos) = spawn_pos else {
        state.timer_secs = 0.8;
        return;
    };

    let toward_player = (player_pos - pos).normalize_or_zero();
    let spawn_index = state.total_spawned;

    let enemy_entity = commands
        .spawn((
            Enemy,
            EnemyHealth {
                hp: config.enemy_base_hp,
                max_hp: config.enemy_base_hp,
            },
            EnemyRenderMarker,
            EnemyFireCooldown {
                timer: initial_enemy_fire_timer(spawn_index, config.enemy_fire_cooldown_base),
            },
            Transform::from_translation(pos.extend(0.25)),
            Visibility::default(),
            RigidBody::Dynamic,
            Collider::ball(config.enemy_collider_radius),
            Velocity {
                linvel: toward_player * (config.enemy_max_speed * 0.25),
                angvel: 0.0,
            },
            ExternalForce::default(),
            Damping {
                linear_damping: config.enemy_linear_damping,
                angular_damping: config.enemy_angular_damping,
            },
            Restitution::coefficient(0.25),
            CollisionGroups::new(
                bevy_rapier2d::geometry::Group::GROUP_5,
                bevy_rapier2d::geometry::Group::GROUP_1
                    | bevy_rapier2d::geometry::Group::GROUP_2
                    | bevy_rapier2d::geometry::Group::GROUP_3,
            ),
            ActiveEvents::COLLISION_EVENTS,
        ))
        .id();

    commands.entity(enemy_entity).insert((
        EnemyTier {
            level: enemy_tier_for_stage(stage),
        },
        EnemyStun {
            remaining_secs: 0.0,
        },
    ));

    state.timer_secs = next_cooldown;
}

fn enemy_stun_tick_system(time: Res<Time>, mut q_enemy: Query<&mut EnemyStun, With<Enemy>>) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }

    for mut stun in q_enemy.iter_mut() {
        stun.remaining_secs = (stun.remaining_secs - dt).max(0.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn ion_cannon_fire_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
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

    let forward = player_transform
        .rotation
        .mul_vec3(Vec3::Y)
        .truncate()
        .normalize_or_zero();
    if forward == Vec2::ZERO {
        return;
    }

    let spawn_pos = player_transform.translation.truncate() + forward * 14.0;

    commands.spawn((
        IonCannonShot { age: 0.0 },
        IonCannonShotRenderMarker,
        Transform::from_translation(spawn_pos.extend(0.2)),
        Visibility::default(),
        RigidBody::KinematicVelocityBased,
        Velocity {
            linvel: forward * crate::constants::ION_CANNON_SHOT_SPEED,
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

fn despawn_old_ion_cannon_shots_system(
    mut commands: Commands,
    mut q_shots: Query<(Entity, &mut IonCannonShot, &Transform)>,
    time: Res<Time>,
    config: Res<PhysicsConfig>,
) {
    let dt = time.delta_secs();
    for (entity, mut shot, transform) in q_shots.iter_mut() {
        shot.age += dt;
        let dist = transform.translation.truncate().length();
        if shot.age >= crate::constants::ION_CANNON_SHOT_LIFETIME
            || dist > config.hard_cull_distance
        {
            commands.entity(entity).despawn();
        }
    }
}

fn ion_shot_particles_system(
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

fn stunned_enemy_particles_system(
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

fn ion_cannon_hit_enemy_system(
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
        if tier.level <= max_tier {
            stun.remaining_secs = stun.remaining_secs.max(stun_secs);
            spawn_ion_particles(&mut commands, shot_pos, Vec2::ZERO, Vec2::ZERO);
        }
    }
}

fn enemy_seek_player_system(
    q_player: Query<&Transform, With<Player>>,
    mut q_enemy: Query<(&Transform, &mut ExternalForce, &mut Velocity, &EnemyStun), With<Enemy>>,
    config: Res<PhysicsConfig>,
) {
    let Ok(player_transform) = q_player.single() else {
        return;
    };
    let player_pos = player_transform.translation.truncate();

    for (transform, mut force, mut velocity, stun) in q_enemy.iter_mut() {
        if stun.remaining_secs > 0.0 {
            force.force = Vec2::ZERO;
            velocity.linvel *= 0.95;
            velocity.angvel *= 0.8;
            continue;
        }

        let pos = transform.translation.truncate();
        let to_player = player_pos - pos;
        let dist = to_player.length();
        if dist <= 1e-3 {
            continue;
        }

        let dir = to_player / dist;
        let arrive_alpha = (dist / config.enemy_arrive_radius.max(1.0)).clamp(0.2, 1.0);
        force.force += dir * (config.enemy_seek_force * arrive_alpha);

        let speed = velocity.linvel.length();
        if speed > config.enemy_max_speed {
            velocity.linvel = velocity.linvel.normalize_or_zero() * config.enemy_max_speed;
        }

        let angle = dir.y.atan2(dir.x) - std::f32::consts::FRAC_PI_2;
        velocity.angvel = angle * 0.2;
    }
}

fn enemy_fire_system(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<PhysicsConfig>,
    q_player: Query<&Transform, With<Player>>,
    mut q_enemy: Query<(&Transform, &mut EnemyFireCooldown, &EnemyStun), With<Enemy>>,
    q_enemy_projectiles: Query<(), With<EnemyProjectile>>,
) {
    let Ok(player_transform) = q_player.single() else {
        return;
    };
    let player_pos = player_transform.translation.truncate();

    let active_enemy_projectiles = q_enemy_projectiles.iter().count();
    let projectile_budget_available = active_enemy_projectiles < ENEMY_PROJECTILE_HARD_CAP;

    for (transform, mut cooldown, stun) in q_enemy.iter_mut() {
        cooldown.timer -= time.delta_secs();
        if stun.remaining_secs > 0.0 {
            continue;
        }
        if cooldown.timer > 0.0 {
            continue;
        }

        if !projectile_budget_available {
            cooldown.timer = (config.enemy_fire_cooldown_base * 0.5).max(0.3);
            continue;
        }

        let enemy_pos = transform.translation.truncate();
        let fire_dir = (player_pos - enemy_pos).normalize_or_zero();
        if fire_dir.length_squared() <= 1e-5 {
            cooldown.timer = config.enemy_fire_cooldown_base;
            continue;
        }

        let spawn_pos = enemy_pos + fire_dir * (config.enemy_collider_radius + 6.0);
        commands.spawn((
            EnemyProjectile { age: 0.0 },
            EnemyProjectileRenderMarker,
            Transform::from_translation(spawn_pos.extend(0.2)),
            Visibility::default(),
            RigidBody::KinematicVelocityBased,
            Velocity {
                linvel: fire_dir * config.enemy_projectile_speed,
                angvel: 0.0,
            },
            Collider::ball(config.enemy_projectile_collider_radius),
            Sensor,
            Ccd { enabled: true },
            CollisionGroups::new(
                bevy_rapier2d::geometry::Group::GROUP_6,
                bevy_rapier2d::geometry::Group::GROUP_1 | bevy_rapier2d::geometry::Group::GROUP_2,
            ),
            ActiveCollisionTypes::DYNAMIC_KINEMATIC,
            ActiveEvents::COLLISION_EVENTS,
        ));

        cooldown.timer = config.enemy_fire_cooldown_base;
    }
}

fn despawn_old_enemy_projectiles_system(
    mut commands: Commands,
    mut q: Query<(Entity, &mut EnemyProjectile, &Transform)>,
    time: Res<Time>,
    config: Res<PhysicsConfig>,
) {
    let dt = time.delta_secs();
    for (entity, mut projectile, transform) in q.iter_mut() {
        projectile.age += dt;
        let dist = transform.translation.truncate().length();
        if projectile.age >= config.enemy_projectile_lifetime
            || dist > config.enemy_projectile_max_dist
        {
            commands.entity(entity).despawn();
        }
    }
}

fn apply_enemy_damage(
    commands: &mut Commands,
    score: &mut PlayerScore,
    q_enemy: &mut Query<(Entity, &mut EnemyHealth), With<Enemy>>,
    damage_by_enemy: HashMap<Entity, f32>,
    kill_score: u32,
    award_score: bool,
) {
    for (enemy_entity, damage) in damage_by_enemy {
        let Ok((entity, mut health)) = q_enemy.get_mut(enemy_entity) else {
            continue;
        };
        health.hp -= damage;
        if health.hp <= 0.0 {
            commands.entity(entity).despawn();
            if award_score {
                score.destroyed += 1;
                score.points += kill_score;
            }
        }
    }
}

fn projectile_damage_vs_enemy(config: &PhysicsConfig, level: &PrimaryWeaponLevel) -> f32 {
    config.enemy_damage_from_player_projectile * (1.0 + 0.35 * level.level as f32)
}

fn missile_damage_vs_enemy(config: &PhysicsConfig, level: &SecondaryWeaponLevel) -> f32 {
    config.enemy_damage_from_player_missile * (1.0 + 0.25 * level.level as f32)
}

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

#[allow(clippy::too_many_arguments)]
fn apply_blaster_like_asteroid_hit(
    commands: &mut Commands,
    asteroid_entity: Entity,
    size: &AsteroidSize,
    transform: &Transform,
    velocity: &Velocity,
    vertices: &Vertices,
    proj_pos: Vec2,
    weapon_level: &PrimaryWeaponLevel,
    config: &PhysicsConfig,
    stats: &mut crate::simulation::SimulationStats,
) {
    let pos = transform.translation.truncate();
    let rot = transform.rotation;
    let vel = velocity.linvel;
    let ang_vel = velocity.angvel;
    let n = size.0;

    let world_verts: Vec<Vec2> = vertices
        .0
        .iter()
        .map(|v| pos + rot.mul_vec3(v.extend(0.0)).truncate())
        .collect();

    let impact_dir = (pos - proj_pos).normalize_or_zero();
    let destroy_threshold = weapon_level.max_destroy_size();

    if n <= destroy_threshold {
        commands.entity(asteroid_entity).despawn();
        stats.destroyed_total += 1;

        let drop_count = n.max(1);
        for i in 0..drop_count {
            let angle = std::f32::consts::TAU * (i as f32 / drop_count as f32);
            let offset = Vec2::new(angle.cos(), angle.sin()) * 6.0;
            spawn_ore_drop(commands, pos + offset, vel);
        }
        spawn_impact_particles(commands, proj_pos, impact_dir, vel);
        spawn_debris_particles(commands, pos, vel, n.max(1));
        return;
    }

    spawn_impact_particles(commands, proj_pos, impact_dir, vel);
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

    let max_chip_size = weapon_level.display_level().min(n / 2).max(1);
    let chip_size = if max_chip_size <= 1 {
        1u32
    } else {
        rng.gen_range(1u32..=max_chip_size)
    };

    let chip_vel =
        vel + chip_dir * 40.0 + Vec2::new(rng.gen_range(-15.0..15.0), rng.gen_range(-15.0..15.0));
    spawn_fragment_of_mass(
        commands,
        chip_pos,
        chip_vel,
        0.0,
        config.asteroid_density,
        chip_size,
    );

    let n_verts = world_verts.len();
    let prev_idx = (closest_idx + n_verts - 1) % n_verts;
    let next_idx = (closest_idx + 1) % n_verts;
    let tip = world_verts[closest_idx];
    let cut_a = tip + (world_verts[prev_idx] - tip) * 0.30;
    let cut_b = tip + (world_verts[next_idx] - tip) * 0.30;
    let mut new_world_verts: Vec<Vec2> = Vec::with_capacity(n_verts + 1);
    for (i, &v) in world_verts.iter().enumerate() {
        if i == closest_idx {
            new_world_verts.push(cut_a);
            new_world_verts.push(cut_b);
        } else {
            new_world_verts.push(v);
        }
    }

    let hull_world = compute_convex_hull_from_points(&new_world_verts).unwrap_or(new_world_verts);
    let hull_centroid: Vec2 = hull_world.iter().copied().sum::<Vec2>() / hull_world.len() as f32;
    let new_mass = (n - chip_size).max(1);
    let new_local: Vec<Vec2> = hull_world.iter().map(|v| *v - hull_centroid).collect();
    let target_area = new_mass as f32 / config.asteroid_density;
    let new_local = rescale_vertices_to_area(&new_local, target_area);

    commands.entity(asteroid_entity).despawn();

    let grey = 0.4 + rand::random::<f32>() * 0.3;
    let new_ent = spawn_asteroid_with_vertices(
        commands,
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

#[allow(clippy::too_many_arguments)]
fn enemy_damage_from_player_weapons_system(
    mut commands: Commands,
    mut collision_events: MessageReader<CollisionEvent>,
    mut q_enemy: Query<(Entity, &mut EnemyHealth), With<Enemy>>,
    mut q_projectiles: Query<(&Transform, &mut Projectile)>,
    q_missiles: Query<&Transform, With<Missile>>,
    mut score: ResMut<PlayerScore>,
    weapon_level: Res<PrimaryWeaponLevel>,
    missile_level: Res<SecondaryWeaponLevel>,
    config: Res<PhysicsConfig>,
) {
    let mut damage_by_enemy: HashMap<Entity, f32> = HashMap::default();

    for event in collision_events.read() {
        let (e1, e2) = match event {
            CollisionEvent::Started(e1, e2, _) => (*e1, *e2),
            CollisionEvent::Stopped(..) => continue,
        };

        let Some(enemy_entity) = (if q_enemy.contains(e1) {
            Some(e1)
        } else if q_enemy.contains(e2) {
            Some(e2)
        } else {
            None
        }) else {
            continue;
        };

        let other = if enemy_entity == e1 { e2 } else { e1 };

        if let Ok((projectile_transform, mut projectile)) = q_projectiles.get_mut(other) {
            projectile.was_hit = true;
            let enemy_pos = q_enemy.get(enemy_entity).map(|(_, h)| h.max_hp).ok();
            let _ = enemy_pos;
            let proj_pos = projectile_transform.translation.truncate();
            spawn_impact_particles(&mut commands, proj_pos, Vec2::ZERO, Vec2::ZERO);
            *damage_by_enemy.entry(enemy_entity).or_default() +=
                projectile_damage_vs_enemy(&config, &weapon_level);
            continue;
        }

        if let Ok(missile_transform) = q_missiles.get(other) {
            let missile_pos = missile_transform.translation.truncate();
            spawn_impact_particles(&mut commands, missile_pos, Vec2::ZERO, Vec2::ZERO);
            commands.entity(other).despawn();
            *damage_by_enemy.entry(enemy_entity).or_default() +=
                missile_damage_vs_enemy(&config, &missile_level);
        }
    }

    apply_enemy_damage(
        &mut commands,
        &mut score,
        &mut q_enemy,
        damage_by_enemy,
        config.enemy_kill_score,
        true,
    );
}

fn enemy_collision_damage_system(
    mut commands: Commands,
    mut q_enemy: Query<(Entity, &mut EnemyHealth, &Velocity), With<Enemy>>,
    q_asteroid_vel: Query<&Velocity, With<Asteroid>>,
    rapier_context: ReadRapierContext,
    config: Res<PhysicsConfig>,
) {
    let Ok(rapier) = rapier_context.single() else {
        return;
    };

    let mut damage_by_enemy: HashMap<Entity, f32> = HashMap::default();
    for (enemy_entity, _health, enemy_vel) in q_enemy.iter_mut() {
        let mut total_damage = 0.0_f32;

        for contact_pair in rapier.contact_pairs_with(enemy_entity) {
            if !contact_pair.has_any_active_contact() {
                continue;
            }
            let Some(e1) = contact_pair.collider1() else {
                continue;
            };
            let Some(e2) = contact_pair.collider2() else {
                continue;
            };

            let asteroid_entity = if e1 == enemy_entity {
                e2
            } else if e2 == enemy_entity {
                e1
            } else {
                continue;
            };

            let Ok(ast_vel) = q_asteroid_vel.get(asteroid_entity) else {
                continue;
            };

            let rel_speed = (enemy_vel.linvel - ast_vel.linvel).length();
            if rel_speed > config.enemy_asteroid_collision_damage_threshold {
                total_damage += (rel_speed - config.enemy_asteroid_collision_damage_threshold)
                    * config.enemy_asteroid_collision_damage_scale;
            }
        }

        if total_damage > 0.0 {
            damage_by_enemy.insert(enemy_entity, total_damage);
        }
    }

    for (enemy_entity, damage) in damage_by_enemy {
        let Ok((entity, mut health, _)) = q_enemy.get_mut(enemy_entity) else {
            continue;
        };
        health.hp -= damage;
        if health.hp <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn enemy_player_collision_damage_system(
    mut commands: Commands,
    mut q_player: Query<(Entity, &mut PlayerHealth, &Velocity), With<Player>>,
    mut q_enemy: Query<(Entity, &mut EnemyHealth, &Velocity), With<Enemy>>,
    rapier_context: ReadRapierContext,
    mut lives: ResMut<PlayerLives>,
    mut score: ResMut<PlayerScore>,
    mut next_state: ResMut<NextState<GameState>>,
    config: Res<PhysicsConfig>,
) {
    let Ok((player_entity, mut player_health, player_velocity)) = q_player.single_mut() else {
        return;
    };
    let Ok(rapier) = rapier_context.single() else {
        return;
    };

    let mut damage_by_enemy: HashMap<Entity, f32> = HashMap::default();
    let mut total_player_damage = 0.0_f32;

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

        let enemy_entity = if e1 == player_entity {
            e2
        } else if e2 == player_entity {
            e1
        } else {
            continue;
        };

        let Ok((_, _, enemy_velocity)) = q_enemy.get(enemy_entity) else {
            continue;
        };

        let rel_speed = (player_velocity.linvel - enemy_velocity.linvel).length();
        if rel_speed <= config.damage_speed_threshold {
            continue;
        }

        let overlap_damage = (rel_speed - config.damage_speed_threshold) * 0.35;
        total_player_damage += overlap_damage;
        *damage_by_enemy.entry(enemy_entity).or_default() += overlap_damage;
    }

    for (enemy_entity, damage) in damage_by_enemy {
        let Ok((entity, mut health, _)) = q_enemy.get_mut(enemy_entity) else {
            continue;
        };
        health.hp -= damage;
        if health.hp <= 0.0 {
            commands.entity(entity).despawn();
        }
    }

    if total_player_damage > 0.0 && player_health.inv_timer <= 0.0 {
        player_health.hp -= total_player_damage;
        player_health.inv_timer = config.invincibility_duration;
        player_health.time_since_damage = 0.0;

        if player_health.hp <= 0.0 {
            commands.entity(player_entity).despawn();
            lives.remaining -= 1;
            score.streak = 0;
            if lives.remaining <= 0 {
                lives.remaining = 0;
                next_state.set(GameState::GameOver);
            } else {
                lives.respawn_timer = Some(config.respawn_delay_secs);
            }
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn enemy_projectile_hit_system(
    mut commands: Commands,
    mut collision_events: MessageReader<CollisionEvent>,
    q_enemy_projectiles: Query<&Transform, With<EnemyProjectile>>,
    q_asteroids: Query<
        (&AsteroidSize, &Transform, &Velocity, &Vertices),
        (With<Asteroid>, Without<Planet>),
    >,
    q_planets: Query<(), With<Planet>>,
    mut q_player: Query<(Entity, &mut PlayerHealth), With<Player>>,
    mut lives: ResMut<PlayerLives>,
    mut score: ResMut<PlayerScore>,
    mut stats: ResMut<crate::simulation::SimulationStats>,
    weapon_level: Res<PrimaryWeaponLevel>,
    mut next_state: ResMut<NextState<GameState>>,
    config: Res<PhysicsConfig>,
) {
    let Ok((player_entity, mut health)) = q_player.single_mut() else {
        return;
    };

    let mut processed_projectiles: std::collections::HashSet<Entity> = Default::default();
    let mut processed_asteroids: std::collections::HashSet<Entity> = Default::default();

    for event in collision_events.read() {
        let (e1, e2) = match event {
            CollisionEvent::Started(e1, e2, _) => (*e1, *e2),
            CollisionEvent::Stopped(..) => continue,
        };

        let projectile_entity = if q_enemy_projectiles.contains(e1) {
            e1
        } else if q_enemy_projectiles.contains(e2) {
            e2
        } else {
            continue;
        };

        if processed_projectiles.contains(&projectile_entity) {
            continue;
        }
        processed_projectiles.insert(projectile_entity);

        let other = if projectile_entity == e1 { e2 } else { e1 };
        let proj_pos = q_enemy_projectiles
            .get(projectile_entity)
            .map(|t| t.translation.truncate())
            .unwrap_or(Vec2::ZERO);
        commands.entity(projectile_entity).despawn();

        if q_planets.contains(other) {
            continue;
        }

        if let Ok((size, transform, velocity, vertices)) = q_asteroids.get(other) {
            if processed_asteroids.contains(&other) {
                continue;
            }
            processed_asteroids.insert(other);
            apply_blaster_like_asteroid_hit(
                &mut commands,
                other,
                size,
                transform,
                velocity,
                vertices,
                proj_pos,
                &weapon_level,
                &config,
                &mut stats,
            );
            continue;
        }

        if other != player_entity || health.inv_timer > 0.0 {
            continue;
        }

        let impact_dir = (health.max_hp * Vec2::X).normalize_or_zero();
        spawn_impact_particles(&mut commands, proj_pos, impact_dir, Vec2::ZERO);

        health.hp -= config.enemy_projectile_damage;
        health.inv_timer = config.invincibility_duration;
        health.time_since_damage = 0.0;

        if health.hp <= 0.0 {
            commands.entity(player_entity).despawn();
            lives.remaining -= 1;
            score.streak = 0;
            if lives.remaining <= 0 {
                lives.remaining = 0;
                next_state.set(GameState::GameOver);
            } else {
                lives.respawn_timer = Some(config.respawn_delay_secs);
            }
            break;
        }
    }
}

fn attach_enemy_mesh_system(
    mut commands: Commands,
    query: Query<(Entity, &EnemyHealth), Added<EnemyRenderMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (entity, health) in query.iter() {
        let vertices = [
            Vec2::new(0.0, 11.0),
            Vec2::new(-7.0, -7.0),
            Vec2::new(7.0, -7.0),
        ];
        let mesh = meshes.add(filled_polygon_mesh(&vertices));
        let hp_ratio = (health.hp / health.max_hp.max(1.0)).clamp(0.0, 1.0);
        let tint = Color::srgb(0.75 + 0.20 * hp_ratio, 0.22 + 0.10 * hp_ratio, 0.22);
        let mat = materials.add(ColorMaterial::from_color(tint));
        commands
            .entity(entity)
            .insert((Mesh2d(mesh), MeshMaterial2d(mat)));
    }
}

fn attach_enemy_projectile_mesh_system(
    mut commands: Commands,
    query: Query<Entity, Added<EnemyProjectileRenderMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for entity in query.iter() {
        let vertices = [
            Vec2::new(0.0, 4.0),
            Vec2::new(-2.5, -3.0),
            Vec2::new(2.5, -3.0),
        ];
        let mesh = meshes.add(filled_polygon_mesh(&vertices));
        let mat = materials.add(ColorMaterial::from_color(Color::srgb(1.0, 0.45, 0.25)));
        commands
            .entity(entity)
            .insert((Mesh2d(mesh), MeshMaterial2d(mat)));
    }
}

fn attach_ion_cannon_shot_mesh_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Velocity, &mut Transform), Added<IonCannonShotRenderMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    const ION_RADIUS: f32 = 2.0;
    const ION_LENGTH: f32 = 10.0;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particles::Particle;
    use crate::simulation::SimulationStats;
    use bevy::state::app::StatesPlugin;

    fn enemy_collision_test_app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin));
        app.init_state::<GameState>();
        app.add_message::<CollisionEvent>();
        app.insert_resource(PhysicsConfig::default());
        app.insert_resource(PlayerScore::default());
        app.insert_resource(PlayerLives::default());
        app.insert_resource(PrimaryWeaponLevel::default());
        app.insert_resource(SecondaryWeaponLevel::default());
        app.insert_resource(SimulationStats::default());
        app
    }

    fn particle_count(world: &mut World) -> usize {
        world
            .query_filtered::<Entity, With<Particle>>()
            .iter(world)
            .count()
    }

    #[test]
    fn spawn_offset_is_deterministic() {
        let a = deterministic_spawn_offset(17, 500.0);
        let b = deterministic_spawn_offset(17, 500.0);
        assert!((a - b).length() < 1e-6);
    }

    #[test]
    fn spawn_profile_progression_increases_count_and_reduces_cooldown() {
        let cfg = PhysicsConfig::default();
        let (count0, cooldown0) = enemy_spawn_profile(&cfg, 0, 0.0);
        let (count1, cooldown1) = enemy_spawn_profile(
            &cfg,
            cfg.enemy_stage_score_points * 3,
            cfg.enemy_stage_time_secs * 3.0,
        );
        assert!(count1 >= count0);
        assert!(cooldown1 <= cooldown0);
    }

    #[test]
    fn initial_fire_timer_is_deterministic_and_bounded() {
        let t1 = initial_enemy_fire_timer(42, 2.0);
        let t2 = initial_enemy_fire_timer(42, 2.0);
        assert!((t1 - t2).abs() < 1e-6);
        assert!(t1 >= 0.8 && t1 <= 2.0);
    }

    #[test]
    fn missile_damage_is_higher_than_projectile_damage() {
        let cfg = PhysicsConfig::default();
        let primary = PrimaryWeaponLevel { level: 0 };
        let secondary = SecondaryWeaponLevel { level: 0 };
        let proj = projectile_damage_vs_enemy(&cfg, &primary);
        let missile = missile_damage_vs_enemy(&cfg, &secondary);
        assert!(missile > proj);
    }

    #[test]
    fn player_weapon_damage_scales_with_levels() {
        let cfg = PhysicsConfig::default();
        let p0 = projectile_damage_vs_enemy(&cfg, &PrimaryWeaponLevel { level: 0 });
        let p5 = projectile_damage_vs_enemy(&cfg, &PrimaryWeaponLevel { level: 5 });
        let m0 = missile_damage_vs_enemy(&cfg, &SecondaryWeaponLevel { level: 0 });
        let m5 = missile_damage_vs_enemy(&cfg, &SecondaryWeaponLevel { level: 5 });

        assert!(p5 > p0);
        assert!(m5 > m0);
    }

    #[test]
    fn enemy_collision_filter_accepts_player_weapon_group() {
        use bevy_rapier2d::geometry::Group;

        let projectile_membership = Group::GROUP_3;
        let projectile_filter = Group::GROUP_1 | Group::GROUP_5;

        let enemy_membership = Group::GROUP_5;
        let enemy_filter = Group::GROUP_1 | Group::GROUP_2 | Group::GROUP_3;

        assert!(projectile_membership.intersects(enemy_filter));
        assert!(enemy_membership.intersects(projectile_filter));
    }

    #[test]
    fn player_projectile_hit_enemy_damages_and_spawns_impact_particles() {
        let mut app = enemy_collision_test_app();
        app.add_systems(PostUpdate, enemy_damage_from_player_weapons_system);

        let enemy = app
            .world_mut()
            .spawn(EnemyHealth {
                hp: 100.0,
                max_hp: 100.0,
            })
            .insert(Enemy)
            .id();
        let projectile = app
            .world_mut()
            .spawn((
                Projectile::default(),
                Transform::from_translation(Vec3::new(3.0, 2.0, 0.0)),
            ))
            .id();

        app.world_mut().write_message(CollisionEvent::Started(
            enemy,
            projectile,
            bevy_rapier2d::rapier::geometry::CollisionEventFlags::empty(),
        ));

        app.update();

        let enemy_hp = app.world().get::<EnemyHealth>(enemy).map(|h| h.hp).unwrap();
        assert!(enemy_hp < 100.0);

        let projectile_state = app.world().get::<Projectile>(projectile).unwrap();
        assert!(projectile_state.was_hit);

        assert!(particle_count(app.world_mut()) > 0);
    }

    #[test]
    fn player_missile_hit_enemy_despawns_missile_and_spawns_particles() {
        let mut app = enemy_collision_test_app();
        app.add_systems(PostUpdate, enemy_damage_from_player_weapons_system);

        let enemy = app
            .world_mut()
            .spawn((
                Enemy,
                EnemyHealth {
                    hp: 120.0,
                    max_hp: 120.0,
                },
            ))
            .id();
        let missile = app
            .world_mut()
            .spawn((
                Missile::default(),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ))
            .id();

        app.world_mut().write_message(CollisionEvent::Started(
            enemy,
            missile,
            bevy_rapier2d::rapier::geometry::CollisionEventFlags::empty(),
        ));

        app.update();

        let enemy_hp = app.world().get::<EnemyHealth>(enemy).map(|h| h.hp).unwrap();
        assert!(enemy_hp < 120.0);
        assert!(app.world().get_entity(missile).is_err());
        assert!(particle_count(app.world_mut()) > 0);
    }

    #[test]
    fn enemy_projectile_hit_player_damages_player_and_spawns_particles() {
        let mut app = enemy_collision_test_app();
        app.add_systems(PostUpdate, enemy_projectile_hit_system);

        let player = app
            .world_mut()
            .spawn((
                Player,
                PlayerHealth {
                    hp: 100.0,
                    max_hp: 100.0,
                    inv_timer: 0.0,
                    time_since_damage: 0.0,
                },
            ))
            .id();
        let enemy_projectile = app
            .world_mut()
            .spawn((
                EnemyProjectile { age: 0.0 },
                Transform::from_translation(Vec3::new(12.0, -4.0, 0.0)),
            ))
            .id();

        app.world_mut().write_message(CollisionEvent::Started(
            enemy_projectile,
            player,
            bevy_rapier2d::rapier::geometry::CollisionEventFlags::empty(),
        ));

        app.update();

        let player_hp = app
            .world()
            .get::<PlayerHealth>(player)
            .map(|h| h.hp)
            .unwrap();
        assert!(player_hp < 100.0);
        assert!(app.world().get_entity(enemy_projectile).is_err());
        assert!(particle_count(app.world_mut()) > 0);
    }

    #[test]
    fn enemy_projectile_hit_asteroid_applies_damage_path_and_particles() {
        let mut app = enemy_collision_test_app();
        app.add_systems(PostUpdate, enemy_projectile_hit_system);

        let _player = app
            .world_mut()
            .spawn((
                Player,
                PlayerHealth {
                    hp: 100.0,
                    max_hp: 100.0,
                    inv_timer: 0.0,
                    time_since_damage: 0.0,
                },
            ))
            .id();

        let asteroid = app
            .world_mut()
            .spawn((
                Asteroid,
                AsteroidSize(1),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                Velocity::zero(),
                Vertices(vec![
                    Vec2::new(0.0, 5.0),
                    Vec2::new(-4.0, -3.0),
                    Vec2::new(4.0, -3.0),
                ]),
            ))
            .id();

        let enemy_projectile = app
            .world_mut()
            .spawn((
                EnemyProjectile { age: 0.0 },
                Transform::from_translation(Vec3::new(1.0, 1.0, 0.0)),
            ))
            .id();

        app.world_mut().write_message(CollisionEvent::Started(
            enemy_projectile,
            asteroid,
            bevy_rapier2d::rapier::geometry::CollisionEventFlags::empty(),
        ));

        app.update();

        assert!(app.world().get_entity(enemy_projectile).is_err());
        assert!(app.world().get_entity(asteroid).is_err());
        assert!(particle_count(app.world_mut()) > 0);

        let stats = app.world().resource::<SimulationStats>();
        assert!(stats.destroyed_total >= 1);
    }
}
