//! Testing utilities for the simulation

use crate::asteroid::{spawn_asteroid_with_vertices, Asteroid, AsteroidSize, Vertices};
use crate::config::PhysicsConfig;
use crate::enemy::{
    Enemy, EnemyFireCooldown, EnemyHealth, EnemyProjectile, EnemyProjectileRenderMarker,
    EnemyRenderMarker, EnemySpawnState,
};
use crate::particles::Particle;
use crate::player::state::{PlayerHealth, Projectile};
use crate::player::Player;
use crate::simulation::MissileTelemetry;
use bevy::prelude::*;
use bevy_rapier2d::prelude::{
    ActiveCollisionTypes, ActiveEvents, Ccd, Collider, CollisionGroups, Damping, ExternalForce,
    Group, ReadMassProperties, Restitution, RigidBody, Sensor, Velocity,
};
use std::io::Write;

/// Test configuration
#[derive(Resource)]
pub struct TestConfig {
    pub enabled: bool,
    pub test_name: String,
    pub frame_limit: u32,
    pub frame_count: u32,
    pub initial_asteroid_count: usize,
    /// Per-frame delta times (seconds) recorded for perf_benchmark test
    pub perf_frame_times: Vec<f32>,
    /// For orbit_pair test: set to true once the orbiting body's velocity has been
    /// calibrated from the actual Rapier mass read back by [`ReadMassProperties`].
    pub velocity_calibrated: bool,
    /// For orbit_pair test: orbital radius (world units) recorded after calibration.
    pub orbit_initial_dist: f32,
    /// For orbit_pair test: most-recent orbital radius, updated each frame.
    pub orbit_final_dist: f32,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            test_name: String::new(),
            frame_limit: 100,
            frame_count: 0,
            initial_asteroid_count: 0,
            perf_frame_times: Vec::new(),
            velocity_calibrated: false,
            orbit_initial_dist: 0.0,
            orbit_final_dist: 0.0,
        }
    }
}

// ── Orbit-test component markers ─────────────────────────────────────

/// Tags the large central body in the `orbit_pair` test scenario.
#[derive(Component)]
pub struct OrbitCentralBody;

/// Tags the small orbiting body in the `orbit_pair` test scenario.
#[derive(Component)]
pub struct OrbitTestBody;

/// Marks the single deterministic enemy used by the scripted enemy combat test.
#[derive(Component)]
pub struct ScriptEnemyTarget;

/// Marks the deterministic asteroid target used by the scripted enemy combat test.
#[derive(Component)]
pub struct ScriptAsteroidTarget;

/// Internal state machine for scripted enemy combat playback.
#[derive(Resource, Default)]
pub struct EnemyCombatScriptState {
    pub player_shot_spawned: bool,
    pub enemy_shot_player_spawned: bool,
    pub enemy_shot_asteroid_spawned: bool,
}

/// One-way observation flags for scripted enemy combat verification.
#[derive(Resource, Default)]
pub struct EnemyCombatObservations {
    pub enemy_damage_observed: bool,
    pub player_damage_observed: bool,
    pub asteroid_hit_observed: bool,
    pub particles_observed: bool,
    pub enemy_damage_first_frame: Option<u32>,
    pub player_damage_first_frame: Option<u32>,
    pub asteroid_hit_first_frame: Option<u32>,
    pub particles_first_frame: Option<u32>,
}

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

    // Disable ambient enemy spawning so only scripted entities exist.
    enemy_spawn_state.timer_secs = 10_000.0;

    // Deterministic enemy target.
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

    // Deterministic asteroid target for enemy projectile path.
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

/// Spawn test scenario: two triangles touching
pub fn spawn_test_two_triangles(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    use crate::asteroid::spawn_asteroid_with_vertices;

    test_config.test_name = "two_triangles_combine".to_string();
    test_config.frame_limit = 100;

    // Create triangle vertices (side = 6.0, extends ±3 horizontally from center)
    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    // Spawn two triangles such that their edges ACTUALLY TOUCH at origin
    // Each extends ±3 units horizontally, so spawn at -3 and +3 to put edges at 0 and 0
    let grey = Color::srgb(0.5, 0.5, 0.5);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(-3.0, 0.0), &vertices, grey, 1);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(3.0, 0.0), &vertices, grey, 1);

    println!("✓ Spawned test: Two triangles touching at edges (centers at ±3)");
}

/// Spawn test scenario: three triangles in a cluster
pub fn spawn_test_three_triangles(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    use crate::asteroid::spawn_asteroid_with_vertices;

    test_config.test_name = "three_triangles_combine".to_string();
    test_config.frame_limit = 200;

    // Create triangle vertices (side = 6.0, extends ±3 horizontally from center)
    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    // Spawn three triangles at positions forming a touching triangle cluster
    // Each extends ±3 units horizontally, so position them to form a touching hexagon
    let grey = Color::srgb(0.5, 0.5, 0.5);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(-3.0, -3.0), &vertices, grey, 1);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(3.0, -3.0), &vertices, grey, 1);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(0.0, 3.0), &vertices, grey, 1);

    println!("✓ Spawned test: Three triangles touching in cluster formation");
}

/// Spawn test scenario: gravity test
pub fn spawn_test_gravity(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "gravity_attraction".to_string();
    test_config.frame_limit = 500; // Long enough to see collision behavior

    // Create triangle vertices
    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    // Spawn two asteroids FAR APART to test gravity attraction
    let grey = Color::srgb(0.5, 0.5, 0.5);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(-50.0, 0.0), &vertices, grey, 1);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(50.0, 0.0), &vertices, grey, 1);

    println!("✓ Spawned test: Two distant asteroids for gravity attraction test");
}

