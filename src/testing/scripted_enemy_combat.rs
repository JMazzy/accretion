use crate::asteroid::{spawn_asteroid_with_vertices, Asteroid};
use crate::config::PhysicsConfig;
use crate::enemy::{
    Enemy, EnemyFireCooldown, EnemyHealth, EnemyProjectile, EnemyProjectileRenderMarker,
    EnemyRenderMarker, EnemySpawnState,
};
use crate::particles::Particle;
use crate::player::state::{PlayerHealth, Projectile};
use crate::player::Player;
use bevy::prelude::*;
use bevy_rapier2d::prelude::{
    ActiveCollisionTypes, ActiveEvents, Ccd, Collider, CollisionGroups, Damping, ExternalForce,
    Group, Restitution, RigidBody, Sensor, Velocity,
};

use super::{
    EnemyCombatObservations, EnemyCombatScriptState, ScriptAsteroidTarget, ScriptEnemyTarget,
    TestConfig,
};

/// Spawn deterministic entities for scripted enemy-combat verification.
///
/// Use with `ACCRETION_TEST=enemy_combat_scripted cargo run --release`.
pub fn spawn_test_enemy_combat_scripted(
    mut commands: Commands,
    mut test_config: ResMut<TestConfig>,
    mut enemy_spawn_state: ResMut<EnemySpawnState>,
    config: Res<PhysicsConfig>,
) {
    test_config.test_name = "enemy_combat_scripted".to_string();
    test_config.frame_limit = 180;

    commands.insert_resource(EnemyCombatScriptState::default());
    commands.insert_resource(EnemyCombatObservations::default());

    enemy_spawn_state.timer_secs = 10_000.0;

    commands.spawn((
        Enemy,
        EnemyRenderMarker,
        ScriptEnemyTarget,
        EnemyHealth {
            hp: config.enemy_base_hp,
            max_hp: config.enemy_base_hp,
        },
        EnemyFireCooldown { timer: 10_000.0 },
        Transform::from_translation(Vec3::new(240.0, 0.0, 0.25)),
        Visibility::default(),
        RigidBody::Dynamic,
        Collider::ball(config.enemy_collider_radius),
        Velocity::zero(),
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
    ));

    let side = 6.0;
    let h = side * 3.0_f32.sqrt() / 2.0;
    let tri = vec![
        Vec2::new(0.0, h / 2.0),
        Vec2::new(-side / 2.0, -h / 2.0),
        Vec2::new(side / 2.0, -h / 2.0),
    ];
    let asteroid_entity = spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(40.0, 160.0),
        &tri,
        Color::srgb(0.6, 0.6, 0.6),
        1,
    );
    commands
        .entity(asteroid_entity)
        .insert(ScriptAsteroidTarget);

    println!("✓ Spawned test: enemy_combat_scripted");
    println!("  Player at origin; enemy at (240,0); asteroid target at (40,160)");
    println!("  Script: frame 10 player→enemy, frame 25 enemy→player, frame 40 enemy→asteroid");
}

