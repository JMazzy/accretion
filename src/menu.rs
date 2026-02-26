//! Main-menu splash screen and in-game pause menu — `GameState` definition,
//! `MainMenuPlugin`, and pause menu systems.
//!
//! ## States
//!
//! | State              | Description                                          |
//! |--------------------|------------------------------------------------------|
//! | `MainMenu`         | Initial state; splash screen shown                   |
//! | `Playing`          | Simulation running; all game systems active          |
//! | `Paused`           | Simulation frozen; in-game pause overlay is visible  |
//!
//! ## Systems (registered by `MainMenuPlugin`)
//!
//! | System                    | Schedule                      | Purpose                            |
//! |---------------------------|-------------------------------|------------------------------------|
//! | `setup_main_menu_when_font_ready` | `Update / in MainMenu` | Spawn menu after font is loaded    |
//! | `cleanup_main_menu`       | `OnExit(MainMenu)`            | Despawn menu UI entities           |
//! | `menu_button_system`      | `Update / in MainMenu`        | Handle Start / Quit clicks         |
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
    Player, PlayerLives, PlayerScore, PrimaryWeaponLevel, SecondaryWeaponLevel,
};
use crate::save::{
    load_slot, slot_loadable, slot_metadata, PendingLoadedSnapshot, SaveSlotRequest,
    SAVE_SLOT_COUNT,
};

// ── Game state ────────────────────────────────────────────────────────────────

/// Top-level application state machine.
///
/// Every simulation system in [`crate::simulation::SimulationPlugin`] runs
/// under `.run_if(in_state(GameState::Playing))`, so they are fully inactive
/// while the menu is displayed or the game is paused.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    /// Main-menu splash screen; shown on startup.
    #[default]
    MainMenu,
    /// Load game slot picker shown from MainMenu.
    LoadGameMenu,
    /// Scenario / save picker shown after clicking Start Game.
    ScenarioSelect,
    /// Active simulation / gameplay.
    Playing,
    /// Simulation frozen; in-game pause overlay is visible.
    Paused,
    /// Ore shop open; simulation frozen, consumable upgrades available.
    OreShop,
    /// Player has exhausted all lives; game-over overlay shown.
    GameOver,
}

// ── Scenario selection ────────────────────────────────────────────────────────

/// Tracks which state to return to when the ore shop is closed.
///
/// The ore shop can be opened from `Playing` (Tab key) or from `Paused`
/// (button press).  This resource lets the close handler return to the
/// correct state without hard-coding the originating state.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShopReturnState {
    #[default]
    Playing,
    Paused,
}

/// Which scenario the player has chosen to play.
///
/// Written by [`scenario_select_button_system`] and read by the world-spawn
/// system in `main.rs` to determine which asteroid layout to generate.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectedScenario {
    /// 100 asteroids distributed by noise clusters — the classic asteroid field.
    #[default]
    Field,
    /// One very large planetoid at the origin with rings of smaller asteroids
    /// in near-circular orbits around it.  Ring composition now features mixed
    /// polygon sizes for visual variety.
    Orbit,
    /// Twenty large, fast-moving asteroids on crossing trajectories.  They
    /// fragment on impact rather than merging — dodge and destroy them.
    Comets,
    /// 250 unit triangles packed into a dense field with tiny initial velocities.
    /// Gravity pulls them into aggregate clusters very quickly.
    Shower,
}

// ── Component markers ─────────────────────────────────────────────────────────

/// Root node of the main-menu UI; entire tree is despawned on `OnExit(MainMenu)`.
#[derive(Component)]
pub struct MainMenuRoot;

/// Tags the "Start Game" button.
#[derive(Component)]
pub struct MenuStartButton;

/// Tags the "Load Game" button.
#[derive(Component)]
pub struct MenuLoadButton;

/// Tags the "Quit" button.
#[derive(Component)]
pub struct MenuQuitButton;

// ── LoadGameMenu component markers ──────────────────────────────────────────

/// Root node of the load-game screen; despawned on `OnExit(LoadGameMenu)`.
#[derive(Component)]
pub struct LoadGameRoot;

/// Tags the "Load Slot 1" button.
#[derive(Component)]
pub struct LoadSlot1Button;

/// Tags the "Load Slot 2" button.
#[derive(Component)]
pub struct LoadSlot2Button;

/// Tags the "Load Slot 3" button.
#[derive(Component)]
pub struct LoadSlot3Button;

/// Tags the "Back" button on the load-game screen.
#[derive(Component)]
pub struct LoadGameBackButton;

// ── ScenarioSelect component markers ─────────────────────────────────────────

/// Root node of the scenario-select screen; despawned on `OnExit(ScenarioSelect)`.
#[derive(Component)]
pub struct ScenarioSelectRoot;

/// Tags the "Field" scenario button.
#[derive(Component)]
pub struct ScenarioFieldButton;

/// Tags the "Orbit" scenario button.
#[derive(Component)]
pub struct ScenarioOrbitButton;

/// Tags the "Comets" scenario button.
#[derive(Component)]
pub struct ScenarioCometButton;

/// Tags the "Shower" scenario button.
#[derive(Component)]
pub struct ScenarioShowerButton;

/// Tags the "Back" button on the scenario-select screen.
#[derive(Component)]
pub struct ScenarioBackButton;

// ── Pause-menu component markers ──────────────────────────────────────────────

/// Root node of the pause-menu overlay; entire tree is despawned on `OnExit(Paused)`.
#[derive(Component)]
pub struct PauseMenuRoot;

/// Tags the "Resume" button in the pause menu.
#[derive(Component)]
pub struct PauseResumeButton;

/// Tags the "Debug Overlays" toggle button in the pause menu.
#[derive(Component)]
pub struct PauseDebugButton;

/// Tags the "Main Menu" button in the pause menu.
/// Pressing it cleans up the game world and returns to [`GameState::MainMenu`].
#[derive(Component)]
pub struct PauseMainMenuButton;

/// Tags the "SAVE SLOT 1" button in the pause menu.
#[derive(Component)]
pub struct PauseSaveSlot1Button;

/// Tags the "SAVE SLOT 2" button in the pause menu.
#[derive(Component)]
pub struct PauseSaveSlot2Button;

/// Tags the "SAVE SLOT 3" button in the pause menu.
#[derive(Component)]
pub struct PauseSaveSlot3Button;

// ── Ore shop component markers ────────────────────────────────────────────────

/// Root node of the ore shop overlay; despawned when the shop is closed.
#[derive(Component)]
pub struct OreShopRoot;

/// Tags the "BUY HEALTH" button inside the ore shop.
#[derive(Component)]
pub struct OreShopHealButton;

/// Tags the "BUY MISSILE" button inside the ore shop.
#[derive(Component)]
pub struct OreShopMissileButton;

/// Tags the "CLOSE" button inside the ore shop.
#[derive(Component)]
pub struct OreShopCloseButton;

/// Tags the ore count text in the ore shop.
#[derive(Component)]
pub struct OreShopOreText;

/// Tags the health row status text.
#[derive(Component)]
pub struct OreShopHealText;

/// Tags the missile row status text.
#[derive(Component)]
pub struct OreShopMissileText;

/// Tags the "BUY UPGRADE" button inside the unified ore shop.
#[derive(Component)]
pub struct OreShopUpgradeButton;

/// Tags the missile upgrade button in the ore shop.
#[derive(Component)]
pub struct OreShopMissileUpgradeButton;

/// Tags the magnet upgrade button in the ore shop.
#[derive(Component)]
pub struct OreShopMagnetUpgradeButton;

// ── Game-Over component markers ──────────────────────────────────────────────

/// Root node of the game-over overlay; despawned on `OnExit(GameOver)`.
#[derive(Component)]
pub struct GameOverRoot;

/// Tags the "Play Again" button in the game-over overlay.
#[derive(Component)]
pub struct GameOverPlayAgainButton;

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

// ── Colour helpers ────────────────────────────────────────────────────────────

fn start_bg() -> Color {
    Color::srgb(0.08, 0.36, 0.14)
}
fn start_border() -> Color {
    Color::srgb(0.18, 0.72, 0.28)
}
fn start_text() -> Color {
    Color::srgb(0.75, 1.0, 0.80)
}
fn quit_bg() -> Color {
    Color::srgb(0.28, 0.06, 0.06)
}
fn quit_border() -> Color {
    Color::srgb(0.60, 0.12, 0.12)
}
fn quit_text() -> Color {
    Color::srgb(1.0, 0.65, 0.65)
}
fn title_color() -> Color {
    Color::srgb(0.95, 0.88, 0.45)
}
fn subtitle_color() -> Color {
    Color::srgb(0.55, 0.55, 0.65)
}
fn hint_color() -> Color {
    Color::srgb(0.28, 0.28, 0.35)
}

