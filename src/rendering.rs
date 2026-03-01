//! Rendering systems: stats overlay, debug control panel, and gizmo overlays.
//!
//! ## Layer Model
//!
//! | Layer              | Technology   | Default | Controlled by           |
//! |--------------------|--------------|---------|-------------------------|
//! | Asteroid fills     | `Mesh2d`     | ON      | `wireframe_only` flag   |
//! | Wireframe outlines | `Mesh2d`     | OFF     | `wireframe_only` (swap) |
//! | Wireframe overlay  | `Mesh2d`     | OFF     | `show_wireframes`       |
//! | Force vectors      | `Mesh2d`     | OFF     | `show_force_vectors`    |
//! | Velocity arrows    | `Mesh2d`     | OFF     | `show_velocity_arrows`  |
//! | Culling boundary   | `Mesh2d`     | OFF     | `show_boundary`         |
//! | Player ship fill   | `Mesh2d`     | ON      | `wireframe_only` flag   |
//! | Ship outline       | `Mesh2d`     | OFF     | `show_ship_outline`     |
//! | Aim indicator      | `Mesh2d`     | OFF     | `show_aim_indicator`    |
//! | Projectile fills   | `Mesh2d`     | ON      | `wireframe_only` flag   |
//! | Projectile outline | `Mesh2d`     | OFF     | `show_projectile_outline`|
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
//! | `sync_debug_line_layers_system` | Update | Refresh retained debug line layers   |

use crate::asteroid::{Asteroid, GravityForce, Vertices};
use crate::asteroid_rendering::ring_mesh;
use crate::config::PhysicsConfig;
use crate::graphics::{EmojiFont, GameFont, SymbolFont, SymbolFont2, UnicodeFallbackFont};
use crate::mining::{OreAffinityLevel, PlayerOre};
use crate::player::state::MissileAmmo;
use crate::player::Player;
use crate::player::{
    IonCannonCooldown, IonCannonLevel, PlayerLives, PlayerScore, PrimaryWeaponLevel,
    SecondaryWeaponLevel, TractorBeamLevel, TractorHoldState, TractorThrowCooldown,
};
use crate::simulation::{ProfilerStats, SimulationStats};
use crate::spatial_partition::SpatialGrid;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use bevy_mesh::{Indices, PrimitiveTopology};
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

/// Marker for simulation stats text child.
#[derive(Component)]
pub struct StatsOverlayText;

/// Marker for the permanent score HUD node.
#[derive(Component)]
pub struct HudScoreDisplay;

/// Marker for score HUD text child.
#[derive(Component)]
pub struct HudScoreText;

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

/// Marker for the lives numeric readout text.
#[derive(Component)]
pub struct LivesHudValueText;

/// Marker for the missile-ammo HUD node (row 3, below lives HUD).
#[derive(Component)]
pub struct MissileHudDisplay;

/// Marker for the missile ammo numeric readout text.
#[derive(Component)]
pub struct MissileHudValueText;

/// Marker for the ore-count HUD node (row 4, below missiles HUD).
#[derive(Component)]
pub struct OreHudDisplay;

/// Marker for ore count readout text.
#[derive(Component)]
pub struct OreHudValueText;

/// Marker for blaster level readout text.
#[derive(Component)]
pub struct BlasterHudValueText;

/// Marker for missile level readout text.
#[derive(Component)]
pub struct MissileLevelHudValueText;

/// Marker for ore magnet level readout text.
#[derive(Component)]
pub struct MagnetHudValueText;

/// Marker for tractor level and state readout text.
#[derive(Component)]
pub struct TractorHudValueText;

/// Marker for ion cannon level and cooldown readout text.
#[derive(Component)]
pub struct IonHudValueText;

/// Marker for the physics-inspector text node.
#[derive(Component)]
pub struct PhysicsInspectorDisplay;

/// Marker for physics-inspector text child.
#[derive(Component)]
pub struct PhysicsInspectorText;

/// Marker for the profiler text node.
#[derive(Component)]
pub struct ProfilerDisplay;

/// Marker for profiler text child.
#[derive(Component)]
pub struct ProfilerText;

/// Marker for retained asteroid wireframe overlay line mesh.
#[derive(Component)]
pub struct WireframeOverlayLayer;

