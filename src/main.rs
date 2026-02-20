use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_rapier2d::prelude::*;
use std::env;

mod asteroid;
mod constants;
mod error;
mod graphics;
mod player;
mod rendering;
mod simulation;
mod spatial_partition;
mod testing;

use testing::{
    spawn_test_culling_verification, spawn_test_gentle_approach, spawn_test_gravity,
    spawn_test_gravity_boundary, spawn_test_high_speed_collision, spawn_test_large_small_pair,
    spawn_test_mixed_size_asteroids, spawn_test_near_miss, spawn_test_passing_asteroid,
    spawn_test_perf_benchmark, spawn_test_three_triangles, spawn_test_two_triangles, TestConfig,
};

fn spawn_initial_world(mut commands: Commands) {
    asteroid::spawn_initial_asteroids(&mut commands, 200);
}

fn setup_player_startup(commands: Commands) {
    player::spawn_player(commands);
}

/// Configure Rapier physics: disable gravity for the space simulation.
fn setup_physics_config(mut config: Query<&mut RapierConfiguration>) {
    for mut cfg in config.iter_mut() {
        cfg.gravity = Vec2::ZERO;
    }
}

fn main() {
    // Check for test mode
    let test_mode = env::var("GRAV_SIM_TEST").ok();

    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Particle Simulation".into(),
            resolution: WindowResolution::new(1200, 680),
            ..Default::default()
        }),
        ..Default::default()
    }))
    .insert_resource(ClearColor(Color::BLACK))
    // pixels_per_meter(1.0) keeps world units identical to old physics behaviour
    // (scale = 1.0 was the default in bevy_rapier2d 0.18).  Setting this to any
    // larger value shrinks collider mass in physics-space quadratically and causes
    // ExternalForce to produce runaway acceleration at the same numeric values.
    .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(1.0))
    .add_plugins(simulation::SimulationPlugin)
    .insert_resource(player::PlayerFireCooldown::default())
    .add_systems(
        Startup,
        (
            graphics::setup_camera,
            rendering::setup_stats_text.after(graphics::setup_camera),
            setup_physics_config,
        ),
    );

    // Add testing systems if in test mode
    if let Some(test_name) = test_mode {
        let test_config = TestConfig {
            enabled: true,
            ..Default::default()
        };

        app.insert_resource(test_config);

        // Add startup system based on test name
        match test_name.as_str() {
            "two_triangles" => app.add_systems(
                Startup,
                spawn_test_two_triangles.after(graphics::setup_camera),
            ),
            "three_triangles" => app.add_systems(
                Startup,
                spawn_test_three_triangles.after(graphics::setup_camera),
            ),
            "gravity" => app.add_systems(Startup, spawn_test_gravity.after(graphics::setup_camera)),
            "high_speed_collision" => app.add_systems(
                Startup,
                spawn_test_high_speed_collision.after(graphics::setup_camera),
            ),
            "near_miss" => {
                app.add_systems(Startup, spawn_test_near_miss.after(graphics::setup_camera))
            }
            "gentle_approach" => app.add_systems(
                Startup,
                spawn_test_gentle_approach.after(graphics::setup_camera),
            ),
            "culling_verification" => app.add_systems(
                Startup,
                spawn_test_culling_verification.after(graphics::setup_camera),
            ),
            "mixed_size_asteroids" => app.add_systems(
                Startup,
                spawn_test_mixed_size_asteroids.after(graphics::setup_camera),
            ),
            "large_small_pair" => app.add_systems(
                Startup,
                spawn_test_large_small_pair.after(graphics::setup_camera),
            ),
            "gravity_boundary" => app.add_systems(
                Startup,
                spawn_test_gravity_boundary.after(graphics::setup_camera),
            ),
            "passing_asteroid" => app.add_systems(
                Startup,
                spawn_test_passing_asteroid.after(graphics::setup_camera),
            ),
            "perf_benchmark" => app.add_systems(
                Startup,
                spawn_test_perf_benchmark.after(graphics::setup_camera),
            ),
            _ => app.add_systems(
                Startup,
                spawn_test_two_triangles.after(graphics::setup_camera),
            ),
        };

        // Test systems must run AFTER asteroid_formation_system in PostUpdate
        // Ensure formations happen before we verify
        app.add_systems(
            PostUpdate,
            (
                testing::test_logging_system,
                testing::test_verification_system,
            )
                .after(simulation::asteroid_formation_system),
        );

        println!("Running test: {}", test_name);
    } else {
        app.insert_resource(TestConfig::default()).add_systems(
            Startup,
            (
                spawn_initial_world.after(graphics::setup_camera),
                setup_player_startup.after(graphics::setup_camera),
            ),
        );
    }

    app.run();
}
