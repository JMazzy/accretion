use super::TestConfig;
use crate::asteroid::{spawn_asteroid_with_vertices, Vertices};
use bevy::prelude::*;
use bevy_rapier2d::prelude::Velocity;

/// Spawn test scenario: two triangles touching
pub fn spawn_test_two_triangles(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "two_triangles_combine".to_string();
    test_config.frame_limit = 100;

    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    let grey = Color::srgb(0.5, 0.5, 0.5);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(-3.0, 0.0), &vertices, grey, 1);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(3.0, 0.0), &vertices, grey, 1);

    println!("✓ Spawned test: Two triangles touching at edges (centers at ±3)");
}

/// Spawn test scenario: three triangles in a cluster
pub fn spawn_test_three_triangles(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "three_triangles_combine".to_string();
    test_config.frame_limit = 200;

    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    let grey = Color::srgb(0.5, 0.5, 0.5);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(-3.0, -3.0), &vertices, grey, 1);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(3.0, -3.0), &vertices, grey, 1);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(0.0, 3.0), &vertices, grey, 1);

    println!("✓ Spawned test: Three triangles touching in cluster formation");
}

/// Spawn test scenario: gravity test
pub fn spawn_test_gravity(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    test_config.test_name = "gravity_attraction".to_string();
    test_config.frame_limit = 500;

    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

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
    test_config.test_name = "high_speed_collision".to_string();
    test_config.frame_limit = 300;

    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    let grey = Color::srgb(0.5, 0.5, 0.5);
    let e1 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(-30.0, 0.0), &vertices, grey, 1);
    let e2 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(30.0, 0.0), &vertices, grey, 1);

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
    test_config.test_name = "near_miss".to_string();
    test_config.frame_limit = 300;

    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    let grey = Color::srgb(0.5, 0.5, 0.5);
    let e1 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(-40.0, 3.0), &vertices, grey, 1);
    let e2 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(40.0, -3.0), &vertices, grey, 1);

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
    test_config.test_name = "gentle_approach".to_string();
    test_config.frame_limit = 800;

    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

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
    test_config.test_name = "culling_verification".to_string();
    test_config.frame_limit = 30;

    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    let grey = Color::srgb(0.5, 0.5, 0.5);

    spawn_asteroid_with_vertices(&mut commands, Vec2::new(0.0, 0.0), &vertices, grey, 1);

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

    let side_small = 6.0;
    let height_small = side_small * 3.0_f32.sqrt() / 2.0;
    let vertices_small = vec![
        Vec2::new(0.0, height_small / 2.0),
        Vec2::new(-side_small / 2.0, -height_small / 2.0),
        Vec2::new(side_small / 2.0, -height_small / 2.0),
    ];

    let vertices_large = vec![
        Vec2::new(-15.0, -15.0),
        Vec2::new(15.0, -15.0),
        Vec2::new(15.0, 15.0),
        Vec2::new(-15.0, 15.0),
    ];

    let grey_dark = Color::srgb(0.3, 0.3, 0.3);
    let grey_light = Color::srgb(0.7, 0.7, 0.7);

    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(0.0, 0.0),
        &vertices_large,
        grey_dark,
        1,
    );

    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(25.0, 0.0),
        &vertices_small,
        grey_light,
        1,
    );
    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(0.0, 50.0),
        &vertices_small,
        grey_light,
        1,
    );
    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(-100.0, 0.0),
        &vertices_small,
        grey_light,
        1,
    );
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
    test_config.frame_limit = 250;

    let side_small = 6.0;
    let height_small = side_small * 3.0_f32.sqrt() / 2.0;
    let vertices_small = vec![
        Vec2::new(0.0, height_small / 2.0),
        Vec2::new(-side_small / 2.0, -height_small / 2.0),
        Vec2::new(side_small / 2.0, -height_small / 2.0),
    ];

    let vertices_large = vec![
        Vec2::new(-15.0, -15.0),
        Vec2::new(15.0, -15.0),
        Vec2::new(15.0, 15.0),
        Vec2::new(-15.0, 15.0),
    ];

    let grey_dark = Color::srgb(0.3, 0.3, 0.3);
    let grey_light = Color::srgb(0.7, 0.7, 0.7);

    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(-30.0, 0.0),
        &vertices_large,
        grey_dark,
        1,
    );

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
    test_config.test_name = "gravity_boundary".to_string();
    test_config.frame_limit = 300;

    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    let grey = Color::srgb(0.5, 0.5, 0.5);

    spawn_asteroid_with_vertices(&mut commands, Vec2::new(0.0, 0.0), &vertices, grey, 1);

    let e2 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(300.0, 0.0), &vertices, grey, 1);

    commands.entity(e2).insert(Velocity {
        linvel: Vec2::new(0.1, 0.0),
        angvel: 0.0,
    });

    println!("✓ Spawned test: Gravity boundary (at 300u max distance)");
}

/// Spawn test scenario: small asteroid passing by large asteroid
pub fn spawn_test_passing_asteroid(mut commands: Commands, mut test_config: ResMut<TestConfig>) {
    use crate::asteroid::{Asteroid, AsteroidSize, NeighborCount};
    use bevy_rapier2d::prelude::{
        ActiveEvents, Collider, CollisionGroups, ExternalForce, Group, Restitution, RigidBody,
    };

    test_config.test_name = "passing_asteroid".to_string();
    test_config.frame_limit = 500;

    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let small_verts = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

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

    let large_entity =
        spawn_asteroid_with_vertices(&mut commands, Vec2::new(0.0, 0.0), &large_verts, grey, 1);

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
                linvel: Vec2::new(30.0, 0.0),
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
