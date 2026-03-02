use super::*;

fn default_campaign_slot_name(slot: u8) -> String {
    format!("Campaign Slot {slot}")
}

fn set_editor_from_slot(editor: &mut CampaignNameEditor, slot: u8) {
    editor.selected_slot = slot;
    let meta = campaign_slot_metadata(slot);
    editor.buffer = meta
        .name
        .unwrap_or_else(|| default_campaign_slot_name(slot));
}

pub fn setup_campaign_select_menu(
    mut commands: Commands,
    font: Res<GameFont>,
    mut editor: ResMut<CampaignNameEditor>,
) {
    let selected_slot = editor.selected_slot;
    set_editor_from_slot(&mut editor, selected_slot);

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
            CampaignSelectRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("✦ CAMPAIGN SLOTS ✦"),
                TextFont {
                    font: font.0.clone(),
                    font_size: 42.0,
                    ..default()
                },
                TextColor(title_color()),
            ));

            spacer(root, 10.0);

            root.spawn((
                Text::new("Choose a campaign slot, edit name, then start"),
                TextFont {
                    font: font.0.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(subtitle_color()),
            ));

            spacer(root, 24.0);

            for slot in 1..=SAVE_SLOT_COUNT {
                let meta = campaign_slot_metadata(slot);
                let name = meta
                    .name
                    .unwrap_or_else(|| default_campaign_slot_name(slot));
                let mission_label = meta
                    .mission_index
                    .map(|m| format!("mission {m}"))
                    .unwrap_or_else(|| "new run".to_string());
                let availability = if meta.loadable {
                    "resume"
                } else if meta.exists {
                    "not loadable"
                } else {
                    "empty"
                };
                let saved = meta
                    .saved_at_unix
                    .map(|t| format!("saved {t}"))
                    .unwrap_or_else(|| "unsaved".to_string());
                let status = format!(
                    "{} • {} • {} • {}",
                    meta.status, availability, mission_label, saved
                );

                let mut entity = root.spawn((
                    Button,
                    Node {
                        width: Val::Px(420.0),
                        height: Val::Px(78.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(scenario_card_bg()),
                    BorderColor::all(scenario_card_border()),
                ));

                match slot {
                    1 => {
                        entity.insert(CampaignSlot1Button);
                    }
                    2 => {
                        entity.insert(CampaignSlot2Button);
                    }
                    _ => {
                        entity.insert(CampaignSlot3Button);
                    }
                }

                entity.with_children(|btn| {
                    btn.spawn((
                        Text::new(format!("SLOT {} • {}", meta.slot, name)),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 17.0,
                            ..default()
                        },
                        TextColor(scenario_label_color()),
                    ));
                    btn.spawn((
                        Text::new(status),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(scenario_desc_color()),
                    ));
                });

                spacer(root, 10.0);
            }

            spacer(root, 14.0);

            root.spawn((
                Text::new(format!("Selected Slot: {}", editor.selected_slot)),
                TextFont {
                    font: font.0.clone(),
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::srgb(0.80, 0.90, 1.0)),
                CampaignSelectedSlotText,
            ));

            root.spawn((
                Text::new(format!("Name: {}", editor.buffer)),
                TextFont {
                    font: font.0.clone(),
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::srgb(0.86, 0.86, 0.95)),
                CampaignNameValueText,
            ));

            spacer(root, 8.0);

            root.spawn((
                Text::new("Type A-Z / 0-9 / Space / - ; Backspace to edit"),
                TextFont {
                    font: font.0.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(hint_color()),
            ));

            spacer(root, 14.0);

            root.spawn((
                Button,
                Node {
                    width: Val::Px(220.0),
                    height: Val::Px(46.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(pause_debug_bg()),
                BorderColor::all(pause_debug_border()),
                CampaignSaveNameButton,
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("SAVE NAME"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(pause_debug_text()),
                ));
            });

            spacer(root, 10.0);

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
                CampaignStartButton,
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new("START / RESUME"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(start_text()),
                ));
            });

            spacer(root, 10.0);

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
                CampaignBackButton,
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

pub fn cleanup_campaign_select_menu(
    mut commands: Commands,
    query: Query<Entity, With<CampaignSelectRoot>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn append_if_pressed(keys: &ButtonInput<KeyCode>, key: KeyCode, ch: char, out: &mut String) {
    if keys.just_pressed(key) {
        out.push(ch);
    }
}

pub fn campaign_name_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut editor: ResMut<CampaignNameEditor>,
) {
    if keys.just_pressed(KeyCode::Backspace) {
        editor.buffer.pop();
    }

    append_if_pressed(&keys, KeyCode::Space, ' ', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::Minus, '-', &mut editor.buffer);

    append_if_pressed(&keys, KeyCode::Digit0, '0', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::Digit1, '1', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::Digit2, '2', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::Digit3, '3', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::Digit4, '4', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::Digit5, '5', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::Digit6, '6', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::Digit7, '7', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::Digit8, '8', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::Digit9, '9', &mut editor.buffer);

    append_if_pressed(&keys, KeyCode::KeyA, 'A', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyB, 'B', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyC, 'C', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyD, 'D', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyE, 'E', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyF, 'F', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyG, 'G', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyH, 'H', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyI, 'I', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyJ, 'J', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyK, 'K', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyL, 'L', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyM, 'M', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyN, 'N', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyO, 'O', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyP, 'P', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyQ, 'Q', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyR, 'R', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyS, 'S', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyT, 'T', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyU, 'U', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyV, 'V', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyW, 'W', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyX, 'X', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyY, 'Y', &mut editor.buffer);
    append_if_pressed(&keys, KeyCode::KeyZ, 'Z', &mut editor.buffer);

    const MAX_NAME_LEN: usize = 28;
    if editor.buffer.len() > MAX_NAME_LEN {
        editor.buffer.truncate(MAX_NAME_LEN);
    }
}

pub fn campaign_name_display_system(
    editor: Res<CampaignNameEditor>,
    mut slot_text: Query<
        &mut Text,
        (
            With<CampaignSelectedSlotText>,
            Without<CampaignNameValueText>,
        ),
    >,
    mut name_text: Query<
        &mut Text,
        (
            With<CampaignNameValueText>,
            Without<CampaignSelectedSlotText>,
        ),
    >,
) {
    if !editor.is_changed() {
        return;
    }

    for mut text in slot_text.iter_mut() {
        text.0 = format!("Selected Slot: {}", editor.selected_slot);
    }
    for mut text in name_text.iter_mut() {
        text.0 = format!("Name: {}", editor.buffer);
    }
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub fn campaign_select_button_system(
    mut commands: Commands,
    slot1_query: Query<
        (&Interaction, &Children),
        (Changed<Interaction>, With<CampaignSlot1Button>),
    >,
    slot2_query: Query<
        (&Interaction, &Children),
        (Changed<Interaction>, With<CampaignSlot2Button>),
    >,
    slot3_query: Query<
        (&Interaction, &Children),
        (Changed<Interaction>, With<CampaignSlot3Button>),
    >,
    save_name_query: Query<
        (&Interaction, &Children),
        (Changed<Interaction>, With<CampaignSaveNameButton>),
    >,
    start_query: Query<
        (&Interaction, &Children),
        (Changed<Interaction>, With<CampaignStartButton>),
    >,
    back_query: Query<(&Interaction, &Children), (Changed<Interaction>, With<CampaignBackButton>)>,
    mut btn_text: Query<&mut TextColor>,
    mut next_state: ResMut<NextState<GameState>>,
    mut selected_mode: ResMut<SelectedGameMode>,
    mut active_campaign_slot: ResMut<ActiveCampaignSlot>,
    mut editor: ResMut<CampaignNameEditor>,
) {
    let mut handle_slot = |slot: u8, interaction: &Interaction| -> bool {
        if *interaction != Interaction::Pressed {
            return false;
        }
        set_editor_from_slot(&mut editor, slot);
        true
    };

    for (interaction, children) in slot1_query.iter() {
        let _ = handle_slot(1, interaction);
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
                        *color = TextColor(scenario_label_color());
                    }
                }
            }
            Interaction::Pressed => {}
        }
    }

    for (interaction, children) in slot2_query.iter() {
        let _ = handle_slot(2, interaction);
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
                        *color = TextColor(scenario_label_color());
                    }
                }
            }
            Interaction::Pressed => {}
        }
    }

    for (interaction, children) in slot3_query.iter() {
        let _ = handle_slot(3, interaction);
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
                        *color = TextColor(scenario_label_color());
                    }
                }
            }
            Interaction::Pressed => {}
        }
    }

    for (interaction, children) in save_name_query.iter() {
        if *interaction == Interaction::Pressed {
            let name = editor.buffer.trim();
            let chosen_name = if name.is_empty() {
                default_campaign_slot_name(editor.selected_slot)
            } else {
                name.to_string()
            };
            let meta = campaign_slot_metadata(editor.selected_slot);
            let mission = meta.mission_index.unwrap_or(1);
            if let Err(err) =
                save_campaign_slot_named(editor.selected_slot, chosen_name.clone(), mission)
            {
                error!(
                    "Failed saving campaign slot {} name '{}': {}",
                    editor.selected_slot, chosen_name, err
                );
            } else {
                editor.buffer = chosen_name;
            }
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
                        *color = TextColor(pause_debug_text());
                    }
                }
            }
            Interaction::Pressed => {}
        }
    }

    for (interaction, children) in start_query.iter() {
        if *interaction == Interaction::Pressed {
            let slot = editor.selected_slot;
            let typed_name = editor.buffer.trim();
            let chosen_name = if typed_name.is_empty() {
                default_campaign_slot_name(slot)
            } else {
                typed_name.to_string()
            };
            let meta = campaign_slot_metadata(slot);
            let mission = meta.mission_index.unwrap_or(1);

            if let Err(err) = save_campaign_slot_named(slot, chosen_name.clone(), mission) {
                error!(
                    "Failed pre-start campaign save for slot {} name '{}': {}",
                    slot, chosen_name, err
                );
            }

            match ensure_campaign_slot(slot) {
                Ok(mut snapshot) => {
                    snapshot.name = chosen_name.clone();
                    commands.insert_resource(PendingLoadedCampaign(Some(snapshot)));
                    active_campaign_slot.slot = slot;
                    active_campaign_slot.name = chosen_name;
                    *selected_mode = SelectedGameMode::Campaign;
                    next_state.set(GameState::Playing);
                }
                Err(err) => {
                    error!("Failed loading campaign slot {}: {}", slot, err);
                }
            }
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
                        *color = TextColor(start_text());
                    }
                }
            }
            Interaction::Pressed => {}
        }
    }

    for (interaction, children) in back_query.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::MainMenu);
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
                        *color = TextColor(back_text());
                    }
                }
            }
            Interaction::Pressed => {}
        }
    }
}
