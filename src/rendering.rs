//! Rendering systems: stats overlay, debug control panel, and gizmo overlays.
//!
//! ## Layer Model
//!
//! | Layer              | Technology   | Default | Controlled by           |
//! |--------------------|--------------|---------|-------------------------|
//! | Asteroid fills     | `Mesh2d`     | ON      | `wireframe_only` flag   |
//! | Wireframe outlines | `Mesh2d`     | OFF     | `wireframe_only` (swap) |
//! | Gizmo wf overlay   | Gizmos       | OFF     | `show_wireframes`       |
//! | Force vectors      | Gizmos       | OFF     | `show_force_vectors`    |
//! | Velocity arrows    | Gizmos       | OFF     | `show_velocity_arrows`  |
//! | Culling boundary   | `Mesh2d`     | OFF     | `show_boundary`         |
//! | Player ship fill   | `Mesh2d`     | ON      | `wireframe_only` flag   |
//! | Ship outline       | Gizmos       | OFF     | `show_ship_outline`     |
//! | Aim indicator      | `Mesh2d`     | OFF     | `show_aim_indicator`    |
//! | Projectile fills   | `Mesh2d`     | ON      | `wireframe_only` flag   |
//! | Projectile outline | Gizmos       | OFF     | `show_projectile_outline`|
//! | Health bar         | `Mesh2d`     | always  | —                       |
//! | Stats overlay      | Bevy UI      | OFF     | `show_stats`            |
//! | Score HUD          | Bevy UI      | always  | —                       |
//! | Debug panel        | Bevy UI      | hidden  | Pause menu button       |
//!
//! ## System Responsibilities
//!
//! | System                        | Schedule | Purpose                             |
//! |-------------------------------|----------|-------------------------------------|
//! | `setup_boundary_ring`         | Startup  | Spawn retained GPU boundary ring    |
//! | `setup_stats_text`            | Startup  | Spawn fixed stats text node         |
//! | `setup_debug_panel`           | Startup  | Spawn collapsible debug panel       |
//! | `setup_hud_score`             | Startup  | Spawn permanent score HUD node      |
//! | `setup_stats_overlay`         | Startup  | Spawn toggleable stats overlay node |
//! | `stats_display_system`        | Update   | Refresh live/culled/merged text     |
//! | `hud_score_display_system`    | Update   | Refresh score HUD text              |
//! | `sync_boundary_ring_visibility_system` | Update | Show/hide boundary ring   |
//! | `sync_stats_overlay_visibility_system` | Update | Show/hide stats overlay   |
//! | `debug_panel_button_system`   | Update   | Process toggle button clicks        |
//! | `gizmo_rendering_system`      | Update   | Draw gizmo overlays per OverlayState|

use crate::asteroid::{Asteroid, GravityForce, Vertices};
use crate::asteroid_rendering::ring_mesh;
use crate::config::PhysicsConfig;
use crate::graphics::GameFont;
use crate::mining::PlayerOre;
use crate::player::state::MissileAmmo;
use crate::player::Player;
use crate::player::{PlayerLives, PlayerScore, PrimaryWeaponLevel};
use crate::simulation::{ProfilerStats, SimulationStats};
use crate::spatial_partition::SpatialGrid;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use bevy_rapier2d::prelude::{ReadRapierContext, Velocity};
use std::collections::HashMap;

// ── Overlay state resource ────────────────────────────────────────────────────

/// Controls which debug overlay layers are rendered at runtime.
///
/// Mutated by the debug panel UI; read by all gizmo rendering systems.
#[derive(Resource, Clone, Debug, Default)]
pub struct OverlayState {
    /// Draw translucent polygon outlines over the `Mesh2d` asteroid fills.
    pub show_wireframes: bool,
    /// Draw red force-vector lines per asteroid (capped at `force_vector_hide_threshold`).
    pub show_force_vectors: bool,
    /// Draw the yellow culling-boundary circle at the world origin.
    pub show_boundary: bool,
    /// Draw cyan velocity arrows on each asteroid.
    pub show_velocity_arrows: bool,
    /// Hide `Mesh2d` fills and render asteroids as white gizmo wireframes only.
    pub wireframe_only: bool,
    /// Whether the debug panel is currently visible.
    pub menu_open: bool,
    /// Draw a gizmo wireframe outline over the `Mesh2d` player ship.
    pub show_ship_outline: bool,
    /// Draw the orange aim-direction indicator from the player ship.
    pub show_aim_indicator: bool,
    /// Draw gizmo circle outlines over the `Mesh2d` projectile fills.
    pub show_projectile_outline: bool,
    /// Draw spatial partition split-cell lines from the KD-tree index.
    pub show_debug_grid: bool,
    /// Show in-game profiler timings overlay.
    pub show_profiler: bool,
    /// Show the simulation statistics overlay (Live/Culled/Merged/Split/Destroyed).
    pub show_stats: bool,
    /// Show the physics inspector overlay (entity IDs, velocities, contacts).
    pub show_physics_inspector: bool,
}

