use super::*;

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
fn setup_scenario_select(
    mut commands: Commands,
    font: Res<GameFont>,
    unicode_font: Res<crate::graphics::UnicodeFallbackFont>,
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
            ScenarioSelectRoot,
        ))
        .with_children(|root| {
            // ── Title ────────────────────────────────────────────────────────
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|title| {
                title.spawn((
                    Text::new("✦ "),
                    TextFont {
                        font: unicode_font.0.clone(),
                        font_size: 42.0,
                        ..default()
                    },
                    TextColor(title_color()),
                ));
                title.spawn((
                    Text::new("SCENARIOS & SAVES"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 42.0,
                        ..default()
                    },
                    TextColor(title_color()),
                ));
                title.spawn((
                    Text::new(" ✧"),
                    TextFont {
                        font: unicode_font.0.clone(),
                        font_size: 42.0,
                        ..default()
                    },
                    TextColor(title_color()),
                ));
            });

            spacer(root, 8.0);

            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|subtitle| {
                subtitle.spawn((
                    Text::new("Choose a scenario to play "),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(subtitle_color()),
                ));
                subtitle.spawn((
                    Text::new("✦"),
                    TextFont {
                        font: unicode_font.0.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(subtitle_color()),
                ));
            });

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
                card.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|label| {
                    label.spawn((
                        Text::new("🪨 "),
                        TextFont {
                            font: emoji_font.0.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(scenario_label_color()),
                    ));
                    label.spawn((
                        Text::new("FIELD"),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(scenario_label_color()),
                    ));
                    label.spawn((
                        Text::new(" 🪨"),
                        TextFont {
                            font: emoji_font.0.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(scenario_label_color()),
                    ));
                });
                spacer(card, 6.0);
                card.spawn((
                    Text::new(
                        "Asteroid-only clustered field with seeded variation.\n\
                         Multiple patchy pockets and varied starts each run.",
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
                card.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|label| {
                    label.spawn((
                        Text::new("🪐 "),
                        TextFont {
                            font: emoji_font.0.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(scenario_label_color()),
                    ));
                    label.spawn((
                        Text::new("ORBIT"),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(scenario_label_color()),
                    ));
                    label.spawn((
                        Text::new(" 🪐"),
                        TextFont {
                            font: emoji_font.0.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(scenario_label_color()),
                    ));
                });
                spacer(card, 6.0);
                card.spawn((
                    Text::new(
                        "A strengthened central planetoid with jittered debris rings.\n\
                         Strong gravity well with varied but coherent orbital flow.",
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
                card.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|label| {
                    label.spawn((
                        Text::new("☄ "),
                        TextFont {
                            font: unicode_font.0.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(scenario_label_color()),
                    ));
                    label.spawn((
                        Text::new("COMETS"),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(scenario_label_color()),
                    ));
                    label.spawn((
                        Text::new(" ☄"),
                        TextFont {
                            font: unicode_font.0.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(scenario_label_color()),
                    ));
                });
                spacer(card, 6.0);
                card.spawn((
                    Text::new(
                        "Large-biased mixed bodies spawn near the outer boundary.\n\
                         Gentle inward trajectories with tangential crossing flow.",
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
                card.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|label| {
                    label.spawn((
                        Text::new("🌠 "),
                        TextFont {
                            font: emoji_font.0.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(scenario_label_color()),
                    ));
                    label.spawn((
                        Text::new("SHOWER"),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(scenario_label_color()),
                    ));
                    label.spawn((
                        Text::new(" 🌠"),
                        TextFont {
                            font: emoji_font.0.clone(),
                            font_size: 22.0,
                            ..default()
                        },
                        TextColor(scenario_label_color()),
                    ));
                });
                spacer(card, 6.0);
                card.spawn((
                    Text::new(
                        "Dense small-body-biased outer shower with inward rain.\n\
                         Distinct from Comets by smaller average mass and tighter clutter.",
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
                    Text::new("✧ "),
                    TextFont {
                        font: unicode_font.0.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(back_text()),
                ));
                btn.spawn((
                    Text::new("BACK"),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(back_text()),
                ));
                btn.spawn((
                    Text::new(" ✧"),
                    TextFont {
                        font: unicode_font.0.clone(),
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(back_text()),
                ));
            });
        });
}

/// Spawn scenario select only after the required fonts are loaded.
pub fn setup_scenario_select_when_fonts_ready(
    commands: Commands,
    font: Res<GameFont>,
    unicode_font: Res<crate::graphics::UnicodeFallbackFont>,
    emoji_font: Res<crate::graphics::EmojiFont>,
    loaded_fonts: Res<Assets<Font>>,
    existing_menu: Query<Entity, With<ScenarioSelectRoot>>,
) {
    if !existing_menu.is_empty() {
        return;
    }

    if !loaded_fonts.contains(font.0.id())
        || !loaded_fonts.contains(unicode_font.0.id())
        || !loaded_fonts.contains(emoji_font.0.id())
    {
        return;
    }

    setup_scenario_select(commands, font, unicode_font, emoji_font);
}

/// Recursively despawn all scenario-select entities.
pub fn cleanup_scenario_select(
    mut commands: Commands,
    query: Query<Entity, With<ScenarioSelectRoot>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

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
    child_nodes: Query<&Children>,
    mut next_state: ResMut<NextState<GameState>>,
    mut selected: ResMut<SelectedScenario>,
) {
    fn set_descendant_text_color(
        root_children: &Children,
        color: Color,
        text_query: &mut Query<&mut TextColor>,
        children_query: &Query<&Children>,
    ) {
        let mut stack: Vec<Entity> = root_children.iter().collect();
        while let Some(entity) = stack.pop() {
            if let Ok(mut text_color) = text_query.get_mut(entity) {
                *text_color = TextColor(color);
            }
            if let Ok(children) = children_query.get(entity) {
                stack.extend(children.iter());
            }
        }
    }

    for (interaction, children) in field_query.iter() {
        match interaction {
            Interaction::Pressed => {
                *selected = SelectedScenario::Field;
                next_state.set(GameState::Playing);
            }
            Interaction::Hovered => {
                set_descendant_text_color(
                    children,
                    scenario_active_text(),
                    &mut btn_text,
                    &child_nodes,
                );
            }
            Interaction::None => {
                set_descendant_text_color(
                    children,
                    scenario_label_color(),
                    &mut btn_text,
                    &child_nodes,
                );
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
                set_descendant_text_color(
                    children,
                    scenario_active_text(),
                    &mut btn_text,
                    &child_nodes,
                );
            }
            Interaction::None => {
                set_descendant_text_color(
                    children,
                    scenario_label_color(),
                    &mut btn_text,
                    &child_nodes,
                );
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
                set_descendant_text_color(
                    children,
                    scenario_active_text(),
                    &mut btn_text,
                    &child_nodes,
                );
            }
            Interaction::None => {
                set_descendant_text_color(
                    children,
                    scenario_label_color(),
                    &mut btn_text,
                    &child_nodes,
                );
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
                set_descendant_text_color(
                    children,
                    scenario_active_text(),
                    &mut btn_text,
                    &child_nodes,
                );
            }
            Interaction::None => {
                set_descendant_text_color(
                    children,
                    scenario_label_color(),
                    &mut btn_text,
                    &child_nodes,
                );
            }
        }
    }

    for (interaction, children) in back_query.iter() {
        match interaction {
            Interaction::Pressed => {
                next_state.set(GameState::MainMenu);
            }
            Interaction::Hovered => {
                set_descendant_text_color(children, Color::WHITE, &mut btn_text, &child_nodes);
            }
            Interaction::None => {
                set_descendant_text_color(children, back_text(), &mut btn_text, &child_nodes);
            }
        }
    }
}