// ── OnEnter(MainMenu): spawn UI ───────────────────────────────────────────────

/// Spawn the full-screen main-menu overlay.
///
/// Layout:
/// ```text
/// ┌─────────────────────────────────────────────┐
/// │         Accretion                           │
/// │   A gravitational aggregation simulation    │
/// │                                             │
/// │         [ START GAME ]                      │
/// │            [ QUIT ]                         │
/// │                                             │
/// │          v0.1.0  ·  Bevy 0.17               │
/// └─────────────────────────────────────────────┘
/// ```
pub fn setup_main_menu(mut commands: Commands, font: Res<GameFont>) {
    // ── Full-screen background ────────────────────────────────────────────────
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            MainMenuRoot,
        ))
        .with_children(|root| {
            // ── Title ─────────────────────────────────────────────────────────
            root.spawn((
                Text::new("Accretion"),
                TextFont {
                    font: font.0.clone(),
                    font_size: 56.0,
                    ..default()
                },
                TextColor(title_color()),
            ));

            spacer(root, 10.0);

            // ── Subtitle ──────────────────────────────────────────────────────
            root.spawn((
                Text::new("A gravitational aggregation simulation"),
                TextFont {
                    font: font.0.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(subtitle_color()),
            ));

            spacer(root, 52.0);

            // ── Start Game button ─────────────────────────────────────────────
            root.spawn((
                Button,
                Node {
                    width: Val::Px(220.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(start_bg()),
                BorderColor::all(start_border()),
                MenuStartButton,
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("START GAME"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(start_text()),
                ));
            });

            spacer(root, 14.0);

            // ── Load Game button ─────────────────────────────────────────────
            root.spawn((
                Button,
                Node {
                    width: Val::Px(220.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(pause_debug_bg()),
                BorderColor::all(pause_debug_border()),
                MenuLoadButton,
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("LOAD GAME"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(pause_debug_text()),
                ));
            });

            spacer(root, 14.0);

            // ── Quit button ───────────────────────────────────────────────────
            root.spawn((
                Button,
                Node {
                    width: Val::Px(220.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(quit_bg()),
                BorderColor::all(quit_border()),
                MenuQuitButton,
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("QUIT"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(quit_text()),
                ));
            });

            spacer(root, 52.0);

            // ── Version footnote ──────────────────────────────────────────────
            root.spawn((
                Text::new("v0.1.0  ·  Bevy 0.17"),
                TextFont {
                    font: font.0.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(hint_color()),
            ));
        });
}

/// Spawn the main menu once the configured font asset is loaded.
///
/// This prevents first-frame fallback text when entering `MainMenu` before
/// the font handle has finished loading.
pub fn setup_main_menu_when_font_ready(
    commands: Commands,
    font: Res<GameFont>,
    loaded_fonts: Res<Assets<Font>>,
    existing_menu: Query<Entity, With<MainMenuRoot>>,
) {
    if !existing_menu.is_empty() {
        return;
    }

    if !loaded_fonts.contains(font.0.id()) {
        return;
    }

    setup_main_menu(commands, font);
}

/// Spawn a fixed-height invisible spacer node.
fn spacer(parent: &mut ChildSpawnerCommands<'_>, px: f32) {
    parent.spawn(Node {
        height: Val::Px(px),
        ..default()
    });
}

// ── OnExit(MainMenu): despawn UI ──────────────────────────────────────────────

/// Recursively despawn all main-menu entities.
pub fn cleanup_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenuRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

// ── Update (MainMenu only): button interaction ────────────────────────────────

/// Handle Start Game and Quit button presses.
///
/// - **Start Game** → transitions to [`GameState::Playing`], which triggers
///   `OnEnter(Playing)` to spawn the world and player.
/// - **Quit** → sends [`AppExit`] to gracefully shut down.
#[allow(clippy::type_complexity)]
pub fn menu_button_system(
    start_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<MenuStartButton>)>,
    load_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<MenuLoadButton>)>,
    quit_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<MenuQuitButton>)>,
    mut btn_text: Query<&mut TextColor>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<bevy::app::AppExit>,
) {
    for (interaction, children) in start_query.iter() {
        // Tint button text on hover; trigger on press
        match interaction {
            Interaction::Pressed => {
                next_state.set(GameState::ScenarioSelect);
            }
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(Color::WHITE);
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(start_text());
                    }
                }
            }
        }
    }

    for (interaction, children) in load_query.iter() {
        match interaction {
            Interaction::Pressed => {
                next_state.set(GameState::LoadGameMenu);
            }
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(Color::WHITE);
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(pause_debug_text());
                    }
                }
            }
        }
    }

    for (interaction, children) in quit_query.iter() {
        match interaction {
            Interaction::Pressed => {
                exit.write(bevy::app::AppExit::Success);
            }
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(Color::WHITE);
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(quit_text());
                    }
                }
            }
        }
    }
}

// ── OnEnter(LoadGameMenu): spawn load-game screen ───────────────────────────

fn format_saved_at(unix_secs: u64) -> String {
    if unix_secs == 0 {
        "saved: unknown".to_string()
    } else {
        format!("saved: unix {unix_secs}")
    }
}

pub fn setup_load_game_menu(mut commands: Commands, font: Res<GameFont>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            LoadGameRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("LOAD GAME"),
                TextFont {
                    font: font.0.clone(),
                    font_size: 42.0,
                    ..default()
                },
                TextColor(title_color()),
            ));

            spacer(root, 10.0);

            root.spawn((
                Text::new("Choose a save slot"),
                TextFont {
                    font: font.0.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(subtitle_color()),
            ));

            spacer(root, 30.0);

            for slot in 1..=SAVE_SLOT_COUNT {
                let meta = slot_metadata(slot);
                let button_bg = if meta.loadable {
                    start_bg()
                } else if meta.exists {
                    Color::srgb(0.22, 0.10, 0.10)
                } else {
                    Color::srgb(0.10, 0.10, 0.10)
                };
                let button_border = if meta.loadable {
                    start_border()
                } else if meta.exists {
                    Color::srgb(0.55, 0.25, 0.25)
                } else {
                    Color::srgb(0.22, 0.22, 0.22)
                };
                let button_text_color = if meta.loadable {
                    start_text()
                } else {
                    Color::srgb(0.45, 0.45, 0.45)
                };
                let label = if meta.loadable {
                    format!("LOAD SLOT {}", meta.slot)
                } else if meta.exists {
                    format!("SLOT {} ({})", meta.slot, meta.status)
                } else {
                    format!("SLOT {} (EMPTY)", meta.slot)
                };
                let details = if let Some(scenario) = meta.scenario {
                    let ts = meta.saved_at_unix.unwrap_or(0);
                    format!("{}  •  {}", scenario.label(), format_saved_at(ts))
                } else if meta.exists {
                    "unreadable save file".to_string()
                } else {
                    "no save data".to_string()
                };

                let mut entity = root.spawn((
                    Button,
                    Node {
                        width: Val::Px(260.0),
                        height: Val::Px(72.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(button_bg),
                    BorderColor::all(button_border),
                ));

                match slot {
                    1 => {
                        entity.insert(LoadSlot1Button);
                    }
                    2 => {
                        entity.insert(LoadSlot2Button);
                    }
                    _ => {
                        entity.insert(LoadSlot3Button);
                    }
                }

                entity.with_children(|btn| {
                    btn.spawn((
                        Text::new(label),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 17.0,
                            ..default()
                        },
                        TextColor(button_text_color),
                    ));
                    btn.spawn((
                        Text::new(details),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.60, 0.66, 0.72)),
                    ));
                });

                spacer(root, 12.0);
            }

            spacer(root, 16.0);

            root.spawn((
                Button,
                Node {
                    width: Val::Px(180.0),
                    height: Val::Px(44.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(back_bg()),
                BorderColor::all(back_border()),
                LoadGameBackButton,
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("BACK"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(back_text()),
                ));
            });
        });
}

