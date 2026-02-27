use super::TestConfig;
use crate::asteroid::spawn_asteroid_with_vertices;
use bevy::prelude::*;

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

fn spawn_standard_100_grid(commands: &mut Commands) {
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
            spawn_asteroid_with_vertices(commands, Vec2::new(x, y), &vertices, grey, 1);
        }
    }
}
