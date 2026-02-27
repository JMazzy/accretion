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
