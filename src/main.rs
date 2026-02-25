use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_rapier2d::prelude::*;
use std::env;

mod asteroid;
mod asteroid_rendering;
mod config;
mod constants;
mod error;
mod graphics;
mod menu;
mod mining;
mod particles;
mod player;
mod rendering;
mod simulation;
mod spatial_partition;
mod testing;

use config::PhysicsConfig;
use menu::{GameState, SelectedScenario};
use testing::{
    spawn_test_all_three, spawn_test_baseline_100, spawn_test_culling_verification,
    spawn_test_gentle_approach, spawn_test_gravity, spawn_test_gravity_boundary,
    spawn_test_high_speed_collision, spawn_test_kdtree_only, spawn_test_large_small_pair,
    spawn_test_mixed_size_asteroids, spawn_test_near_miss, spawn_test_orbit_pair,
    spawn_test_passing_asteroid, spawn_test_perf_benchmark, spawn_test_soft_boundary_only,
    spawn_test_three_triangles, spawn_test_tidal_only, spawn_test_two_triangles, TestConfig,
};

/// Spawn the initial asteroid world for the chosen scenario.
///
/// Registered via `OnTransition{ScenarioSelect→Playing}` so it runs only after
/// the player selects a scenario from the scenario-select screen.
/// Using OnTransition (not OnEnter) prevents re-spawning on Paused↔Playing or
/// GameOver→Playing transitions.
fn spawn_initial_world(
    mut commands: Commands,
    config: Res<PhysicsConfig>,
    scenario: Res<SelectedScenario>,
) {
    match *scenario {
        SelectedScenario::Field => {
            asteroid::spawn_initial_asteroids(&mut commands, 100, &config);
            // Classic field also includes one large planetoid offset from the player.
            asteroid::spawn_planetoid(&mut commands, Vec2::new(700.0, 400.0), &config);
        }
        SelectedScenario::Orbit => {
            asteroid::spawn_orbit_scenario(&mut commands, &config);
        }
        SelectedScenario::Comets => {
            asteroid::spawn_comets_scenario(&mut commands, &config);
        }
        SelectedScenario::Shower => {
            asteroid::spawn_shower_scenario(&mut commands, &config);
        }
    }
}

/// Configure Rapier physics: disable gravity for the space simulation.
fn setup_physics_config(mut config: Query<&mut RapierConfiguration>) {
    for mut cfg in config.iter_mut() {
        cfg.gravity = Vec2::ZERO;
    }
}

fn main() {
    // Check for test mode — bypasses the menu and starts directly in Playing.
    let test_mode = env::var("ACCRETION_TEST").ok();

    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Accretion".into(),
            resolution: WindowResolution::new(1200, 680),
            ..Default::default()
        }),
        ..Default::default()
    }))
    .insert_resource(ClearColor(Color::BLACK))
    // Insert PhysicsConfig with compiled defaults; load_physics_config will
    // overwrite it from assets/physics.toml (if present) in the Startup schedule.
    .insert_resource(PhysicsConfig::default())
    // pixels_per_meter(1.0) keeps world units identical to old physics behaviour
    // (scale = 1.0 was the default in bevy_rapier2d 0.18).  Setting this to any
    // larger value shrinks collider mass in physics-space quadratically and causes
    // ExternalForce to produce runaway acceleration at the same numeric values.
    .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(1.0))
    .insert_resource(player::PlayerFireCooldown::default())
    // Global startup: config + camera + physics settings (needed by both menu and gameplay).
    .add_systems(
        Startup,
        (
            config::load_physics_config,
            graphics::setup_camera.after(config::load_physics_config),
            setup_physics_config,
        ),
    )
    // Game-world setup: runs only on the ScenarioSelect → Playing transition (not on
    // Paused → Playing resume), so world entities and HUD are spawned exactly once per session.
    .add_systems(
        OnTransition {
            exited: GameState::ScenarioSelect,
            entered: GameState::Playing,
        },
        (
            rendering::setup_boundary_ring,
            rendering::setup_hud_score,
            rendering::setup_lives_hud,
            rendering::setup_missile_hud,
            rendering::setup_ore_hud,
            rendering::setup_stats_text,
            rendering::setup_debug_panel,
        ),
    );

    // ── State and simulation ──────────────────────────────────────────────────

    if test_mode.is_some() {
        // Test mode: bypass the menu — start directly in Playing so that all
        // simulation systems (gated on `in_state(GameState::Playing)`) run
        // from the very first frame.
        app.insert_state(GameState::Playing)
            .add_plugins(particles::ParticlesPlugin)
            .add_plugins(simulation::SimulationPlugin)
            .add_plugins(mining::MiningPlugin);
    } else {
        // World and player spawned only when transitioning from ScenarioSelect → Playing.
        // Using OnTransition (not OnEnter) prevents re-spawning on Paused → Playing resume.
        // resume_physics is included here because returning to the menu from a paused game
        // (Paused → MainMenu) leaves the pipeline disabled; it must be re-enabled for the
        // new session to actually simulate.
        app.add_plugins(menu::MainMenuPlugin)
            .add_plugins(particles::ParticlesPlugin)
            .add_plugins(simulation::SimulationPlugin)
            .add_plugins(mining::MiningPlugin)
            .add_systems(
                OnTransition {
                    exited: GameState::ScenarioSelect,
                    entered: GameState::Playing,
                },
                (spawn_initial_world, player::spawn_player, menu::resume_physics),
            )
            // GameOver → Playing: re-spawn the player ship with fresh lives.  Lives are reset
            // by game_over_button_system before this transition fires.
            .add_systems(
                OnTransition {
                    exited: GameState::GameOver,
                    entered: GameState::Playing,
                },
                player::spawn_player,
            )
            .insert_resource(TestConfig::default());
    }

    // ── Test-mode wiring ──────────────────────────────────────────────────────

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
            "orbit_pair" => app.add_systems(
                Startup,
                spawn_test_orbit_pair.after(config::load_physics_config),
            ),
            _ => app.add_systems(
                Startup,
                spawn_test_two_triangles.after(config::load_physics_config),
            ),
        };

        // Test systems must run AFTER asteroid_formation_system in PostUpdate
        // Ensure formations happen before we verify
        app.add_systems(
            PostUpdate,
            (
                testing::test_logging_system,
                testing::orbit_pair_calibrate_and_track_system,
                testing::test_verification_system,
            )
                .chain()
                .after(simulation::asteroid_formation_system),
        );

        println!("Running test: {}", test_name);
    }

    app.run();
}