// ── Component markers ─────────────────────────────────────────────────────────

/// Marker for the stats text root node.
#[derive(Component)]
pub struct StatsTextDisplay;

/// Marker for the permanent score HUD node.
#[derive(Component)]
pub struct HudScoreDisplay;

/// Marker for the retrained GPU boundary-ring entity.
#[derive(Component)]
pub struct BoundaryRing;

/// Marker for the debug overlay panel root node.
#[derive(Component)]
pub struct DebugPanel;

/// Marker for the lives / respawn-countdown HUD node.
#[derive(Component)]
pub struct LivesHudDisplay;

/// Marker for the respawn-countdown text within the lives HUD.
#[derive(Component)]
pub struct RespawnCountdownText;

/// Marker for the missile-ammo HUD node (row 3, below lives HUD).
#[derive(Component)]
pub struct MissileHudDisplay;

/// Marker for the ore-count HUD node (row 4, below missiles HUD).
#[derive(Component)]
pub struct OreHudDisplay;

/// Marker for the physics-inspector text node.
#[derive(Component)]
pub struct PhysicsInspectorDisplay;

/// Marker for the profiler text node.
#[derive(Component)]
pub struct ProfilerDisplay;

/// Tags a toggle button in the debug panel with the overlay field it controls.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OverlayToggle {
    Wireframes,
    ForceVectors,
    Boundary,
    VelocityArrows,
    WireframeOnly,
    ShipOutline,
    AimIndicator,
    ProjectileOutline,
    DebugGrid,
    Profiler,
    StatsOverlay,
    PhysicsInspector,
}

impl OverlayToggle {
    /// Read the current value of this toggle from [`OverlayState`].
    pub fn get(self, state: &OverlayState) -> bool {
        match self {
            Self::Wireframes => state.show_wireframes,
            Self::ForceVectors => state.show_force_vectors,
            Self::Boundary => state.show_boundary,
            Self::VelocityArrows => state.show_velocity_arrows,
            Self::WireframeOnly => state.wireframe_only,
            Self::ShipOutline => state.show_ship_outline,
            Self::AimIndicator => state.show_aim_indicator,
            Self::ProjectileOutline => state.show_projectile_outline,
            Self::DebugGrid => state.show_debug_grid,
            Self::Profiler => state.show_profiler,
            Self::StatsOverlay => state.show_stats,
            Self::PhysicsInspector => state.show_physics_inspector,
        }
    }

    /// Flip the corresponding field in [`OverlayState`].
    pub fn toggle(self, state: &mut OverlayState) {
        match self {
            Self::Wireframes => state.show_wireframes = !state.show_wireframes,
            Self::ForceVectors => state.show_force_vectors = !state.show_force_vectors,
            Self::Boundary => state.show_boundary = !state.show_boundary,
            Self::VelocityArrows => state.show_velocity_arrows = !state.show_velocity_arrows,
            Self::WireframeOnly => state.wireframe_only = !state.wireframe_only,
            Self::ShipOutline => state.show_ship_outline = !state.show_ship_outline,
            Self::AimIndicator => state.show_aim_indicator = !state.show_aim_indicator,
            Self::ProjectileOutline => {
                state.show_projectile_outline = !state.show_projectile_outline;
            }
            Self::DebugGrid => state.show_debug_grid = !state.show_debug_grid,
            Self::Profiler => state.show_profiler = !state.show_profiler,
            Self::StatsOverlay => state.show_stats = !state.show_stats,
            Self::PhysicsInspector => {
                state.show_physics_inspector = !state.show_physics_inspector;
            }
        }
    }