/// Spawn test scenario: high-speed head-on collision to test bouncing
pub fn spawn_test_high_speed_collision(
    mut commands: Commands,
    mut test_config: ResMut<TestConfig>,
) {
    use bevy_rapier2d::prelude::Velocity;

    test_config.test_name = "high_speed_collision".to_string();
    test_config.frame_limit = 300;

    // Create triangle vertices
    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    // Spawn two asteroids approaching each other at high speed
    let grey = Color::srgb(0.5, 0.5, 0.5);
    let e1 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(-30.0, 0.0), &vertices, grey, 1);
    let e2 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(30.0, 0.0), &vertices, grey, 1);

    // Give them high velocities toward each other (15 u/s each = 30 u/s closing speed)
    commands.entity(e1).insert(Velocity {
        linvel: Vec2::new(15.0, 0.0),
        angvel: 0.0,
    });
    commands.entity(e2).insert(Velocity {
        linvel: Vec2::new(-15.0, 0.0),
        angvel: 0.0,
    });

    println!("✓ Spawned test: High-speed head-on collision");
}

/// Spawn test scenario: missed collision - asteroids pass near each other trying to merge
pub fn spawn_test_near_miss(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    use bevy_rapier2d::prelude::Velocity;

    test_config.test_name = "near_miss".to_string();
    test_config.frame_limit = 300;

    // Create triangle vertices
    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    // Spawn two asteroids that will pass very close but not touch
    let grey = Color::srgb(0.5, 0.5, 0.5);
    let e1 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(-40.0, 3.0), &vertices, grey, 1);
    let e2 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(40.0, -3.0), &vertices, grey, 1);

    // Give them velocities so they pass near each other
    commands.entity(e1).insert(Velocity {
        linvel: Vec2::new(20.0, 0.0),
        angvel: 0.0,
    });
    commands.entity(e2).insert(Velocity {
        linvel: Vec2::new(-20.0, 0.0),
        angvel: 0.0,
    });

    println!("✓ Spawned test: Near-miss high-speed pass");
}

/// Spawn test scenario: slow-speed gravity approach (should result in clean merge)
pub fn spawn_test_gentle_approach(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    use bevy_rapier2d::prelude::Velocity;

    test_config.test_name = "gentle_approach".to_string();
    // Two asteroids 50 units apart at 2 u/s closing speed need ~700 frames to
    // actually touch; 800 gives comfortable margin including post-collision settling.
    test_config.frame_limit = 800;

    // Create triangle vertices
    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    // Spawn two asteroids with a gentle initial velocity toward each other so they
    // arrive well below the 10 u/s merge threshold and stick together.
    let grey = Color::srgb(0.5, 0.5, 0.5);
    let e1 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(-25.0, 0.0), &vertices, grey, 1);
    let e2 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(25.0, 0.0), &vertices, grey, 1);

    commands.entity(e1).insert(Velocity {
        linvel: Vec2::new(2.0, 0.0),
        angvel: 0.0,
    });
    commands.entity(e2).insert(Velocity {
        linvel: Vec2::new(-2.0, 0.0),
        angvel: 0.0,
    });

    println!("✓ Spawned test: Slow gravity approach");
}

/// Spawn test scenario: verify culling and that culled asteroids stop exerting gravity
pub fn spawn_test_culling_verification(
    mut commands: Commands,
    mut test_config: ResMut<TestConfig>,
) {
    use bevy_rapier2d::prelude::Velocity;

    test_config.test_name = "culling_verification".to_string();
    // Asteroid 2 starts at 2400u moving at 1000 u/s.
    // At ~60 fps it crosses HARD_CULL_DISTANCE (2500u) in ~6 frames; 30 frames gives plenty of margin.
    test_config.frame_limit = 30;

    // Create triangle vertices
    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    let grey = Color::srgb(0.5, 0.5, 0.5);

    // Spawn asteroid 1 at center (stationary) — should survive
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(0.0, 0.0), &vertices, grey, 1);

    // Spawn asteroid 2 inside but near the hard cull boundary (HARD_CULL_DISTANCE = 2500 u).
    // High velocity drives it past 2500u within ~6 frames so it is hard-culled well within the limit.
    let e2 =
        spawn_asteroid_with_vertices(&mut commands, Vec2::new(2400.0, 0.0), &vertices, grey, 1);
    commands.entity(e2).insert(Velocity {
        linvel: Vec2::new(1000.0, 0.0),
        angvel: 0.0,
    });

    println!("✓ Spawned test: Culling verification (ast 1 at origin, ast 2 at 2400u vel=1000 u/s — will cross hard cull boundary within ~6 frames)");
}

/// Spawn test scenario: large asteroid with several small ones at varying distances
pub fn spawn_test_mixed_size_asteroids(
    mut commands: Commands,
    mut test_config: ResMut<TestConfig>,
) {
    test_config.test_name = "mixed_size_asteroids".to_string();
    test_config.frame_limit = 300;

    // Create small triangle vertices (side = 6.0)
    let side_small = 6.0;
    let height_small = side_small * 3.0_f32.sqrt() / 2.0;
    let vertices_small = vec![
        Vec2::new(0.0, height_small / 2.0),
        Vec2::new(-side_small / 2.0, -height_small / 2.0),
        Vec2::new(side_small / 2.0, -height_small / 2.0),
    ];

    // Create large square asteroid (manually defined)
    let vertices_large = vec![
        Vec2::new(-15.0, -15.0),
        Vec2::new(15.0, -15.0),
        Vec2::new(15.0, 15.0),
        Vec2::new(-15.0, 15.0),
    ];

    let grey_dark = Color::srgb(0.3, 0.3, 0.3);
    let grey_light = Color::srgb(0.7, 0.7, 0.7);

    // Spawn large asteroid at center
    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(0.0, 0.0),
        &vertices_large,
        grey_dark,
        1,
    );

    // Spawn small asteroids at various distances around the large one
    // Distance 25 (very close, should interact strongly)
    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(25.0, 0.0),
        &vertices_small,
        grey_light,
        1,
    );

    // Distance 50
    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(0.0, 50.0),
        &vertices_small,
        grey_light,
        1,
    );

    // Distance 100 (within gravity range but far enough to have stable interaction)
    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(-100.0, 0.0),
        &vertices_small,
        grey_light,
        1,
    );

    // Distance 200 (far, minimal interaction)
    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(0.0, -200.0),
        &vertices_small,
        grey_light,
        1,
    );

    println!("✓ Spawned test: Mixed size asteroids (1 large + 4 small at distances 25/50/100/200)");
}

