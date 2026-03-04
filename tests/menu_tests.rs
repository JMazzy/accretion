//! Headless unit tests for the [`GameState`] state machine.
//!
//! These tests use [`MinimalPlugins`] — no window, no rendering, no physics —
//! so they run fast and deterministically in CI.
//!
//! Covered scenarios:
//! 1. Default initial state is `MainMenu`.
//! 2. A `NextState` request transitions from `MainMenu` → `Playing`.
//! 3. `Playing` state persists across frames with no new transition request.
//! 4. `insert_state` can force-start directly in `Playing` (test-mode path).
//! 5. Campaign flow can transition through `CampaignSelect`.

use accretion::campaign::{
    CampaignMissionCatalog, CampaignProgressionState, CampaignSession, CampaignWaveDirector,
    CampaignWavePhase,
};
use accretion::config::PhysicsConfig;
use accretion::enemy::{Enemy, EnemySpawnState};
use accretion::menu::{GameState, SelectedGameMode};
use accretion::mining::{OrePickup, PlayerOre};
use accretion::player;
use accretion::player::ion_cannon::IonCannonShot;
use accretion::player::rendering::{
    AimIndicatorMesh, HealthBarBg, HealthBarFill, PlayerUiEntities,
};
use accretion::player::state::{Missile, MissileAmmo, PlayerLives, PlayerScore, Projectile};
use accretion::simulation::SimulationStats;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a minimal headless app with just the state registered via `init_state`.
///
/// `MinimalPlugins` provides the required scheduling infrastructure.
/// `StatesPlugin` adds the `StateTransition` schedule needed by `init_state`.
/// No window or rendering is created.
fn app_with_default_state() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));
    app.init_state::<GameState>();
    app
}

/// Build a minimal headless app with the state forced into `Playing` from the
/// start (mirrors the test-mode path in `main.rs`).
fn app_with_playing_state() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));
    app.insert_state(GameState::Playing);
    app
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// The default variant of `GameState` is `MainMenu`.
#[test]
fn default_state_is_main_menu() {
    let mut app = app_with_default_state();
    app.update(); // run one frame so StateTransition fires
    let state = app.world().resource::<State<GameState>>();
    assert_eq!(
        *state.get(),
        GameState::MainMenu,
        "initial state must be MainMenu"
    );
}

/// Requesting `Playing` via `NextState` transitions the state on the next
/// `StateTransition` pass (which Bevy runs before each `Update`).
#[test]
fn transition_main_menu_to_playing() {
    let mut app = app_with_default_state();
    app.update(); // settle into MainMenu

    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);

    app.update(); // StateTransition fires; state becomes Playing

    let state = app.world().resource::<State<GameState>>();
    assert_eq!(
        *state.get(),
        GameState::Playing,
        "state must be Playing after explicit transition"
    );
}

/// `Playing` state persists across additional frames — no accidental reversion.
#[test]
fn playing_state_persists_across_frames() {
    let mut app = app_with_default_state();
    app.update();

    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    app.update();

    // Run several more frames without another transition request.
    for _ in 0..5 {
        app.update();
    }

    let state = app.world().resource::<State<GameState>>();
    assert_eq!(
        *state.get(),
        GameState::Playing,
        "Playing must remain stable without a new transition"
    );
}

/// `insert_state` can force the initial state to `Playing` directly,
/// which is the `GRAV_SIM_TEST` code path in `main.rs`.
#[test]
fn insert_state_starts_in_playing() {
    let mut app = app_with_playing_state();
    app.update();

    let state = app.world().resource::<State<GameState>>();
    assert_eq!(
        *state.get(),
        GameState::Playing,
        "insert_state(Playing) must start directly in Playing"
    );
}

/// Requesting `Playing` when already in `Playing` is a no-op — state stays.
#[test]
fn redundant_transition_to_playing_is_stable() {
    let mut app = app_with_playing_state();
    app.update();

    // Request Playing again while already in Playing.
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    app.update();

    let state = app.world().resource::<State<GameState>>();
    assert_eq!(
        *state.get(),
        GameState::Playing,
        "redundant Playing → Playing transition must leave state unchanged"
    );
}