    /// Human-readable label displayed next to the toggle button.
    pub fn label(self) -> &'static str {
        match self {
            Self::Wireframes => "Wireframe Outlines",
            Self::ForceVectors => "Force Vectors",
            Self::Boundary => "Culling Boundary",
            Self::VelocityArrows => "Velocity Arrows",
            Self::WireframeOnly => "Wireframe-Only Mode",
            Self::ShipOutline => "Ship Outline",
            Self::AimIndicator => "Aim Indicator",
            Self::ProjectileOutline => "Projectile Outline",
            Self::DebugGrid => "Spatial Grid",
            Self::Profiler => "Profiler",
            Self::StatsOverlay => "Stats Overlay",
            Self::PhysicsInspector => "Physics Inspector",
        }
    }
}

// ── Colour helpers ────────────────────────────────────────────────────────────

fn on_bg() -> Color {
    Color::srgb(0.08, 0.44, 0.12)
}
fn off_bg() -> Color {
    Color::srgb(0.35, 0.07, 0.07)
}
fn on_text() -> Color {
    Color::srgb(0.75, 1.0, 0.80)
}
fn off_text() -> Color {
    Color::srgb(0.65, 0.65, 0.65)
}

// ── Startup: boundary ring ───────────────────────────────────────────────────

/// Spawn the cull-boundary indicator as a retained GPU ring mesh.
///
/// This replaces the previous per-frame `gizmos.circle_2d()` call with a
/// static `Mesh2d` entity that has **zero per-frame CPU cost** — its
/// visibility is toggled once when `show_boundary` changes.
///
/// Must be ordered after [`crate::config::load_physics_config`] so the
/// correct `cull_distance` is used.
pub fn setup_boundary_ring(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    config: Res<PhysicsConfig>,
) {
    // 3-unit thick ring at the cull boundary; yellow to match previous gizmo.
    let mesh = meshes.add(ring_mesh(config.cull_distance, 3.0, 128));
    let mat = materials.add(ColorMaterial::from_color(Color::srgba(1.0, 1.0, 0.0, 0.85)));
    commands.spawn((
        Mesh2d(mesh),
        MeshMaterial2d(mat),
        Transform::from_translation(Vec3::new(0.0, 0.0, -0.5)), // behind asteroids
        Visibility::Hidden, // off by default, toggled by show_boundary flag
        BoundaryRing,
    ));
}

/// Show or hide the boundary ring when `show_boundary` changes.
///
/// Only re-runs when [`OverlayState`] is mutated — zero overhead every other frame.
pub fn sync_boundary_ring_visibility_system(
    overlay: Res<OverlayState>,
    mut query: Query<&mut Visibility, With<BoundaryRing>>,
) {
    if !overlay.is_changed() {
        return;
    }
    let vis = if overlay.show_boundary {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut v in query.iter_mut() {
        *v = vis;
    }
}

// ── Startup: score HUD ────────────────────────────────────────────────────────

/// Spawn the permanent top-left score HUD (always visible).
pub fn setup_hud_score(mut commands: Commands, config: Res<PhysicsConfig>, font: Res<GameFont>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                ..default()
            },
            HudScoreDisplay,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Score: 0"),
                TextFont {
                    font: font.0.clone(),
                    font_size: config.stats_font_size,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.88, 0.45)),
            ));
        });
}

// ── Startup: stats overlay text ───────────────────────────────────────────────

/// Spawn the lives counter and respawn-countdown HUD (always visible during play).
///
/// Structure (top-left column, below score):
/// ```text
///  Lives: ♥ ♥ ♥
///  RESPAWNING IN 2.4s   ← hidden while alive
/// ```
pub fn setup_lives_hud(mut commands: Commands, config: Res<PhysicsConfig>, font: Res<GameFont>) {
    let row_h = config.stats_font_size + 6.0;
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0 + row_h),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            },
            LivesHudDisplay,
        ))
        .with_children(|parent| {
            // Lives counter row
            parent.spawn((
                Text::new("Lives: * * *"),
                TextFont {
                    font: font.0.clone(),
                    font_size: config.stats_font_size,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.45, 0.45)),
            ));
            // Respawn countdown — hidden while alive
            parent.spawn((
                Text::new(""),
                TextFont {
                    font: font.0.clone(),
                    font_size: config.stats_font_size - 2.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.75, 0.0)),
                Visibility::Hidden,
                RespawnCountdownText,
            ));
        });
}

