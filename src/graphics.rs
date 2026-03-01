use bevy::prelude::*;
use std::collections::HashSet;
use std::fs;
use ttf_parser::Face;

fn preferred_unicode_fallback() -> (&'static str, &'static str, &'static str) {
    if fs::metadata("assets/fonts/NotoSans-Regular.ttf").is_ok() {
        (
            "noto_sans",
            "fonts/NotoSans-Regular.ttf",
            "assets/fonts/NotoSans-Regular.ttf",
        )
    } else {
        (
            "unicode",
            "fonts/DejaVuSans.ttf",
            "assets/fonts/DejaVuSans.ttf",
        )
    }
}

/// Game font resource — stores the loaded Tektur font handle.
///
/// All UI text in menus and HUDs references `font.0.clone()` instead of
/// using the default Bevy font.  Created by [`load_game_font`] at startup.
#[derive(Resource, Default)]
pub struct GameFont(pub Handle<Font>);

/// Symbol font resource — stores the loaded Noto Sans Symbols font handle.
#[derive(Resource, Default)]
pub struct SymbolFont(pub Handle<Font>);

/// Secondary symbol font resource — stores Noto Sans Symbols 2 handle.
#[derive(Resource, Default)]
pub struct SymbolFont2(pub Handle<Font>);

/// Noto Sans fallback font for symbols not covered by primary UI fonts.
#[derive(Resource, Default)]
pub struct UnicodeFallbackFont(pub Handle<Font>);

/// Emoji fallback font for glyphs like rocket/magnet/gem.
#[derive(Resource, Default)]
pub struct EmojiFont(pub Handle<Font>);

/// Load the custom Tektur font from assets at startup.
///
/// Must run before any UI setup systems that spawn text.
pub fn load_game_font(mut font: ResMut<GameFont>, asset_server: Res<AssetServer>) {
    let font_handle = asset_server.load("fonts/Tektur-Regular.ttf");
    font.0 = font_handle;
    eprintln!("[SETUP] Game font loaded");
}

/// Load the symbol font used for icon-style HUD glyphs.
pub fn load_symbol_font(mut font: ResMut<SymbolFont>, asset_server: Res<AssetServer>) {
    let font_handle = asset_server.load("fonts/NotoSansSymbols-Regular.ttf");
    font.0 = font_handle;
    eprintln!("[SETUP] Symbol font loaded");
}

/// Load the secondary symbol font (Noto Sans Symbols 2).
pub fn load_symbol_font_2(mut font: ResMut<SymbolFont2>, asset_server: Res<AssetServer>) {
    let font_handle = asset_server.load("fonts/NotoSansSymbols2-Regular.ttf");
    font.0 = font_handle;
    eprintln!("[SETUP] Symbol font 2 loaded");
}

/// Load Noto Sans fallback font used for selected missing symbols.
pub fn load_unicode_fallback_font(
    mut font: ResMut<UnicodeFallbackFont>,
    asset_server: Res<AssetServer>,
) {
    let (font_name, asset_path, _) = preferred_unicode_fallback();
    let font_handle = asset_server.load(asset_path);
    font.0 = font_handle;
    eprintln!("[SETUP] {} fallback font loaded", font_name);
}

/// Load emoji fallback font used for selected missing emoji symbols.
pub fn load_emoji_font(mut font: ResMut<EmojiFont>, asset_server: Res<AssetServer>) {
    let font_handle = asset_server.load("fonts/NotoEmoji-Regular.ttf");
    font.0 = font_handle;
    eprintln!("[SETUP] Emoji fallback font loaded");
}

fn codepoints_for_text(text: &str) -> impl Iterator<Item = u32> + '_ {
    text.chars()
        .map(|ch| ch as u32)
        .filter(|cp| *cp != 0xFE0F && *cp != 0x200D)
}

fn load_font_codepoints(path: &str) -> Option<HashSet<u32>> {
    let bytes = fs::read(path).ok()?;
    let face = Face::parse(&bytes, 0).ok()?;
    let mut set = HashSet::new();
    let cmap = face.tables().cmap?;
    for subtable in cmap.subtables {
        subtable.codepoints(|cp| {
            set.insert(cp);
        });
    }
    Some(set)
}