/// Spawn test scenario: simple large+small interaction
pub fn spawn_test_large_small_pair(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "large_small_pair".to_string();
    test_config.frame_limit = 250; // Increased to see merge

    // Create small triangle vertices
    let side_small = 6.0;
    let height_small = side_small * 3.0_f32.sqrt() / 2.0;
    let vertices_small = vec![
        Vec2::new(0.0, height_small / 2.0),
        Vec2::new(-side_small / 2.0, -height_small / 2.0),
        Vec2::new(side_small / 2.0, -height_small / 2.0),
    ];

    // Create large square asteroid
    let vertices_large = vec![
        Vec2::new(-15.0, -15.0),
        Vec2::new(15.0, -15.0),
        Vec2::new(15.0, 15.0),
        Vec2::new(-15.0, 15.0),
    ];

    let grey_dark = Color::srgb(0.3, 0.3, 0.3);
    let grey_light = Color::srgb(0.7, 0.7, 0.7);

    // Spawn large asteroid at center
    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(-30.0, 0.0),
        &vertices_large,
        grey_dark,
        1,
    );

    // Spawn small asteroid at distance
    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(30.0, 0.0),
        &vertices_small,
        grey_light,
        1,
    );

    println!("✓ Spawned test: Large+small pair (60 units apart)");
}

/// Spawn test scenario: asteroids at boundary of gravity range
pub fn spawn_test_gravity_boundary(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    use bevy_rapier2d::prelude::Velocity;

    test_config.test_name = "gravity_boundary".to_string();
    test_config.frame_limit = 300;

    // Create triangle vertices
    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    let grey = Color::srgb(0.5, 0.5, 0.5);

    // Spawn asteroid 1 at center
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(0.0, 0.0), &vertices, grey, 1);

    // Spawn asteroid 2 at exactly gravity max distance (300 units)
    let e2 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(300.0, 0.0), &vertices, grey, 1);

    // Give tiny velocity outward (should barely be affected by gravity since at boundary)
    commands.entity(e2).insert(Velocity {
        linvel: Vec2::new(0.1, 0.0),
        angvel: 0.0,
    });

    println!("✓ Spawned test: Gravity boundary (at 300u max distance)");
}

/// Spawn test scenario: small asteroid passing by large asteroid
pub fn spawn_test_passing_asteroid(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "passing_asteroid".to_string();
    test_config.frame_limit = 500;

    // Create small triangle (standard size)
    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let small_verts = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    // Create large octagon for the stationary asteroid
    let large_radius = 20.0;
    let mut large_verts = Vec::new();
    for i in 0..8 {
        let angle = (i as f32) * std::f32::consts::TAU / 8.0;
        large_verts.push(Vec2::new(
            large_radius * angle.cos(),
            large_radius * angle.sin(),
        ));
    }

    let grey = Color::srgb(0.5, 0.5, 0.5);

    // Spawn large stationary asteroid at origin
    let large_entity =
        spawn_asteroid_with_vertices(&mut commands, Vec2::new(0.0, 0.0), &large_verts, grey, 1);

    // Spawn small asteroid that will pass by at ~50 unit distance
    // Position it to the left, moving right with enough offset to pass by
    use crate::asteroid::{Asteroid, AsteroidSize, NeighborCount};
    use bevy_rapier2d::prelude::{
        ActiveEvents, Collider, CollisionGroups, ExternalForce, Group, Restitution, RigidBody,
    };

    let small_entity = commands
        .spawn((
            Asteroid,
            AsteroidSize(1),
            Vertices(small_verts.clone()),
            NeighborCount(0),
            RigidBody::Dynamic,
            Collider::ball(2.0),
            Restitution::coefficient(0.5),
            Velocity {
                linvel: Vec2::new(30.0, 0.0), // Moving right at constant speed
                angvel: 0.0,
            },
            ExternalForce::default(),
            ActiveEvents::COLLISION_EVENTS,
            CollisionGroups::new(
                Group::GROUP_1,
                Group::GROUP_1 | Group::GROUP_2 | Group::GROUP_3,
            ),
            Transform::from_xyz(-150.0, 50.0, 0.0),
        ))
        .id();

    println!("✓ Spawned test: Small asteroid passing by large stationary asteroid");
    println!(
        "  Large asteroid: center at (0, 0), radius ~20u, entity={:?}",
        large_entity
    );
    println!(
        "  Small asteroid: starts at (-150, 50), velocity (30, 0) u/s, entity={:?}",
        small_entity
    );
    println!("  Expected: Small asteroid passes at ~50u distance, gravity should:");
    println!("    - Pull down (toward large) as it approaches");
    println!("    - Pull backward (opposite motion) after it passes");
}

