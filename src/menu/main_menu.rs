use super::*;

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
fn setup_main_menu(
    mut commands: Commands,
    font: Res<GameFont>,
    symbol_font_2: Res<crate::graphics::SymbolFont2>,
    emoji_font: Res<crate::graphics::EmojiFont>,
) {
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
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|title| {
                title.spawn((
                    Text::new("🌌"),
                    TextFont {
                        font: emoji_font.0.clone(),
                        font_size: 56.0,
                        ..default()
                    },
                    TextColor(title_color()),
                ));
                title.spawn((
                    Text::new("Accretion"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 56.0,
                        ..default()
                    },
                    TextColor(title_color()),
                ));
                title.spawn((
                    Text::new("🌌"),
                    TextFont {
                        font: emoji_font.0.clone(),
                        font_size: 56.0,
                        ..default()
                    },
                    TextColor(title_color()),
                ));
            });

            spacer(root, 10.0);

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
                    Text::new("✦ "),
                    TextFont {
                        font: symbol_font_2.0.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(start_text()),
                ));
                btn.spawn((
                    Text::new("START GAME"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(start_text()),
                ));
                btn.spawn((
                    Text::new(" ✦"),
                    TextFont {
                        font: symbol_font_2.0.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(start_text()),
                ));
            });

            spacer(root, 14.0);

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
                    Text::new("✧ "),
                    TextFont {
                        font: symbol_font_2.0.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(pause_debug_text()),
                ));
                btn.spawn((
                    Text::new("LOAD GAME"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(pause_debug_text()),
                ));
                btn.spawn((
                    Text::new(" ✧"),
                    TextFont {
                        font: symbol_font_2.0.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(pause_debug_text()),
                ));
            });

            spacer(root, 14.0);

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
                    Text::new("✦ "),
                    TextFont {
                        font: symbol_font_2.0.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(quit_text()),
                ));
                btn.spawn((
                    Text::new("QUIT"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(quit_text()),
                ));
                btn.spawn((
                    Text::new(" ✦"),
                    TextFont {
                        font: symbol_font_2.0.clone(),
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(quit_text()),
                ));
            });

            spacer(root, 52.0);

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
pub(super) fn setup_main_menu_when_font_ready(
    commands: Commands,
    font: Res<GameFont>,
    emoji_font: Res<crate::graphics::EmojiFont>,
    symbol_font_2: Res<crate::graphics::SymbolFont2>,
    loaded_fonts: Res<Assets<Font>>,
    existing_menu: Query<Entity, With<MainMenuRoot>>,
) {
    if !existing_menu.is_empty() {
        return;
    }

    if !loaded_fonts.contains(font.0.id())
        || !loaded_fonts.contains(symbol_font_2.0.id())
        || !loaded_fonts.contains(emoji_font.0.id())
    {
        return;
    }

    setup_main_menu(commands, font, symbol_font_2, emoji_font);
}

/// Recursively despawn all main-menu entities.
pub(super) fn cleanup_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenuRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Handle Start Game and Quit button presses.
///
/// - **Start Game** → transitions to [`GameState::ScenarioSelect`].
/// - **Load Game** → transitions to [`GameState::LoadGameMenu`].
/// - **Quit** → sends [`AppExit`] to gracefully shut down.
#[allow(clippy::type_complexity)]
pub(super) fn menu_button_system(
    start_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<MenuStartButton>)>,
    load_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<MenuLoadButton>)>,
    quit_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<MenuQuitButton>)>,
    mut btn_text: Query<&mut TextColor>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<bevy::app::AppExit>,
) {
    for (interaction, children) in start_query.iter() {
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