/// Campaign entry uses an intermediate CampaignSelect state before gameplay.
#[test]
fn transition_main_menu_to_campaign_select() {
    let mut app = app_with_default_state();
    app.update();

    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::CampaignSelect);
    app.update();

    let state = app.world().resource::<State<GameState>>();
    assert_eq!(
        *state.get(),
        GameState::CampaignSelect,
        "state must be CampaignSelect after explicit transition"
    );
}

/// Integration coverage for the campaign Play Again path:
///
/// `GameOver -> Playing` should clear stale runtime entities/resources and
/// re-bootstrap campaign mission/wave/progression state for a clean retry.
#[test]
fn campaign_game_over_play_again_resets_runtime_and_progression_state() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));
    app.insert_state(GameState::GameOver);

    app.insert_resource(SelectedGameMode::Campaign);
    app.insert_resource(PhysicsConfig::default());
    app.insert_resource(CampaignMissionCatalog::default());
    app.insert_resource(CampaignSession {
        active: true,
        mission_index: 2,
        ..CampaignSession::default()
    });
    app.insert_resource(CampaignWaveDirector {
        phase: CampaignWavePhase::BossActive,
        current_wave: 3,
        total_waves: 4,
        phase_timer_secs: 0.0,
        target_spawns_this_wave: 8,
        spawned_this_wave: 8,
        max_concurrent_enemies: 3,
        spawn_cooldown_secs: 0.5,
        boss_spawned: true,
        mission_reward_granted: true,
    });
    app.insert_resource(CampaignProgressionState {
        pending_advance: true,
        advance_timer_secs: 0.2,
        mission_failed: true,
        next_mission_pending_shop: Some(3),
    });

    app.insert_resource(PlayerScore {
        hits: 13,
        destroyed: 5,
        streak: 4,
        points: 999,
    });
    app.insert_resource(PlayerLives {
        remaining: 0,
        respawn_timer: Some(1.0),
    });
    app.insert_resource(PlayerOre { count: 77 });
    app.insert_resource(MissileAmmo { count: 1 });
    app.insert_resource(SimulationStats {
        live_count: 42,
        culled_total: 3,
        merged_total: 4,
        split_total: 5,
        destroyed_total: 6,
    });
    app.insert_resource(EnemySpawnState {
        timer_secs: 0.1,
        session_elapsed_secs: 88.0,
        total_spawned: 123,
    });
    app.insert_resource(PlayerUiEntities::default());

    let stale_player = app.world_mut().spawn(player::Player).id();
    let stale_asteroid = app.world_mut().spawn(accretion::asteroid::Asteroid).id();
    let stale_enemy = app.world_mut().spawn(Enemy).id();
    let stale_projectile = app.world_mut().spawn(Projectile::default()).id();
    let stale_missile = app.world_mut().spawn(Missile::default()).id();
    let stale_ion = app
        .world_mut()
        .spawn(IonCannonShot {
            age: 0.0,
            distance_traveled: 0.0,
        })
        .id();
    let stale_particle = app
        .world_mut()
        .spawn(accretion::particles::Particle {
            velocity: Vec2::ZERO,
            age: 0.0,
            lifetime: 1.0,
            r: 1.0,
            g: 1.0,
            b: 1.0,
            material: None,
        })
        .id();
    let stale_ore = app.world_mut().spawn(OrePickup).id();
    let health_bg = app.world_mut().spawn(HealthBarBg).id();
    let health_fill = app
        .world_mut()
        .spawn(HealthBarFill(Handle::<ColorMaterial>::default()))
        .id();
    let aim_indicator = app.world_mut().spawn(AimIndicatorMesh).id();
    {
        let mut ui = app.world_mut().resource_mut::<PlayerUiEntities>();
        ui.health_bar_bg = Some(health_bg);
        ui.health_bar_fill = Some(health_fill);
        ui.aim_indicator = Some(aim_indicator);
    }

    app.add_systems(
        OnTransition {
            exited: GameState::GameOver,
            entered: GameState::Playing,
        },
        (
            accretion::menu::reset_campaign_retry_world,
            accretion::campaign::bootstrap_campaign_session,
            accretion::campaign::bootstrap_campaign_wave_director,
            accretion::campaign::bootstrap_campaign_progression_state,
            player::spawn_player,
        )
            .chain(),
    );

    app.update();
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    app.update();

    let state = app.world().resource::<State<GameState>>();
    assert_eq!(*state.get(), GameState::Playing);

    let score = app.world().resource::<PlayerScore>();
    assert_eq!(score.points, 0);
    assert_eq!(score.hits, 0);

    let lives = app.world().resource::<PlayerLives>();
    assert!(lives.remaining > 0);
    assert_eq!(lives.respawn_timer, None);

    let session = app.world().resource::<CampaignSession>();
    assert!(session.active);
    assert_eq!(session.mission_index, 2);

    let wave = app.world().resource::<CampaignWaveDirector>();
    assert_eq!(wave.phase, CampaignWavePhase::Warmup);
    assert_eq!(wave.current_wave, 1);
    assert_eq!(wave.total_waves, 4);

    let progression = app.world().resource::<CampaignProgressionState>();
    assert!(!progression.pending_advance);
    assert_eq!(progression.advance_timer_secs, 0.0);
    assert!(!progression.mission_failed);
    assert_eq!(progression.next_mission_pending_shop, None);

    let ui = app.world().resource::<PlayerUiEntities>();
    assert!(ui.health_bar_bg.is_none());
    assert!(ui.health_bar_fill.is_none());
    assert!(ui.aim_indicator.is_none());

    assert!(app.world().get_entity(stale_player).is_err());
    assert!(app.world().get_entity(stale_asteroid).is_err());
    assert!(app.world().get_entity(stale_enemy).is_err());
    assert!(app.world().get_entity(stale_projectile).is_err());
    assert!(app.world().get_entity(stale_missile).is_err());
    assert!(app.world().get_entity(stale_ion).is_err());
    assert!(app.world().get_entity(stale_particle).is_err());
    assert!(app.world().get_entity(stale_ore).is_err());
    assert!(app.world().get_entity(health_bg).is_err());
    assert!(app.world().get_entity(health_fill).is_err());
    assert!(app.world().get_entity(aim_indicator).is_err());

    let player_count = {
        let world = app.world_mut();
        world
            .query_filtered::<Entity, With<player::Player>>()
            .iter(world)
            .count()
    };
    assert_eq!(
        player_count, 1,
        "exactly one fresh player should be spawned"
    );
}