/// Marker for retained force-vector overlay line mesh.
#[derive(Component)]
pub struct ForceVectorLayer;

/// Marker for retained velocity-arrow overlay line mesh.
#[derive(Component)]
pub struct VelocityArrowLayer;

/// Marker for retained spatial-grid overlay line mesh.
#[derive(Component)]
pub struct SpatialGridLayer;

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

fn circled_number_level(level: u32) -> &'static str {
    match level {
        1 => "①",
        2 => "②",
        3 => "③",
        4 => "④",
        5 => "⑤",
        6 => "⑥",
        7 => "⑦",
        8 => "⑧",
        9 => "⑨",
        10 => "⑩",
        _ => "?",
    }
}

fn repeated_symbol(symbol: &str, count: u32) -> String {
    let mut out = String::new();
    for index in 0..count {
        if index > 0 {
            out.push(' ');
        }
        out.push_str(symbol);
    }
    out
}

fn slot_indicator(available: u32, max_slots: u32) -> String {
    let clamped = available.min(max_slots);
    let mut out = String::new();
    for index in 0..max_slots {
        if index > 0 {
            out.push(' ');
        }
        out.push_str(if index < clamped { "●" } else { "○" });
    }
    out
}

fn line_segments_mesh(segments: &[(Vec2, Vec2)], half_width: f32) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(segments.len() * 4);
    let mut indices: Vec<u32> = Vec::with_capacity(segments.len() * 6);

    for (i, (a, b)) in segments.iter().copied().enumerate() {
        let delta = b - a;
        if delta.length_squared() <= 1e-6 {
            continue;
        }
        let dir = delta.normalize();
        let perp = Vec2::new(-dir.y, dir.x) * half_width;

        let p0 = a + perp;
        let p1 = b + perp;
        let p2 = b - perp;
        let p3 = a - perp;

        let base = (i * 4) as u32;
        positions.push([p0.x, p0.y, 0.0]);
        positions.push([p1.x, p1.y, 0.0]);
        positions.push([p2.x, p2.y, 0.0]);
        positions.push([p3.x, p3.y, 0.0]);
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0]; positions.len()];
    let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0]; positions.len()];

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

#[derive(Default)]
pub struct DebugLineScratch {
    wire: Vec<(Vec2, Vec2)>,
    force: Vec<(Vec2, Vec2)>,
    velocity: Vec<(Vec2, Vec2)>,
    grid: Vec<(Vec2, Vec2)>,
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

/// Spawn retained `Mesh2d` entities for all debug line overlays.
pub fn setup_debug_line_layers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let empty = meshes.add(line_segments_mesh(&[], 0.4));

    commands.spawn((
        Mesh2d(empty.clone()),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgba(1.0, 1.0, 1.0, 0.4)))),
        Transform::from_translation(Vec3::new(0.0, 0.0, 2.5)),
        Visibility::Hidden,
        WireframeOverlayLayer,
    ));

    commands.spawn((
        Mesh2d(empty.clone()),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgb(1.0, 0.15, 0.15)))),
        Transform::from_translation(Vec3::new(0.0, 0.0, 2.55)),
        Visibility::Hidden,
        ForceVectorLayer,
    ));

    commands.spawn((
        Mesh2d(empty.clone()),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgb(0.2, 0.8, 1.0)))),
        Transform::from_translation(Vec3::new(0.0, 0.0, 2.6)),
        Visibility::Hidden,
        VelocityArrowLayer,
    ));

    commands.spawn((
        Mesh2d(empty),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgba(
            0.35, 0.95, 0.35, 0.45,
        )))),
        Transform::from_translation(Vec3::new(0.0, 0.0, 2.65)),
        Visibility::Hidden,
        SpatialGridLayer,
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