/// Refresh the lives HUD and respawn-countdown text each frame.
pub fn lives_hud_display_system(
    lives: Res<PlayerLives>,
    config: Res<PhysicsConfig>,
    parent_query: Query<&Children, With<LivesHudDisplay>>,
    mut text_query: Query<(&mut Text, &mut Visibility, Option<&RespawnCountdownText>)>,
) {
    if !lives.is_changed() {
        return;
    }
    for children in parent_query.iter() {
        for child in children.iter() {
            let Ok((mut text, mut vis, respawn_tag)) = text_query.get_mut(child) else {
                continue;
            };
            if respawn_tag.is_some() {
                // Respawn countdown text
                if let Some(t) = lives.respawn_timer {
                    *text = Text::new(format!("RESPAWNING IN {t:.1}s…"));
                    *vis = Visibility::Visible;
                } else {
                    *text = Text::new("");
                    *vis = Visibility::Hidden;
                }
            } else {
                // Lives counter — filled (*) and lost (-) markers
                let total = config.player_lives.max(0) as usize;
                let filled = lives.remaining.max(0) as usize;
                let stars: String =
                    "* ".repeat(filled) + &"- ".repeat(total.saturating_sub(filled));
                *text = Text::new(format!("Lives: {}", stars.trim_end()));
            }
        }
    }
}

/// Startup: spawn the missile-ammo indicator HUD (row 3, below lives HUD).
pub fn setup_missile_hud(mut commands: Commands, config: Res<PhysicsConfig>, font: Res<GameFont>) {
    let row_h = config.stats_font_size + 6.0;
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0 + row_h * 3.0),
                ..default()
            },
            MissileHudDisplay,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Missiles: M M M M M"),
                TextFont {
                    font: font.0.clone(),
                    font_size: config.stats_font_size,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.55, 0.1)),
            ));
        });
}

/// Refresh the missile ammo HUD each frame.
pub fn missile_hud_display_system(
    ammo: Res<MissileAmmo>,
    config: Res<PhysicsConfig>,
    parent_query: Query<&Children, With<MissileHudDisplay>>,
    mut text_query: Query<&mut Text>,
) {
    if !ammo.is_changed() {
        return;
    }
    for children in parent_query.iter() {
        for child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                let max = config.missile_ammo_max as usize;
                let count = ammo.count as usize;
                let filled: String = "M ".repeat(count) + &"- ".repeat(max.saturating_sub(count));
                *text = Text::new(format!("Missiles: {}", filled.trim_end()));
            }
        }
    }
}

/// Startup: ore-count HUD (row 4, below missiles).
pub fn setup_ore_hud(mut commands: Commands, config: Res<PhysicsConfig>, font: Res<GameFont>) {
    let row_h = config.stats_font_size + 6.0;
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0 + row_h * 4.0),
                ..default()
            },
            OreHudDisplay,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Ore: 0"),
                TextFont {
                    font: font.0.clone(),
                    font_size: config.stats_font_size,
                    ..default()
                },
                TextColor(Color::srgb(0.35, 1.0, 0.55)),
            ));
        });
}

/// Refresh the ore-count HUD each frame.
///
/// When ore > 0 the text includes key-binding hints for spending it so players
/// can discover the mechanic passively.  The primary weapon upgrade level is
/// shown inline so players always know their current tier.
pub fn ore_hud_display_system(
    ore: Res<PlayerOre>,
    weapon_level: Res<PrimaryWeaponLevel>,
    parent_query: Query<&Children, With<OreHudDisplay>>,
    mut text_query: Query<&mut Text>,
) {
    if !ore.is_changed() && !weapon_level.is_changed() {
        return;
    }
    let level_str = if weapon_level.is_maxed() {
        " | Wpn: MAX".to_string()
    } else {
        format!(" | Wpn: Lv.{}", weapon_level.display_level())
    };
    let display = if ore.count > 0 {
        format!("Ore: {}{}", ore.count, level_str)
    } else {
        format!("Ore: 0{}", level_str)
    };
    for children in parent_query.iter() {
        for child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                *text = Text::new(display.clone());
            }
        }
    }
}