/// Final campaign mission should not queue any next-mission progression after
/// `GameOver -> Playing` retry.
#[test]
fn campaign_game_over_play_again_final_mission_keeps_no_next_mission_pending() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));
    app.insert_state(GameState::GameOver);

    app.insert_resource(SelectedGameMode::Campaign);
    app.insert_resource(PhysicsConfig::default());
    app.insert_resource(CampaignMissionCatalog::default());
    app.insert_resource(CampaignSession {
        active: true,
        mission_index: 3,
        ..CampaignSession::default()
    });
    app.insert_resource(CampaignWaveDirector {
        phase: CampaignWavePhase::Complete,
        current_wave: 5,
        total_waves: 5,
        phase_timer_secs: 0.0,
        target_spawns_this_wave: 10,
        spawned_this_wave: 10,
        max_concurrent_enemies: 4,
        spawn_cooldown_secs: 0.3,
        boss_spawned: true,
        mission_reward_granted: true,
    });
    app.insert_resource(CampaignProgressionState {
        pending_advance: true,
        advance_timer_secs: 0.0,
        mission_failed: true,
        next_mission_pending_shop: Some(99),
    });

    app.insert_resource(PlayerScore {
        hits: 3,
        destroyed: 1,
        streak: 2,
        points: 123,
    });
    app.insert_resource(PlayerLives {
        remaining: 0,
        respawn_timer: Some(0.7),
    });
    app.insert_resource(PlayerOre { count: 11 });
    app.insert_resource(MissileAmmo { count: 2 });
    app.insert_resource(SimulationStats {
        live_count: 7,
        culled_total: 1,
        merged_total: 2,
        split_total: 3,
        destroyed_total: 4,
    });
    app.insert_resource(EnemySpawnState {
        timer_secs: 0.1,
        session_elapsed_secs: 10.0,
        total_spawned: 10,
    });
    app.insert_resource(PlayerUiEntities::default());

    app.add_systems(
        OnTransition {
            exited: GameState::GameOver,
            entered: GameState::Playing,
        },
        (
            accretion::menu::reset_campaign_retry_world,
            accretion::campaign::bootstrap_campaign_session,
            accretion::campaign::bootstrap_campaign_wave_director,
            accretion::campaign::bootstrap_campaign_progression_state,
            player::spawn_player,
        )
            .chain(),
    );

    app.update();
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    app.update();

    let state = app.world().resource::<State<GameState>>();
    assert_eq!(*state.get(), GameState::Playing);

    let session = app.world().resource::<CampaignSession>();
    assert!(session.active);
    assert_eq!(session.mission_index, 3);
    assert!(session.next_mission_id.is_none());

    let wave = app.world().resource::<CampaignWaveDirector>();
    assert_eq!(wave.phase, CampaignWavePhase::Warmup);
    assert_eq!(wave.current_wave, 1);
    assert_eq!(wave.total_waves, session.wave_count.max(1));

    let progression = app.world().resource::<CampaignProgressionState>();
    assert!(!progression.pending_advance);
    assert_eq!(progression.advance_timer_secs, 0.0);
    assert!(!progression.mission_failed);
    assert_eq!(progression.next_mission_pending_shop, None);
}