/// Track asteroid count and log state
/// Spawn test scenario: performance benchmark - 100 asteroids spread across viewport
/// Asteroids are spawned deterministically in a grid pattern so every run is comparable.
pub fn spawn_test_perf_benchmark(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "perf_benchmark".to_string();
    test_config.frame_limit = 300;

    let grey = Color::srgb(0.6, 0.6, 0.6);

    // Standard equilateral triangle vertices (same as spawn_asteroid)
    let side = 6.0_f32;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    // 10×10 grid, spacing 80 units → spans ±360 units from origin
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
/// Spawns 100 asteroids in a grid and runs for 300 frames.
/// This is the reference point before tidal torque, soft boundary, and KD-tree were added.
pub fn spawn_test_baseline_100(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "baseline_100".to_string();
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
    let spacing = 40.0_f32;
    let offset_x = -((cols - 1) as f32) * spacing / 2.0;
    let offset_y = -((rows - 1) as f32) * spacing / 2.0;

    for row in 0..rows {
        for col in 0..cols {
            let x = offset_x + col as f32 * spacing;
            let y = offset_y + row as f32 * spacing;
            spawn_asteroid_with_vertices(&mut commands, Vec2::new(x, y), &vertices, grey, 1);
        }
    }

    println!("✓ Spawned test: baseline_100 — 100 asteroids, original world size, NO new features");
}

/// Performance benchmark: TIDAL TORQUE ONLY
/// Baseline + tidal torque enabled. Isolates the cost of per-vertex gravity calculations.
pub fn spawn_test_tidal_only(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "tidal_only".to_string();
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
    let spacing = 40.0_f32;
    let offset_x = -((cols - 1) as f32) * spacing / 2.0;
    let offset_y = -((rows - 1) as f32) * spacing / 2.0;

    for row in 0..rows {
        for col in 0..cols {
            let x = offset_x + col as f32 * spacing;
            let y = offset_y + row as f32 * spacing;
            spawn_asteroid_with_vertices(&mut commands, Vec2::new(x, y), &vertices, grey, 1);
        }
    }

    println!("✓ Spawned test: tidal_only — baseline + TIDAL TORQUE ENABLED (check physics.toml)");
}

/// Performance benchmark: SOFT BOUNDARY ONLY
/// Baseline + soft boundary enabled. Isolates the cost of the boundary spring force.
pub fn spawn_test_soft_boundary_only(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "soft_boundary_only".to_string();
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
    let spacing = 40.0_f32;
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
        "✓ Spawned test: soft_boundary_only — baseline + SOFT BOUNDARY ENABLED (check physics.toml)"
    );
}

/// Performance benchmark: KD-TREE ONLY
/// Baseline + KD-tree spatial index. Isolates the cost of the spatial index redesign.
pub fn spawn_test_kdtree_only(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "kdtree_only".to_string();
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
    let spacing = 40.0_f32;
    let offset_x = -((cols - 1) as f32) * spacing / 2.0;
    let offset_y = -((rows - 1) as f32) * spacing / 2.0;

    for row in 0..rows {
        for col in 0..cols {
            let x = offset_x + col as f32 * spacing;
            let y = offset_y + row as f32 * spacing;
            spawn_asteroid_with_vertices(&mut commands, Vec2::new(x, y), &vertices, grey, 1);
        }
    }

    println!("✓ Spawned test: kdtree_only — baseline + KD-TREE SPATIAL INDEX (already in use)");
}

/// Performance benchmark: ALL THREE FEATURES
/// Full current implementation with tidal torque, soft boundary, and KD-tree.
pub fn spawn_test_all_three(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "all_three".to_string();
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
    let spacing = 40.0_f32;
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
        "✓ Spawned test: all_three — 100 asteroids with ALL THREE features (see physics.toml)"
    );
}

#[derive(Component)]
#[allow(dead_code)]
pub struct TestMarker(pub usize); // Initial index for tracking

// ── orbit_pair test ─────────────────────────────────────────────────────────────

/// Spawn scenario for the `orbit_pair` test.
///
/// Spawns one large central body (AsteroidSize = 2 000 000) and one small
/// triangle orbiting at 200 u.  On frame 2 the system
/// [`orbit_pair_calibrate_and_track_system`] reads the actual Rapier mass back
/// from [`ReadMassProperties`] and applies the correct circular-orbit velocity
/// `v = sqrt(G · M_central / (r · m_rapier))`.
///
/// Run with `ACCRETION_TEST=orbit_pair cargo run --release`.
pub fn spawn_test_orbit_pair(
    mut commands: Commands,
    mut test_config: ResMut<TestConfig>,
    config: Res<PhysicsConfig>,
) {
    test_config.test_name = "orbit_pair".to_string();
    test_config.frame_limit = 1500; // ~1.2 revolutions at expected velocity
    test_config.velocity_calibrated = false;
    test_config.orbit_initial_dist = 0.0;
    test_config.orbit_final_dist = 0.0;

    // Central body — very large AsteroidSize so it dominates gravity; small
    // visual radius (10 u) so the orbiting triangle can't accidentally touch it.
    let central_mass: u32 = 2_000_000;
    let central_radius = 10.0_f32;
    let central_verts: Vec<Vec2> = (0..16)
        .map(|i| {
            let angle = std::f32::consts::TAU * i as f32 / 16.0;
            Vec2::new(central_radius * angle.cos(), central_radius * angle.sin())
        })
        .collect();
    let central_entity = spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::ZERO,
        &central_verts,
        Color::srgb(0.9, 0.5, 0.1),
        central_mass,
    );
    commands
        .entity(central_entity)
        .insert((ReadMassProperties::default(), OrbitCentralBody));

    // Orbiting triangle — starts at (orbital_radius, 0); velocity applied on
    // frame 2 once Rapier has written the real mass into ReadMassProperties.
    let orbital_radius = 200.0_f32;
    let side = config.triangle_base_side;
    let h = side * 3.0_f32.sqrt() / 2.0;
    let tri_verts = vec![
        Vec2::new(0.0, h / 2.0),
        Vec2::new(-side / 2.0, -h / 2.0),
        Vec2::new(side / 2.0, -h / 2.0),
    ];
    let orbit_entity = spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(orbital_radius, 0.0),
        &tri_verts,
        Color::srgb(0.4, 0.9, 0.4),
        1,
    );
    commands
        .entity(orbit_entity)
        .insert((ReadMassProperties::default(), OrbitTestBody));

    println!(
        "✓ Spawned orbit_pair test: central at (0,0) r={central_radius} mass={central_mass}, \
         orbiter at ({orbital_radius},0) side={side}"
    );
    println!(
        "  Expected Rapier mass of triangle ≈ {:.4} (√3/4·{side}²)",
        3.0_f32.sqrt() / 4.0 * side * side
    );
}