/// Spawn the permanent score HUD (always visible).
pub fn setup_hud_score(mut commands: Commands, config: Res<PhysicsConfig>, font: Res<GameFont>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(14.0),
                top: Val::Px(10.0),
                ..default()
            },
            HudScoreDisplay,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("0"),
                TextFont {
                    font: font.0.clone(),
                    font_size: (config.stats_font_size * 2.0).max(28.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                HudScoreText,
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
pub fn setup_lives_hud(
    mut commands: Commands,
    config: Res<PhysicsConfig>,
    font: Res<GameFont>,
    symbol_font_2: Res<SymbolFont2>,
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            },
            LivesHudDisplay,
        ))
        .with_children(|parent| {
            // Lives counter row
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(5.0),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Text::new(repeated_symbol("⮝", config.player_lives.max(0) as u32)),
                        TextFont {
                            font: symbol_font_2.0.clone(),
                            font_size: config.stats_font_size,
                            ..default()
                        },
                        TextColor(Color::srgb(0.95, 0.45, 0.45)),
                        LivesHudValueText,
                    ));
                });
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
#[allow(clippy::type_complexity)]
pub fn lives_hud_display_system(
    lives: Res<PlayerLives>,
    mut text_query: Query<(
        &mut Text,
        Option<&mut Visibility>,
        Option<&RespawnCountdownText>,
        Option<&LivesHudValueText>,
    )>,
) {
    if !lives.is_changed() {
        return;
    }
    let remaining = lives.remaining.max(0);
    for (mut text, vis, respawn_tag, lives_value_tag) in text_query.iter_mut() {
        if lives_value_tag.is_some() {
            *text = Text::new(repeated_symbol("⮝", remaining as u32));
            continue;
        }

        if respawn_tag.is_some() {
            let Some(mut vis) = vis else {
                continue;
            };
            if let Some(t) = lives.respawn_timer {
                *text = Text::new(format!("RESPAWNING IN {t:.1}s…"));
                *vis = Visibility::Visible;
            } else {
                *text = Text::new("");
                *vis = Visibility::Hidden;
            }
        }
    }
}

/// Startup: spawn the missile-ammo indicator HUD (row 3, below lives HUD).
pub fn setup_missile_hud(
    _commands: Commands,
    _config: Res<PhysicsConfig>,
    _font: Res<GameFont>,
    _emoji_font: Res<EmojiFont>,
) {
}

/// Refresh the missile ammo HUD each frame.
pub fn missile_hud_display_system(
    ammo: Res<MissileAmmo>,
    config: Res<PhysicsConfig>,
    mut text_query: Query<&mut Text, With<MissileHudValueText>>,
) {
    if !ammo.is_changed() {
        return;
    }
    for mut text in text_query.iter_mut() {
        *text = Text::new(slot_indicator(ammo.count, config.missile_ammo_max));
    }
}

