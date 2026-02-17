use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use std::env;

mod asteroid;
mod graphics;
mod simulation;
mod testing;

use testing::{TestConfig, spawn_test_two_triangles, spawn_test_three_triangles, spawn_test_gravity};

fn main() {
    // Check for test mode
    let test_mode = env::var("GRAV_SIM_TEST").ok();
    
    let mut app = App::new();
    
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Particle Simulation".into(),
            resolution: (1200.0, 680.0).into(),
            ..Default::default()
        }),
        ..Default::default()
    }))
    .insert_resource(ClearColor(Color::BLACK))
    .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(50.0))
    .insert_resource(RapierConfiguration {
        gravity: Vec2::ZERO,
        ..Default::default()
    })
    .add_plugins(simulation::SimulationPlugin)
    .add_systems(Startup, graphics::setup_camera);
    
    // Add testing systems if in test mode
    if let Some(test_name) = test_mode {
        let mut test_config = TestConfig::default();
        test_config.enabled = true;
        
        app.insert_resource(test_config);
        
        // Add startup system based on test name
        match test_name.as_str() {
            "two_triangles" => app.add_systems(Startup, spawn_test_two_triangles.after(graphics::setup_camera)),
            "three_triangles" => app.add_systems(Startup, spawn_test_three_triangles.after(graphics::setup_camera)),
            "gravity" => app.add_systems(Startup, spawn_test_gravity.after(graphics::setup_camera)),
            _ => app.add_systems(Startup, spawn_test_two_triangles.after(graphics::setup_camera)),
        };
        
        // Test systems must run AFTER asteroid_formation_system in PostUpdate
        // Ensure formations happen before we verify
        app.add_systems(
            PostUpdate,
            (
                testing::test_logging_system,
                testing::test_verification_system,
            ).after(simulation::asteroid_formation_system),
        );
        
        println!("Running test: {}", test_name);
    } else {
        app.insert_resource(TestConfig::default());
    }
    
    app.run();
}
