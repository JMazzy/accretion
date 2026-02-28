use super::*;

/// Spawn the game-over overlay centred over the frozen world.
///
/// Shows final score and a "PLAY AGAIN" button that re-spawns the player
/// with a fresh set of lives without resetting the asteroid field.
pub(super) fn setup_game_over(
    mut commands: Commands,
    score: Res<PlayerScore>,
    font: Res<GameFont>,
) {
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
                        Text::new("❤️ GAME OVER ❤️"),
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
                            "💎 Score: {}   ({} hits · {} destroyed)",
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
                            Text::new("⮝ PLAY AGAIN ⮝"),
                            TextFont {
                                font: font.0.clone(),
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(pause_resume_text()),
                        ));
                    });

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
                            Text::new("↭ QUIT ↭"),
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
                        Text::new("Press Enter to ⮝ play again"),
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

/// Recursively despawn all game-over overlay entities.
pub(super) fn cleanup_game_over(mut commands: Commands, query: Query<Entity, With<GameOverRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Handle Play Again / Quit actions in the game-over overlay.
#[allow(clippy::type_complexity)]
pub(super) fn game_over_button_system(
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