/// Early campaign missions may have a valid next mission id, but retrying from
/// Game Over must still clear any stale queued advancement/intermission state.
#[test]
fn campaign_game_over_play_again_mission_one_clears_stale_pending_advance() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));
    app.insert_state(GameState::GameOver);

    app.insert_resource(SelectedGameMode::Campaign);
    app.insert_resource(PhysicsConfig::default());
    app.insert_resource(CampaignMissionCatalog::default());
    app.insert_resource(CampaignSession {
        active: true,
        mission_index: 1,
        ..CampaignSession::default()
    });
    app.insert_resource(CampaignWaveDirector {
        phase: CampaignWavePhase::Complete,
        current_wave: 3,
        total_waves: 3,
        phase_timer_secs: 0.0,
        target_spawns_this_wave: 6,
        spawned_this_wave: 6,
        max_concurrent_enemies: 2,
        spawn_cooldown_secs: 0.4,
        boss_spawned: true,
        mission_reward_granted: true,
    });
    app.insert_resource(CampaignProgressionState {
        pending_advance: true,
        advance_timer_secs: 0.0,
        mission_failed: true,
        next_mission_pending_shop: Some(2),
    });

    app.insert_resource(PlayerScore {
        hits: 10,
        destroyed: 4,
        streak: 5,
        points: 500,
    });
    app.insert_resource(PlayerLives {
        remaining: 0,
        respawn_timer: Some(1.2),
    });
    app.insert_resource(PlayerOre { count: 9 });
    app.insert_resource(MissileAmmo { count: 1 });
    app.insert_resource(SimulationStats {
        live_count: 9,
        culled_total: 1,
        merged_total: 1,
        split_total: 2,
        destroyed_total: 3,
    });
    app.insert_resource(EnemySpawnState {
        timer_secs: 0.2,
        session_elapsed_secs: 12.0,
        total_spawned: 22,
    });
    app.insert_resource(PlayerUiEntities::default());

    app.add_systems(
        OnTransition {
            exited: GameState::GameOver,
            entered: GameState::Playing,
        },
        (
            accretion::menu::reset_campaign_retry_world,
            accretion::campaign::bootstrap_campaign_session,
            accretion::campaign::bootstrap_campaign_wave_director,
            accretion::campaign::bootstrap_campaign_progression_state,
            player::spawn_player,
        )
            .chain(),
    );

    app.update();
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    app.update();

    let state = app.world().resource::<State<GameState>>();
    assert_eq!(*state.get(), GameState::Playing);

    let session = app.world().resource::<CampaignSession>();
    assert!(session.active);
    assert_eq!(session.mission_index, 1);
    assert_eq!(session.next_mission_id, Some(2));

    let wave = app.world().resource::<CampaignWaveDirector>();
    assert_eq!(wave.phase, CampaignWavePhase::Warmup);
    assert_eq!(wave.current_wave, 1);
    assert_eq!(wave.total_waves, session.wave_count.max(1));

    let progression = app.world().resource::<CampaignProgressionState>();
    assert!(!progression.pending_advance);
    assert_eq!(progression.advance_timer_secs, 0.0);
    assert!(!progression.mission_failed);
    assert_eq!(progression.next_mission_pending_shop, None);
}