/// Startup: bottom-left HUD indicator block.
pub fn setup_ore_hud(
    mut commands: Commands,
    config: Res<PhysicsConfig>,
    font: Res<GameFont>,
    symbol_font: Res<SymbolFont>,
    symbol_font_2: Res<SymbolFont2>,
    emoji_font: Res<EmojiFont>,
    _unicode_fallback_font: Res<UnicodeFallbackFont>,
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                bottom: Val::Px(12.0),
                ..default()
            },
            OreHudDisplay,
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    ..default()
                })
                .with_children(|col| {
                    // Ore count
                    col.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(5.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Text::new("💎"),
                            TextFont {
                                font: emoji_font.0.clone(),
                                font_size: config.stats_font_size,
                                ..default()
                            },
                            TextColor(Color::srgb(0.35, 1.0, 0.55)),
                        ));
                        row.spawn((
                            Text::new("0"),
                            TextFont {
                                font: font.0.clone(),
                                font_size: config.stats_font_size,
                                ..default()
                            },
                            TextColor(Color::srgb(0.35, 1.0, 0.55)),
                            OreHudValueText,
                        ));
                    });

                    // Blaster
                    col.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(3.0),
                        ..default()
                    })
                    .with_children(|entry| {
                        entry.spawn((
                            Text::new("⛯"),
                            TextFont {
                                font: symbol_font.0.clone(),
                                font_size: config.stats_font_size,
                                ..default()
                            },
                            TextColor(Color::srgb(1.0, 0.92, 0.3)),
                        ));
                        entry.spawn((
                            Text::new("①"),
                            TextFont {
                                font: symbol_font.0.clone(),
                                font_size: config.stats_font_size,
                                ..default()
                            },
                            TextColor(Color::srgb(1.0, 0.92, 0.3)),
                            BlasterHudValueText,
                        ));
                    });

                    // Missile [symbol] [level] [count]
                    col.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(3.0),
                        ..default()
                    })
                    .with_children(|entry| {
                        entry.spawn((
                            Text::new("🚀"),
                            TextFont {
                                font: emoji_font.0.clone(),
                                font_size: config.stats_font_size,
                                ..default()
                            },
                            TextColor(Color::srgb(1.0, 0.55, 0.1)),
                        ));
                        entry.spawn((
                            Text::new("①"),
                            TextFont {
                                font: symbol_font.0.clone(),
                                font_size: config.stats_font_size,
                                ..default()
                            },
                            TextColor(Color::srgb(1.0, 0.55, 0.1)),
                            MissileLevelHudValueText,
                        ));
                        entry.spawn((
                            Text::new(slot_indicator(
                                config.missile_ammo_max,
                                config.missile_ammo_max,
                            )),
                            TextFont {
                                font: symbol_font_2.0.clone(),
                                font_size: config.stats_font_size,
                                ..default()
                            },
                            TextColor(Color::srgb(1.0, 0.55, 0.1)),
                            MissileHudValueText,
                        ));
                    });

                    // Magnet
                    col.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(3.0),
                        ..default()
                    })
                    .with_children(|entry| {
                        entry.spawn((
                            Text::new("🧲"),
                            TextFont {
                                font: emoji_font.0.clone(),
                                font_size: config.stats_font_size,
                                ..default()
                            },
                            TextColor(Color::srgb(0.95, 0.35, 0.35)),
                        ));
                        entry.spawn((
                            Text::new("①"),
                            TextFont {
                                font: symbol_font.0.clone(),
                                font_size: config.stats_font_size,
                                ..default()
                            },
                            TextColor(Color::srgb(0.95, 0.35, 0.35)),
                            MagnetHudValueText,
                        ));
                    });

                    // Tractor
                    col.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(3.0),
                        ..default()
                    })
                    .with_children(|entry| {
                        entry.spawn((
                            Text::new("✦"),
                            TextFont {
                                font: symbol_font_2.0.clone(),
                                font_size: config.stats_font_size,
                                ..default()
                            },
                            TextColor(Color::srgb(0.35, 0.9, 0.95)),
                        ));
                        entry.spawn((
                            Text::new("① ○"),
                            TextFont {
                                font: symbol_font_2.0.clone(),
                                font_size: config.stats_font_size - 1.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.35, 0.9, 0.95)),
                            TractorHudValueText,
                        ));
                    });

                    // Ion Cannon
                    col.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(3.0),
                        ..default()
                    })
                    .with_children(|entry| {
                        entry.spawn((
                            Text::new("⚛"),
                            TextFont {
                                font: symbol_font.0.clone(),
                                font_size: config.stats_font_size,
                                ..default()
                            },
                            TextColor(Color::srgb(0.55, 0.85, 1.0)),
                        ));
                        entry.spawn((
                            Text::new("① ⚡"),
                            TextFont {
                                font: symbol_font_2.0.clone(),
                                font_size: config.stats_font_size - 1.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.55, 0.85, 1.0)),
                            IonHudValueText,
                        ));
                    });
                });
        });
}

