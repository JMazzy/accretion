//! Main-menu splash screen — `GameState` definition and `MainMenuPlugin`.
//!
//! ## States
//!
//! | State              | Description                            |
//! |--------------------|----------------------------------------|
//! | `MainMenu`         | Initial state; splash screen shown     |
//! | `Playing`          | Simulation running; all game systems active |
//!
//! ## Systems (registered by `MainMenuPlugin`)
//!
//! | System                  | Schedule                     | Purpose                     |
//! |-------------------------|------------------------------|-----------------------------|
//! | `setup_main_menu`       | `OnEnter(MainMenu)`          | Spawn full-screen menu UI   |
//! | `cleanup_main_menu`     | `OnExit(MainMenu)`           | Despawn menu UI entities    |
//! | `menu_button_system`    | `Update / in MainMenu`       | Handle Start / Quit clicks  |

use bevy::prelude::*;

// ── Game state ────────────────────────────────────────────────────────────────

/// Top-level application state machine.
///
/// Every simulation system in [`crate::simulation::SimulationPlugin`] runs
/// under `.run_if(in_state(GameState::Playing))`, so they are fully inactive
/// while the menu is displayed.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    /// Main-menu splash screen; shown on startup.
    #[default]
    MainMenu,
    /// Active simulation / gameplay.
    Playing,
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
            .add_systems(OnEnter(GameState::MainMenu), setup_main_menu)
            .add_systems(OnExit(GameState::MainMenu), cleanup_main_menu)
            .add_systems(
                Update,
                menu_button_system.run_if(in_state(GameState::MainMenu)),
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
