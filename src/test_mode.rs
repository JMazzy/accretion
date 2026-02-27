use bevy::prelude::*;

use crate::config;
use crate::menu::GameState;
use crate::player;
use crate::simulation;
use crate::testing::{
    self, spawn_test_all_three, spawn_test_all_three_225_enemy5, spawn_test_baseline_100,
    spawn_test_baseline_225, spawn_test_culling_verification, spawn_test_enemy_combat_scripted,
    spawn_test_gentle_approach, spawn_test_gravity, spawn_test_gravity_boundary,
    spawn_test_high_speed_collision, spawn_test_kdtree_only, spawn_test_large_small_pair,
    spawn_test_mixed_content_225_enemy8, spawn_test_mixed_content_324_enemy12,
    spawn_test_mixed_size_asteroids, spawn_test_near_miss, spawn_test_orbit_pair,
    spawn_test_passing_asteroid, spawn_test_perf_benchmark, spawn_test_soft_boundary_only,
    spawn_test_three_triangles, spawn_test_tidal_only, spawn_test_two_triangles, TestConfig,
};

pub fn configure_test_mode(app: &mut App, test_name: &str) {
    app.insert_resource(TestConfig {
        enabled: true,
        ..Default::default()
    });

    add_test_startup_system(app, test_name);

    app.add_systems(
        Update,
        (
            testing::enemy_combat_script_system,
            testing::mixed_perf_projectile_stimulus_system,
        )
            .run_if(in_state(GameState::Playing)),
    );

    app.add_systems(
        PostUpdate,
        (
            testing::test_logging_system,
            testing::orbit_pair_calibrate_and_track_system,
            testing::enemy_combat_observer_system,
            testing::test_verification_system,
        )
            .chain()
            .after(simulation::asteroid_formation_system),
    );

    println!("Running test: {}", test_name);
}

fn add_test_startup_system(app: &mut App, test_name: &str) {
    match test_name {
        "two_triangles" => app.add_systems(
            Startup,
            spawn_test_two_triangles.after(config::load_physics_config),
        ),
        "three_triangles" => app.add_systems(
            Startup,
            spawn_test_three_triangles.after(config::load_physics_config),
        ),
        "gravity" => app.add_systems(
            Startup,
            spawn_test_gravity.after(config::load_physics_config),
        ),
        "high_speed_collision" => app.add_systems(
            Startup,
            spawn_test_high_speed_collision.after(config::load_physics_config),
        ),
        "near_miss" => app.add_systems(
            Startup,
            spawn_test_near_miss.after(config::load_physics_config),
        ),
        "gentle_approach" => app.add_systems(
            Startup,
            spawn_test_gentle_approach.after(config::load_physics_config),
        ),
        "culling_verification" => app.add_systems(
            Startup,
            spawn_test_culling_verification.after(config::load_physics_config),
        ),
        "mixed_size_asteroids" => app.add_systems(
            Startup,
            spawn_test_mixed_size_asteroids.after(config::load_physics_config),
        ),
        "large_small_pair" => app.add_systems(
            Startup,
            spawn_test_large_small_pair.after(config::load_physics_config),
        ),
        "gravity_boundary" => app.add_systems(
            Startup,
            spawn_test_gravity_boundary.after(config::load_physics_config),
        ),
        "passing_asteroid" => app.add_systems(
            Startup,
            spawn_test_passing_asteroid.after(config::load_physics_config),
        ),
        "perf_benchmark" => app.add_systems(
            Startup,
            spawn_test_perf_benchmark.after(config::load_physics_config),
        ),
        "baseline_100" => app.add_systems(
            Startup,
            spawn_test_baseline_100.after(config::load_physics_config),
        ),
        "tidal_only" => app.add_systems(
            Startup,
            spawn_test_tidal_only.after(config::load_physics_config),
        ),
        "soft_boundary_only" => app.add_systems(
            Startup,
            spawn_test_soft_boundary_only.after(config::load_physics_config),
        ),
        "kdtree_only" => app.add_systems(
            Startup,
            spawn_test_kdtree_only.after(config::load_physics_config),
        ),
        "all_three" => app.add_systems(
            Startup,
            spawn_test_all_three.after(config::load_physics_config),
        ),
        "baseline_225" => app.add_systems(
            Startup,
            spawn_test_baseline_225.after(config::load_physics_config),
        ),
        "all_three_225_enemy5" => app.add_systems(
            Startup,
            (player::spawn_player, spawn_test_all_three_225_enemy5)
                .chain()
                .after(config::load_physics_config),
        ),
        "mixed_content_225_enemy8" => app.add_systems(
            Startup,
            (player::spawn_player, spawn_test_mixed_content_225_enemy8)
                .chain()
                .after(config::load_physics_config),
        ),
        "mixed_content_324_enemy12" => app.add_systems(
            Startup,
            (player::spawn_player, spawn_test_mixed_content_324_enemy12)
                .chain()
                .after(config::load_physics_config),
        ),
        "orbit_pair" => app.add_systems(
            Startup,
            spawn_test_orbit_pair.after(config::load_physics_config),
        ),
        "enemy_combat_scripted" => app.add_systems(
            Startup,
            (player::spawn_player, spawn_test_enemy_combat_scripted)
                .chain()
                .after(config::load_physics_config),
        ),
        _ => app.add_systems(
            Startup,
            spawn_test_two_triangles.after(config::load_physics_config),
        ),
    };
}
