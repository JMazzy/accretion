use bevy::prelude::*;

pub(super) fn start_bg() -> Color {
    Color::srgb(0.08, 0.36, 0.14)
}
pub(super) fn start_border() -> Color {
    Color::srgb(0.18, 0.72, 0.28)
}
pub(super) fn start_text() -> Color {
    Color::srgb(0.75, 1.0, 0.80)
}
pub(super) fn quit_bg() -> Color {
    Color::srgb(0.28, 0.06, 0.06)
}
pub(super) fn quit_border() -> Color {
    Color::srgb(0.60, 0.12, 0.12)
}
pub(super) fn quit_text() -> Color {
    Color::srgb(1.0, 0.65, 0.65)
}
pub(super) fn title_color() -> Color {
    Color::srgb(0.95, 0.88, 0.45)
}
pub(super) fn subtitle_color() -> Color {
    Color::srgb(0.55, 0.55, 0.65)
}
pub(super) fn hint_color() -> Color {
    Color::srgb(0.28, 0.28, 0.35)
}

pub(super) fn scenario_card_bg() -> Color {
    Color::srgb(0.06, 0.09, 0.18)
}
pub(super) fn scenario_card_border() -> Color {
    Color::srgb(0.22, 0.38, 0.72)
}
pub(super) fn scenario_active_text() -> Color {
    Color::srgb(0.80, 0.95, 1.0)
}
pub(super) fn scenario_label_color() -> Color {
    Color::srgb(0.90, 0.90, 1.0)
}
pub(super) fn scenario_desc_color() -> Color {
    Color::srgb(0.45, 0.50, 0.65)
}
pub(super) fn back_bg() -> Color {
    Color::srgb(0.12, 0.12, 0.18)
}
pub(super) fn back_border() -> Color {
    Color::srgb(0.30, 0.30, 0.46)
}
pub(super) fn back_text() -> Color {
    Color::srgb(0.55, 0.55, 0.70)
}

pub(super) fn pause_resume_bg() -> Color {
    Color::srgb(0.08, 0.36, 0.14)
}
pub(super) fn pause_resume_border() -> Color {
    Color::srgb(0.18, 0.72, 0.28)
}
pub(super) fn pause_resume_text() -> Color {
    Color::srgb(0.75, 1.0, 0.80)
}
pub(super) fn pause_debug_bg() -> Color {
    Color::srgb(0.10, 0.18, 0.36)
}
pub(super) fn pause_debug_border() -> Color {
    Color::srgb(0.22, 0.44, 0.78)
}
pub(super) fn pause_debug_text() -> Color {
    Color::srgb(0.65, 0.80, 1.0)
}
pub(super) fn shop_buy_bg() -> Color {
    Color::srgb(0.06, 0.22, 0.06)
}
pub(super) fn shop_buy_border() -> Color {
    Color::srgb(0.18, 0.56, 0.18)
}
pub(super) fn shop_buy_text() -> Color {
    Color::srgb(0.55, 1.0, 0.55)
}
pub(super) fn shop_close_bg() -> Color {
    Color::srgb(0.14, 0.14, 0.20)
}
pub(super) fn shop_close_border() -> Color {
    Color::srgb(0.30, 0.30, 0.46)
}
pub(super) fn shop_close_text() -> Color {
    Color::srgb(0.65, 0.65, 0.80)
}
pub(super) fn ore_shop_btn_border() -> Color {
    Color::srgb(0.62, 0.44, 0.12)
}
pub(super) fn ore_shop_btn_text() -> Color {
    Color::srgb(1.0, 0.80, 0.30)
}
pub(super) fn ore_shop_item_bg() -> Color {
    Color::srgb(0.08, 0.10, 0.06)
}
pub(super) fn ore_shop_item_border() -> Color {
    Color::srgb(0.30, 0.40, 0.18)
}
pub(super) fn ore_shop_item_text() -> Color {
    Color::srgb(0.75, 0.90, 0.55)
}

pub(super) fn format_saved_at(unix_secs: u64) -> String {
    if unix_secs == 0 {
        "saved: unknown".to_string()
    } else {
        format!("saved: unix {unix_secs}")
    }
}

pub(super) fn spacer(parent: &mut ChildSpawnerCommands<'_>, px: f32) {
    parent.spawn(Node {
        height: Val::Px(px),
        ..default()
    });
}

pub(super) fn pause_spacer(parent: &mut ChildSpawnerCommands<'_>, px: f32) {
    parent.spawn(Node {
        height: Val::Px(px),
        ..default()
    });
}