/// Refresh the ore-count HUD each frame.
///
/// When ore > 0 the text includes key-binding hints for spending it so players
/// can discover the mechanic passively.  The primary weapon upgrade level is
/// shown inline so players always know their current tier.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn ore_hud_display_system(
    ore: Res<PlayerOre>,
    weapon_level: Res<PrimaryWeaponLevel>,
    missile_level: Res<SecondaryWeaponLevel>,
    magnet_level: Res<OreAffinityLevel>,
    tractor_level: Res<TractorBeamLevel>,
    tractor_hold_state: Res<TractorHoldState>,
    tractor_throw_cooldown: Res<TractorThrowCooldown>,
    ion_level: Res<IonCannonLevel>,
    ion_cooldown: Res<IonCannonCooldown>,
    mut text_query: Query<(
        &mut Text,
        AnyOf<(
            &OreHudValueText,
            &BlasterHudValueText,
            &MissileLevelHudValueText,
            &MagnetHudValueText,
            &TractorHudValueText,
            &IonHudValueText,
        )>,
    )>,
) {
    if !ore.is_changed()
        && !weapon_level.is_changed()
        && !missile_level.is_changed()
        && !magnet_level.is_changed()
        && !tractor_level.is_changed()
        && !tractor_hold_state.is_changed()
        && !tractor_throw_cooldown.is_changed()
        && !ion_level.is_changed()
        && !ion_cooldown.is_changed()
    {
        return;
    }

    let blaster_text = circled_number_level(weapon_level.display_level()).to_string();
    let missile_text = circled_number_level(missile_level.display_level()).to_string();
    let magnet_text = circled_number_level(magnet_level.display_level()).to_string();
    let tractor_level_text = circled_number_level(tractor_level.display_level()).to_string();
    let ion_level_text = circled_number_level(ion_level.display_level()).to_string();
    let ion_state_text = if ion_cooldown.timer_secs <= 0.0 {
        "⚡"
    } else {
        "⌛"
    };
    let tractor_state_text = if !tractor_hold_state.engaged {
        "○"
    } else if tractor_throw_cooldown.timer_secs <= 0.0 {
        "⚡"
    } else {
        "⌛"
    };

    for (mut text, tags) in text_query.iter_mut() {
        if tags.0.is_some() {
            *text = Text::new(ore.count.to_string());
        } else if tags.1.is_some() {
            *text = Text::new(blaster_text.clone());
        } else if tags.2.is_some() {
            *text = Text::new(missile_text.clone());
        } else if tags.3.is_some() {
            *text = Text::new(magnet_text.clone());
        } else if tags.4.is_some() {
            *text = Text::new(format!("{} {}", tractor_level_text, tractor_state_text));
        } else if tags.5.is_some() {
            *text = Text::new(format!("{} {}", ion_level_text, ion_state_text));
        }
    }
}