pub fn cleanup_load_game_menu(mut commands: Commands, query: Query<Entity, With<LoadGameRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

#[allow(clippy::type_complexity)]
pub fn load_game_menu_button_system(
    mut commands: Commands,
    slot1_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<LoadSlot1Button>)>,
    slot2_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<LoadSlot2Button>)>,
    slot3_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<LoadSlot3Button>)>,
    back_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<LoadGameBackButton>)>,
    mut btn_text: Query<&mut TextColor>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let mut handle_slot = |slot: u8, interaction: &Interaction| -> bool {
        if *interaction != Interaction::Pressed {
            return false;
        }

        match load_slot(slot) {
            Ok(snapshot) => {
                commands.insert_resource(PendingLoadedSnapshot(Some(snapshot)));
                next_state.set(GameState::Playing);
                true
            }
            Err(err) => {
                error!("Failed to load slot {}: {}", slot, err);
                false
            }
        }
    };

    for (interaction, children) in slot1_query.iter() {
        if handle_slot(1, interaction) {
            return;
        }
        match interaction {
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(Color::WHITE);
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(if slot_loadable(1) {
                            start_text()
                        } else {
                            Color::srgb(0.45, 0.45, 0.45)
                        });
                    }
                }
            }
            Interaction::Pressed => {}
        }
    }

    for (interaction, children) in slot2_query.iter() {
        if handle_slot(2, interaction) {
            return;
        }
        match interaction {
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(Color::WHITE);
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(if slot_loadable(2) {
                            start_text()
                        } else {
                            Color::srgb(0.45, 0.45, 0.45)
                        });
                    }
                }
            }
            Interaction::Pressed => {}
        }
    }

    for (interaction, children) in slot3_query.iter() {
        if handle_slot(3, interaction) {
            return;
        }
        match interaction {
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(Color::WHITE);
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(if slot_loadable(3) {
                            start_text()
                        } else {
                            Color::srgb(0.45, 0.45, 0.45)
                        });
                    }
                }
            }
            Interaction::Pressed => {}
        }
    }

    for (interaction, children) in back_query.iter() {
        match interaction {
            Interaction::Pressed => {
                next_state.set(GameState::MainMenu);
            }
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(Color::WHITE);
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(back_text());
                    }
                }
            }
        }
    }
}

// ── OnEnter(ScenarioSelect): spawn scenario-select screen ────────────────────

/// Colour palette for scenario cards.
fn scenario_card_bg() -> Color {
    Color::srgb(0.06, 0.09, 0.18)
}
fn scenario_card_border() -> Color {
    Color::srgb(0.22, 0.38, 0.72)
}
fn scenario_active_text() -> Color {
    Color::srgb(0.80, 0.95, 1.0)
}
fn scenario_label_color() -> Color {
    Color::srgb(0.90, 0.90, 1.0)
}
fn scenario_desc_color() -> Color {
    Color::srgb(0.45, 0.50, 0.65)
}
fn back_bg() -> Color {
    Color::srgb(0.12, 0.12, 0.18)
}
fn back_border() -> Color {
    Color::srgb(0.30, 0.30, 0.46)
}
fn back_text() -> Color {
    Color::srgb(0.55, 0.55, 0.70)
}

/// Spawn the full-screen scenario / save selection UI.
///
/// Layout:
/// ```text
/// ┌───────────────────────────────────────────────┐
/// │         SCENARIOS & SAVES                     │
/// │      Choose a scenario to play                │
/// │                                               │
/// │   ┌─────────────────────────────────────┐     │
/// │   │  FIELD                              │     │
/// │   │  100 asteroids in noise clusters    │     │
/// │   └─────────────────────────────────────┘     │
/// │   ┌─────────────────────────────────────┐     │
/// │   │  ORBIT                              │     │
/// │   │  Planetoid with orbital debris rings│     │
/// │   └─────────────────────────────────────┘     │
/// │                                               │
/// │              [ BACK ]                         │
/// └───────────────────────────────────────────────┘
/// ```
pub fn setup_scenario_select(mut commands: Commands, font: Res<GameFont>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            ScenarioSelectRoot,
        ))
        .with_children(|root| {
            // ── Title ────────────────────────────────────────────────────────
            root.spawn((
                Text::new("SCENARIOS & SAVES"),
                TextFont {
                    font: font.0.clone(),
                    font_size: 42.0,
                    ..default()
                },
                TextColor(title_color()),
            ));

            spacer(root, 8.0);

            root.spawn((
                Text::new("Choose a scenario to play"),
                TextFont {
                    font: font.0.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(subtitle_color()),
            ));

            spacer(root, 36.0);

            // ── FIELD card ───────────────────────────────────────────────────
            root.spawn((
                Button,
                Node {
                    width: Val::Px(460.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    padding: UiRect {
                        left: Val::Px(22.0),
                        right: Val::Px(22.0),
                        top: Val::Px(18.0),
                        bottom: Val::Px(18.0),
                    },
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(scenario_card_bg()),
                BorderColor::all(scenario_card_border()),
                ScenarioFieldButton,
            ))
            .with_children(|card| {
                card.spawn((
                    Text::new("FIELD"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(scenario_label_color()),
                ));
                spacer(card, 6.0);
                card.spawn((
                    Text::new(
                        "100 asteroids distributed across gravity-well clusters.\n\
                         The original chaotic asteroid field.",
                    ),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(scenario_desc_color()),
                ));
            });

            spacer(root, 14.0);

            // ── ORBIT card ───────────────────────────────────────────────────
            root.spawn((
                Button,
                Node {
                    width: Val::Px(460.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    padding: UiRect {
                        left: Val::Px(22.0),
                        right: Val::Px(22.0),
                        top: Val::Px(18.0),
                        bottom: Val::Px(18.0),
                    },
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(scenario_card_bg()),
                BorderColor::all(scenario_card_border()),
                ScenarioOrbitButton,
            ))
            .with_children(|card| {
                card.spawn((
                    Text::new("ORBIT"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(scenario_label_color()),
                ));
                spacer(card, 6.0);
                card.spawn((
                    Text::new(
                        "A massive planetoid at the centre, ringed by debris\n\
                         fields of smaller asteroids in near-circular orbits.",
                    ),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(scenario_desc_color()),
                ));
            });

            spacer(root, 14.0);

            // ── COMETS card ──────────────────────────────────────────────────
            root.spawn((
                Button,
                Node {
                    width: Val::Px(460.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    padding: UiRect {
                        left: Val::Px(22.0),
                        right: Val::Px(22.0),
                        top: Val::Px(18.0),
                        bottom: Val::Px(18.0),
                    },
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(scenario_card_bg()),
                BorderColor::all(scenario_card_border()),
                ScenarioCometButton,
            ))
            .with_children(|card| {
                card.spawn((
                    Text::new("COMETS"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(scenario_label_color()),
                ));
                spacer(card, 6.0);
                card.spawn((
                    Text::new(
                        "Twenty large, fast-moving boulders on crossing trajectories.\n\
                         They fragment on impact — dodge and shoot before they escape.",
                    ),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(scenario_desc_color()),
                ));
            });

            spacer(root, 14.0);

            // ── SHOWER card ──────────────────────────────────────────────────
            root.spawn((
                Button,
                Node {
                    width: Val::Px(460.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    padding: UiRect {
                        left: Val::Px(22.0),
                        right: Val::Px(22.0),
                        top: Val::Px(18.0),
                        bottom: Val::Px(18.0),
                    },
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(scenario_card_bg()),
                BorderColor::all(scenario_card_border()),
                ScenarioShowerButton,
            ))
            .with_children(|card| {
                card.spawn((
                    Text::new("SHOWER"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(scenario_label_color()),
                ));
                spacer(card, 6.0);
                card.spawn((
                    Text::new(
                        "250 unit triangles, near-zero velocity.  Watch gravity\n\
                         pull them into growing clusters in real time.",
                    ),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(scenario_desc_color()),
                ));
            });

            spacer(root, 36.0);

            // ── Back button ──────────────────────────────────────────────────
            root.spawn((
                Button,
                Node {
                    width: Val::Px(160.0),
                    height: Val::Px(44.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(back_bg()),
                BorderColor::all(back_border()),
                ScenarioBackButton,
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("BACK"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(back_text()),
                ));
            });
        });
}

// ── OnExit(ScenarioSelect): despawn screen ────────────────────────────────────

/// Recursively despawn all scenario-select entities.
pub fn cleanup_scenario_select(
    mut commands: Commands,
    query: Query<Entity, With<ScenarioSelectRoot>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

// ── Update (ScenarioSelect): button interaction ───────────────────────────────

/// Handle Field, Orbit, Comets, Shower, and Back button presses on the scenario-select screen.
///
/// - **Field**  → records [`SelectedScenario::Field`]  then transitions to [`GameState::Playing`].
/// - **Orbit**  → records [`SelectedScenario::Orbit`]  then transitions to [`GameState::Playing`].
/// - **Comets** → records [`SelectedScenario::Comets`] then transitions to [`GameState::Playing`].
/// - **Shower** → records [`SelectedScenario::Shower`] then transitions to [`GameState::Playing`].
/// - **Back**   → returns to [`GameState::MainMenu`].
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub fn scenario_select_button_system(
    field_query: Query<
        (&Interaction, &Children),
        (Changed<Interaction>, With<ScenarioFieldButton>),
    >,
    orbit_query: Query<
        (&Interaction, &Children),
        (Changed<Interaction>, With<ScenarioOrbitButton>),
    >,
    comet_query: Query<
        (&Interaction, &Children),
        (Changed<Interaction>, With<ScenarioCometButton>),
    >,
    shower_query: Query<
        (&Interaction, &Children),
        (Changed<Interaction>, With<ScenarioShowerButton>),
    >,
    back_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<ScenarioBackButton>)>,
    mut btn_text: Query<&mut TextColor>,
    mut next_state: ResMut<NextState<GameState>>,
    mut selected: ResMut<SelectedScenario>,
) {
    for (interaction, children) in field_query.iter() {
        match interaction {
            Interaction::Pressed => {
                *selected = SelectedScenario::Field;
                next_state.set(GameState::Playing);
            }
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(scenario_active_text());
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(scenario_label_color());
                    }
                }
            }
        }
    }

    for (interaction, children) in orbit_query.iter() {
        match interaction {
            Interaction::Pressed => {
                *selected = SelectedScenario::Orbit;
                next_state.set(GameState::Playing);
            }
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(scenario_active_text());
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(scenario_label_color());
                    }
                }
            }
        }
    }

    for (interaction, children) in comet_query.iter() {
        match interaction {
            Interaction::Pressed => {
                *selected = SelectedScenario::Comets;
                next_state.set(GameState::Playing);
            }
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(scenario_active_text());
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(scenario_label_color());
                    }
                }
            }
        }
    }

    for (interaction, children) in shower_query.iter() {
        match interaction {
            Interaction::Pressed => {
                *selected = SelectedScenario::Shower;
                next_state.set(GameState::Playing);
            }
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(scenario_active_text());
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(scenario_label_color());
                    }
                }
            }
        }
    }

    for (interaction, children) in back_query.iter() {
        match interaction {
            Interaction::Pressed => {
                next_state.set(GameState::MainMenu);
            }
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(Color::WHITE);
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(back_text());
                    }
                }
            }
        }
    }
}

