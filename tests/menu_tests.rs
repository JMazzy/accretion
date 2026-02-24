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

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use accretion::menu::GameState;

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