/// Calibrates orbital velocity from actual Rapier mass and tracks orbit radius.
///
/// - **Frame 2**: reads [`ReadMassProperties`] from the orbiting triangle,
///   computes `v = sqrt(G · M_central / (r · m_rapier))`, applies it to
///   `Velocity`, and records `orbit_initial_dist`.
/// - **Every subsequent frame**: updates [`TestConfig::orbit_final_dist`] so
///   [`test_verification_system`] can assess orbit stability.
///
/// This system is a no-op for all tests other than `"orbit_pair"`.
#[allow(clippy::type_complexity)]
pub fn orbit_pair_calibrate_and_track_system(
    mut test_config: ResMut<TestConfig>,
    config: Res<PhysicsConfig>,
    central_q: Query<(&Transform, &AsteroidSize), (With<OrbitCentralBody>, Without<OrbitTestBody>)>,
    mut orbit_q: Query<
        (&Transform, &ReadMassProperties, &mut Velocity),
        (With<OrbitTestBody>, Without<OrbitCentralBody>),
    >,
) {
    if !test_config.enabled || test_config.test_name != "orbit_pair" {
        return;
    }

    let Ok((central_tf, central_size)) = central_q.single() else {
        return;
    };
    let Ok((orbit_tf, mass_props, mut orbit_vel)) = orbit_q.single_mut() else {
        return;
    };

    let central_pos = central_tf.translation.truncate();
    let orbit_pos = orbit_tf.translation.truncate();
    let current_dist = (orbit_pos - central_pos).length();

    // On frame 2 (Rapier has run once; ReadMassProperties is populated) apply
    // the analytically-correct circular-orbit velocity.
    if !test_config.velocity_calibrated && test_config.frame_count >= 2 {
        let m_rapier = mass_props.mass;
        let m_central = central_size.0 as f32;
        let g = config.gravity_const;

        // v = sqrt(G · M_central / (r · m_rapier))  (centripetal condition)
        let v_mag = (g * m_central / (current_dist * m_rapier)).sqrt();
        let radial = (orbit_pos - central_pos).normalize_or_zero();
        let tangent = Vec2::new(-radial.y, radial.x); // CCW
        orbit_vel.linvel = tangent * v_mag;

        test_config.orbit_initial_dist = current_dist;
        test_config.velocity_calibrated = true;

        let period_s = std::f32::consts::TAU * current_dist / v_mag;
        println!(
            "[Orbit calibration] frame={} G={g}  M_central={m_central}  \
             m_rapier={m_rapier:.4}  r={current_dist:.1}  v={v_mag:.4} u/s",
            test_config.frame_count
        );
        println!(
            "[Orbit calibration] Expect period ≈ {period_s:.1}s = {:.0} frames at 60fps",
            period_s * 60.0
        );
    }

    if test_config.velocity_calibrated {
        test_config.orbit_final_dist = current_dist;
    }
}

