use bevy::prelude::*;

/// Setup camera for 2D rendering
pub fn setup_camera(mut commands: Commands) {
    // Default Camera2d with default scale shows roughly the full window area
    commands.spawn(Camera2dBundle::default());
    eprintln!("[SETUP] Camera spawned");
}