fn spawn_scripted_player_projectile(commands: &mut Commands, start: Vec2, dir: Vec2, speed: f32) {
    commands.spawn((
        Projectile {
            age: 0.0,
            was_hit: false,
        },
        Transform::from_translation(start.extend(0.0)),
        Visibility::default(),
        RigidBody::KinematicVelocityBased,
        Velocity {
            linvel: dir.normalize_or_zero() * speed,
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

fn spawn_scripted_enemy_projectile(commands: &mut Commands, start: Vec2, dir: Vec2, speed: f32) {
    commands.spawn((
        EnemyProjectile { age: 0.0 },
        EnemyProjectileRenderMarker,
        Transform::from_translation(start.extend(0.2)),
        Visibility::default(),
        RigidBody::KinematicVelocityBased,
        Velocity {
            linvel: dir.normalize_or_zero() * speed,
            angvel: 0.0,
        },
        Collider::ball(3.0),
        Sensor,
        Ccd { enabled: true },
        CollisionGroups::new(Group::GROUP_6, Group::GROUP_1 | Group::GROUP_2),
        ActiveCollisionTypes::DYNAMIC_KINEMATIC,
        ActiveEvents::COLLISION_EVENTS,
    ));
}

/// Drives deterministic shot timing for the scripted enemy combat test.
pub fn enemy_combat_script_system(
    mut commands: Commands,
    test_config: Res<TestConfig>,
    script_state: Option<ResMut<EnemyCombatScriptState>>,
    config: Res<PhysicsConfig>,
    q_player: Query<&Transform, With<Player>>,
    q_enemy: Query<&Transform, (With<Enemy>, With<ScriptEnemyTarget>)>,
    q_asteroid: Query<&Transform, (With<Asteroid>, With<ScriptAsteroidTarget>)>,
) {
    if !test_config.enabled || test_config.test_name != "enemy_combat_scripted" {
        return;
    }

    let Some(mut script_state) = script_state else {
        return;
    };

    let Ok(player_tf) = q_player.single() else {
        return;
    };
    let Ok(enemy_tf) = q_enemy.single() else {
        return;
    };
    let Ok(asteroid_tf) = q_asteroid.single() else {
        return;
    };

    let frame = test_config.frame_count;
    let player_pos = player_tf.translation.truncate();
    let enemy_pos = enemy_tf.translation.truncate();
    let asteroid_pos = asteroid_tf.translation.truncate();

    if frame >= 10 && !script_state.player_shot_spawned {
        let dir = enemy_pos - player_pos;
        spawn_scripted_player_projectile(
            &mut commands,
            player_pos + dir.normalize_or_zero() * 16.0,
            dir,
            config.projectile_speed,
        );
        script_state.player_shot_spawned = true;
        println!(
            "[Script] frame {}: spawned player projectile toward enemy",
            frame
        );
    }

    if frame >= 25 && !script_state.enemy_shot_player_spawned {
        let dir = player_pos - enemy_pos;
        spawn_scripted_enemy_projectile(
            &mut commands,
            enemy_pos + dir.normalize_or_zero() * (config.enemy_collider_radius + 6.0),
            dir,
            config.enemy_projectile_speed,
        );
        script_state.enemy_shot_player_spawned = true;
        println!(
            "[Script] frame {}: spawned enemy projectile toward player",
            frame
        );
    }

    if frame >= 40 && !script_state.enemy_shot_asteroid_spawned {
        let dir = asteroid_pos - enemy_pos;
        spawn_scripted_enemy_projectile(
            &mut commands,
            enemy_pos + dir.normalize_or_zero() * (config.enemy_collider_radius + 6.0),
            dir,
            config.enemy_projectile_speed,
        );
        script_state.enemy_shot_asteroid_spawned = true;
        println!(
            "[Script] frame {}: spawned enemy projectile toward asteroid",
            frame
        );
    }
}

/// Collect one-way runtime observations used by scripted test verification.
pub fn enemy_combat_observer_system(
    test_config: Res<TestConfig>,
    observations: Option<ResMut<EnemyCombatObservations>>,
    q_enemy: Query<&EnemyHealth, (With<Enemy>, With<ScriptEnemyTarget>)>,
    q_player: Query<&PlayerHealth, With<Player>>,
    q_asteroid: Query<Entity, (With<Asteroid>, With<ScriptAsteroidTarget>)>,
    q_particles: Query<Entity, With<Particle>>,
) {
    if !test_config.enabled || test_config.test_name != "enemy_combat_scripted" {
        return;
    }

    let Some(mut observations) = observations else {
        return;
    };

    if !observations.enemy_damage_observed {
        if let Ok(enemy_hp) = q_enemy.single() {
            if enemy_hp.hp < enemy_hp.max_hp {
                observations.enemy_damage_observed = true;
                observations.enemy_damage_first_frame = Some(test_config.frame_count);
                println!("[Observe] enemy damage observed");
            }
        }
    }

    if !observations.player_damage_observed {
        if let Ok(player_hp) = q_player.single() {
            if player_hp.hp < player_hp.max_hp {
                observations.player_damage_observed = true;
                observations.player_damage_first_frame = Some(test_config.frame_count);
                println!("[Observe] player damage observed");
            }
        }
    }

    if !observations.asteroid_hit_observed && q_asteroid.is_empty() {
        observations.asteroid_hit_observed = true;
        observations.asteroid_hit_first_frame = Some(test_config.frame_count);
        println!("[Observe] scripted asteroid was hit/despawned");
    }

    if !observations.particles_observed && !q_particles.is_empty() {
        observations.particles_observed = true;
        observations.particles_first_frame = Some(test_config.frame_count);
        println!("[Observe] impact particles observed");
    }
}
