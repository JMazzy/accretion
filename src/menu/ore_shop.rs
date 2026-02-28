use super::*;

/// Spawn the ore shop UI overlay.
///
/// Called by [`setup_ore_shop`] (the `OnEnter(OreShop)` system) and by the
/// button system after a purchase so the labels refresh in place.
#[allow(clippy::too_many_arguments)]
fn spawn_ore_shop_overlay(
    commands: &mut Commands,
    config: &PhysicsConfig,
    ore: u32,
    hp: f32,
    max_hp: f32,
    heal_amount: f32,
    ammo: u32,
    ammo_max: u32,
    weapon_level: &PrimaryWeaponLevel,
    missile_level: &SecondaryWeaponLevel,
    magnet_level: &OreAffinityLevel,
    tractor_level: &TractorBeamLevel,
    ion_level: &IonCannonLevel,
    font: &GameFont,
) {
    let ore_text = format!("💎 available: {ore}");

    let can_heal = ore > 0 && hp < max_hp;
    let heal_btn_bg = if can_heal {
        ore_shop_item_bg()
    } else {
        Color::srgb(0.10, 0.10, 0.10)
    };
    let heal_btn_border = if can_heal {
        ore_shop_item_border()
    } else {
        Color::srgb(0.22, 0.22, 0.22)
    };
    let heal_btn_text_color = if can_heal {
        ore_shop_item_text()
    } else {
        Color::srgb(0.38, 0.38, 0.38)
    };
    let heal_label = format!(
        "❤️ HEAL ❤️  (❤️: {:.0} / {:.0})  -  1 💎 -> +{:.0} ❤️",
        hp, max_hp, heal_amount
    );

    let can_missile = ore > 0 && ammo < ammo_max;
    let missile_btn_bg = if can_missile {
        ore_shop_item_bg()
    } else {
        Color::srgb(0.10, 0.10, 0.10)
    };
    let missile_btn_border = if can_missile {
        ore_shop_item_border()
    } else {
        Color::srgb(0.22, 0.22, 0.22)
    };
    let missile_btn_text_color = if can_missile {
        ore_shop_item_text()
    } else {
        Color::srgb(0.38, 0.38, 0.38)
    };
    let missile_label = format!("🚀 MISSILE 🚀  ({ammo} / {ammo_max})  -  1 💎 -> +1 🚀",);

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
            ZIndex(300),
            OreShopRoot,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(36.0)),
                        row_gap: Val::Px(16.0),
                        border: UiRect::all(Val::Px(2.0)),
                        min_width: Val::Px(400.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.05, 0.05, 0.03)),
                    BorderColor::all(ore_shop_btn_border()),
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new("💎 ORE SHOP 💎"),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 32.0,
                            ..default()
                        },
                        TextColor(ore_shop_btn_text()),
                    ));

                    card.spawn(Node {
                        height: Val::Px(4.0),
                        ..default()
                    });

                    // Ore counter
                    card.spawn((
                        Text::new(ore_text),
                        TextFont {
                            font: font.0.clone(),
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.25, 0.95, 0.50)),
                        OreShopOreText,
                    ));

                    card.spawn(Node {
                        height: Val::Px(4.0),
                        ..default()
                    });

                    // ── Consumables row ───────────────────────────────────────
                    card.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|consumables| {
                        consumables
                            .spawn((
                                Button,
                                Node {
                                    width: Val::Px(380.0),
                                    height: Val::Px(52.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(heal_btn_bg),
                                BorderColor::all(heal_btn_border),
                                OreShopHealButton,
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(heal_label),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(heal_btn_text_color),
                                    OreShopHealText,
                                ));
                            });

                        consumables
                            .spawn((
                                Button,
                                Node {
                                    width: Val::Px(380.0),
                                    height: Val::Px(52.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(missile_btn_bg),
                                BorderColor::all(missile_btn_border),
                                OreShopMissileButton,
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(missile_label),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(missile_btn_text_color),
                                    OreShopMissileText,
                                ));
                            });
                    });

                    card.spawn(Node {
                        height: Val::Px(8.0),
                        ..default()
                    });

                    // ── Upgrades row ───────────────────────────────────────────
                    card.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        align_items: AlignItems::FlexStart,
                        ..default()
                    })
                    .with_children(|upgrades_row| {
                        // ── Weapon card ──────────────────────────────────────
                        upgrades_row
                            .spawn((
                                Node {
                                    width: Val::Px(248.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(6.0),
                                    padding: UiRect::all(Val::Px(12.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.09, 0.09, 0.08)),
                                BorderColor::all(Color::srgb(0.22, 0.22, 0.22)),
                            ))
                            .with_children(|card_col| {
                                let can_upgrade =
                                    !weapon_level.is_maxed() && weapon_level.can_afford_next(ore);
                                let upg_btn_bg = if can_upgrade {
                                    shop_buy_bg()
                                } else {
                                    Color::srgb(0.14, 0.14, 0.14)
                                };
                                let upg_btn_border = if can_upgrade {
                                    shop_buy_border()
                                } else {
                                    Color::srgb(0.28, 0.28, 0.28)
                                };
                                let upg_btn_text_color = if can_upgrade {
                                    shop_buy_text()
                                } else {
                                    Color::srgb(0.40, 0.40, 0.40)
                                };
                                let upg_label = if weapon_level.is_maxed() {
                                    "— MAX LEVEL —".to_string()
                                } else {
                                    let cost = weapon_level.cost_for_next_level().unwrap_or(0);
                                    format!("UPGRADE ({cost} 💎)")
                                };
                                let cost_status = if weapon_level.is_maxed() {
                                    "MAX LEVEL REACHED".to_string()
                                } else {
                                    let cost = weapon_level.cost_for_next_level().unwrap_or(0);
                                    if can_upgrade {
                                        format!("Cost: {cost} 💎")
                                    } else {
                                        format!("Need {cost} 💎")
                                    }
                                };
                                let level_text = format!(
                                    "Level {} / {}",
                                    weapon_level.display_level(),
                                    crate::constants::PRIMARY_WEAPON_MAX_LEVEL
                                );
                                let range_text = if weapon_level.is_maxed() {
                                    format!("Destroy size: {}", weapon_level.max_destroy_size())
                                } else {
                                    format!(
                                        "Destroy size: {} -> {}",
                                        weapon_level.max_destroy_size(),
                                        weapon_level.max_destroy_size() + 1
                                    )
                                };

                                card_col.spawn((
                                    Text::new("⛯ BLASTER ⛯"),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.45, 0.45, 0.45)),
                                ));
                                card_col.spawn((
                                    Text::new(level_text),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.85, 0.85, 0.85)),
                                ));
                                card_col.spawn((
                                    Text::new(range_text),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.55, 0.65, 0.60)),
                                ));
                                card_col.spawn((
                                    Text::new(cost_status),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(if weapon_level.is_maxed() {
                                        Color::srgb(0.90, 0.80, 0.30)
                                    } else if can_upgrade {
                                        Color::srgb(0.75, 0.90, 0.75)
                                    } else {
                                        Color::srgb(0.75, 0.40, 0.40)
                                    }),
                                ));
                                card_col
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Percent(100.0),
                                            height: Val::Px(42.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(2.0)),
                                            ..default()
                                        },
                                        BackgroundColor(upg_btn_bg),
                                        BorderColor::all(upg_btn_border),
                                        OreShopUpgradeButton,
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new(upg_label),
                                            TextFont {
                                                font: font.0.clone(),
                                                font_size: 14.0,
                                                ..default()
                                            },
                                            TextColor(upg_btn_text_color),
                                        ));
                                    });
                            });

                        // ── Missile card ─────────────────────────────────────
                        upgrades_row
                            .spawn((
                                Node {
                                    width: Val::Px(248.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(6.0),
                                    padding: UiRect::all(Val::Px(12.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.09, 0.09, 0.08)),
                                BorderColor::all(Color::srgb(0.22, 0.22, 0.22)),
                            ))
                            .with_children(|card_col| {
                                let can_upgrade =
                                    !missile_level.is_maxed() && missile_level.can_afford_next(ore);
                                let upg_btn_bg = if can_upgrade {
                                    shop_buy_bg()
                                } else {
                                    Color::srgb(0.14, 0.14, 0.14)
                                };
                                let upg_btn_border = if can_upgrade {
                                    shop_buy_border()
                                } else {
                                    Color::srgb(0.28, 0.28, 0.28)
                                };
                                let upg_btn_text_color = if can_upgrade {
                                    shop_buy_text()
                                } else {
                                    Color::srgb(0.40, 0.40, 0.40)
                                };
                                let upg_label = if missile_level.is_maxed() {
                                    "— MAX LEVEL —".to_string()
                                } else {
                                    let cost = missile_level.cost_for_next_level().unwrap_or(0);
                                    format!("UPGRADE ({cost} 💎)")
                                };
                                let cost_status = if missile_level.is_maxed() {
                                    "MAX LEVEL REACHED".to_string()
                                } else {
                                    let cost = missile_level.cost_for_next_level().unwrap_or(0);
                                    if can_upgrade {
                                        format!("Cost: {cost} 💎")
                                    } else {
                                        format!("Need {cost} 💎")
                                    }
                                };
                                let level_text = format!(
                                    "Level {} / {}",
                                    missile_level.display_level(),
                                    crate::constants::SECONDARY_WEAPON_MAX_LEVEL
                                );
                                let range_text = if missile_level.is_maxed() {
                                    format!("Destroy size: {}", missile_level.destroy_threshold())
                                } else {
                                    format!(
                                        "Destroy size: {} -> {}",
                                        missile_level.destroy_threshold(),
                                        missile_level.destroy_threshold() + 1
                                    )
                                };

                                card_col.spawn((
                                    Text::new("🚀 MISSILE 🚀"),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.45, 0.45, 0.45)),
                                ));
                                card_col.spawn((
                                    Text::new(level_text),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.85, 0.85, 0.85)),
                                ));
                                card_col.spawn((
                                    Text::new(range_text),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.55, 0.65, 0.60)),
                                ));
                                card_col.spawn((
                                    Text::new(cost_status),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(if missile_level.is_maxed() {
                                        Color::srgb(0.90, 0.80, 0.30)
                                    } else if can_upgrade {
                                        Color::srgb(0.75, 0.90, 0.75)
                                    } else {
                                        Color::srgb(0.75, 0.40, 0.40)
                                    }),
                                ));
                                card_col
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Percent(100.0),
                                            height: Val::Px(42.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(2.0)),
                                            ..default()
                                        },
                                        BackgroundColor(upg_btn_bg),
                                        BorderColor::all(upg_btn_border),
                                        OreShopMissileUpgradeButton,
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new(upg_label),
                                            TextFont {
                                                font: font.0.clone(),
                                                font_size: 14.0,
                                                ..default()
                                            },
                                            TextColor(upg_btn_text_color),
                                        ));
                                    });
                            });

                        // ── Magnet card ──────────────────────────────────────
                        upgrades_row
                            .spawn((
                                Node {
                                    width: Val::Px(248.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(6.0),
                                    padding: UiRect::all(Val::Px(12.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.09, 0.09, 0.08)),
                                BorderColor::all(Color::srgb(0.22, 0.22, 0.22)),
                            ))
                            .with_children(|card_col| {
                                let can_upgrade =
                                    !magnet_level.is_maxed() && magnet_level.can_afford_next(ore);
                                let upg_btn_bg = if can_upgrade {
                                    shop_buy_bg()
                                } else {
                                    Color::srgb(0.14, 0.14, 0.14)
                                };
                                let upg_btn_border = if can_upgrade {
                                    shop_buy_border()
                                } else {
                                    Color::srgb(0.28, 0.28, 0.28)
                                };
                                let upg_btn_text_color = if can_upgrade {
                                    shop_buy_text()
                                } else {
                                    Color::srgb(0.40, 0.40, 0.40)
                                };
                                let upg_label = if magnet_level.is_maxed() {
                                    "— MAX LEVEL —".to_string()
                                } else {
                                    let cost = magnet_level.cost_for_next_level().unwrap_or(0);
                                    format!("UPGRADE ({cost} 💎)")
                                };
                                let cost_status = if magnet_level.is_maxed() {
                                    "MAX LEVEL REACHED".to_string()
                                } else {
                                    let cost = magnet_level.cost_for_next_level().unwrap_or(0);
                                    if can_upgrade {
                                        format!("Cost: {cost} 💎")
                                    } else {
                                        format!("Need {cost} 💎")
                                    }
                                };
                                let level_text = format!(
                                    "Level {} / {}",
                                    magnet_level.display_level(),
                                    crate::constants::ORE_AFFINITY_MAX_LEVEL
                                );
                                let range_text = if magnet_level.is_maxed() {
                                    format!("Pull radius: {:.0} px", magnet_level.radius_at_level())
                                } else {
                                    format!(
                                        "Pull radius: {:.0} -> {:.0} px",
                                        magnet_level.radius_at_level(),
                                        magnet_level.radius_at_level() + 50.0
                                    )
                                };

                                card_col.spawn((
                                    Text::new("🧲 MAGNET 🧲"),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.45, 0.45, 0.45)),
                                ));
                                card_col.spawn((
                                    Text::new(level_text),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.85, 0.85, 0.85)),
                                ));
                                card_col.spawn((
                                    Text::new(range_text),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.55, 0.65, 0.60)),
                                ));
                                card_col.spawn((
                                    Text::new(cost_status),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(if magnet_level.is_maxed() {
                                        Color::srgb(0.90, 0.80, 0.30)
                                    } else if can_upgrade {
                                        Color::srgb(0.75, 0.90, 0.75)
                                    } else {
                                        Color::srgb(0.75, 0.40, 0.40)
                                    }),
                                ));
                                card_col
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Percent(100.0),
                                            height: Val::Px(42.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(2.0)),
                                            ..default()
                                        },
                                        BackgroundColor(upg_btn_bg),
                                        BorderColor::all(upg_btn_border),
                                        OreShopMagnetUpgradeButton,
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new(upg_label),
                                            TextFont {
                                                font: font.0.clone(),
                                                font_size: 14.0,
                                                ..default()
                                            },
                                            TextColor(upg_btn_text_color),
                                        ));
                                    });
                            });

                        // ── Tractor card ─────────────────────────────────────
                        upgrades_row
                            .spawn((
                                Node {
                                    width: Val::Px(248.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(6.0),
                                    padding: UiRect::all(Val::Px(12.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.09, 0.09, 0.08)),
                                BorderColor::all(Color::srgb(0.22, 0.22, 0.22)),
                            ))
                            .with_children(|card_col| {
                                let can_upgrade =
                                    !tractor_level.is_maxed() && tractor_level.can_afford_next(ore);
                                let upg_btn_bg = if can_upgrade {
                                    shop_buy_bg()
                                } else {
                                    Color::srgb(0.14, 0.14, 0.14)
                                };
                                let upg_btn_border = if can_upgrade {
                                    shop_buy_border()
                                } else {
                                    Color::srgb(0.28, 0.28, 0.28)
                                };
                                let upg_btn_text_color = if can_upgrade {
                                    shop_buy_text()
                                } else {
                                    Color::srgb(0.40, 0.40, 0.40)
                                };
                                let upg_label = if tractor_level.is_maxed() {
                                    "— MAX LEVEL —".to_string()
                                } else {
                                    let cost = tractor_level.cost_for_next_level().unwrap_or(0);
                                    format!("UPGRADE ({cost} 💎)")
                                };
                                let cost_status = if tractor_level.is_maxed() {
                                    "MAX LEVEL REACHED".to_string()
                                } else {
                                    let cost = tractor_level.cost_for_next_level().unwrap_or(0);
                                    if can_upgrade {
                                        format!("Cost: {cost} 💎")
                                    } else {
                                        format!("Need {cost} 💎")
                                    }
                                };
                                let level_text = format!(
                                    "Level {} / {}",
                                    tractor_level.display_level(),
                                    crate::constants::TRACTOR_BEAM_MAX_LEVEL
                                );
                                let range_text = if tractor_level.is_maxed() {
                                    format!("Range: {:.0} px", tractor_level.range_at_level(config))
                                } else {
                                    let next = TractorBeamLevel {
                                        level: tractor_level.level + 1,
                                    };
                                    format!(
                                        "Range: {:.0} -> {:.0} px",
                                        tractor_level.range_at_level(config),
                                        next.range_at_level(config)
                                    )
                                };

                                card_col.spawn((
                                    Text::new("↭ TRACTOR ↭"),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.45, 0.45, 0.45)),
                                ));
                                card_col.spawn((
                                    Text::new(level_text),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.85, 0.85, 0.85)),
                                ));
                                card_col.spawn((
                                    Text::new(range_text),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.55, 0.65, 0.60)),
                                ));
                                card_col.spawn((
                                    Text::new(cost_status),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(if tractor_level.is_maxed() {
                                        Color::srgb(0.90, 0.80, 0.30)
                                    } else if can_upgrade {
                                        Color::srgb(0.75, 0.90, 0.75)
                                    } else {
                                        Color::srgb(0.75, 0.40, 0.40)
                                    }),
                                ));
                                card_col
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Percent(100.0),
                                            height: Val::Px(42.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(2.0)),
                                            ..default()
                                        },
                                        BackgroundColor(upg_btn_bg),
                                        BorderColor::all(upg_btn_border),
                                        OreShopTractorUpgradeButton,
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new(upg_label),
                                            TextFont {
                                                font: font.0.clone(),
                                                font_size: 14.0,
                                                ..default()
                                            },
                                            TextColor(upg_btn_text_color),
                                        ));
                                    });
                            });

                        // ── Ion cannon card ─────────────────────────────────
                        upgrades_row
                            .spawn((
                                Node {
                                    width: Val::Px(248.0),
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(6.0),
                                    padding: UiRect::all(Val::Px(12.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.09, 0.09, 0.08)),
                                BorderColor::all(Color::srgb(0.22, 0.22, 0.22)),
                            ))
                            .with_children(|card_col| {
                                let can_upgrade =
                                    !ion_level.is_maxed() && ion_level.can_afford_next(ore);
                                let upg_btn_bg = if can_upgrade {
                                    shop_buy_bg()
                                } else {
                                    Color::srgb(0.14, 0.14, 0.14)
                                };
                                let upg_btn_border = if can_upgrade {
                                    shop_buy_border()
                                } else {
                                    Color::srgb(0.28, 0.28, 0.28)
                                };
                                let upg_btn_text_color = if can_upgrade {
                                    shop_buy_text()
                                } else {
                                    Color::srgb(0.40, 0.40, 0.40)
                                };
                                let upg_label = if ion_level.is_maxed() {
                                    "— MAX LEVEL —".to_string()
                                } else {
                                    let cost = ion_level.cost_for_next_level().unwrap_or(0);
                                    format!("UPGRADE ({cost} 💎)")
                                };
                                let cost_status = if ion_level.is_maxed() {
                                    "MAX LEVEL REACHED".to_string()
                                } else {
                                    let cost = ion_level.cost_for_next_level().unwrap_or(0);
                                    if can_upgrade {
                                        format!("Cost: {cost} 💎")
                                    } else {
                                        format!("Need {cost} 💎")
                                    }
                                };

                                let level_text = format!(
                                    "Level {} / {}",
                                    ion_level.display_level(),
                                    crate::constants::ION_CANNON_MAX_LEVEL
                                );
                                let current_stun = ion_level.stun_duration_secs();
                                let (next_stun, next_tier) = if ion_level.is_maxed() {
                                    (current_stun, ion_level.max_enemy_tier_affected())
                                } else {
                                    let next = IonCannonLevel {
                                        level: ion_level.level + 1,
                                    };
                                    (next.stun_duration_secs(), next.max_enemy_tier_affected())
                                };
                                let effectiveness_text = if ion_level.is_maxed() {
                                    format!(
                                        "Stun: {:.1}s | Affects up to tier {}",
                                        current_stun,
                                        ion_level.max_enemy_tier_affected()
                                    )
                                } else {
                                    format!(
                                        "Stun: {:.1} -> {:.1}s | Tier: {} -> {}",
                                        current_stun,
                                        next_stun,
                                        ion_level.max_enemy_tier_affected(),
                                        next_tier
                                    )
                                };

                                card_col.spawn((
                                    Text::new("⚛ ION CANNON ⚛"),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.45, 0.45, 0.45)),
                                ));
                                card_col.spawn((
                                    Text::new(level_text),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.85, 0.85, 0.85)),
                                ));
                                card_col.spawn((
                                    Text::new(effectiveness_text),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.55, 0.65, 0.60)),
                                ));
                                card_col.spawn((
                                    Text::new(cost_status),
                                    TextFont {
                                        font: font.0.clone(),
                                        font_size: 13.0,
                                        ..default()
                                    },
                                    TextColor(if ion_level.is_maxed() {
                                        Color::srgb(0.90, 0.80, 0.30)
                                    } else if can_upgrade {
                                        Color::srgb(0.75, 0.90, 0.75)
                                    } else {
                                        Color::srgb(0.75, 0.40, 0.40)
                                    }),
                                ));
                                card_col
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Percent(100.0),
                                            height: Val::Px(42.0),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(2.0)),
                                            ..default()
                                        },
                                        BackgroundColor(upg_btn_bg),
                                        BorderColor::all(upg_btn_border),
                                        OreShopIonUpgradeButton,
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            Text::new(upg_label),
                                            TextFont {
                                                font: font.0.clone(),
                                                font_size: 14.0,
                                                ..default()
                                            },
                                            TextColor(upg_btn_text_color),
                                        ));
                                    });
                            });
                    });

                    card.spawn(Node {
                        height: Val::Px(4.0),
                        ..default()
                    });

                    // ── Close button ──────────────────────────────────────────
                    card.spawn((
                        Button,
                        Node {
                            width: Val::Px(220.0),
                            height: Val::Px(44.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(shop_close_bg()),
                        BorderColor::all(shop_close_border()),
                        OreShopCloseButton,
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("CLOSE"),
                            TextFont {
                                font: font.0.clone(),
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(shop_close_text()),
                        ));
                    });

                    card.spawn((
                        Text::new("Press Tab or ESC to close"),
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

/// Spawn the ore shop overlay when entering [`GameState::OreShop`].
#[allow(clippy::too_many_arguments)]
pub fn setup_ore_shop(
    mut commands: Commands,
    ore: Res<PlayerOre>,
    q_health: Query<&PlayerHealth, With<Player>>,
    ammo: Res<MissileAmmo>,
    config: Res<PhysicsConfig>,
    weapon_level: Res<PrimaryWeaponLevel>,
    missile_level: Res<SecondaryWeaponLevel>,
    magnet_level: Res<OreAffinityLevel>,
    tractor_level: Res<TractorBeamLevel>,
    ion_level: Res<IonCannonLevel>,
    font: Res<GameFont>,
) {
    let (hp, max_hp) = q_health
        .single()
        .map(|h| (h.hp, h.max_hp))
        .unwrap_or((config.player_max_hp, config.player_max_hp));
    spawn_ore_shop_overlay(
        &mut commands,
        &config,
        ore.count,
        hp,
        max_hp,
        config.ore_heal_amount,
        ammo.count,
        config.missile_ammo_max,
        &weapon_level,
        &missile_level,
        &magnet_level,
        &tractor_level,
        &ion_level,
        &font,
    );
}

/// Despawn the ore shop overlay when exiting [`GameState::OreShop`].
pub fn cleanup_ore_shop(mut commands: Commands, query: Query<Entity, With<OreShopRoot>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Handle button and keyboard interactions in the ore shop.
///
/// - **HEAL** button: spend 1 ore, restore `ore_heal_amount` HP (capped at max).
/// - **MISSILE** button: spend 1 ore, restore 1 missile (capped at `missile_ammo_max`).
/// - **UPGRADE WEAPON** button: spend ore to increase weapon level.
/// - **CLOSE** button / **ESC** / **Tab**: return to the originating state.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn ore_shop_button_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    heal_query: Query<&Interaction, (Changed<Interaction>, With<OreShopHealButton>)>,
    missile_query: Query<&Interaction, (Changed<Interaction>, With<OreShopMissileButton>)>,
    close_query: Query<&Interaction, (Changed<Interaction>, With<OreShopCloseButton>)>,
    upgrade_queries: (
        Query<&Interaction, (Changed<Interaction>, With<OreShopUpgradeButton>)>,
        Query<&Interaction, (Changed<Interaction>, With<OreShopMissileUpgradeButton>)>,
        Query<&Interaction, (Changed<Interaction>, With<OreShopMagnetUpgradeButton>)>,
        Query<&Interaction, (Changed<Interaction>, With<OreShopTractorUpgradeButton>)>,
        Query<&Interaction, (Changed<Interaction>, With<OreShopIonUpgradeButton>)>,
    ),
    shop_root_query: Query<Entity, With<OreShopRoot>>,
    mut ore: ResMut<PlayerOre>,
    mut q_health: Query<&mut PlayerHealth, With<Player>>,
    mut ammo: ResMut<MissileAmmo>,
    config: Res<PhysicsConfig>,
    levels: (
        ResMut<PrimaryWeaponLevel>,
        ResMut<SecondaryWeaponLevel>,
        ResMut<OreAffinityLevel>,
        ResMut<TractorBeamLevel>,
        ResMut<IonCannonLevel>,
    ),
    mut next_state: ResMut<NextState<GameState>>,
    return_state: Res<ShopReturnState>,
    font: Res<GameFont>,
) {
    // Destructure tuple parameters
    let (
        upgrade_query,
        missile_upgrade_query,
        magnet_upgrade_query,
        tractor_upgrade_query,
        ion_upgrade_query,
    ) = upgrade_queries;
    let (mut weapon_level, mut missile_level, mut magnet_level, mut tractor_level, mut ion_level) =
        levels;

    // ── Close (ESC / Tab / button) ────────────────────────────────────────────
    let wants_close = keys.just_pressed(KeyCode::Escape)
        || keys.just_pressed(KeyCode::Tab)
        || close_query.iter().any(|i| *i == Interaction::Pressed);

    if wants_close {
        let target = match *return_state {
            ShopReturnState::Playing => GameState::Playing,
            ShopReturnState::Paused => GameState::Paused,
        };
        next_state.set(target);
        return;
    }

    // ── Heal ──────────────────────────────────────────────────────────────────
    let heal_pressed = heal_query.iter().any(|i| *i == Interaction::Pressed);
    if heal_pressed && ore.count > 0 {
        if let Ok(mut health) = q_health.single_mut() {
            if health.hp < health.max_hp {
                health.hp = (health.hp + config.ore_heal_amount).min(health.max_hp);
                ore.count -= 1;
                let (hp, max_hp) = (health.hp, health.max_hp);
                let ore_count = ore.count;
                let ammo_count = ammo.count;
                let heal_amount = config.ore_heal_amount;
                let ammo_max = config.missile_ammo_max;
                for entity in shop_root_query.iter() {
                    commands.entity(entity).despawn();
                }
                spawn_ore_shop_overlay(
                    &mut commands,
                    &config,
                    ore_count,
                    hp,
                    max_hp,
                    heal_amount,
                    ammo_count,
                    ammo_max,
                    &weapon_level,
                    &missile_level,
                    &magnet_level,
                    &tractor_level,
                    &ion_level,
                    &font,
                );
                return;
            }
        }
    }

    // ── Missile restock ───────────────────────────────────────────────────────
    let missile_pressed = missile_query.iter().any(|i| *i == Interaction::Pressed);
    if missile_pressed && ore.count > 0 && ammo.count < config.missile_ammo_max {
        ammo.count += 1;
        ore.count -= 1;
        let (hp, max_hp) = q_health
            .single()
            .map(|h| (h.hp, h.max_hp))
            .unwrap_or((config.player_max_hp, config.player_max_hp));
        let ore_count = ore.count;
        let ammo_count = ammo.count;
        let heal_amount = config.ore_heal_amount;
        let ammo_max = config.missile_ammo_max;
        for entity in shop_root_query.iter() {
            commands.entity(entity).despawn();
        }
        spawn_ore_shop_overlay(
            &mut commands,
            &config,
            ore_count,
            hp,
            max_hp,
            heal_amount,
            ammo_count,
            ammo_max,
            &weapon_level,
            &missile_level,
            &magnet_level,
            &tractor_level,
            &ion_level,
            &font,
        );
        return;
    }

    // ── Weapon upgrade ────────────────────────────────────────────────────────
    let upgrade_pressed = upgrade_query.iter().any(|i| *i == Interaction::Pressed);
    if upgrade_pressed {
        weapon_level.try_upgrade(&mut ore.count);
        let (hp, max_hp) = q_health
            .single()
            .map(|h| (h.hp, h.max_hp))
            .unwrap_or((config.player_max_hp, config.player_max_hp));
        let ore_count = ore.count;
        let ammo_count = ammo.count;
        let heal_amount = config.ore_heal_amount;
        let ammo_max = config.missile_ammo_max;
        for entity in shop_root_query.iter() {
            commands.entity(entity).despawn();
        }
        spawn_ore_shop_overlay(
            &mut commands,
            &config,
            ore_count,
            hp,
            max_hp,
            heal_amount,
            ammo_count,
            ammo_max,
            &weapon_level,
            &missile_level,
            &magnet_level,
            &tractor_level,
            &ion_level,
            &font,
        );
    }

    // ── Missile upgrade ───────────────────────────────────────────────────────
    let missile_upgrade_pressed = missile_upgrade_query
        .iter()
        .any(|i| *i == Interaction::Pressed);
    if missile_upgrade_pressed {
        missile_level.try_upgrade(&mut ore.count);
        let (hp, max_hp) = q_health
            .single()
            .map(|h| (h.hp, h.max_hp))
            .unwrap_or((config.player_max_hp, config.player_max_hp));
        let ore_count = ore.count;
        let ammo_count = ammo.count;
        let heal_amount = config.ore_heal_amount;
        let ammo_max = config.missile_ammo_max;
        for entity in shop_root_query.iter() {
            commands.entity(entity).despawn();
        }
        spawn_ore_shop_overlay(
            &mut commands,
            &config,
            ore_count,
            hp,
            max_hp,
            heal_amount,
            ammo_count,
            ammo_max,
            &weapon_level,
            &missile_level,
            &magnet_level,
            &tractor_level,
            &ion_level,
            &font,
        );
    }

    // ── Magnet upgrade ────────────────────────────────────────────────────────
    let magnet_upgrade_pressed = magnet_upgrade_query
        .iter()
        .any(|i| *i == Interaction::Pressed);
    if magnet_upgrade_pressed {
        magnet_level.try_upgrade(&mut ore.count);
        let (hp, max_hp) = q_health
            .single()
            .map(|h| (h.hp, h.max_hp))
            .unwrap_or((config.player_max_hp, config.player_max_hp));
        let ore_count = ore.count;
        let ammo_count = ammo.count;
        let heal_amount = config.ore_heal_amount;
        let ammo_max = config.missile_ammo_max;
        for entity in shop_root_query.iter() {
            commands.entity(entity).despawn();
        }
        spawn_ore_shop_overlay(
            &mut commands,
            &config,
            ore_count,
            hp,
            max_hp,
            heal_amount,
            ammo_count,
            ammo_max,
            &weapon_level,
            &missile_level,
            &magnet_level,
            &tractor_level,
            &ion_level,
            &font,
        );
    }

    // ── Tractor upgrade ───────────────────────────────────────────────────────
    let tractor_upgrade_pressed = tractor_upgrade_query
        .iter()
        .any(|i| *i == Interaction::Pressed);
    if tractor_upgrade_pressed {
        tractor_level.try_upgrade(&mut ore.count);
        let (hp, max_hp) = q_health
            .single()
            .map(|h| (h.hp, h.max_hp))
            .unwrap_or((config.player_max_hp, config.player_max_hp));
        let ore_count = ore.count;
        let ammo_count = ammo.count;
        let heal_amount = config.ore_heal_amount;
        let ammo_max = config.missile_ammo_max;
        for entity in shop_root_query.iter() {
            commands.entity(entity).despawn();
        }
        spawn_ore_shop_overlay(
            &mut commands,
            &config,
            ore_count,
            hp,
            max_hp,
            heal_amount,
            ammo_count,
            ammo_max,
            &weapon_level,
            &missile_level,
            &magnet_level,
            &tractor_level,
            &ion_level,
            &font,
        );
    }

    // ── Ion cannon upgrade ────────────────────────────────────────────────────
    let ion_upgrade_pressed = ion_upgrade_query.iter().any(|i| *i == Interaction::Pressed);
    if ion_upgrade_pressed {
        ion_level.try_upgrade(&mut ore.count);
        let (hp, max_hp) = q_health
            .single()
            .map(|h| (h.hp, h.max_hp))
            .unwrap_or((config.player_max_hp, config.player_max_hp));
        let ore_count = ore.count;
        let ammo_count = ammo.count;
        let heal_amount = config.ore_heal_amount;
        let ammo_max = config.missile_ammo_max;
        for entity in shop_root_query.iter() {
            commands.entity(entity).despawn();
        }
        spawn_ore_shop_overlay(
            &mut commands,
            &config,
            ore_count,
            hp,
            max_hp,
            heal_amount,
            ammo_count,
            ammo_max,
            &weapon_level,
            &missile_level,
            &magnet_level,
            &tractor_level,
            &ion_level,
            &font,
        );
    }
}
