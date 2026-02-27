use super::TestConfig;
use crate::asteroid::{
    canonical_vertices_for_mass, rescale_vertices_to_area, spawn_asteroid_with_vertices,
    spawn_planet,
};
use crate::config::PhysicsConfig;
use crate::enemy::{
    Enemy, EnemyFireCooldown, EnemyHealth, EnemyProjectile, EnemyProjectileRenderMarker,
    EnemyRenderMarker, EnemySpawnState, EnemyStun, EnemyTier,
};
use crate::player::{
    ion_cannon::{IonCannonShot, IonCannonShotRenderMarker},
    state::{Missile, Projectile},
    Player,
};
use bevy::prelude::*;
use bevy_rapier2d::prelude::{
    ActiveCollisionTypes, ActiveEvents, Ccd, Collider, CollisionGroups, Damping, ExternalForce,
    Group, Restitution, RigidBody, Sensor, Velocity,
};

/// Spawn test scenario: performance benchmark - 100 asteroids spread across viewport
/// Asteroids are spawned deterministically in a grid pattern so every run is comparable.
pub fn spawn_test_perf_benchmark(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "perf_benchmark".to_string();
    test_config.frame_limit = 300;

    let grey = Color::srgb(0.6, 0.6, 0.6);
    let side = 6.0_f32;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    let cols = 10u32;
    let rows = 10u32;
    let spacing = 80.0_f32;
    let offset_x = -((cols - 1) as f32) * spacing / 2.0;
    let offset_y = -((rows - 1) as f32) * spacing / 2.0;

    for row in 0..rows {
        for col in 0..cols {
            let x = offset_x + col as f32 * spacing;
            let y = offset_y + row as f32 * spacing;
            spawn_asteroid_with_vertices(&mut commands, Vec2::new(x, y), &vertices, grey, 1);
        }
    }

    println!(
        "✓ Spawned test: perf_benchmark — {}×{} grid ({} asteroids, {}u spacing)",
        cols,
        rows,
        cols * rows,
        spacing as u32,
    );
}

/// Performance benchmark: BASELINE configuration (original world size, no new features)
pub fn spawn_test_baseline_100(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "baseline_100".to_string();
    test_config.frame_limit = 300;
    spawn_standard_100_grid(&mut commands);
    println!("✓ Spawned test: baseline_100 — 100 asteroids, original world size, NO new features");
}

/// Performance benchmark: TIDAL TORQUE ONLY
pub fn spawn_test_tidal_only(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "tidal_only".to_string();
    test_config.frame_limit = 300;
    spawn_standard_100_grid(&mut commands);
    println!("✓ Spawned test: tidal_only — baseline + TIDAL TORQUE ENABLED (check physics.toml)");
}

/// Performance benchmark: SOFT BOUNDARY ONLY
pub fn spawn_test_soft_boundary_only(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "soft_boundary_only".to_string();
    test_config.frame_limit = 300;
    spawn_standard_100_grid(&mut commands);
    println!(
        "✓ Spawned test: soft_boundary_only — baseline + SOFT BOUNDARY ENABLED (check physics.toml)"
    );
}

/// Performance benchmark: KD-TREE ONLY
pub fn spawn_test_kdtree_only(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "kdtree_only".to_string();
    test_config.frame_limit = 300;
    spawn_standard_100_grid(&mut commands);
    println!("✓ Spawned test: kdtree_only — baseline + KD-TREE SPATIAL INDEX (already in use)");
}

/// Performance benchmark: ALL THREE FEATURES
pub fn spawn_test_all_three(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "all_three".to_string();
    test_config.frame_limit = 300;
    spawn_standard_100_grid(&mut commands);
    println!(
        "✓ Spawned test: all_three — 100 asteroids with ALL THREE features (see physics.toml)"
    );
}

/// Performance benchmark: BASELINE configuration at higher asteroid count (225 asteroids)
pub fn spawn_test_baseline_225(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "baseline_225".to_string();
    test_config.frame_limit = 300;
    spawn_standard_grid(&mut commands, 15, 15, 36.0);
    println!("✓ Spawned test: baseline_225 — 225 asteroids, original world size, NO new features");
}

