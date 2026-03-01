use super::*;

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
                Text::new("💎 LOAD GAME 💎"),
                TextFont {
                    font: font.0.clone(),
                    font_size: 42.0,
                    ..default()
                },
                TextColor(title_color()),
            ));

            spacer(root, 10.0);

            root.spawn((
                Text::new("Choose a save slot ✦"),
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
                    format!("✦ LOAD SLOT {}", meta.slot)
                } else if meta.exists {
                    format!("✦ SLOT {} ({})", meta.slot, meta.status)
                } else {
                    format!("✦ SLOT {} (EMPTY)", meta.slot)
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
                    Text::new("✦ BACK ✦"),
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