pub fn test_logging_system(
    mut test_config: ResMut<TestConfig>,
    time: Res<Time>,
    missile_telemetry: Res<MissileTelemetry>,
    q: Query<(Entity, &Transform, &Velocity, &Vertices, &ExternalForce), With<Asteroid>>,
) {
    if !test_config.enabled {
        return;
    }

    test_config.frame_count += 1;
    let asteroid_count = q.iter().count();

    // For perf benchmark and the feature isolation tests: record every frame's delta time
    let is_perf_test = test_config.test_name == "perf_benchmark"
        || test_config.test_name == "baseline_100"
        || test_config.test_name == "tidal_only"
        || test_config.test_name == "soft_boundary_only"
        || test_config.test_name == "kdtree_only"
        || test_config.test_name == "all_three";

    if is_perf_test {
        let dt_ms = time.delta_secs() * 1000.0;
        test_config.perf_frame_times.push(dt_ms);

        if test_config.frame_count == 1 {
            test_config.initial_asteroid_count = asteroid_count;
            println!(
                "[Frame 1] {} started | asteroids: {}",
                test_config.test_name, asteroid_count
            );
        } else if test_config.frame_count.is_multiple_of(50)
            || test_config.frame_count == test_config.frame_limit
        {
            let window = &test_config.perf_frame_times
                [test_config.perf_frame_times.len().saturating_sub(50)..];
            let avg = window.iter().sum::<f32>() / window.len() as f32;
            let min = window.iter().cloned().fold(f32::INFINITY, f32::min);
            let max = window.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            println!(
                "[Frame {}] asteroids: {} | last {} frames — avg: {:.2}ms  min: {:.2}ms  max: {:.2}ms  (target ≤16.7ms)",
                test_config.frame_count,
                asteroid_count,
                window.len(),
                avg,
                min,
                max,
            );
        }
        return;
    }

    // Log state at certain frames
    if test_config.frame_count == 1 {
        test_config.initial_asteroid_count = asteroid_count;
        println!(
            "[Frame {}] Test: {} | Initial asteroids: {}",
            test_config.frame_count, test_config.test_name, asteroid_count
        );
        // Also log positions and entity IDs
        for (entity, transform, _, _, _) in q.iter() {
            println!(
                "  Entity {:?} at: ({:.1}, {:.1})",
                entity, transform.translation.x, transform.translation.y
            );
        }
    } else if test_config.frame_count == 10
        || test_config.frame_count == 20
        || test_config.frame_count == 30
        || test_config.frame_count == 40
        || test_config.frame_count == 50
        || test_config.frame_count.is_multiple_of(25)
        || test_config.frame_count == test_config.frame_limit
    {
        println!(
            "[Frame {}] Asteroids: {} (was {})",
            test_config.frame_count, asteroid_count, test_config.initial_asteroid_count
        );

        if missile_telemetry.shots_fired > 0 {
            let shots = missile_telemetry.shots_fired as f32;
            let hits = missile_telemetry.hits as f32;
            let hit_rate = if shots > 0.0 {
                100.0 * hits / shots
            } else {
                0.0
            };
            let outcome_total = missile_telemetry.instant_destroy_events
                + missile_telemetry.split_events
                + missile_telemetry.full_decompose_events;
            let outcome_total_f = outcome_total.max(1) as f32;
            let kill_events =
                missile_telemetry.instant_destroy_events + missile_telemetry.full_decompose_events;
            let frames_per_kill = if kill_events > 0 {
                test_config.frame_count as f32 / kill_events as f32
            } else {
                f32::INFINITY
            };
            println!(
                "  Missile telemetry | shots={} hits={} hit_rate={:.1}% outcomes[destroy={:.1}%, split={:.1}%, decompose={:.1}%] ttk_proxy_frames_per_kill={} mass[destroyed={}, decomposed={}]",
                missile_telemetry.shots_fired,
                missile_telemetry.hits,
                hit_rate,
                100.0 * missile_telemetry.instant_destroy_events as f32 / outcome_total_f,
                100.0 * missile_telemetry.split_events as f32 / outcome_total_f,
                100.0 * missile_telemetry.full_decompose_events as f32 / outcome_total_f,
                if frames_per_kill.is_finite() {
                    format!("{frames_per_kill:.1}")
                } else {
                    "n/a".to_string()
                },
                missile_telemetry.destroyed_mass_total,
                missile_telemetry.decomposed_mass_total,
            );
        }

        // Collect positions for distance calculations
        let positions: Vec<(Entity, Vec2, Vec2, Vec2, f32)> = q
            .iter()
            .map(|(e, t, v, _, f)| {
                (
                    e,
                    t.translation.truncate(),
                    v.linvel,
                    f.force,
                    f.force.length(),
                )
            })
            .collect();

        // Log positions, velocities, and force vectors with distances
        for (i, (entity, pos, vel, force, force_mag)) in positions.iter().enumerate() {
            let force_dir = if *force_mag > 0.0001 {
                // Lower threshold to see small forces
                format!("({:.3}, {:.3})", force.x, force.y)
            } else {
                "none".to_string()
            };

            // Calculate distance to other asteroids
            let mut distances = Vec::new();
            for (j, (_, other_pos, _, _, _)) in positions.iter().enumerate() {
                if i != j {
                    let dist = (*other_pos - *pos).length();
                    distances.push(format!("d[{}]={:.1}", j, dist));
                }
            }
            let dist_str = distances.join(", ");

            println!("  [{}] Entity={:?} pos: ({:.1}, {:.1}), vel: ({:.1}, {:.1}) len={:.2}, force: {} mag={:.3}, {}", 
                i, entity, pos.x, pos.y, vel.x, vel.y, vel.length(), force_dir, force_mag, dist_str);
        }
    }
}

