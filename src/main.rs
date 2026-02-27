use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy::winit::WinitSettings;
use bevy_rapier2d::prelude::*;
use std::env;

mod alloc_profile;
mod asteroid;
mod asteroid_rendering;
mod config;
mod constants;
mod enemy;
mod error;
mod graphics;
mod menu;
mod mining;
mod particles;
mod player;
mod rendering;
mod save;
mod simulation;
mod spatial_partition;
mod test_mode;
mod testing;

use config::PhysicsConfig;
use menu::{GameState, SelectedScenario};
use testing::TestConfig;

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

fn add_playing_transition_hud_systems(app: &mut App) {
    add_playing_transition_hud_systems_for(app, GameState::ScenarioSelect);
    add_playing_transition_hud_systems_for(app, GameState::LoadGameMenu);
}

fn add_playing_transition_hud_systems_for(app: &mut App, exited: GameState) {
    app.add_systems(
        OnTransition {
            exited,
            entered: GameState::Playing,
        },
        (
            rendering::setup_boundary_ring,
            rendering::setup_debug_line_layers,
            rendering::setup_hud_score,
            rendering::setup_lives_hud,
            rendering::setup_missile_hud,
            rendering::setup_ore_hud,
            rendering::setup_stats_text,
            rendering::setup_physics_inspector_text,
            rendering::setup_profiler_text,
            rendering::setup_debug_panel,
        ),
    );
}

fn main() {
    alloc_profile::init_from_env();

    // Check for test mode — bypasses the menu and starts directly in Playing.
    let test_mode = env::var("ACCRETION_TEST").ok();

    let mut app = App::new();

    if test_mode.is_some() {
        app.insert_resource(WinitSettings::game());
    }

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
    .insert_resource(config::PhysicsConfigHotReloadState::default())
    // Insert GameFont resource early so menu systems can access it; the actual
    // font handle will be loaded during Startup via load_game_font.
    .insert_resource(graphics::GameFont::default())
    // pixels_per_meter(1.0) keeps world units identical to old physics behaviour
    // (scale = 1.0 was the default in bevy_rapier2d 0.18).  Setting this to any
    // larger value shrinks collider mass in physics-space quadratically and causes
    // ExternalForce to produce runaway acceleration at the same numeric values.
    .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(1.0))
    .add_plugins(FrameTimeDiagnosticsPlugin::default())
    .insert_resource(player::PlayerFireCooldown::default())
    .insert_resource(player::PrimaryWeaponLevel::default())
    .insert_resource(player::SecondaryWeaponLevel::default())
    .insert_resource(player::IonCannonLevel::default())
    .add_plugins(save::SavePlugin)
    // Global startup: config + camera + physics settings (needed by both menu and gameplay).
    .add_systems(
        Startup,
        (
            config::load_physics_config,
            config::init_physics_hot_reload_state.after(config::load_physics_config),
            graphics::load_game_font,
            graphics::setup_camera.after(config::load_physics_config),
            setup_physics_config,
        ),
    )
    .add_systems(Update, config::hot_reload_physics_config)
    // Game-world setup: runs only on transitions into Playing so world entities and HUD are
    // spawned exactly once per session.
    ;

    add_playing_transition_hud_systems(&mut app);

    // ── State and simulation ──────────────────────────────────────────────────

    if test_mode.is_some() {
        // Test mode: bypass the menu — start directly in Playing so that all
        // simulation systems (gated on `in_state(GameState::Playing)`) run
        // from the very first frame.
        app.insert_state(GameState::Playing)
            .add_plugins(particles::ParticlesPlugin)
            .add_plugins(simulation::SimulationPlugin)
            .add_plugins(enemy::EnemyPlugin)
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
            .add_plugins(enemy::EnemyPlugin)
            .add_plugins(mining::MiningPlugin)
            .add_systems(
                OnTransition {
                    exited: GameState::ScenarioSelect,
                    entered: GameState::Playing,
                },
                (
                    spawn_initial_world,
                    player::spawn_player,
                    menu::resume_physics,
                ),
            )
            .add_systems(
                OnTransition {
                    exited: GameState::LoadGameMenu,
                    entered: GameState::Playing,
                },
                (
                    save::apply_pending_loaded_snapshot_system,
                    menu::resume_physics,
                ),
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
        test_mode::configure_test_mode(&mut app, &test_name);
    }

    app.run();
}