// ── Pause physics ─────────────────────────────────────────────────────────────

/// Disable the Rapier physics pipeline so asteroids freeze in place while paused.
pub fn pause_physics(mut config: Query<&mut RapierConfiguration>) {
    for mut cfg in config.iter_mut() {
        cfg.physics_pipeline_active = false;
    }
}

/// Re-enable the Rapier physics pipeline when the player resumes.
pub fn resume_physics(mut config: Query<&mut RapierConfiguration>) {
    for mut cfg in config.iter_mut() {
        cfg.physics_pipeline_active = true;
    }
}

// ── Pause toggle input ────────────────────────────────────────────────────────

/// ESC while in `Playing` → transition to `Paused`.
pub fn toggle_pause_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Paused);
    }
}

/// ESC while in `Paused` → transition back to `Playing`.
pub fn pause_resume_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Playing);
    }
}

/// Tab while in `Playing` → open the ore shop (freeze simulation).
pub fn toggle_ore_shop_system(
    keys: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut return_state: ResMut<ShopReturnState>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        *return_state = if *current_state == GameState::Paused {
            ShopReturnState::Paused
        } else {
            ShopReturnState::Playing
        };
        next_state.set(GameState::OreShop);
    }
}

// ── Pause menu colour helpers ─────────────────────────────────────────────────

fn pause_resume_bg() -> Color {
    Color::srgb(0.08, 0.36, 0.14)
}
fn pause_resume_border() -> Color {
    Color::srgb(0.18, 0.72, 0.28)
}
fn pause_resume_text() -> Color {
    Color::srgb(0.75, 1.0, 0.80)
}
fn pause_debug_bg() -> Color {
    Color::srgb(0.10, 0.18, 0.36)
}
fn pause_debug_border() -> Color {
    Color::srgb(0.22, 0.44, 0.78)
}
fn pause_debug_text() -> Color {
    Color::srgb(0.65, 0.80, 1.0)
}
fn shop_buy_bg() -> Color {
    Color::srgb(0.06, 0.22, 0.06)
}
fn shop_buy_border() -> Color {
    Color::srgb(0.18, 0.56, 0.18)
}
fn shop_buy_text() -> Color {
    Color::srgb(0.55, 1.0, 0.55)
}
fn shop_close_bg() -> Color {
    Color::srgb(0.14, 0.14, 0.20)
}
fn shop_close_border() -> Color {
    Color::srgb(0.30, 0.30, 0.46)
}
fn shop_close_text() -> Color {
    Color::srgb(0.65, 0.65, 0.80)
}
fn ore_shop_btn_border() -> Color {
    Color::srgb(0.62, 0.44, 0.12)
}
fn ore_shop_btn_text() -> Color {
    Color::srgb(1.0, 0.80, 0.30)
}
fn ore_shop_item_bg() -> Color {
    Color::srgb(0.08, 0.10, 0.06)
}
fn ore_shop_item_border() -> Color {
    Color::srgb(0.30, 0.40, 0.18)
}
fn ore_shop_item_text() -> Color {
    Color::srgb(0.75, 0.90, 0.55)
}

// ── OnEnter(Paused): spawn pause overlay ─────────────────────────────────────

/// Spawn the in-game pause overlay.
///
/// Layout (appears centred over the frozen game world):
/// ```text
/// ┌─────────────────────────────────────────────┐
/// │ ░░░░░░░░░ semi-transparent overlay ░░░░░░░░ │
/// │ ░░░░░   ┌───────────────────────┐   ░░░░░░ │
/// │ ░░░░░   │      — PAUSED —       │   ░░░░░░ │
/// │ ░░░░░   │    [ RESUME     ]     │   ░░░░░░ │
/// │ ░░░░░   │    [ DEBUG OVL. ]     │   ░░░░░░ │
/// │ ░░░░░   │    [ QUIT       ]     │   ░░░░░░ │
/// │ ░░░░░   │   ESC to resume       │   ░░░░░░ │
/// │ ░░░░░   └───────────────────────┘   ░░░░░░ │
/// └─────────────────────────────────────────────┘
/// ```
pub fn setup_pause_menu(mut commands: Commands, font: Res<GameFont>) {
    // ── Full-screen dim overlay ───────────────────────────────────────────────
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.70)),
            ZIndex(200),
            PauseMenuRoot,
        ))
        .with_children(|overlay| {
            // ── Centred card ──────────────────────────────────────────────────
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(36.0)),
                        row_gap: Val::Px(14.0),
                        border: UiRect::all(Val::Px(2.0)),
                        min_width: Val::Px(280.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.04, 0.04, 0.07)),
                    BorderColor::all(Color::srgb(0.30, 0.30, 0.46)),
                ))
                .with_children(|card| {
                    // Title
                    card.spawn((
                        Text::new("PAUSED"),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 38.0,
                            ..default()
                        },
                        TextColor(title_color()),
                    ));

                    pause_spacer(card, 4.0);

                    // Resume button
                    card.spawn((
                        Button,
                        Node {
                            width: Val::Px(220.0),
                            height: Val::Px(48.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(pause_resume_bg()),
                        BorderColor::all(pause_resume_border()),
                        PauseResumeButton,
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("RESUME"),
                            TextFont {
                                font: font.0.clone(),
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(pause_resume_text()),
                        ));
                    });

                    // Debug overlays button
                    card.spawn((
                        Button,
                        Node {
                            width: Val::Px(220.0),
                            height: Val::Px(48.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(pause_debug_bg()),
                        BorderColor::all(pause_debug_border()),
                        PauseDebugButton,
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("DEBUG OVERLAYS"),
                            TextFont {
                                font: font.0.clone(),
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(pause_debug_text()),
                        ));
                    });

                    // Save slot buttons
                    card.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Button,
                            Node {
                                width: Val::Px(68.0),
                                height: Val::Px(40.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(shop_buy_bg()),
                            BorderColor::all(shop_buy_border()),
                            PauseSaveSlot1Button,
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("SAVE 1"),
                                TextFont {
                                    font: font.0.clone(),
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(shop_buy_text()),
                            ));
                        });

                        row.spawn((
                            Button,
                            Node {
                                width: Val::Px(68.0),
                                height: Val::Px(40.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(shop_buy_bg()),
                            BorderColor::all(shop_buy_border()),
                            PauseSaveSlot2Button,
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("SAVE 2"),
                                TextFont {
                                    font: font.0.clone(),
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(shop_buy_text()),
                            ));
                        });

                        row.spawn((
                            Button,
                            Node {
                                width: Val::Px(68.0),
                                height: Val::Px(40.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(shop_buy_bg()),
                            BorderColor::all(shop_buy_border()),
                            PauseSaveSlot3Button,
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("SAVE 3"),
                                TextFont {
                                    font: font.0.clone(),
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(shop_buy_text()),
                            ));
                        });
                    });

                    // Upgrades / Shop button removed — use Tab to open Ore Shop directly.

                    // Main Menu button
                    card.spawn((
                        Button,
                        Node {
                            width: Val::Px(220.0),
                            height: Val::Px(48.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(quit_bg()),
                        BorderColor::all(quit_border()),
                        PauseMainMenuButton,
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("MAIN MENU"),
                            TextFont {
                                font: font.0.clone(),
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(quit_text()),
                        ));
                    });

                    pause_spacer(card, 4.0);

                    // Hint text
                    card.spawn((
                        Text::new("ESC → resume  ·  Tab → ore shop  ·  SAVE 1/2/3"),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(hint_color()),
                    ));
                });
        });
}