/// Startup: stats overlay text — Spawn the toggleable simulation-stats overlay (starts hidden; enable via debug panel).
pub fn setup_stats_text(mut commands: Commands, config: Res<PhysicsConfig>, font: Res<GameFont>) {
    let row_h = config.stats_font_size + 6.0;
    // Position below score (row 0), lives HUD (rows 1-2), missile HUD (row 3), ore HUD (row 4).
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0 + row_h * 5.0),
                ..default()
            },
            StatsTextDisplay,
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Live: 0 | Culled: 0 | Merged: 0 | Split: 0 | Destroyed: 0"),
                TextFont {
                    font: font.0.clone(),
                    font_size: config.stats_font_size,
                    ..default()
                },
                TextColor(Color::srgb(0.0, 1.0, 1.0)),
            ));
        });
}

/// Startup: spawn physics-inspector text overlay (hidden by default).
pub fn setup_physics_inspector_text(
    mut commands: Commands,
    config: Res<PhysicsConfig>,
    font: Res<GameFont>,
) {
    let row_h = config.stats_font_size + 6.0;
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0 + row_h * 6.0),
                ..default()
            },
            PhysicsInspectorDisplay,
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Physics Inspector\n(no data)"),
                TextFont {
                    font: font.0.clone(),
                    font_size: (config.stats_font_size - 4.0).max(10.0),
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.95, 0.95)),
            ));
        });
}

/// Startup: spawn profiler text overlay (hidden by default).
pub fn setup_profiler_text(
    mut commands: Commands,
    config: Res<PhysicsConfig>,
    font: Res<GameFont>,
) {
    let row_h = config.stats_font_size + 6.0;
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0 + row_h * 12.0),
                ..default()
            },
            ProfilerDisplay,
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Profiler\n(waiting for timing samples...)"),
                TextFont {
                    font: font.0.clone(),
                    font_size: (config.stats_font_size - 3.0).max(10.0),
                    ..default()
                },
                TextColor(Color::srgb(0.88, 0.95, 0.75)),
            ));
        });
}

// ── Startup: debug panel ──────────────────────────────────────────────────────

/// Spawn the debug overlay panel (hidden until the user presses ESC).
///
/// The panel appears in the top-right corner and provides per-layer toggle
/// buttons for all gizmo overlays plus a wireframe-only fallback mode.
pub fn setup_debug_panel(mut commands: Commands, font: Res<GameFont>) {
    // Each entry: (toggle variant, initial "active" state) — must match OverlayState::default().
    let defaults: &[(OverlayToggle, bool)] = &[
        (OverlayToggle::Boundary, false),
        (OverlayToggle::Wireframes, false),
        (OverlayToggle::ForceVectors, false),
        (OverlayToggle::VelocityArrows, false),
        (OverlayToggle::WireframeOnly, false),
        (OverlayToggle::AimIndicator, false),
        (OverlayToggle::ShipOutline, false),
        (OverlayToggle::ProjectileOutline, false),
        (OverlayToggle::DebugGrid, false),
        (OverlayToggle::Profiler, false),
        (OverlayToggle::StatsOverlay, false),
        (OverlayToggle::PhysicsInspector, false),
    ];

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(10.0),
                width: Val::Px(235.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(6.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.93)),
            BorderColor::all(Color::srgb(0.32, 0.32, 0.44)),
            DebugPanel,
            Visibility::Hidden,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Debug Overlays"),
                TextFont {
                    font: font.0.clone(),
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.88, 0.45)),
            ));

            panel.spawn((
                Text::new("──────────────────────────────"),
                TextFont {
                    font: font.0.clone(),
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgb(0.28, 0.28, 0.38)),
            ));

            for &(toggle, initial) in defaults {
                spawn_toggle_row(panel, toggle, initial, &font);
            }

            panel.spawn((
                Text::new("──────────────────────────────"),
                TextFont {
                    font: font.0.clone(),
                    font_size: 9.0,
                    ..default()
                },
                TextColor(Color::srgb(0.28, 0.28, 0.38)),
            ));

            panel.spawn((
                Text::new("Tip: toggle in pause menu (ESC)"),
                TextFont {
                    font: font.0.clone(),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgb(0.42, 0.42, 0.52)),
            ));
        });
}

