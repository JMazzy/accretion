//! Player-specific rendering: ship mesh fill, wireframe outline, health bar,
//! aim indicator, projectile mesh fills, and the camera follow system.
//!
//! ## Layer model (player / projectile)
//!
//! | Layer                 | Technology | Default | Toggle                   |
//! |-----------------------|------------|---------|--------------------------|
//! | Ship filled polygon   | `Mesh2d`   | ON      | hidden in `wireframe_only`|
//! | Ship wireframe outline| Gizmos     | OFF     | `show_ship_outline`      |
//! | Aim direction indicator| Gizmos    | ON      | `show_aim_indicator`     |
//! | Health bar            | Gizmos     | always  | —                        |
//! | Projectile filled disc| `Mesh2d`   | ON      | hidden in `wireframe_only`|
//! | Projectile outline    | Gizmos     | OFF     | `show_projectile_outline`|

use super::state::{AimDirection, Player, PlayerHealth, Projectile};
use crate::asteroid_rendering::filled_polygon_mesh;
use crate::rendering::OverlayState;
use bevy::prelude::*;

// ── Ship geometry ─────────────────────────────────────────────────────────────

/// Local-space vertices of the player ship polygon (dart / arrowhead shape).
///
/// The ship's nose points along local +Y; the two fins sweep back along −Y.
/// This orientation means the ship always thrusts in its transform's +Y direction.
fn ship_vertices() -> Vec<Vec2> {
    vec![
        Vec2::new(0.0, 12.0),  // nose
        Vec2::new(-8.0, -8.0), // left fin tip
        Vec2::new(-3.0, -4.0), // left fin inner
        Vec2::new(0.0, -10.0), // tail notch
        Vec2::new(3.0, -4.0),  // right fin inner
        Vec2::new(8.0, -8.0),  // right fin tip
    ]
}

/// Approximate a disc as a regular N-gon polygon mesh (reuses `filled_polygon_mesh`).
fn disc_mesh(radius: f32, segments: usize) -> Mesh {
    let verts: Vec<Vec2> = (0..segments)
        .map(|i| {
            let angle = (i as f32) * std::f32::consts::TAU / (segments as f32);
            Vec2::new(angle.cos() * radius, angle.sin() * radius)
        })
        .collect();
    filled_polygon_mesh(&verts)
}

// ── Spawn-time mesh attachment ────────────────────────────────────────────────

