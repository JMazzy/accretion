use super::*;

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

/// Recursively despawn all pause-menu entities.
pub fn cleanup_pause_menu(mut commands: Commands, pause_query: Query<Entity, With<PauseMenuRoot>>) {
    for entity in pause_query.iter() {
        commands.entity(entity).despawn();
    }
}

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