/// Spawn one toggle row: `[ON | OFF]  Label text`.
fn spawn_toggle_row(
    parent: &mut ChildSpawnerCommands<'_>,
    toggle: OverlayToggle,
    initial: bool,
    font: &GameFont,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(7.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Button,
                Node {
                    width: Val::Px(40.0),
                    height: Val::Px(19.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(if initial { on_bg() } else { off_bg() }),
                BorderColor::all(Color::srgb(0.5, 0.5, 0.5)),
                toggle,
            ))
            .with_children(|btn| {
                btn.spawn((
                    Text::new(if initial { "ON" } else { "OFF" }),
                    TextFont {
                        font: font.0.clone(),
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(if initial { on_text() } else { off_text() }),
                ));
            });

            row.spawn((
                Text::new(toggle.label()),
                TextFont {
                    font: font.0.clone(),
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.85, 0.88)),
            ));
        });
}

// ── Update: score HUD ─────────────────────────────────────────────────

/// Refresh the permanent score HUD each frame.
pub fn hud_score_display_system(
    score: Res<PlayerScore>,
    parent_query: Query<&Children, With<HudScoreDisplay>>,
    mut text_query: Query<&mut Text>,
) {
    if !score.is_changed() {
        return;
    }
    for children in parent_query.iter() {
        for child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                let multiplier = score.multiplier();
                if multiplier > 1 {
                    *text = Text::new(format!(
                        "Score: {}  ({} hits, {} destroyed)  ×{} COMBO! [{}]",
                        score.total(),
                        score.hits,
                        score.destroyed,
                        multiplier,
                        score.streak,
                    ));
                } else {
                    *text = Text::new(format!(
                        "Score: {}  ({} hits, {} destroyed)",
                        score.total(),
                        score.hits,
                        score.destroyed
                    ));
                }
            }
        }
    }
}

// ── Update: stats overlay visibility ────────────────────────────────