/// Startup: stats overlay text — Spawn the toggleable simulation-stats overlay (starts hidden; enable via debug panel).
pub fn setup_stats_text(mut commands: Commands, config: Res<PhysicsConfig>, font: Res<GameFont>) {
    let row_h = config.stats_font_size + 6.0;
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0 + row_h * 2.0),
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
                StatsOverlayText,
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
                top: Val::Px(10.0 + row_h * 3.0),
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
                PhysicsInspectorText,
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
                top: Val::Px(10.0 + row_h * 9.0),
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
                ProfilerText,
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
    mut text_query: Query<&mut Text, With<HudScoreText>>,
) {
    if !score.is_changed() {
        return;
    }
    for mut text in text_query.iter_mut() {
        *text = Text::new(score.total().to_string());
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
    score: Res<PlayerScore>,
    mut text_query: Query<&mut Text, With<StatsOverlayText>>,
) {
    for mut text in text_query.iter_mut() {
        *text = Text::new(format!(
            "Live: {} | Culled: {} | Merged: {} | Split: {} | Destroyed: {}\nScoreDBG: hits={} destroyed={} combo={} streak={}",
            stats.live_count,
            stats.culled_total,
            stats.merged_total,
            stats.split_total,
            stats.destroyed_total,
            score.hits,
            score.destroyed,
            score.multiplier(),
            score.streak,
        ));
    }
}

/// Refresh the physics inspector text with IDs, velocities, and active contact counts.
pub fn physics_inspector_display_system(
    overlay: Res<OverlayState>,
    mut text_query: Query<&mut Text, With<PhysicsInspectorText>>,
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
    for mut text in text_query.iter_mut() {
        *text = Text::new(display.clone());
    }
}

/// Refresh profiler text with frame-time and schedule timing breakdown.
pub fn profiler_display_system(
    overlay: Res<OverlayState>,
    profiler: Res<ProfilerStats>,
    diagnostics: Res<DiagnosticsStore>,
    mut text_query: Query<&mut Text, With<ProfilerText>>,
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

    for mut text in text_query.iter_mut() {
        *text = Text::new(display.clone());
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

// ── Update: retained debug-line overlay rendering ───────────────────────────

/// Refresh retained `Mesh2d` line overlays from current simulation state.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn sync_debug_line_layers_system(
    query: Query<(&Transform, &Vertices, &GravityForce, &Velocity), With<Asteroid>>,
    stats: Res<SimulationStats>,
    config: Res<PhysicsConfig>,
    grid: Res<SpatialGrid>,
    overlay: Res<OverlayState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut debug_layers: ParamSet<(
        Query<(&Mesh2d, &mut Visibility), With<WireframeOverlayLayer>>,
        Query<(&Mesh2d, &mut Visibility), With<ForceVectorLayer>>,
        Query<(&Mesh2d, &mut Visibility), With<VelocityArrowLayer>>,
        Query<(&Mesh2d, &mut Visibility), With<SpatialGridLayer>>,
    )>,
    mut scratch: Local<DebugLineScratch>,
) {
    let show_wire = overlay.show_wireframes;
    let show_force =
        overlay.show_force_vectors && stats.live_count < config.force_vector_hide_threshold;
    let show_velocity = overlay.show_velocity_arrows;
    let show_grid = overlay.show_debug_grid;

    if !show_wire && !show_force && !show_velocity && !show_grid {
        if let Ok((_, mut vis)) = debug_layers.p0().single_mut() {
            *vis = Visibility::Hidden;
        }
        if let Ok((_, mut vis)) = debug_layers.p1().single_mut() {
            *vis = Visibility::Hidden;
        }
        if let Ok((_, mut vis)) = debug_layers.p2().single_mut() {
            *vis = Visibility::Hidden;
        }
        if let Ok((_, mut vis)) = debug_layers.p3().single_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }

    if show_wire {
        scratch.wire.clear();
    }
    if show_force {
        scratch.force.clear();
    }
    if show_velocity {
        scratch.velocity.clear();
    }
    if show_grid {
        scratch.grid.clear();
    }

    if show_wire || show_force || show_velocity {
        for (transform, vertices, grav, vel) in query.iter() {
            let pos = transform.translation.truncate();

            if show_wire && vertices.0.len() >= 2 {
                let rot = transform.rotation;
                let n = vertices.0.len();
                for i in 0..n {
                    let v1 = vertices.0[i];
                    let v2 = vertices.0[(i + 1) % n];
                    let p1 = pos + rot.mul_vec3(v1.extend(0.0)).truncate();
                    let p2 = pos + rot.mul_vec3(v2.extend(0.0)).truncate();
                    scratch.wire.push((p1, p2));
                }
            }

            if show_force {
                let force_vec = grav.0 * config.force_vector_display_scale;
                if force_vec.length() > config.force_vector_min_length {
                    scratch.force.push((pos, pos + force_vec));
                }
            }

            if show_velocity {
                let v = vel.linvel;
                if v.length_squared() > 0.5 {
                    let tip = pos + v * 0.15;
                    scratch.velocity.push((pos, tip));

                    let dir = (tip - pos).normalize_or_zero();
                    if dir != Vec2::ZERO {
                        let perp = Vec2::new(-dir.y, dir.x);
                        scratch.velocity.push((tip, tip - dir * 2.2 + perp * 1.2));
                        scratch.velocity.push((tip, tip - dir * 2.2 - perp * 1.2));
                    }
                }
            }
        }
    }

    if show_grid {
        let half = config.cull_distance;
        let min = Vec2::new(-half, -half);
        let max = Vec2::new(half, half);
        grid.collect_debug_split_lines(min, max, &mut scratch.grid);
    }

    if let Ok((mesh_handle, mut vis)) = debug_layers.p0().single_mut() {
        *vis = if show_wire {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if show_wire {
            if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
                *mesh = line_segments_mesh(&scratch.wire, 0.28);
            }
        }
    }

    if let Ok((mesh_handle, mut vis)) = debug_layers.p1().single_mut() {
        *vis = if show_force {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if show_force {
            if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
                *mesh = line_segments_mesh(&scratch.force, 0.35);
            }
        }
    }

    if let Ok((mesh_handle, mut vis)) = debug_layers.p2().single_mut() {
        *vis = if show_velocity {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if show_velocity {
            if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
                *mesh = line_segments_mesh(&scratch.velocity, 0.32);
            }
        }
    }

    if let Ok((mesh_handle, mut vis)) = debug_layers.p3().single_mut() {
        *vis = if show_grid {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if show_grid {
            if let Some(mesh) = meshes.get_mut(&mesh_handle.0) {
                *mesh = line_segments_mesh(&scratch.grid, 0.20);
            }
        }
    }
}