/// Attach a filled `Mesh2d` polygon to the player ship on spawn.
///
/// Runs only once per player entity (via [`Added<Player>`]).  The ship
/// transform is managed by Rapier so rotation is applied automatically.
pub fn attach_player_ship_mesh_system(
    mut commands: Commands,
    query: Query<Entity, Added<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    overlay: Res<OverlayState>,
) {
    for entity in query.iter() {
        let mesh_handle = meshes.add(filled_polygon_mesh(&ship_vertices()));
        // Dark teal fill: complements the cyan gizmo outline colour used at full HP.
        let mat_handle = materials.add(ColorMaterial::from_color(Color::srgb(0.08, 0.30, 0.32)));
        let visibility = if overlay.wireframe_only {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        commands.entity(entity).insert((
            Mesh2d(mesh_handle),
            MeshMaterial2d(mat_handle),
            visibility,
        ));
    }
}

/// Attach a filled disc `Mesh2d` to every newly-fired projectile.
///
/// Runs once per projectile entity (via [`Added<Projectile>`]).
pub fn attach_projectile_mesh_system(
    mut commands: Commands,
    query: Query<Entity, Added<Projectile>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    overlay: Res<OverlayState>,
) {
    // Visual radius matches the gizmo circle used previously.
    const PROJ_RADIUS: f32 = 3.0;
    let mat_handle = materials.add(ColorMaterial::from_color(Color::srgb(1.0, 0.85, 0.1)));
    for entity in query.iter() {
        let mesh_handle = meshes.add(disc_mesh(PROJ_RADIUS, 12));
        let visibility = if overlay.wireframe_only {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        commands.entity(entity).insert((
            Mesh2d(mesh_handle),
            MeshMaterial2d(mat_handle.clone()),
            visibility,
        ));
    }
}

/// Propagate the `wireframe_only` flag to all live ship and projectile meshes.
///
/// Only runs when [`OverlayState`] changes, so the per-frame cost is negligible.
#[allow(clippy::type_complexity)]
pub fn sync_player_and_projectile_mesh_visibility_system(
    overlay: Res<OverlayState>,
    mut q_ship: Query<&mut Visibility, (With<Player>, With<Mesh2d>, Without<Projectile>)>,
    mut q_proj: Query<&mut Visibility, (With<Projectile>, With<Mesh2d>, Without<Player>)>,
) {
    if !overlay.is_changed() {
        return;
    }
    let vis = if overlay.wireframe_only {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    for mut v in q_ship.iter_mut() {
        *v = vis;
    }
    for mut v in q_proj.iter_mut() {
        *v = vis;
    }
}

// ── Gizmo rendering ───────────────────────────────────────────────────────────

/// Draw optional gizmo overlays for the player ship, aim indicator, and projectiles.
///
/// Which layers are drawn is controlled by [`OverlayState`]:
/// - **Ship outline**: `show_ship_outline` OR `wireframe_only` → coloured polygon loop + nose line
/// - **Aim indicator**: `show_aim_indicator` → orange line + dot
/// - **Health bar**: always shown (UI-like feedback, not a geometric overlay)
/// - **Projectile outlines**: `show_projectile_outline` OR `wireframe_only` → yellow circle
pub fn player_gizmo_system(
    mut gizmos: Gizmos,
    q_player: Query<(&Transform, &PlayerHealth), With<Player>>,
    q_projectiles: Query<&Transform, With<Projectile>>,
    aim: Res<AimDirection>,
    overlay: Res<OverlayState>,
) {
    // ── Ship ──────────────────────────────────────────────────────────────────
    if let Ok((transform, health)) = q_player.single() {
        let pos = transform.translation.truncate();
        let rot = transform.rotation;
        let verts = ship_vertices();

        let hp_frac = (health.hp / health.max_hp).clamp(0.0, 1.0);

        // ── Ship wireframe outline (optional) ─────────────────────────────────
        if overlay.show_ship_outline || overlay.wireframe_only {
            // Tint: cyan at full health, red at zero health
            let ship_color = Color::srgb(1.0 - hp_frac * 0.8, hp_frac * 0.6 + 0.2, hp_frac);

            for i in 0..verts.len() {
                let v1 = verts[i];
                let v2 = verts[(i + 1) % verts.len()];
                let p1 = pos + rot.mul_vec3(v1.extend(0.0)).truncate();
                let p2 = pos + rot.mul_vec3(v2.extend(0.0)).truncate();
                gizmos.line_2d(p1, p2, ship_color);
            }

            // Nose direction indicator (white)
            let nose_world = pos + rot.mul_vec3(Vec3::new(0.0, 12.0, 0.0)).truncate();
            gizmos.line_2d(pos, nose_world, Color::WHITE);
        }

        // ── Aim indicator (optional) ──────────────────────────────────────────
        if overlay.show_aim_indicator && aim.0.length_squared() > 0.01 {
            let aim_tip = pos + aim.0.normalize_or_zero() * 35.0;
            gizmos.line_2d(pos, aim_tip, Color::srgb(1.0, 0.5, 0.0));
            gizmos.circle_2d(aim_tip, 3.0, Color::srgb(1.0, 0.5, 0.0));
        }

        // ── Health bar (always shown) ─────────────────────────────────────────
        let bar_half = 20.0;
        let bar_y_offset = 18.0;
        let bar_start = pos + Vec2::new(-bar_half, bar_y_offset);
        let bar_end_full = pos + Vec2::new(bar_half, bar_y_offset);
        let bar_end_hp = bar_start + Vec2::new(bar_half * 2.0 * hp_frac, 0.0);
        // Background (dark red track)
        gizmos.line_2d(bar_start, bar_end_full, Color::srgba(0.4, 0.0, 0.0, 0.8));
        // Fill (green at full HP → red at zero)
        if hp_frac > 0.0 {
            let fill_color = Color::srgb(1.0 - hp_frac, hp_frac, 0.0);
            gizmos.line_2d(bar_start, bar_end_hp, fill_color);
        }
    }

    // ── Projectile outlines (optional) ────────────────────────────────────────
    if overlay.show_projectile_outline || overlay.wireframe_only {
        let proj_color = Color::srgb(1.0, 0.9, 0.2); // yellow
        for transform in q_projectiles.iter() {
            let p = transform.translation.truncate();
            gizmos.circle_2d(p, 3.0, proj_color);
        }
    }
}

// ── Camera ─────────────────────────────────────────────────────────────────────

/// Keep the camera centred on the player ship every frame.
///
/// Camera Z is preserved (used internally by Bevy for rendering order).
/// Zoom scale is applied separately in `simulation::camera_zoom_system`.
pub fn camera_follow_system(
    q_player: Query<&Transform, With<Player>>,
    mut q_camera: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    let Ok(player_transform) = q_player.single() else {
        return;
    };
    let Ok(mut cam) = q_camera.single_mut() else {
        return;
    };

    cam.translation.x = player_transform.translation.x;
    cam.translation.y = player_transform.translation.y;
}