/// Log symbol font coverage and probable substitutions for configured UI symbols.
pub fn log_font_substitution_diagnostics() {
    let (unicode_font_name, _, unicode_font_disk_path) = preferred_unicode_fallback();
    let font_catalog = [
        ("game", "assets/fonts/Tektur-Regular.ttf"),
        (unicode_font_name, unicode_font_disk_path),
        ("symbol1", "assets/fonts/NotoSansSymbols-Regular.ttf"),
        ("symbol2", "assets/fonts/NotoSansSymbols2-Regular.ttf"),
        ("emoji", "assets/fonts/NotoEmoji-Regular.ttf"),
    ];

    let coverage: Vec<(&str, HashSet<u32>)> = font_catalog
        .iter()
        .filter_map(|(name, path)| load_font_codepoints(path).map(|cp| (*name, cp)))
        .collect();

    if coverage.is_empty() {
        warn!("[FONT-DIAG] No font coverage data loaded.");
        return;
    }

    let assignments = [
        ("hud_lives", "⮝", "symbol2"),
        ("hud_blaster", "⛯", "symbol1"),
        ("hud_missile", "🚀", "emoji"),
        ("hud_magnet", "🧲", "emoji"),
        ("hud_tractor", "✦", "symbol2"),
        ("hud_ion", "⚛", "symbol1"),
        ("hud_ore", "💎", "emoji"),
        ("hud_level_1", "①", "symbol1"),
        ("hud_level_2", "②", "symbol1"),
        ("hud_level_3", "③", "symbol1"),
        ("hud_level_4", "④", "symbol1"),
        ("hud_level_5", "⑤", "symbol1"),
        ("hud_level_6", "⑥", "symbol1"),
        ("hud_level_7", "⑦", "symbol1"),
        ("hud_level_8", "⑧", "symbol1"),
        ("hud_level_9", "⑨", "symbol1"),
        ("hud_level_10", "⑩", "symbol1"),
        ("hud_missile_slot_full", "●", "symbol2"),
        ("hud_missile_slot_empty", "○", "symbol2"),
        ("hud_tractor_off", "○", "symbol2"),
        ("hud_tractor_ready", "⚡", "symbol2"),
        ("hud_tractor_cooldown", "⌛", "symbol2"),
        ("hud_ion_ready", "⚡", "symbol2"),
        ("hud_ion_cooldown", "⌛", "symbol2"),
        ("hud_hp", "❤️", "emoji"),
        ("menu_symbol_ore", "💎", "emoji"),
        ("menu_symbol_missile", "🚀", "emoji"),
        ("menu_symbol_magnet", "🧲", "emoji"),
        ("menu_symbol_tractor", "✦", "symbol2"),
        ("menu_symbol_ion", "⚛", "symbol1"),
        ("menu_symbol_lives", "⮝", "symbol2"),
        ("menu_symbol_blaster", "⛯", "symbol1"),
        ("menu_symbol_hp", "❤️", "emoji"),
        ("menu_main_spiral", "🌌", "emoji"),
        ("menu_main_star_fill", "✦", "symbol2"),
        ("menu_main_star_outline", "✧", "symbol2"),
        ("menu_scenario_field", "🪨", "emoji"),
        ("menu_scenario_orbit", "🪐", "emoji"),
        ("menu_scenario_comets", "☄", "symbol2"),
        ("menu_scenario_shower", "🌠", "emoji"),
    ];

    info!("[FONT-DIAG] checking configured UI symbol font assignments...");
    for (label, symbol, assigned_font) in assignments {
        let needed: Vec<u32> = codepoints_for_text(symbol).collect();
        let assigned_set = coverage
            .iter()
            .find_map(|(name, set)| (*name == assigned_font).then_some(set));

        let assigned_ok = assigned_set
            .map(|set| needed.iter().all(|cp| set.contains(cp)))
            .unwrap_or(false);

        if assigned_ok {
            info!(
                "[FONT-DIAG] '{}' uses '{}' (direct coverage)",
                symbol, assigned_font
            );
            continue;
        }

        let fallback = coverage
            .iter()
            .find_map(|(name, set)| needed.iter().all(|cp| set.contains(cp)).then_some(*name));

        if let Some(fallback_name) = fallback {
            warn!(
                "[FONT-DIAG] '{}' missing in assigned '{}', probable substitution via '{}'.",
                symbol, assigned_font, fallback_name
            );
            info!("[FONT-DIAG] source='{}' symbol='{}'", label, symbol);
        } else {
            warn!(
                "[FONT-DIAG] '{}' missing in assigned '{}' and no known fallback covers it.",
                symbol, assigned_font
            );
        }
    }
}

/// Setup camera for 2D rendering
pub fn setup_camera(mut commands: Commands) {
    // Default Camera2d with default scale shows roughly the full window area
    commands.spawn(Camera2d);
    eprintln!("[SETUP] Camera spawned");
}