/// Show or hide the simulation stats overlay based on [`OverlayState::show_stats`].
///
/// Only re-runs when `OverlayState` changes (negligible per-frame cost).
pub fn sync_stats_overlay_visibility_system(
    overlay: Res<OverlayState>,
    mut query: Query<&mut Visibility, With<StatsTextDisplay>>,
) {
    if !overlay.is_changed() {
        return;
    }
    let vis = if overlay.show_stats {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut v in query.iter_mut() {
        *v = vis;
    }
}

/// Show or hide the physics inspector overlay based on [`OverlayState`].
pub fn sync_physics_inspector_visibility_system(
    overlay: Res<OverlayState>,
    mut query: Query<&mut Visibility, With<PhysicsInspectorDisplay>>,
) {
    if !overlay.is_changed() {
        return;
    }
    let vis = if overlay.show_physics_inspector {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut v in query.iter_mut() {
        *v = vis;
    }
}

/// Show or hide the profiler overlay based on [`OverlayState`].
pub fn sync_profiler_visibility_system(
    overlay: Res<OverlayState>,
    mut query: Query<&mut Visibility, With<ProfilerDisplay>>,
) {
    if !overlay.is_changed() {
        return;
    }
    let vis = if overlay.show_profiler {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut v in query.iter_mut() {
        *v = vis;
    }
}

// ── Update: stats text ────────────────────────────────────────────────────────

/// Refresh the stats text content each frame.
pub fn stats_display_system(
    stats: Res<SimulationStats>,
    parent_query: Query<&Children, With<StatsTextDisplay>>,
    mut text_query: Query<&mut Text>,
) {
    for children in parent_query.iter() {
        for child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                *text = Text::new(format!(
                    "Live: {} | Culled: {} | Merged: {} | Split: {} | Destroyed: {}",
                    stats.live_count,
                    stats.culled_total,
                    stats.merged_total,
                    stats.split_total,
                    stats.destroyed_total
                ));
            }
        }
    }
}

/// Refresh the physics inspector text with IDs, velocities, and active contact counts.
pub fn physics_inspector_display_system(
    overlay: Res<OverlayState>,
    parent_query: Query<&Children, With<PhysicsInspectorDisplay>>,
    mut text_query: Query<&mut Text>,
    q_player: Query<(Entity, &Transform, &Velocity), With<Player>>,
    q_asteroids: Query<(Entity, &Transform, &Velocity), With<Asteroid>>,
    rapier_context: ReadRapierContext,
) {
    if !overlay.show_physics_inspector {
        return;
    }

    let mut contact_counts: HashMap<Entity, u32> = HashMap::new();
    let mut active_pairs = 0_u32;
    if let Ok(rapier) = rapier_context.single() {
        for pair in rapier
            .simulation
            .contact_pairs(rapier.colliders, rapier.rigidbody_set)
        {
            if !pair.has_any_active_contact() {
                continue;
            }
            active_pairs += 1;
            if let Some(e1) = pair.collider1() {
                *contact_counts.entry(e1).or_default() += 1;
            }
            if let Some(e2) = pair.collider2() {
                *contact_counts.entry(e2).or_default() += 1;
            }
        }
    }

    let mut lines = Vec::with_capacity(10);
    lines.push(format!("Contacts(active pairs): {active_pairs}"));

    if let Ok((entity, transform, velocity)) = q_player.single() {
        let p = transform.translation.truncate();
        let v = velocity.linvel;
        let c = contact_counts.get(&entity).copied().unwrap_or(0);
        lines.push(format!(
            "Player id={} pos=({:.0},{:.0}) vel=({:.1},{:.1}) contacts={}",
            entity.index(),
            p.x,
            p.y,
            v.x,
            v.y,
            c
        ));
    } else {
        lines.push("Player: none".to_string());
    }

    lines.push(format!("Asteroids: {}", q_asteroids.iter().len()));

    for (i, (entity, transform, velocity)) in q_asteroids.iter().take(4).enumerate() {
        let p = transform.translation.truncate();
        let v = velocity.linvel;
        let c = contact_counts.get(&entity).copied().unwrap_or(0);
        lines.push(format!(
            "A{} id={} pos=({:.0},{:.0}) vel=({:.1},{:.1}) c={}",
            i,
            entity.index(),
            p.x,
            p.y,
            v.x,
            v.y,
            c
        ));
    }

    let display = lines.join("\n");
    for children in parent_query.iter() {
        for child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                *text = Text::new(display.clone());
            }
        }
    }
}

/// Refresh profiler text with frame-time and schedule timing breakdown.
pub fn profiler_display_system(
    overlay: Res<OverlayState>,
    profiler: Res<ProfilerStats>,
    diagnostics: Res<DiagnosticsStore>,
    parent_query: Query<&Children, With<ProfilerDisplay>>,
    mut text_query: Query<&mut Text>,
) {
    if !overlay.show_profiler {
        return;
    }

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);
    let frame_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let display = format!(
        "Profiler\nFrame: {frame_ms:.2} ms ({fps:.1} FPS)\n\nECS/Update\n  Group1(Input+Core): {g1:.2} ms\n  Group2A(Mesh+Camera): {g2a:.2} ms\n  Group2B(Overlay+Player): {g2b:.2} ms\n  Update Total: {ut:.2} ms\n\nPhysics\n  FixedUpdate: {fx:.2} ms\n  PostUpdate: {po:.2} ms",
        g1 = profiler.update_group1_ms,
        g2a = profiler.update_group2a_ms,
        g2b = profiler.update_group2b_ms,
        ut = profiler.update_total_ms,
        fx = profiler.fixed_update_ms,
        po = profiler.post_update_ms,
    );

    for children in parent_query.iter() {
        for child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                *text = Text::new(display.clone());
            }
        }
    }
}

// ── Update: toggle button interaction ────────────────────────────────────────