/// Spawn a fixed-height invisible spacer within the pause card.
fn pause_spacer(parent: &mut ChildSpawnerCommands<'_>, px: f32) {
    parent.spawn(Node {
        height: Val::Px(px),
        ..default()
    });
}

// ── OnExit(Paused): despawn pause overlay ────────────────────────────────────

/// Recursively despawn all pause-menu entities.
pub fn cleanup_pause_menu(mut commands: Commands, pause_query: Query<Entity, With<PauseMenuRoot>>) {
    for entity in pause_query.iter() {
        commands.entity(entity).despawn();
    }
}

// ── Ore shop ──────────────────────────────────────────────────────────────────

/// Spawn the ore shop UI overlay.
///
/// Called by [`setup_ore_shop`] (the `OnEnter(OreShop)` system) and by the
/// button system after a purchase so the labels refresh in place.
#[allow(clippy::too_many_arguments)]
fn spawn_ore_shop_overlay(
    commands: &mut Commands,
    ore: u32,
    hp: f32,
    max_hp: f32,
    heal_amount: f32,
    ammo: u32,
    ammo_max: u32,
    weapon_level: &PrimaryWeaponLevel,
    missile_level: &SecondaryWeaponLevel,
    magnet_level: &OreAffinityLevel,
    font: &GameFont,
) {
    let ore_text = format!("Ore available: {ore}");

    let can_heal = ore > 0 && hp < max_hp;
    let heal_btn_bg = if can_heal {
        ore_shop_item_bg()
    } else {
        Color::srgb(0.10, 0.10, 0.10)
    };
    let heal_btn_border = if can_heal {
        ore_shop_item_border()
    } else {
        Color::srgb(0.22, 0.22, 0.22)
    };
    let heal_btn_text_color = if can_heal {
        ore_shop_item_text()
    } else {
        Color::srgb(0.38, 0.38, 0.38)
    };
    let heal_label = format!(
        "HEAL  (HP: {:.0} / {:.0})  -  1 ore -> +{:.0} HP",
        hp, max_hp, heal_amount
    );

    let can_missile = ore > 0 && ammo < ammo_max;
    let missile_btn_bg = if can_missile {
        ore_shop_item_bg()
    } else {
        Color::srgb(0.10, 0.10, 0.10)
    };
    let missile_btn_border = if can_missile {
        ore_shop_item_border()
    } else {
        Color::srgb(0.22, 0.22, 0.22)
    };
    let missile_btn_text_color = if can_missile {
        ore_shop_item_text()
    } else {
        Color::srgb(0.38, 0.38, 0.38)
    };
    let missile_label = format!("MISSILE  ({ammo} / {ammo_max})  -  1 ore -> +1 missile",);

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.70)),
            ZIndex(300),
            OreShopRoot,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(36.0)),
                        row_gap: Val::Px(16.0),
                        border: UiRect::all(Val::Px(2.0)),
                        min_width: Val::Px(400.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.05, 0.05, 0.03)),
                    BorderColor::all(ore_shop_btn_border()),
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new("ORE SHOP"),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 32.0,
                            ..default()
                        },
                        TextColor(ore_shop_btn_text()),
                    ));

                    card.spawn(Node {
                        height: Val::Px(4.0),
                        ..default()
                    });

                    // Ore counter
                    card.spawn((
                        Text::new(ore_text),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.25, 0.95, 0.50)),
                        OreShopOreText,
                    ));

                    card.spawn(Node {
                        height: Val::Px(4.0),
                        ..default()
                    });

                    // ── Consumables row ───────────────────────────────────────
                    card.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|consumables| {
                        consumables
                            .spawn((
                                Button,
                                Node {
                                    width: Val::Px(380.0),
                                    height: Val::Px(52.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(heal_btn_bg),
                                BorderColor::all(heal_btn_border),
                                OreShopHealButton,
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(heal_label),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(heal_btn_text_color),
                                    OreShopHealText,
                                ));
                            });

                        consumables
                            .spawn((
                                Button,
                                Node {
                                    width: Val::Px(380.0),
                                    height: Val::Px(52.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(missile_btn_bg),
                                BorderColor::all(missile_btn_border),
                                OreShopMissileButton,
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(missile_label),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(missile_btn_text_color),
                                    OreShopMissileText,
                                ));
                            });
                    });

                    card.spawn(Node {
                        height: Val::Px(8.0),
                        ..default()
                    });

                    // ── Upgrades row (3 cards) ───────────────────────────────
                    card.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        align_items: AlignItems::FlexStart,
                        ..default()
                    })
                    .with_children(|upgrades_row| {
                        // ── Weapon card ──────────────────────────────────────
                        upgrades_row
                            .spawn((
                                Node {
                                    width: Val::Px(248.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(6.0),
                                    padding: UiRect::all(Val::Px(12.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.09, 0.09, 0.08)),
                                BorderColor::all(Color::srgb(0.22, 0.22, 0.22)),
                            ))
                            .with_children(|card_col| {
                                let can_upgrade =
                                    !weapon_level.is_maxed() && weapon_level.can_afford_next(ore);
                                let upg_btn_bg = if can_upgrade {
                                    shop_buy_bg()
                                } else {
                                    Color::srgb(0.14, 0.14, 0.14)
                                };
                                let upg_btn_border = if can_upgrade {
                                    shop_buy_border()
                                } else {
                                    Color::srgb(0.28, 0.28, 0.28)
                                };
                                let upg_btn_text_color = if can_upgrade {
                                    shop_buy_text()
                                } else {
                                    Color::srgb(0.40, 0.40, 0.40)
                                };
                                let upg_label = if weapon_level.is_maxed() {
                                    "— MAX LEVEL —".to_string()
                                } else {
                                    let cost = weapon_level.cost_for_next_level().unwrap_or(0);
                                    format!("UPGRADE ({cost} ore)")
                                };
                                let cost_status = if weapon_level.is_maxed() {
                                    "MAX LEVEL REACHED".to_string()
                                } else {
                                    let cost = weapon_level.cost_for_next_level().unwrap_or(0);
                                    if can_upgrade {
                                        format!("Cost: {cost} ore")
                                    } else {
                                        format!("Need {cost} ore")
                                    }
                                };
                                let level_text = format!(
                                    "Level {} / {}",
                                    weapon_level.display_level(),
                                    crate::constants::PRIMARY_WEAPON_MAX_LEVEL
                                );
                                let range_text = if weapon_level.is_maxed() {
                                    format!("Destroy size: {}", weapon_level.max_destroy_size())
                                } else {
                                    format!(
                                        "Destroy size: {} -> {}",
                                        weapon_level.max_destroy_size(),
                                        weapon_level.max_destroy_size() + 1
                                    )
                                };

                                card_col.spawn((
                                    Text::new("WEAPON"),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.45, 0.45, 0.45)),
                                ));
                                card_col.spawn((
                                    Text::new(level_text),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.85, 0.85, 0.85)),
                                ));
                                card_col.spawn((
                                    Text::new(range_text),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.55, 0.65, 0.60)),
                                ));
                                card_col.spawn((
                                    Text::new(cost_status),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(if weapon_level.is_maxed() {
                                        Color::srgb(0.90, 0.80, 0.30)
                                    } else if can_upgrade {
                                        Color::srgb(0.75, 0.90, 0.75)
                                    } else {
                                        Color::srgb(0.75, 0.40, 0.40)
                                    }),
                                ));
                                card_col
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Percent(100.0),
                                            height: Val::Px(42.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(2.0)),
                                            ..default()
                                        },
                                        BackgroundColor(upg_btn_bg),
                                        BorderColor::all(upg_btn_border),
                                        OreShopUpgradeButton,
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new(upg_label),
                                            TextFont {
                                                font: font.0.clone(),
                                                font_size: 14.0,
                                                ..default()
                                            },
                                            TextColor(upg_btn_text_color),
                                        ));
                                    });
                            });

                        // ── Missile card ─────────────────────────────────────
                        upgrades_row
                            .spawn((
                                Node {
                                    width: Val::Px(248.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(6.0),
                                    padding: UiRect::all(Val::Px(12.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.09, 0.09, 0.08)),
                                BorderColor::all(Color::srgb(0.22, 0.22, 0.22)),
                            ))
                            .with_children(|card_col| {
                                let can_upgrade =
                                    !missile_level.is_maxed() && missile_level.can_afford_next(ore);
                                let upg_btn_bg = if can_upgrade {
                                    shop_buy_bg()
                                } else {
                                    Color::srgb(0.14, 0.14, 0.14)
                                };
                                let upg_btn_border = if can_upgrade {
                                    shop_buy_border()
                                } else {
                                    Color::srgb(0.28, 0.28, 0.28)
                                };
                                let upg_btn_text_color = if can_upgrade {
                                    shop_buy_text()
                                } else {
                                    Color::srgb(0.40, 0.40, 0.40)
                                };
                                let upg_label = if missile_level.is_maxed() {
                                    "— MAX LEVEL —".to_string()
                                } else {
                                    let cost = missile_level.cost_for_next_level().unwrap_or(0);
                                    format!("UPGRADE ({cost} ore)")
                                };
                                let cost_status = if missile_level.is_maxed() {
                                    "MAX LEVEL REACHED".to_string()
                                } else {
                                    let cost = missile_level.cost_for_next_level().unwrap_or(0);
                                    if can_upgrade {
                                        format!("Cost: {cost} ore")
                                    } else {
                                        format!("Need {cost} ore")
                                    }
                                };
                                let level_text = format!(
                                    "Level {} / {}",
                                    missile_level.display_level(),
                                    crate::constants::SECONDARY_WEAPON_MAX_LEVEL
                                );
                                let range_text = if missile_level.is_maxed() {
                                    format!("Destroy size: {}", missile_level.destroy_threshold())
                                } else {
                                    format!(
                                        "Destroy size: {} -> {}",
                                        missile_level.destroy_threshold(),
                                        missile_level.destroy_threshold() + 1
                                    )
                                };

                                card_col.spawn((
                                    Text::new("MISSILE"),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.45, 0.45, 0.45)),
                                ));
                                card_col.spawn((
                                    Text::new(level_text),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.85, 0.85, 0.85)),
                                ));
                                card_col.spawn((
                                    Text::new(range_text),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.55, 0.65, 0.60)),
                                ));
                                card_col.spawn((
                                    Text::new(cost_status),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(if missile_level.is_maxed() {
                                        Color::srgb(0.90, 0.80, 0.30)
                                    } else if can_upgrade {
                                        Color::srgb(0.75, 0.90, 0.75)
                                    } else {
                                        Color::srgb(0.75, 0.40, 0.40)
                                    }),
                                ));
                                card_col
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Percent(100.0),
                                            height: Val::Px(42.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(2.0)),
                                            ..default()
                                        },
                                        BackgroundColor(upg_btn_bg),
                                        BorderColor::all(upg_btn_border),
                                        OreShopMissileUpgradeButton,
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new(upg_label),
                                            TextFont {
                                                font: font.0.clone(),
                                                font_size: 14.0,
                                                ..default()
                                            },
                                            TextColor(upg_btn_text_color),
                                        ));
                                    });
                            });

                        // ── Magnet card ──────────────────────────────────────
                        upgrades_row
                            .spawn((
                                Node {
                                    width: Val::Px(248.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(6.0),
                                    padding: UiRect::all(Val::Px(12.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.09, 0.09, 0.08)),
                                BorderColor::all(Color::srgb(0.22, 0.22, 0.22)),
                            ))
                            .with_children(|card_col| {
                                let can_upgrade =
                                    !magnet_level.is_maxed() && magnet_level.can_afford_next(ore);
                                let upg_btn_bg = if can_upgrade {
                                    shop_buy_bg()
                                } else {
                                    Color::srgb(0.14, 0.14, 0.14)
                                };
                                let upg_btn_border = if can_upgrade {
                                    shop_buy_border()
                                } else {
                                    Color::srgb(0.28, 0.28, 0.28)
                                };
                                let upg_btn_text_color = if can_upgrade {
                                    shop_buy_text()
                                } else {
                                    Color::srgb(0.40, 0.40, 0.40)
                                };
                                let upg_label = if magnet_level.is_maxed() {
                                    "— MAX LEVEL —".to_string()
                                } else {
                                    let cost = magnet_level.cost_for_next_level().unwrap_or(0);
                                    format!("UPGRADE ({cost} ore)")
                                };
                                let cost_status = if magnet_level.is_maxed() {
                                    "MAX LEVEL REACHED".to_string()
                                } else {
                                    let cost = magnet_level.cost_for_next_level().unwrap_or(0);
                                    if can_upgrade {
                                        format!("Cost: {cost} ore")
                                    } else {
                                        format!("Need {cost} ore")
                                    }
                                };
                                let level_text = format!(
                                    "Level {} / {}",
                                    magnet_level.display_level(),
                                    crate::constants::ORE_AFFINITY_MAX_LEVEL
                                );
                                let range_text = if magnet_level.is_maxed() {
                                    format!("Pull radius: {:.0} px", magnet_level.radius_at_level())
                                } else {
                                    format!(
                                        "Pull radius: {:.0} -> {:.0} px",
                                        magnet_level.radius_at_level(),
                                        magnet_level.radius_at_level() + 50.0
                                    )
                                };

                                card_col.spawn((
                                    Text::new("MAGNET"),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.45, 0.45, 0.45)),
                                ));
                                card_col.spawn((
                                    Text::new(level_text),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.85, 0.85, 0.85)),
                                ));
                                card_col.spawn((
                                    Text::new(range_text),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.55, 0.65, 0.60)),
                                ));
                                card_col.spawn((
                                    Text::new(cost_status),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(if magnet_level.is_maxed() {
                                        Color::srgb(0.90, 0.80, 0.30)
                                    } else if can_upgrade {
                                        Color::srgb(0.75, 0.90, 0.75)
                                    } else {
                                        Color::srgb(0.75, 0.40, 0.40)
                                    }),
                                ));
                                card_col
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Percent(100.0),
                                            height: Val::Px(42.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(2.0)),
                                            ..default()
                                        },
                                        BackgroundColor(upg_btn_bg),
                                        BorderColor::all(upg_btn_border),
                                        OreShopMagnetUpgradeButton,
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new(upg_label),
                                            TextFont {
                                                font: font.0.clone(),
                                                font_size: 14.0,
                                                ..default()
                                            },
                                            TextColor(upg_btn_text_color),
                                        ));
                                    });
                            });
                    });

                    card.spawn(Node {
                        height: Val::Px(4.0),
                        ..default()
                    });

                    // ── Close button ──────────────────────────────────────────
                    card.spawn((
                        Button,
                        Node {
                            width: Val::Px(220.0),
                            height: Val::Px(44.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(shop_close_bg()),
                        BorderColor::all(shop_close_border()),
                        OreShopCloseButton,
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("CLOSE"),
                            TextFont {
                                font: font.0.clone(),
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(shop_close_text()),
                        ));
                    });

                    card.spawn((
                        Text::new("Press Tab or ESC to close"),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(hint_color()),
                    ));
                });
        });
}