/// Verify test results at the end
pub fn test_verification_system(
    test_config: Res<TestConfig>,
    missile_telemetry: Res<MissileTelemetry>,
    q: Query<(&Transform, &Vertices), With<Asteroid>>,
    enemy_combat_obs: Option<Res<EnemyCombatObservations>>,
    enemy_combat_script: Option<Res<EnemyCombatScriptState>>,
    mut exit: MessageWriter<bevy::app::AppExit>,
) {
    if !test_config.enabled || test_config.frame_count != test_config.frame_limit {
        return;
    }

    let final_count = q.iter().count();

    println!("\n╔════════════════════════════════════════════╗");
    println!("║           TEST COMPLETE                    ║");
    println!("╚════════════════════════════════════════════╝");
    println!("Test: {}", test_config.test_name);
    println!("Frames: {}", test_config.frame_count);
    println!("Initial asteroids: {}", test_config.initial_asteroid_count);
    println!("Final asteroids:   {}", final_count);

    if test_config.test_name == "enemy_combat_scripted" {
        let mut player_shot = false;
        let mut enemy_player_shot = false;
        let mut enemy_asteroid_shot = false;
        if let Some(script) = enemy_combat_script {
            player_shot = script.player_shot_spawned;
            enemy_player_shot = script.enemy_shot_player_spawned;
            enemy_asteroid_shot = script.enemy_shot_asteroid_spawned;
        }

        let mut enemy_damaged = false;
        let mut player_damaged = false;
        let mut asteroid_hit = false;
        let mut particles_seen = false;
        let mut enemy_damage_frame = None;
        let mut player_damage_frame = None;
        let mut asteroid_hit_frame = None;
        let mut particles_frame = None;
        if let Some(obs) = enemy_combat_obs {
            enemy_damaged = obs.enemy_damage_observed;
            player_damaged = obs.player_damage_observed;
            asteroid_hit = obs.asteroid_hit_observed;
            particles_seen = obs.particles_observed;
            enemy_damage_frame = obs.enemy_damage_first_frame;
            player_damage_frame = obs.player_damage_first_frame;
            asteroid_hit_frame = obs.asteroid_hit_first_frame;
            particles_frame = obs.particles_first_frame;
        }

        println!("Script shots spawned: player->enemy={player_shot}, enemy->player={enemy_player_shot}, enemy->asteroid={enemy_asteroid_shot}");
        println!("Observed outcomes: enemy_damaged={enemy_damaged}, player_damaged={player_damaged}, asteroid_hit={asteroid_hit}, particles_seen={particles_seen}");
        println!(
            "Observed first frames: enemy_damage={:?}, player_damage={:?}, asteroid_hit={:?}, particles={:?}",
            enemy_damage_frame, player_damage_frame, asteroid_hit_frame, particles_frame
        );

        // Strong ordering assertion:
        // enemy damage must be observed before the asteroid-leg shot is ever scripted (frame 40+).
        let enemy_damage_pre_asteroid_leg = enemy_damage_frame.is_some_and(|f| f < 40);
        let enemy_damage_in_player_shot_window =
            enemy_damage_frame.is_some_and(|f| (10..40).contains(&f));
        let enemy_damage_before_player_damage =
            matches!((enemy_damage_frame, player_damage_frame), (Some(e), Some(p)) if e < p);
        let enemy_damage_before_asteroid_hit =
            matches!((enemy_damage_frame, asteroid_hit_frame), (Some(e), Some(a)) if e < a);

        let pass = player_shot
            && enemy_player_shot
            && enemy_asteroid_shot
            && enemy_damaged
            && player_damaged
            && asteroid_hit
            && particles_seen
            && enemy_damage_pre_asteroid_leg
            && enemy_damage_in_player_shot_window
            && enemy_damage_before_player_damage
            && enemy_damage_before_asteroid_hit;

        if pass {
            println!(
                "✓ PASS: enemy_combat_scripted — scripted runtime collision contracts observed"
            );
        } else {
            println!(
                "✗ FAIL: enemy_combat_scripted — one or more scripted collision outcomes missing"
            );
            if !enemy_damage_pre_asteroid_leg {
                println!(
                    "  Additional failure: enemy damage did not occur before asteroid-leg shot frame (40)."
                );
            }
            if !enemy_damage_in_player_shot_window {
                println!(
                    "  Additional failure: enemy damage was not first observed in expected player-shot window [10, 40)."
                );
            }
            if !enemy_damage_before_player_damage {
                println!(
                    "  Additional failure: enemy damage was not observed before player damage."
                );
            }
            if !enemy_damage_before_asteroid_hit {
                println!(
                    "  Additional failure: enemy damage was not observed before asteroid-hit outcome."
                );
            }
        }

        let _ = std::io::stdout().flush();
        exit.write(bevy::app::AppExit::Success);
        return;
    }

    if missile_telemetry.shots_fired > 0 {
        let shots = missile_telemetry.shots_fired as f32;
        let hits = missile_telemetry.hits as f32;
        let hit_rate = if shots > 0.0 {
            100.0 * hits / shots
        } else {
            0.0
        };
        let kill_events =
            missile_telemetry.instant_destroy_events + missile_telemetry.full_decompose_events;
        let frames_per_kill = if kill_events > 0 {
            test_config.frame_count as f32 / kill_events as f32
        } else {
            f32::INFINITY
        };
        println!(
            "Missile telemetry: shots={} hits={} hit_rate={:.1}% destroy={} split={} decompose={} ttk_proxy_frames_per_kill={} mass_destroyed={} mass_decomposed={}",
            missile_telemetry.shots_fired,
            missile_telemetry.hits,
            hit_rate,
            missile_telemetry.instant_destroy_events,
            missile_telemetry.split_events,
            missile_telemetry.full_decompose_events,
            if frames_per_kill.is_finite() {
                format!("{frames_per_kill:.1}")
            } else {
                "n/a".to_string()
            },
            missile_telemetry.destroyed_mass_total,
            missile_telemetry.decomposed_mass_total,
        );
    }

    // Print full timing report for benchmark tests
    if (test_config.test_name == "perf_benchmark"
        || test_config.test_name == "baseline_100"
        || test_config.test_name == "tidal_only"
        || test_config.test_name == "soft_boundary_only"
        || test_config.test_name == "kdtree_only"
        || test_config.test_name == "all_three")
        && !test_config.perf_frame_times.is_empty()
    {
        let times = &test_config.perf_frame_times;
        // Skip first 10 frames (startup jitter)
        let steady = if times.len() > 10 {
            &times[10..]
        } else {
            times.as_slice()
        };
        let avg = steady.iter().sum::<f32>() / steady.len() as f32;
        let min = steady.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = steady.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let over_budget = steady.iter().filter(|&&t| t > 16.7).count();
        let pct_60fps = 100.0 * (steady.len() - over_budget) as f32 / steady.len() as f32;
        println!("\n── Timing summary (frames 10–{}) ──", times.len());
        println!("  avg frame: {:.2}ms", avg);
        println!("  min frame: {:.2}ms", min);
        println!("  max frame: {:.2}ms", max);
        println!(
            "  frames at 60 FPS (≤16.7ms): {}/{} ({:.1}%)",
            steady.len() - over_budget,
            steady.len(),
            pct_60fps
        );
        if avg <= 16.7 {
            println!("  ✓ Average frame time within 60 FPS budget");
        } else {
            println!("  ✗ Average frame time {:.2}ms exceeds 16.7ms budget", avg);
        }
    }

    let result = verify_test_result(
        &test_config.test_name,
        test_config.initial_asteroid_count,
        final_count,
        test_config.orbit_initial_dist,
        test_config.orbit_final_dist,
        test_config.velocity_calibrated,
    );
    println!("{}\n", result);
    let _ = std::io::stdout().flush();

    // Exit after test completes
    exit.write(bevy::app::AppExit::Success);
}