/// Handle clicks on debug panel toggle buttons.
///
/// On press: flip the overlay flag, update button background colour and text.
#[allow(clippy::type_complexity)]
pub fn debug_panel_button_system(
    mut overlay: ResMut<OverlayState>,
    mut btn_query: Query<
        (
            &Interaction,
            &OverlayToggle,
            &Children,
            &mut BackgroundColor,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<(&mut Text, &mut TextColor)>,
) {
    for (interaction, &toggle, children, mut bg) in btn_query.iter_mut() {
        if *interaction != Interaction::Pressed {
            continue;
        }
        toggle.toggle(&mut overlay);
        let active = toggle.get(&overlay);

        *bg = BackgroundColor(if active { on_bg() } else { off_bg() });

        for child in children.iter() {
            if let Ok((mut text, mut color)) = text_query.get_mut(child) {
                *text = Text::new(if active { "ON" } else { "OFF" });
                *color = TextColor(if active { on_text() } else { off_text() });
            }
        }
    }
}

// ── Update: gizmo overlay rendering ──────────────────────────────────────────

/// Draw all enabled gizmo overlay layers based on [`OverlayState`].
///
/// The culling boundary circle and asteroid wireframes in `wireframe_only` mode
/// are now handled by retained `Mesh2d` entities — this system only handles the
/// semi-transparent additive overlay (`show_wireframes`), force vectors, and
/// velocity arrows.
pub fn gizmo_rendering_system(
    mut gizmos: Gizmos,
    query: Query<(&Transform, &Vertices, &GravityForce, &Velocity), With<Asteroid>>,
    stats: Res<SimulationStats>,
    config: Res<PhysicsConfig>,
    grid: Res<SpatialGrid>,
    mut split_lines: Local<Vec<(Vec2, Vec2)>>,
    overlay: Res<OverlayState>,
) {
    // ── Wireframe outlines (semi-transparent additive overlay) ─────────────────
    // NOTE: wireframe_only mode now uses retained Mesh2d polygon-outline meshes
    // (see sync_asteroid_render_mode_system). This branch only handles the
    // show_wireframes semi-transparent overlay on top of fills.
    if overlay.show_wireframes {
        for (transform, vertices, _, _) in query.iter() {
            if vertices.0.len() < 2 {
                continue;
            }
            let pos = transform.translation.truncate();
            let rot = transform.rotation;
            let n = vertices.0.len();
            for i in 0..n {
                let v1 = vertices.0[i];
                let v2 = vertices.0[(i + 1) % n];
                let p1 = pos + rot.mul_vec3(v1.extend(0.0)).truncate();
                let p2 = pos + rot.mul_vec3(v2.extend(0.0)).truncate();
                gizmos.line_2d(p1, p2, Color::srgba(1.0, 1.0, 1.0, 0.4));
            }
        }
    }

    // ── Force vectors ─────────────────────────────────────────────────────────
    if overlay.show_force_vectors && stats.live_count < config.force_vector_hide_threshold {
        for (transform, _, grav, _) in query.iter() {
            let pos = transform.translation.truncate();
            let force_vec = grav.0 * config.force_vector_display_scale;
            if force_vec.length() > config.force_vector_min_length {
                gizmos.line_2d(pos, pos + force_vec, Color::srgb(1.0, 0.15, 0.15));
            }
        }
    }

    // ── Velocity arrows ───────────────────────────────────────────────────────
    if overlay.show_velocity_arrows {
        for (transform, _, _, vel) in query.iter() {
            let pos = transform.translation.truncate();
            let v = vel.linvel;
            if v.length_squared() > 0.5 {
                let tip = pos + v * 0.15;
                gizmos.line_2d(pos, tip, Color::srgb(0.2, 0.8, 1.0));
                gizmos.circle_2d(tip, 1.5, Color::srgb(0.2, 0.8, 1.0));
            }
        }
    }

    // ── Spatial partition split cells (KD-tree) ─────────────────────────────
    if overlay.show_debug_grid {
        let half = config.cull_distance;
        let min = Vec2::new(-half, -half);
        let max = Vec2::new(half, half);
        grid.collect_debug_split_lines(min, max, &mut split_lines);
        for (a, b) in split_lines.iter().copied() {
            gizmos.line_2d(a, b, Color::srgba(0.35, 0.95, 0.35, 0.45));
        }
    }

    // NOTE: The culling boundary circle is now a retained Mesh2d ring entity
    // managed by `setup_boundary_ring` + `sync_boundary_ring_visibility_system`.
    // No per-frame gizmo call needed here.
}