/// Performance benchmark: ALL THREE FEATURES + multi-enemy load at higher asteroid count.
///
/// Expects player to be spawned in startup chain before this system.
pub fn spawn_test_all_three_225_enemy5(
    mut commands: Commands,
    mut test_config: ResMut<TestConfig>,
    mut enemy_spawn_state: ResMut<EnemySpawnState>,
    config: Res<PhysicsConfig>,
) {
    test_config.test_name = "all_three_225_enemy5".to_string();
    test_config.frame_limit = 300;

    spawn_standard_grid(&mut commands, 15, 15, 36.0);

    enemy_spawn_state.timer_secs = 10_000.0;

    let enemy_positions = [
        Vec2::new(360.0, 0.0),
        Vec2::new(-360.0, 0.0),
        Vec2::new(0.0, 360.0),
        Vec2::new(0.0, -360.0),
        Vec2::new(255.0, 255.0),
    ];
    for (idx, pos) in enemy_positions.into_iter().enumerate() {
        spawn_benchmark_enemy(&mut commands, &config, pos, idx as u64);
    }

    println!(
        "✓ Spawned test: all_three_225_enemy5 — 225 asteroids + 5 enemies with ALL THREE features"
    );
}

/// Performance benchmark: mixed-content heavy load
///
/// Includes variable asteroid masses/shapes, planets, more enemies, and scripted
/// spawning of all projectile classes during the run.
pub fn spawn_test_mixed_content_225_enemy8(
    mut commands: Commands,
    mut test_config: ResMut<TestConfig>,
    mut enemy_spawn_state: ResMut<EnemySpawnState>,
    config: Res<PhysicsConfig>,
) {
    test_config.test_name = "mixed_content_225_enemy8".to_string();
    test_config.frame_limit = 300;

    spawn_mixed_asteroid_field(&mut commands, &config, 15, 15, 52.0);

    spawn_planet(&mut commands, Vec2::new(700.0, 420.0), &config);
    spawn_planet(&mut commands, Vec2::new(-740.0, -360.0), &config);

    enemy_spawn_state.timer_secs = 10_000.0;

    let enemy_positions = [
        Vec2::new(420.0, 0.0),
        Vec2::new(-420.0, 0.0),
        Vec2::new(0.0, 420.0),
        Vec2::new(0.0, -420.0),
        Vec2::new(300.0, 300.0),
        Vec2::new(-300.0, 300.0),
        Vec2::new(300.0, -300.0),
        Vec2::new(-300.0, -300.0),
    ];
    for (idx, pos) in enemy_positions.into_iter().enumerate() {
        spawn_benchmark_enemy(&mut commands, &config, pos, idx as u64);
    }

    println!(
        "✓ Spawned test: mixed_content_225_enemy8 — varied asteroids + 2 planets + 8 enemies + scripted projectile mix"
    );
}

/// Performance benchmark: heavier-scale mixed-content load.
///
/// 324 asteroids + 12 enemies + 3 planets with the same scripted projectile
/// mix used by the 225 benchmark to better expose scaling costs.
pub fn spawn_test_mixed_content_324_enemy12(
    mut commands: Commands,
    mut test_config: ResMut<TestConfig>,
    mut enemy_spawn_state: ResMut<EnemySpawnState>,
    config: Res<PhysicsConfig>,
) {
    test_config.test_name = "mixed_content_324_enemy12".to_string();
    test_config.frame_limit = 300;

    spawn_mixed_asteroid_field(&mut commands, &config, 18, 18, 48.0);

    spawn_planet(&mut commands, Vec2::new(840.0, 520.0), &config);
    spawn_planet(&mut commands, Vec2::new(-860.0, -500.0), &config);
    spawn_planet(&mut commands, Vec2::new(0.0, 920.0), &config);

    enemy_spawn_state.timer_secs = 10_000.0;

    let enemy_positions = [
        Vec2::new(520.0, 0.0),
        Vec2::new(-520.0, 0.0),
        Vec2::new(0.0, 520.0),
        Vec2::new(0.0, -520.0),
        Vec2::new(380.0, 380.0),
        Vec2::new(-380.0, 380.0),
        Vec2::new(380.0, -380.0),
        Vec2::new(-380.0, -380.0),
        Vec2::new(700.0, 180.0),
        Vec2::new(-700.0, 180.0),
        Vec2::new(700.0, -180.0),
        Vec2::new(-700.0, -180.0),
    ];
    for (idx, pos) in enemy_positions.into_iter().enumerate() {
        spawn_benchmark_enemy(&mut commands, &config, pos, idx as u64);
    }

    println!(
        "✓ Spawned test: mixed_content_324_enemy12 — 324 varied asteroids + 3 planets + 12 enemies + scripted projectile mix"
    );
}

