use bevy::prelude::*;

/// Game font resource — stores the loaded Tektur font handle.
///
/// All UI text in menus and HUDs references `font.0.clone()` instead of
/// using the default Bevy font.  Created by [`load_game_font`] at startup.
#[derive(Resource, Default)]
pub struct GameFont(pub Handle<Font>);

/// Load the custom Tektur font from assets at startup.
///
/// Must run before any UI setup systems that spawn text.
pub fn load_game_font(mut font: ResMut<GameFont>, asset_server: Res<AssetServer>) {
    let font_handle = asset_server.load("fonts/Tektur/Tektur-VariableFont_wdth,wght.ttf");
    font.0 = font_handle;
    eprintln!("[SETUP] Game font loaded");
}

/// Setup camera for 2D rendering
pub fn setup_camera(mut commands: Commands) {
    // Default Camera2d with default scale shows roughly the full window area
    commands.spawn(Camera2d);
    eprintln!("[SETUP] Camera spawned");
}
