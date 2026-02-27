//! Menu/state orchestration façade.
//!
//! This module wires `MainMenuPlugin` and re-exports menu state/types while
//! concrete UI/system implementations live under `src/menu/`.
//!
//! ## Registered States
//!
//! | State              | Description                                          |
//! |--------------------|------------------------------------------------------|
//! | `MainMenu`         | Initial state; splash screen shown                    |
//! | `LoadGameMenu`     | Save-slot load screen                                 |
//! | `ScenarioSelect`   | Scenario picker                                       |
//! | `Playing`          | Simulation running; all game systems active           |
//! | `Paused`           | Simulation frozen; in-game pause overlay is visible   |
//! | `OreShop`          | Shop overlay (simulation paused)                      |
//! | `GameOver`         | Game-over overlay                                     |
//!
//! ## Systems Registered by `MainMenuPlugin`
//!
//! | System                    | Schedule                      | Purpose                            |
//! |---------------------------|-------------------------------|------------------------------------|
//! | `setup_main_menu_when_font_ready` | `Update / in MainMenu` | Spawn menu after font is loaded    |
//! | `cleanup_main_menu`       | `OnExit(MainMenu)`            | Despawn menu UI entities           |
//! | `menu_button_system`      | `Update / in MainMenu`        | Handle Start / Load / Quit clicks  |
//! | `setup_pause_menu`        | `OnEnter(Paused)`             | Spawn semi-transparent pause overlay|
//! | `cleanup_pause_menu`      | `OnExit(Paused)`              | Despawn pause overlay entities     |
//! | `pause_physics`           | `OnEnter(Paused)`             | Disable Rapier physics pipeline    |
//! | `resume_physics`          | `OnExit(Paused)`              | Re-enable Rapier physics pipeline  |
//! | `pause_menu_button_system`| `Update / in Paused`          | Handle Resume / Debug / Quit clicks|
//! | `toggle_pause_system`     | `Update / in Playing`         | ESC → transition to Paused         |
//! | `pause_resume_input_system`| `Update / in Paused`         | ESC → transition back to Playing   |
//! | `toggle_ore_shop_system`  | `Update / in Playing`         | Tab → transition to OreShop        |
//! | `ore_shop_button_system`  | `Update / in OreShop`         | Handle ore shop button presses     |

use bevy::prelude::*;
use bevy_rapier2d::prelude::RapierConfiguration;

use crate::config::PhysicsConfig;
use crate::graphics::GameFont;
use crate::mining::{OreAffinityLevel, PlayerOre};
use crate::player::{
    state::{MissileAmmo, PlayerHealth},
    IonCannonLevel, Player, PlayerLives, PlayerScore, PrimaryWeaponLevel, SecondaryWeaponLevel,
    TractorBeamLevel,
};
use crate::save::{
    load_slot, slot_loadable, slot_metadata, PendingLoadedSnapshot, SaveSlotRequest,
    SAVE_SLOT_COUNT,
};

#[path = "menu/types.rs"]
mod menu_types;
pub use menu_types::*;
#[path = "menu/common.rs"]
mod menu_common;
use menu_common::*;
#[path = "menu/main_menu.rs"]
mod menu_main_menu;
use menu_main_menu::{cleanup_main_menu, menu_button_system, setup_main_menu_when_font_ready};
#[path = "menu/game_over.rs"]
mod menu_game_over;
use menu_game_over::{cleanup_game_over, game_over_button_system, setup_game_over};
#[path = "menu/load_game.rs"]
mod menu_load_game;
use menu_load_game::{cleanup_load_game_menu, load_game_menu_button_system, setup_load_game_menu};
#[path = "menu/scenario_select.rs"]
mod menu_scenario_select;
use menu_scenario_select::{
    cleanup_scenario_select, scenario_select_button_system, setup_scenario_select,
};
#[path = "menu/pause.rs"]
mod menu_pause;
use menu_pause::{
    cleanup_pause_menu, pause_menu_button_system, pause_physics, pause_resume_input_system,
    setup_pause_menu, toggle_ore_shop_system, toggle_pause_system,
};
#[path = "menu/ore_shop.rs"]
mod menu_ore_shop;
use menu_ore_shop::{cleanup_ore_shop, ore_shop_button_system, setup_ore_shop};
#[path = "menu/cleanup.rs"]
mod menu_cleanup;
use menu_cleanup::cleanup_game_world;
pub use menu_pause::resume_physics;