fn is_mixed_perf_stimulus_scenario(name: &str) -> bool {
    name == "mixed_content_225_enemy8" || name == "mixed_content_324_enemy12"
}

pub fn mixed_perf_projectile_stimulus_system(
    mut commands: Commands,
    test_config: Res<TestConfig>,
    config: Res<PhysicsConfig>,
    q_player: Query<&Transform, With<Player>>,
    q_enemy: Query<&Transform, With<Enemy>>,
) {
    if !test_config.enabled || !is_mixed_perf_stimulus_scenario(&test_config.test_name) {
        return;
    }

    let Ok(player_tf) = q_player.single() else {
        return;
    };

    let frame = test_config.frame_count;
    let player_pos = player_tf.translation.truncate();

    let target_enemy = q_enemy
        .iter()
        .min_by(|a, b| {
            let da = a.translation.truncate().distance_squared(player_pos);
            let db = b.translation.truncate().distance_squared(player_pos);
            da.total_cmp(&db)
        })
        .map(|tf| tf.translation.truncate())
        .unwrap_or(player_pos + Vec2::X * 150.0);

    if frame > 0 && frame.is_multiple_of(15) {
        spawn_perf_player_projectile(
            &mut commands,
            player_pos,
            target_enemy,
            config.projectile_speed,
        );
    }
    if frame > 0 && frame.is_multiple_of(45) {
        spawn_perf_player_missile(
            &mut commands,
            player_pos,
            target_enemy,
            config.missile_initial_speed,
            config.missile_collider_radius,
        );
    }
    if frame > 0 && frame.is_multiple_of(60) {
        spawn_perf_ion_shot(&mut commands, player_pos, target_enemy);
    }
    if frame > 0 && frame.is_multiple_of(20) {
        for enemy_tf in q_enemy.iter().take(4) {
            spawn_perf_enemy_projectile(
                &mut commands,
                enemy_tf.translation.truncate(),
                player_pos,
                config.enemy_projectile_speed,
                config.enemy_projectile_collider_radius,
            );
        }
    }
}

fn spawn_standard_100_grid(commands: &mut Commands) {
    spawn_standard_grid(commands, 10, 10, 40.0);
}

fn spawn_standard_grid(commands: &mut Commands, cols: u32, rows: u32, spacing: f32) {
    let grey = Color::srgb(0.6, 0.6, 0.6);
    let side = 6.0_f32;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    let offset_x = -((cols - 1) as f32) * spacing / 2.0;
    let offset_y = -((rows - 1) as f32) * spacing / 2.0;

    for row in 0..rows {
        for col in 0..cols {
            let x = offset_x + col as f32 * spacing;
            let y = offset_y + row as f32 * spacing;
            spawn_asteroid_with_vertices(commands, Vec2::new(x, y), &vertices, grey, 1);
        }
    }
}

fn spawn_mixed_asteroid_field(
    commands: &mut Commands,
    config: &PhysicsConfig,
    cols: u32,
    rows: u32,
    spacing: f32,
) {
    let offset_x = -((cols - 1) as f32) * spacing / 2.0;
    let offset_y = -((rows - 1) as f32) * spacing / 2.0;

    for row in 0..rows {
        for col in 0..cols {
            let idx = row * cols + col;
            let mass = 1 + (idx % 12);
            let base = canonical_vertices_for_mass(mass);
            let vertices = rescale_vertices_to_area(&base, mass as f32 / config.asteroid_density);
            let x = offset_x + col as f32 * spacing;
            let y = offset_y + row as f32 * spacing;

            let shade = 0.4 + (idx % 5) as f32 * 0.1;
            spawn_asteroid_with_vertices(
                commands,
                Vec2::new(x, y),
                &vertices,
                Color::srgb(shade, shade, shade),
                mass,
            );
        }
    }
}