/// Spawn the ore shop overlay when entering [`GameState::OreShop`].
#[allow(clippy::too_many_arguments)]
pub fn setup_ore_shop(
    mut commands: Commands,
    ore: Res<PlayerOre>,
    q_health: Query<&PlayerHealth, With<Player>>,
    ammo: Res<MissileAmmo>,
    config: Res<PhysicsConfig>,
    weapon_level: Res<PrimaryWeaponLevel>,
    missile_level: Res<SecondaryWeaponLevel>,
    magnet_level: Res<OreAffinityLevel>,
    font: Res<GameFont>,
) {
    let (hp, max_hp) = q_health
        .single()
        .map(|h| (h.hp, h.max_hp))
        .unwrap_or((config.player_max_hp, config.player_max_hp));
    spawn_ore_shop_overlay(
        &mut commands,
        ore.count,
        hp,
        max_hp,
        config.ore_heal_amount,
        ammo.count,
        config.missile_ammo_max,
        &weapon_level,
        &missile_level,
        &magnet_level,
        &font,
    );
}

/// Despawn the ore shop overlay when exiting [`GameState::OreShop`].
pub fn cleanup_ore_shop(mut commands: Commands, query: Query<Entity, With<OreShopRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Handle button and keyboard interactions in the ore shop.
///
/// - **HEAL** button: spend 1 ore, restore `ore_heal_amount` HP (capped at max).
/// - **MISSILE** button: spend 1 ore, restore 1 missile (capped at `missile_ammo_max`).
/// - **UPGRADE WEAPON** button: spend ore to increase weapon level.
/// - **CLOSE** button / **ESC** / **Tab**: return to the originating state.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn ore_shop_button_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    heal_query: Query<&Interaction, (Changed<Interaction>, With<OreShopHealButton>)>,
    missile_query: Query<&Interaction, (Changed<Interaction>, With<OreShopMissileButton>)>,
    close_query: Query<&Interaction, (Changed<Interaction>, With<OreShopCloseButton>)>,
    upgrade_queries: (
        Query<&Interaction, (Changed<Interaction>, With<OreShopUpgradeButton>)>,
        Query<&Interaction, (Changed<Interaction>, With<OreShopMissileUpgradeButton>)>,
        Query<&Interaction, (Changed<Interaction>, With<OreShopMagnetUpgradeButton>)>,
    ),
    shop_root_query: Query<Entity, With<OreShopRoot>>,
    mut ore: ResMut<PlayerOre>,
    mut q_health: Query<&mut PlayerHealth, With<Player>>,
    mut ammo: ResMut<MissileAmmo>,
    config: Res<PhysicsConfig>,
    levels: (
        ResMut<PrimaryWeaponLevel>,
        ResMut<SecondaryWeaponLevel>,
        ResMut<OreAffinityLevel>,
    ),
    mut next_state: ResMut<NextState<GameState>>,
    return_state: Res<ShopReturnState>,
    font: Res<GameFont>,
) {
    // Destructure tuple parameters
    let (upgrade_query, missile_upgrade_query, magnet_upgrade_query) = upgrade_queries;
    let (mut weapon_level, mut missile_level, mut magnet_level) = levels;
    // ── Close (ESC / Tab / button) ────────────────────────────────────────────
    let wants_close = keys.just_pressed(KeyCode::Escape)
        || keys.just_pressed(KeyCode::Tab)
        || close_query.iter().any(|i| *i == Interaction::Pressed);

    if wants_close {
        let target = match *return_state {
            ShopReturnState::Playing => GameState::Playing,
            ShopReturnState::Paused => GameState::Paused,
        };
        next_state.set(target);
        return;
    }

    // ── Heal ──────────────────────────────────────────────────────────────────
    let heal_pressed = heal_query.iter().any(|i| *i == Interaction::Pressed);
    if heal_pressed && ore.count > 0 {
        if let Ok(mut health) = q_health.single_mut() {
            if health.hp < health.max_hp {
                health.hp = (health.hp + config.ore_heal_amount).min(health.max_hp);
                ore.count -= 1;
                let (hp, max_hp) = (health.hp, health.max_hp);
                let ore_count = ore.count;
                let ammo_count = ammo.count;
                let heal_amount = config.ore_heal_amount;
                let ammo_max = config.missile_ammo_max;
                for entity in shop_root_query.iter() {
                    commands.entity(entity).despawn();
                }
                spawn_ore_shop_overlay(
                    &mut commands,
                    ore_count,
                    hp,
                    max_hp,
                    heal_amount,
                    ammo_count,
                    ammo_max,
                    &weapon_level,
                    &missile_level,
                    &magnet_level,
                    &font,
                );
                return;
            }
        }
    }

    // ── Missile restock ───────────────────────────────────────────────────────
    let missile_pressed = missile_query.iter().any(|i| *i == Interaction::Pressed);
    if missile_pressed && ore.count > 0 && ammo.count < config.missile_ammo_max {
        ammo.count += 1;
        ore.count -= 1;
        let (hp, max_hp) = q_health
            .single()
            .map(|h| (h.hp, h.max_hp))
            .unwrap_or((config.player_max_hp, config.player_max_hp));
        let ore_count = ore.count;
        let ammo_count = ammo.count;
        let heal_amount = config.ore_heal_amount;
        let ammo_max = config.missile_ammo_max;
        for entity in shop_root_query.iter() {
            commands.entity(entity).despawn();
        }
        spawn_ore_shop_overlay(
            &mut commands,
            ore_count,
            hp,
            max_hp,
            heal_amount,
            ammo_count,
            ammo_max,
            &weapon_level,
            &missile_level,
            &magnet_level,
            &font,
        );
        return;
    }

    // ── Weapon upgrade ────────────────────────────────────────────────────────
    let upgrade_pressed = upgrade_query.iter().any(|i| *i == Interaction::Pressed);
    if upgrade_pressed {
        weapon_level.try_upgrade(&mut ore.count);
        let (hp, max_hp) = q_health
            .single()
            .map(|h| (h.hp, h.max_hp))
            .unwrap_or((config.player_max_hp, config.player_max_hp));
        let ore_count = ore.count;
        let ammo_count = ammo.count;
        let heal_amount = config.ore_heal_amount;
        let ammo_max = config.missile_ammo_max;
        for entity in shop_root_query.iter() {
            commands.entity(entity).despawn();
        }
        spawn_ore_shop_overlay(
            &mut commands,
            ore_count,
            hp,
            max_hp,
            heal_amount,
            ammo_count,
            ammo_max,
            &weapon_level,
            &missile_level,
            &magnet_level,
            &font,
        );
    }

    // ── Missile upgrade ───────────────────────────────────────────────────────
    let missile_upgrade_pressed = missile_upgrade_query
        .iter()
        .any(|i| *i == Interaction::Pressed);
    if missile_upgrade_pressed {
        missile_level.try_upgrade(&mut ore.count);
        let (hp, max_hp) = q_health
            .single()
            .map(|h| (h.hp, h.max_hp))
            .unwrap_or((config.player_max_hp, config.player_max_hp));
        let ore_count = ore.count;
        let ammo_count = ammo.count;
        let heal_amount = config.ore_heal_amount;
        let ammo_max = config.missile_ammo_max;
        for entity in shop_root_query.iter() {
            commands.entity(entity).despawn();
        }
        spawn_ore_shop_overlay(
            &mut commands,
            ore_count,
            hp,
            max_hp,
            heal_amount,
            ammo_count,
            ammo_max,
            &weapon_level,
            &missile_level,
            &magnet_level,
            &font,
        );
    }

    // ── Magnet upgrade ────────────────────────────────────────────────────────
    let magnet_upgrade_pressed = magnet_upgrade_query
        .iter()
        .any(|i| *i == Interaction::Pressed);
    if magnet_upgrade_pressed {
        magnet_level.try_upgrade(&mut ore.count);
        let (hp, max_hp) = q_health
            .single()
            .map(|h| (h.hp, h.max_hp))
            .unwrap_or((config.player_max_hp, config.player_max_hp));
        let ore_count = ore.count;
        let ammo_count = ammo.count;
        let heal_amount = config.ore_heal_amount;
        let ammo_max = config.missile_ammo_max;
        for entity in shop_root_query.iter() {
            commands.entity(entity).despawn();
        }
        spawn_ore_shop_overlay(
            &mut commands,
            ore_count,
            hp,
            max_hp,
            heal_amount,
            ammo_count,
            ammo_max,
            &weapon_level,
            &missile_level,
            &magnet_level,
            &font,
        );
    }
}

