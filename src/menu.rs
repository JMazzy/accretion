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
    /// Active simulation / gameplay.
    Playing,
    /// Simulation frozen; in-game pause overlay is visible.
    Paused,
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
            // ── Main menu ─────────────────────────────────────────────────────
            .add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
            .add_systems(OnExit(GameState::MainMenu), cleanup_main_menu)
            .add_systems(
                Update,
                menu_button_system.run_if(in_state(GameState::MainMenu)),
            )
            // ── Pause menu ────────────────────────────────────────────────────
            .add_systems(
                OnEnter(GameState::Paused),
                (setup_pause_menu, pause_physics),
            )
            .add_systems(
                OnExit(GameState::Paused),
                (cleanup_pause_menu, resume_physics),
            )
            .add_systems(
                Update,
                (pause_menu_button_system, pause_resume_input_system)
                    .run_if(in_state(GameState::Paused)),
            )
            .add_systems(
                Update,
                toggle_pause_system.run_if(in_state(GameState::Playing)),
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

                    // Quit button
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

// ── Update (Paused only): button interaction ──────────────────────────────────

/// Handle Resume, Debug Overlays, and Quit button presses in the pause menu.
///
/// - **Resume** → transitions back to [`GameState::Playing`].
/// - **Debug Overlays** → opens / closes the floating debug overlay panel.
/// - **Quit** → sends [`AppExit`] to gracefully shut down.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn pause_menu_button_system(
    resume_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<PauseResumeButton>)>,
    debug_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<PauseDebugButton>)>,
    quit_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<MenuQuitButton>)>,
    mut btn_text: Query<&mut TextColor>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<bevy::app::AppExit>,
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