fn spawn_benchmark_enemy(commands: &mut Commands, config: &PhysicsConfig, pos: Vec2, idx: u64) {
    let phase = ((idx.wrapping_mul(1_103_515_245).wrapping_add(12_345)) % 10_000) as f32 / 10_000.0;
    let fire_timer = config.enemy_fire_cooldown_base * (0.4 + 0.6 * phase);
    let toward_origin = (-pos).normalize_or_zero();

    let enemy_entity = commands
        .spawn((
            Enemy,
            EnemyHealth {
                hp: config.enemy_base_hp,
                max_hp: config.enemy_base_hp,
            },
            EnemyRenderMarker,
            EnemyFireCooldown { timer: fire_timer },
            Transform::from_translation(pos.extend(0.25)),
            Visibility::default(),
            RigidBody::Dynamic,
            Collider::ball(config.enemy_collider_radius),
            Velocity {
                linvel: toward_origin * (config.enemy_max_speed * 0.25),
                angvel: 0.0,
            },
            ExternalForce::default(),
            Damping {
                linear_damping: config.enemy_linear_damping,
                angular_damping: config.enemy_angular_damping,
            },
            Restitution::coefficient(0.25),
            CollisionGroups::new(
                Group::GROUP_5,
                Group::GROUP_1 | Group::GROUP_2 | Group::GROUP_3,
            ),
            ActiveEvents::COLLISION_EVENTS,
        ))
        .id();

    commands.entity(enemy_entity).insert((
        EnemyTier { level: 1 },
        EnemyStun {
            remaining_secs: 0.0,
        },
    ));
}

fn spawn_perf_player_projectile(commands: &mut Commands, start: Vec2, target: Vec2, speed: f32) {
    let dir = (target - start).normalize_or_zero();
    commands.spawn((
        Projectile {
            age: 0.0,
            distance_traveled: 0.0,
            was_hit: false,
        },
        Transform::from_translation((start + dir * 14.0).extend(0.0)),
        Visibility::default(),
        RigidBody::KinematicVelocityBased,
        Velocity {
            linvel: dir * speed,
            angvel: 0.0,
        },
        Collider::ball(2.0),
        Sensor,
        Ccd { enabled: true },
        CollisionGroups::new(Group::GROUP_3, Group::GROUP_1 | Group::GROUP_5),
        ActiveCollisionTypes::DYNAMIC_KINEMATIC,
        ActiveEvents::COLLISION_EVENTS,
    ));
}

fn spawn_perf_player_missile(
    commands: &mut Commands,
    start: Vec2,
    target: Vec2,
    speed: f32,
    radius: f32,
) {
    let dir = (target - start).normalize_or_zero();
    commands.spawn((
        Missile {
            age: 0.0,
            distance_traveled: 0.0,
            trail_emit_timer: 0.0,
        },
        Transform::from_translation((start + dir * 16.0).extend(0.0)),
        Visibility::default(),
        RigidBody::KinematicVelocityBased,
        Velocity {
            linvel: dir * speed,
            angvel: 0.0,
        },
        Collider::ball(radius),
        Sensor,
        Ccd { enabled: true },
        CollisionGroups::new(Group::GROUP_3, Group::GROUP_1 | Group::GROUP_5),
        ActiveCollisionTypes::DYNAMIC_KINEMATIC,
        ActiveEvents::COLLISION_EVENTS,
    ));
}

fn spawn_perf_ion_shot(commands: &mut Commands, start: Vec2, target: Vec2) {
    let dir = (target - start).normalize_or_zero();
    commands.spawn((
        IonCannonShot {
            age: 0.0,
            distance_traveled: 0.0,
        },
        IonCannonShotRenderMarker,
        Transform::from_translation((start + dir * 14.0).extend(0.2)),
        Visibility::default(),
        RigidBody::KinematicVelocityBased,
        Velocity {
            linvel: dir * crate::constants::ION_CANNON_SHOT_SPEED,
            angvel: 0.0,
        },
        Collider::ball(crate::constants::ION_CANNON_SHOT_COLLIDER_RADIUS),
        Sensor,
        Ccd { enabled: true },
        CollisionGroups::new(Group::GROUP_3, Group::GROUP_5),
        ActiveCollisionTypes::DYNAMIC_KINEMATIC,
        ActiveEvents::COLLISION_EVENTS,
    ));
}

fn spawn_perf_enemy_projectile(
    commands: &mut Commands,
    start: Vec2,
    target: Vec2,
    speed: f32,
    radius: f32,
) {
    let dir = (target - start).normalize_or_zero();
    commands.spawn((
        EnemyProjectile {
            age: 0.0,
            distance_traveled: 0.0,
        },
        EnemyProjectileRenderMarker,
        Transform::from_translation((start + dir * 12.0).extend(0.2)),
        Visibility::default(),
        RigidBody::KinematicVelocityBased,
        Velocity {
            linvel: dir * speed,
            angvel: 0.0,
        },
        Collider::ball(radius),
        Sensor,
        Ccd { enabled: true },
        CollisionGroups::new(Group::GROUP_6, Group::GROUP_1 | Group::GROUP_2),
        ActiveCollisionTypes::DYNAMIC_KINEMATIC,
        ActiveEvents::COLLISION_EVENTS,
    ));
}