// ── OnTransition(Paused → MainMenu): despawn entire game world ───────────────

/// Despawn all simulation entities and reset per-session resources so the game
/// is completely clean when the player returns to the main menu.
///
/// Runs on `OnTransition { Paused → MainMenu }` (after `OnExit(Paused)` has
/// already removed the pause overlay).
///
/// The Rapier physics pipeline is explicitly disabled here as a safeguard
/// against parry2d BVH "key not present" panics: `step_simulation` must not
/// run with a live pipeline while entity handles are being flushed from
/// Rapier's internal data structures.  `resume_physics` is called again on
/// the `ScenarioSelect → Playing` transition when a new session begins.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn cleanup_game_world(
    mut commands: Commands,
    asteroids: Query<Entity, With<crate::asteroid::Asteroid>>,
    players: Query<Entity, With<crate::player::Player>>,
    projectiles: Query<Entity, With<crate::player::state::Projectile>>,
    missiles: Query<Entity, With<crate::player::state::Missile>>,
    particles: Query<Entity, With<crate::particles::Particle>>,
    ore_pickups: Query<Entity, With<crate::mining::OrePickup>>,
    hud: Query<
        Entity,
        Or<(
            With<crate::rendering::HudScoreDisplay>,
            With<crate::rendering::StatsTextDisplay>,
            With<crate::rendering::PhysicsInspectorDisplay>,
            With<crate::rendering::ProfilerDisplay>,
            With<crate::rendering::DebugPanel>,
            With<crate::rendering::LivesHudDisplay>,
            With<crate::rendering::MissileHudDisplay>,
            With<crate::rendering::BoundaryRing>,
            With<crate::rendering::OreHudDisplay>,
        )>,
    >,
    player_ui: Query<
        Entity,
        Or<(
            With<crate::player::rendering::HealthBarBg>,
            With<crate::player::rendering::HealthBarFill>,
            With<crate::player::rendering::AimIndicatorMesh>,
        )>,
    >,
    mut player_ui_res: ResMut<crate::player::PlayerUiEntities>,
    mut score: ResMut<PlayerScore>,
    mut lives: ResMut<PlayerLives>,
    mut overlay: ResMut<crate::rendering::OverlayState>,
    mut sim_stats: ResMut<crate::simulation::SimulationStats>,
    mut ore: ResMut<crate::mining::PlayerOre>,
    mut rapier_config: Query<&mut RapierConfiguration>,
) {
    for e in asteroids
        .iter()
        .chain(players.iter())
        .chain(projectiles.iter())
        .chain(missiles.iter())
        .chain(particles.iter())
        .chain(ore_pickups.iter())
        .chain(hud.iter())
        .chain(player_ui.iter())
    {
        commands.entity(e).despawn();
    }
    *player_ui_res = crate::player::PlayerUiEntities::default();
    *score = PlayerScore::default();
    lives.reset();
    *overlay = crate::rendering::OverlayState::default();
    *sim_stats = crate::simulation::SimulationStats::default();
    *ore = crate::mining::PlayerOre::default();
    // Reset upgrades so a new session starts fresh.
    commands.insert_resource(PrimaryWeaponLevel::default());
    commands.insert_resource(SecondaryWeaponLevel::default());
    commands.insert_resource(OreAffinityLevel::default());
    // Keep the physics pipeline disabled until a new session begins.
    // resume_physics is called on OnTransition { ScenarioSelect → Playing }.
    for mut cfg in rapier_config.iter_mut() {
        cfg.physics_pipeline_active = false;
    }
}

