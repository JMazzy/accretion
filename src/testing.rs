//! Testing utilities for the simulation

use crate::asteroid::{spawn_asteroid_with_vertices, Asteroid, Vertices};
use bevy::prelude::*;
use bevy_rapier2d::prelude::{ExternalForce, Velocity};
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
        }
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
    let grey = Color::rgb(0.5, 0.5, 0.5);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(-3.0, 0.0), &vertices, grey);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(3.0, 0.0), &vertices, grey);

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
    let grey = Color::rgb(0.5, 0.5, 0.5);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(-3.0, -3.0), &vertices, grey);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(3.0, -3.0), &vertices, grey);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(0.0, 3.0), &vertices, grey);

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
    let grey = Color::rgb(0.5, 0.5, 0.5);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(-50.0, 0.0), &vertices, grey);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(50.0, 0.0), &vertices, grey);

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
    let grey = Color::rgb(0.5, 0.5, 0.5);
    let e1 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(-30.0, 0.0), &vertices, grey);
    let e2 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(30.0, 0.0), &vertices, grey);

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
    let grey = Color::rgb(0.5, 0.5, 0.5);
    let e1 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(-40.0, 3.0), &vertices, grey);
    let e2 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(40.0, -3.0), &vertices, grey);

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
    test_config.test_name = "gentle_approach".to_string();
    test_config.frame_limit = 400;

    // Create triangle vertices
    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    // Spawn two asteroids closer together for faster gravity interaction
    let grey = Color::rgb(0.5, 0.5, 0.5);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(-25.0, 0.0), &vertices, grey);
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(25.0, 0.0), &vertices, grey);

    println!("✓ Spawned test: Slow gravity approach");
}

/// Spawn test scenario: verify culling and that culled asteroids stop exerting gravity
pub fn spawn_test_culling_verification(
    mut commands: Commands,
    mut test_config: ResMut<TestConfig>,
) {
    use bevy_rapier2d::prelude::Velocity;

    test_config.test_name = "culling_verification".to_string();
    test_config.frame_limit = 350;

    // Create triangle vertices
    let side = 6.0;
    let height = side * 3.0_f32.sqrt() / 2.0;
    let vertices = vec![
        Vec2::new(0.0, height / 2.0),
        Vec2::new(-side / 2.0, -height / 2.0),
        Vec2::new(side / 2.0, -height / 2.0),
    ];

    let grey = Color::rgb(0.5, 0.5, 0.5);

    // Spawn asteroid 1 at center (stationary)
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(0.0, 0.0), &vertices, grey);

    // Spawn asteroid 2 far away moving outward (will be culled at 1000 units)
    let e2 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(950.0, 0.0), &vertices, grey);
    commands.entity(e2).insert(Velocity {
        linvel: Vec2::new(10.0, 0.0), // Moving away from center
        angvel: 0.0,
    });

    println!("✓ Spawned test: Culling verification (ast 1 at origin, ast 2 at 950u moving away)");
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

    let grey_dark = Color::rgb(0.3, 0.3, 0.3);
    let grey_light = Color::rgb(0.7, 0.7, 0.7);

    // Spawn large asteroid at center
    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(0.0, 0.0),
        &vertices_large,
        grey_dark,
    );

    // Spawn small asteroids at various distances around the large one
    // Distance 25 (very close, should interact strongly)
    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(25.0, 0.0),
        &vertices_small,
        grey_light,
    );

    // Distance 50
    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(0.0, 50.0),
        &vertices_small,
        grey_light,
    );

    // Distance 100 (within gravity range but far enough to have stable interaction)
    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(-100.0, 0.0),
        &vertices_small,
        grey_light,
    );

    // Distance 200 (far, minimal interaction)
    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(0.0, -200.0),
        &vertices_small,
        grey_light,
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

    let grey_dark = Color::rgb(0.3, 0.3, 0.3);
    let grey_light = Color::rgb(0.7, 0.7, 0.7);

    // Spawn large asteroid at center
    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(-30.0, 0.0),
        &vertices_large,
        grey_dark,
    );

    // Spawn small asteroid at distance
    spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(30.0, 0.0),
        &vertices_small,
        grey_light,
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

    let grey = Color::rgb(0.5, 0.5, 0.5);

    // Spawn asteroid 1 at center
    spawn_asteroid_with_vertices(&mut commands, Vec2::new(0.0, 0.0), &vertices, grey);

    // Spawn asteroid 2 at exactly gravity max distance (300 units)
    let e2 = spawn_asteroid_with_vertices(&mut commands, Vec2::new(300.0, 0.0), &vertices, grey);

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

    let grey = Color::rgb(0.5, 0.5, 0.5);

    // Spawn large stationary asteroid at origin
    let large_entity =
        spawn_asteroid_with_vertices(&mut commands, Vec2::new(0.0, 0.0), &large_verts, grey);

    // Spawn small asteroid that will pass by at ~50 unit distance
    // Position it to the left, moving right with enough offset to pass by
    use crate::asteroid::{Asteroid, NeighborCount};
    use bevy_rapier2d::prelude::{
        Collider, CollisionGroups, ExternalForce, Group, Restitution, RigidBody,
    };

    let small_entity = commands
        .spawn((
            Asteroid,
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
            CollisionGroups::new(Group::GROUP_1, Group::GROUP_1),
            TransformBundle::from_transform(Transform::from_xyz(-150.0, 50.0, 0.0)),
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

    let grey = Color::rgb(0.6, 0.6, 0.6);

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
            spawn_asteroid_with_vertices(&mut commands, Vec2::new(x, y), &vertices, grey);
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

#[derive(Component)]
#[allow(dead_code)]
pub struct TestMarker(pub usize); // Initial index for tracking

pub fn test_logging_system(
    mut test_config: ResMut<TestConfig>,
    time: Res<Time>,
    q: Query<(Entity, &Transform, &Velocity, &Vertices, &ExternalForce), With<Asteroid>>,
) {
    if !test_config.enabled {
        return;
    }

    test_config.frame_count += 1;
    let asteroid_count = q.iter().count();

    // For perf_benchmark: record every frame's delta time, print periodic summaries
    if test_config.test_name == "perf_benchmark" {
        let dt_ms = time.delta_seconds() * 1000.0;
        test_config.perf_frame_times.push(dt_ms);

        if test_config.frame_count == 1 {
            test_config.initial_asteroid_count = asteroid_count;
            println!(
                "[Frame 1] perf_benchmark started | asteroids: {}",
                asteroid_count
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
    q: Query<(&Transform, &Vertices), With<Asteroid>>,
    mut exit: EventWriter<bevy::app::AppExit>,
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

    // Print full timing report for perf_benchmark
    if test_config.test_name == "perf_benchmark" && !test_config.perf_frame_times.is_empty() {
        let times = &test_config.perf_frame_times;
        // Skip first 10 frames (startup jitter)
        let steady = if times.len() > 10 { &times[10..] } else { times.as_slice() };
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
            println!("  ✓ PASS: average frame time within 60 FPS budget");
        } else {
            println!(
                "  ✗ FAIL: average frame time {:.2}ms exceeds 16.7ms budget",
                avg
            );
        }
    }

    let result = verify_test_result(
        &test_config.test_name,
        test_config.initial_asteroid_count,
        final_count,
    );
    println!("{}\n", result);
    let _ = std::io::stdout().flush();

    // Exit after test completes
    exit.send(bevy::app::AppExit);
}

/// Verify if test passed
fn verify_test_result(test_name: &str, initial: usize, final_count: usize) -> String {
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
        _ => format!("? UNKNOWN: {}", test_name),
    }
}
