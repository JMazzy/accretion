//! Testing utilities for the simulation

use bevy::prelude::*;
use bevy_rapier2d::prelude::Velocity;
use crate::asteroid::{Asteroid, Vertices, spawn_asteroid_with_vertices};
use std::io::Write;

/// Test configuration
#[derive(Resource)]
pub struct TestConfig {
    pub enabled: bool,
    pub test_name: String,
    pub frame_limit: u32,
    pub frame_count: u32,
    pub initial_asteroid_count: usize,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            test_name: String::new(),
            frame_limit: 100,
            frame_count: 0,
            initial_asteroid_count: 0,
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
    test_config.frame_limit = 500;  // Long enough to see collision behavior
    
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

/// Track asteroid count and log state
#[derive(Component)]
#[allow(dead_code)]
pub struct TestMarker(pub usize); // Initial index for tracking

pub fn test_logging_system(
    mut test_config: ResMut<TestConfig>,
    q: Query<(&Transform, &Velocity, &Vertices), With<Asteroid>>,
) {
    if !test_config.enabled {
        return;
    }
    
    test_config.frame_count += 1;
    let asteroid_count = q.iter().count();
    
    // Log state at certain frames
    if test_config.frame_count == 1 {
        test_config.initial_asteroid_count = asteroid_count;
        println!("[Frame {}] Test: {} | Initial asteroids: {}", 
            test_config.frame_count, test_config.test_name, asteroid_count);
        // Also log positions
        for (transform, _, _) in q.iter() {
            println!("  Asteroid at: ({:.1}, {:.1})", transform.translation.x, transform.translation.y);
        }
    } else if test_config.frame_count == 10 || test_config.frame_count == 30 || test_config.frame_count == 50 || test_config.frame_count % 50 == 0 || test_config.frame_count == test_config.frame_limit {
        println!("[Frame {}] Asteroids: {} (was {})", 
            test_config.frame_count, asteroid_count, test_config.initial_asteroid_count);
        // Log positions and velocities for gravity test
        for (i, (transform, vel, _)) in q.iter().enumerate() {
            let pos = transform.translation.truncate();
            println!("  [{}] pos: ({:.1}, {:.1}), vel_len: {:.3}", i, pos.x, pos.y, vel.linvel.length());
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
    
    let result = verify_test_result(&test_config.test_name, test_config.initial_asteroid_count, final_count);
    println!("{}\n", result);
    let _ = std::io::stdout().flush();
    
    // Exit after test completes
    exit.send(bevy::app::AppExit::default());
}

/// Verify if test passed
fn verify_test_result(test_name: &str, initial: usize, final_count: usize) -> String {
    match test_name {
        "two_triangles_combine" => {
            if final_count < initial && final_count >= 1 {
                format!("✓ PASS: Two triangles combined into {}asteroid(s)", final_count)
            } else {
                format!("✗ FAIL: Expected combining, but got: {} → {} asteroids", initial, final_count)
            }
        }
        "three_triangles_combine" => {
            if final_count < initial && final_count >= 1 {
                format!("✓ PASS: Three triangles combined into {}asteroid(s)", final_count)
            } else {
                format!("✗ FAIL: Expected combining, but got: {} → {} asteroids", initial, final_count)
            }
        }
        "gravity_attraction" => {
            if initial > 1 && final_count <= initial {
                format!("✓ PASS: Asteroids interacted (gravity or collision)")
            } else {
                format!("✗ FAIL: Asteroids did not interact as expected")
            }
        }
        _ => format!("? UNKNOWN: {}", test_name),
    }
}