// ── Update (Paused only): button interaction ──────────────────────────────────

/// Handle Resume, Debug Overlays, and Main Menu button presses in the pause menu.
///
/// - **Resume** → transitions back to [`GameState::Playing`].
/// - **Debug Overlays** → opens / closes the floating debug overlay panel.
/// - **Main Menu** → cleans up the game world and returns to [`GameState::MainMenu`].
/// - (Ore shop opened via Tab key; see [`toggle_ore_shop_system`].)
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn pause_menu_button_system(
    resume_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<PauseResumeButton>)>,
    debug_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<PauseDebugButton>)>,
    save1_query: Query<
        (&Interaction, &Children),
        (Changed<Interaction>, With<PauseSaveSlot1Button>),
    >,
    save2_query: Query<
        (&Interaction, &Children),
        (Changed<Interaction>, With<PauseSaveSlot2Button>),
    >,
    save3_query: Query<
        (&Interaction, &Children),
        (Changed<Interaction>, With<PauseSaveSlot3Button>),
    >,
    quit_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<PauseMainMenuButton>)>,
    mut btn_text: Query<&mut TextColor>,
    mut next_state: ResMut<NextState<GameState>>,
    mut debug_panel_query: Query<&mut Visibility, With<crate::rendering::DebugPanel>>,
    mut overlay: ResMut<crate::rendering::OverlayState>,
    mut save_writer: MessageWriter<SaveSlotRequest>,
) {
    for (interaction, children) in resume_query.iter() {
        match interaction {
            Interaction::Pressed => {
                next_state.set(GameState::Playing);
            }
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(Color::WHITE);
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(pause_resume_text());
                    }
                }
            }
        }
    }

    for (interaction, children) in debug_query.iter() {
        match interaction {
            Interaction::Pressed => {
                overlay.menu_open = !overlay.menu_open;
                let vis = if overlay.menu_open {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
                for mut v in debug_panel_query.iter_mut() {
                    *v = vis;
                }
            }
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(Color::WHITE);
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(pause_debug_text());
                    }
                }
            }
        }
    }

    for (interaction, children) in save1_query.iter() {
        match interaction {
            Interaction::Pressed => {
                save_writer.write(SaveSlotRequest { slot: 1 });
            }
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(Color::WHITE);
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(shop_buy_text());
                    }
                }
            }
        }
    }

    for (interaction, children) in save2_query.iter() {
        match interaction {
            Interaction::Pressed => {
                save_writer.write(SaveSlotRequest { slot: 2 });
            }
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(Color::WHITE);
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(shop_buy_text());
                    }
                }
            }
        }
    }

    for (interaction, children) in save3_query.iter() {
        match interaction {
            Interaction::Pressed => {
                save_writer.write(SaveSlotRequest { slot: 3 });
            }
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(Color::WHITE);
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(shop_buy_text());
                    }
                }
            }
        }
    }

    for (interaction, children) in quit_query.iter() {
        match interaction {
            Interaction::Pressed => {
                next_state.set(GameState::MainMenu);
            }
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(Color::WHITE);
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(quit_text());
                    }
                }
            }
        }
    }
}

// ── OnEnter(GameOver): spawn game-over overlay ────────────────────────────────

/// Spawn the game-over overlay centred over the frozen world.
///
/// Shows final score and a "PLAY AGAIN" button that re-spawns the player
/// with a fresh set of lives without resetting the asteroid field.
pub fn setup_game_over(mut commands: Commands, score: Res<PlayerScore>, font: Res<GameFont>) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.82)),
            ZIndex(300),
            GameOverRoot,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(40.0)),
                        row_gap: Val::Px(16.0),
                        border: UiRect::all(Val::Px(2.0)),
                        min_width: Val::Px(320.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.06, 0.02, 0.02)),
                    BorderColor::all(Color::srgb(0.55, 0.10, 0.10)),
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new("GAME OVER"),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 46.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.22, 0.22)),
                    ));

                    pause_spacer(card, 4.0);

                    card.spawn((
                        Text::new(format!(
                            "Score: {}   ({} hits · {} destroyed)",
                            score.total(),
                            score.hits,
                            score.destroyed
                        )),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(subtitle_color()),
                    ));

                    pause_spacer(card, 8.0);

                    // Play Again button
                    card.spawn((
                        Button,
                        Node {
                            width: Val::Px(220.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(pause_resume_bg()),
                        BorderColor::all(pause_resume_border()),
                        GameOverPlayAgainButton,
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("PLAY AGAIN"),
                            TextFont {
                                font: font.0.clone(),
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(pause_resume_text()),
                        ));
                    });

                    // Quit button
                    card.spawn((
                        Button,
                        Node {
                            width: Val::Px(220.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(quit_bg()),
                        BorderColor::all(quit_border()),
                        MenuQuitButton,
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("QUIT"),
                            TextFont {
                                font: font.0.clone(),
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(quit_text()),
                        ));
                    });

                    pause_spacer(card, 4.0);

                    card.spawn((
                        Text::new("Press Enter to play again"),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(hint_color()),
                    ));
                });
        });
}

// ── OnExit(GameOver): despawn overlay ────────────────────────────────────────

/// Recursively despawn all game-over overlay entities.
pub fn cleanup_game_over(mut commands: Commands, query: Query<Entity, With<GameOverRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

// ── Update (GameOver): button + keyboard interaction ─────────────────────────

/// Handle Play Again / Quit actions in the game-over overlay.
///
/// - **PLAY AGAIN** (button or Enter): reset [`PlayerLives`] → transition
///   back to `Playing` so `OnTransition{GameOver→Playing}` re-spawns the ship.
/// - **QUIT** (button): sends [`AppExit`].
#[allow(clippy::type_complexity)]
pub fn game_over_button_system(
    play_query: Query<
        (&Interaction, &Children),
        (Changed<Interaction>, With<GameOverPlayAgainButton>),
    >,
    quit_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<MenuQuitButton>)>,
    mut btn_text: Query<&mut TextColor>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<bevy::app::AppExit>,
    keys: Res<ButtonInput<KeyCode>>,
    mut lives: ResMut<PlayerLives>,
) {
    let wants_play_again = keys.just_pressed(KeyCode::Enter)
        || play_query.iter().any(|(i, _)| *i == Interaction::Pressed);

    if wants_play_again {
        lives.reset();
        next_state.set(GameState::Playing);
        return;
    }

    for (interaction, children) in play_query.iter() {
        match interaction {
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(Color::WHITE);
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(pause_resume_text());
                    }
                }
            }
            Interaction::Pressed => {}
        }
    }

    for (interaction, children) in quit_query.iter() {
        match interaction {
            Interaction::Pressed => {
                exit.write(bevy::app::AppExit::Success);
            }
            Interaction::Hovered => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(Color::WHITE);
                    }
                }
            }
            Interaction::None => {
                for child in children.iter() {
                    if let Ok(mut color) = btn_text.get_mut(child) {
                        *color = TextColor(quit_text());
                    }
                }
            }
        }
    }
}