// ── Plugin ────────────────────────────────────────────────────────────────────

/// Registers `GameState`, the menu UI setup/teardown, and the button handler.
///
/// This plugin must be added to the app **before** any plugin that calls
/// `.run_if(in_state(GameState::Playing))`, so the state is always registered
/// first.
pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_resource::<SelectedScenario>()
            .init_resource::<ShopReturnState>()
            // ── Main menu ─────────────────────────────────────────────────────
            .add_systems(
                Update,
                setup_main_menu_when_font_ready.run_if(in_state(GameState::MainMenu)),
            )
            .add_systems(OnExit(GameState::MainMenu), cleanup_main_menu)
            .add_systems(
                Update,
                menu_button_system.run_if(in_state(GameState::MainMenu)),
            )
            // ── Load game menu ───────────────────────────────────────────────
            .add_systems(OnEnter(GameState::LoadGameMenu), setup_load_game_menu)
            .add_systems(OnExit(GameState::LoadGameMenu), cleanup_load_game_menu)
            .add_systems(
                Update,
                load_game_menu_button_system.run_if(in_state(GameState::LoadGameMenu)),
            )
            // ── Scenario select ───────────────────────────────────────────────
            .add_systems(OnEnter(GameState::ScenarioSelect), setup_scenario_select)
            .add_systems(OnExit(GameState::ScenarioSelect), cleanup_scenario_select)
            .add_systems(
                Update,
                scenario_select_button_system.run_if(in_state(GameState::ScenarioSelect)),
            )
            // ── Pause menu ────────────────────────────────────────────────────
            .add_systems(
                OnEnter(GameState::Paused),
                (setup_pause_menu, pause_physics),
            )
            .add_systems(OnExit(GameState::Paused), cleanup_pause_menu)
            // resume_physics only on Paused → Playing (not Paused → MainMenu).
            // Keeping the pipeline disabled when returning to the menu prevents
            // parry2d BVH "key not present" panics that occur when step_simulation
            // runs with a re-enabled pipeline before despawned entity handles are
            // flushed from Rapier's internal data structures.
            .add_systems(
                OnTransition {
                    exited: GameState::Paused,
                    entered: GameState::Playing,
                },
                resume_physics,
            )
            .add_systems(
                Update,
                (
                    pause_menu_button_system,
                    pause_resume_input_system,
                    toggle_ore_shop_system,
                )
                    .run_if(in_state(GameState::Paused)),
            )
            .add_systems(
                Update,
                (toggle_pause_system, toggle_ore_shop_system).run_if(in_state(GameState::Playing)),
            )
            // ── Ore shop ──────────────────────────────────────────────────────
            .add_systems(OnEnter(GameState::OreShop), (setup_ore_shop, pause_physics))
            .add_systems(OnExit(GameState::OreShop), cleanup_ore_shop)
            // Resume physics only when returning to Playing, not when returning
            // to Paused (physics were already paused in that case).
            .add_systems(
                OnTransition {
                    exited: GameState::OreShop,
                    entered: GameState::Playing,
                },
                resume_physics,
            )
            .add_systems(
                Update,
                ore_shop_button_system.run_if(in_state(GameState::OreShop)),
            )
            // ── Game Over ─────────────────────────────────────────────────────
            .add_systems(OnEnter(GameState::GameOver), setup_game_over)
            .add_systems(OnExit(GameState::GameOver), cleanup_game_over)
            .add_systems(
                Update,
                game_over_button_system.run_if(in_state(GameState::GameOver)),
            )
            // ── Quit to main menu ─────────────────────────────────────────────
            // Despawn all game-world entities and reset resources so the engine
            // is clean for the next play session.
            .add_systems(
                OnTransition {
                    exited: GameState::Paused,
                    entered: GameState::MainMenu,
                },
                cleanup_game_world,
            );
    }
}
