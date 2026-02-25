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
//! | `setup_main_menu`         | `OnEnter(MainMenu)`           | Spawn full-screen menu UI          |
//! | `cleanup_main_menu`       | `OnExit(MainMenu)`            | Despawn menu UI entities           |
//! | `menu_button_system`      | `Update / in MainMenu`        | Handle Start / Quit clicks         |
//! | `setup_pause_menu`        | `OnEnter(Paused)`             | Spawn semi-transparent pause overlay|
//! | `cleanup_pause_menu`      | `OnExit(Paused)`              | Despawn pause overlay entities     |
//! | `pause_physics`           | `OnEnter(Paused)`             | Disable Rapier physics pipeline    |
//! | `resume_physics`          | `OnExit(Paused)`              | Re-enable Rapier physics pipeline  |
//! | `pause_menu_button_system`| `Update / in Paused`          | Handle Resume / Debug / Quit clicks|
//! | `toggle_pause_system`     | `Update / in Playing`         | ESC → transition to Paused         |
//! | `pause_resume_input_system`| `Update / in Paused`         | ESC → transition back to Playing   |

use bevy::prelude::*;
use bevy_rapier2d::prelude::RapierConfiguration;

use crate::player::{PlayerLives, PlayerScore};

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
    /// Scenario / save picker shown after clicking Start Game.
    ScenarioSelect,
    /// Active simulation / gameplay.
    Playing,
    /// Simulation frozen; in-game pause overlay is visible.
    Paused,
    /// Player has exhausted all lives; game-over overlay shown.
    GameOver,
}

// ── Scenario selection ────────────────────────────────────────────────────────

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

/// Tags the "Quit" button.
#[derive(Component)]
pub struct MenuQuitButton;

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
            // ── Main menu ─────────────────────────────────────────────────────
            .add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
            .add_systems(OnExit(GameState::MainMenu), cleanup_main_menu)
            .add_systems(
                Update,
                menu_button_system.run_if(in_state(GameState::MainMenu)),
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
            .add_systems(
                OnExit(GameState::Paused),
                cleanup_pause_menu,
            )
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
                (pause_menu_button_system, pause_resume_input_system)
                    .run_if(in_state(GameState::Paused)),
            )
            .add_systems(
                Update,
                toggle_pause_system.run_if(in_state(GameState::Playing)),
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
/// │         ASTEROID SIMULATOR                  │
/// │   A gravitational aggregation simulation    │
/// │                                             │
/// │         [ START GAME ]                      │
/// │            [ QUIT ]                         │
/// │                                             │
/// │          v0.1.0  ·  Bevy 0.17               │
/// └─────────────────────────────────────────────┘
/// ```
pub fn setup_main_menu(mut commands: Commands) {
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
                Text::new("ASTEROID SIMULATOR"),
                TextFont {
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
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(start_text()),
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
                    font_size: 12.0,
                    ..default()
                },
                TextColor(hint_color()),
            ));
        });
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
pub fn setup_scenario_select(mut commands: Commands) {
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
                    font_size: 42.0,
                    ..default()
                },
                TextColor(title_color()),
            ));

            spacer(root, 8.0);

            root.spawn((
                Text::new("Choose a scenario to play"),
                TextFont {
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
pub fn setup_pause_menu(mut commands: Commands) {
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
                        Text::new("— PAUSED —"),
                        TextFont {
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
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(pause_debug_text()),
                        ));
                    });

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
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(quit_text()),
                        ));
                    });

                    pause_spacer(card, 4.0);

                    // Hint text
                    card.spawn((
                        Text::new("Press ESC to resume"),
                        TextFont {
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
pub fn cleanup_pause_menu(mut commands: Commands, query: Query<Entity, With<PauseMenuRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
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
    hud: Query<
        Entity,
        Or<(
            With<crate::rendering::HudScoreDisplay>,
            With<crate::rendering::StatsTextDisplay>,
            With<crate::rendering::DebugPanel>,
            With<crate::rendering::LivesHudDisplay>,
            With<crate::rendering::MissileHudDisplay>,
            With<crate::rendering::BoundaryRing>,
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
    mut rapier_config: Query<&mut RapierConfiguration>,
) {
    for e in asteroids
        .iter()
        .chain(players.iter())
        .chain(projectiles.iter())
        .chain(missiles.iter())
        .chain(particles.iter())
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
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn pause_menu_button_system(
    resume_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<PauseResumeButton>)>,
    debug_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<PauseDebugButton>)>,
    quit_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<PauseMainMenuButton>)>,
    mut btn_text: Query<&mut TextColor>,
    mut next_state: ResMut<NextState<GameState>>,
    mut debug_panel_query: Query<&mut Visibility, With<crate::rendering::DebugPanel>>,
    mut overlay: ResMut<crate::rendering::OverlayState>,
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
                // Toggle the floating debug panel open / closed.
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
pub fn setup_game_over(mut commands: Commands, score: Res<PlayerScore>) {
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