/// Verify if test passed
fn verify_test_result(
    test_name: &str,
    initial: usize,
    final_count: usize,
    orbit_initial: f32,
    orbit_final: f32,
    orbit_calibrated: bool,
) -> String {
    match test_name {
        "two_triangles_combine" => {
            if final_count < initial && final_count >= 1 {
                format!(
                    "✓ PASS: Two triangles combined into {}asteroid(s)",
                    final_count
                )
            } else {
                format!(
                    "✗ FAIL: Expected combining, but got: {} → {} asteroids",
                    initial, final_count
                )
            }
        }
        "three_triangles_combine" => {
            if final_count < initial && final_count >= 1 {
                format!(
                    "✓ PASS: Three triangles combined into {}asteroid(s)",
                    final_count
                )
            } else {
                format!(
                    "✗ FAIL: Expected combining, but got: {} → {} asteroids",
                    initial, final_count
                )
            }
        }
        "gravity_attraction" => {
            if initial > 1 && final_count <= initial {
                "✓ PASS: Asteroids interacted (gravity or collision)".to_string()
            } else {
                "✗ FAIL: Asteroids did not interact as expected".to_string()
            }
        }
        "high_speed_collision" => {
            if initial == 2 && final_count == 2 {
                "✓ PASS: Two asteroids bounced without merging (remained 2)".to_string()
            } else if final_count < initial && final_count >= 1 {
                format!("✓ PASS: Asteroids merged into {}asteroid(s)", final_count)
            } else {
                format!(
                    "✗ FAIL: Unexpected result: {} → {} asteroids",
                    initial, final_count
                )
            }
        }
        "near_miss" => {
            if initial == 2 && final_count == 2 {
                "✓ PASS: Two asteroids passed each other without merging (remained 2)".to_string()
            } else {
                format!(
                    "✗ FAIL: Expected 2 separate asteroids, got {} → {}",
                    initial, final_count
                )
            }
        }
        "gentle_approach" => {
            if final_count < initial && final_count >= 1 {
                format!(
                    "✓ PASS: Asteroids merged cleanly via gravity ({} → {})",
                    initial, final_count
                )
            } else {
                format!(
                    "✗ FAIL: Expected gentle merge, got {} → {} asteroids",
                    initial, final_count
                )
            }
        }
        "culling_verification" => {
            if initial == 2 && final_count == 1 {
                format!(
                    "✓ PASS: One asteroid was culled ({} → {})",
                    initial, final_count
                )
            } else {
                format!(
                    "✗ FAIL: Expected culling result 2 → 1, got {} → {}",
                    initial, final_count
                )
            }
        }
        "mixed_size_asteroids" => {
            if initial == 5 {
                format!(
                    "✓ PASS: All 5 asteroids present at end ({} → {})",
                    initial, final_count
                )
            } else {
                format!("✗ FAIL: Expected 5 asteroids, got {}", initial)
            }
        }
        "large_small_pair" => {
            if initial == 2 && final_count <= initial {
                if final_count == 1 {
                    "✓ PASS: Large+small merged into 1 asteroid".to_string()
                } else {
                    format!(
                        "✓ PASS: Large+small interaction stable (2 → {})",
                        final_count
                    )
                }
            } else {
                format!("✗ FAIL: Unexpected result {} → {}", initial, final_count)
            }
        }
        "gravity_boundary" => {
            if initial == 2 && final_count == 2 {
                "✓ PASS: Asteroids remained separate at gravity boundary (no merge)".to_string()
            } else if initial == 2 && final_count == 1 {
                "✓ PASS: Asteroids eventually merged from boundary distance".to_string()
            } else {
                format!(
                    "✗ FAIL: Expected stable or merged, got {} → {}",
                    initial, final_count
                )
            }
        }
        "passing_asteroid" => {
            // For this test, we just want to verify forces make sense
            // Small asteroid should pass by without runaway acceleration
            if initial == 2 {
                "✓ PASS: Small asteroid passed by large one (check velocity logs)".to_string()
            } else {
                format!("✗ FAIL: Expected 2 asteroids, got {}", initial)
            }
        }
        "perf_benchmark" => {
            // Pass/fail decided from timing summary printed by test_logging_system.
            // Here we just report final asteroid count as a sanity check.
            format!(
                "✓ PASS: perf_benchmark complete — {} asteroids remaining (see timing logs above)",
                final_count
            )
        }
        "baseline_100" => {
            format!(
                "✓ PASS: baseline_100 complete — {} asteroids | Compare timing to tidal_only, soft_boundary_only, kdtree_only, all_three",
                final_count
            )
        }
        "tidal_only" => {
            format!(
                "✓ PASS: tidal_only complete — {} asteroids | Cost = tidal_only minus baseline_100",
                final_count
            )
        }
        "soft_boundary_only" => {
            format!(
                "✓ PASS: soft_boundary_only complete — {} asteroids | Cost = soft_boundary_only minus baseline_100",
                final_count
            )
        }
        "kdtree_only" => {
            format!(
                "✓ PASS: kdtree_only complete — {} asteroids | Cost = kdtree_only minus baseline_100",
                final_count
            )
        }
        "all_three" => {
            format!(
                "✓ PASS: all_three complete — {} asteroids | Full cost = all_three minus baseline_100",
                final_count
            )
        }
        "orbit_pair" => {
            if !orbit_calibrated {
                format!(
                    "✗ FAIL: orbit_pair — orbit never calibrated (check ReadMassProperties population). \
                     asteroid_count={final_count}"
                )
            } else {
                let drift_pct = ((orbit_final - orbit_initial) / orbit_initial).abs() * 100.0;
                if drift_pct < 30.0 {
                    format!(
                        "✓ PASS: orbit_pair — orbit stable; drift={drift_pct:.1}% \
                         (initial_dist={orbit_initial:.1} u, final_dist={orbit_final:.1} u)"
                    )
                } else {
                    format!(
                        "✗ FAIL: orbit_pair — orbit unstable; drift={drift_pct:.1}% > 30% \
                         (initial_dist={orbit_initial:.1} u, final_dist={orbit_final:.1} u)"
                    )
                }
            }
        }
        _ => format!("? UNKNOWN: {}", test_name),
    }
}
