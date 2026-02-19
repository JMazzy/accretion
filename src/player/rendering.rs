//! Player-specific rendering: ship wireframe, health bar, aim indicator,
//! projectile circles, and the camera follow system.

use super::state::{AimDirection, Player, PlayerHealth, Projectile};
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

// ── Gizmo rendering ───────────────────────────────────────────────────────────

/// Draw the player ship outline, health bar, aim indicator, and all projectile circles.
///
/// - **Ship colour**: shifts from cyan (full HP) toward red (low HP)
/// - **Health bar**: pixel-wide horizontal bar above the ship, green→red fill
/// - **Aim indicator**: orange line + dot extending in the current `AimDirection`
/// - **Projectiles**: yellow circles at each live projectile position
pub fn player_gizmo_system(
    mut gizmos: Gizmos,
    q_player: Query<(&Transform, &PlayerHealth), With<Player>>,
    q_projectiles: Query<&Transform, With<Projectile>>,
    aim: Res<AimDirection>,
) {
    // ── Ship ──────────────────────────────────────────────────────────────────
    if let Ok((transform, health)) = q_player.get_single() {
        let pos = transform.translation.truncate();
        let rot = transform.rotation;
        let verts = ship_vertices();

        let hp_frac = (health.hp / health.max_hp).clamp(0.0, 1.0);
        // Tint: cyan at full health, red at zero health
        let ship_color = Color::rgb(1.0 - hp_frac * 0.8, hp_frac * 0.6 + 0.2, hp_frac);

        // Ship outline
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

        // Aim indicator: orange line + dot at the fire direction tip
        if aim.0.length_squared() > 0.01 {
            let aim_tip = pos + aim.0.normalize_or_zero() * 35.0;
            gizmos.line_2d(pos, aim_tip, Color::rgb(1.0, 0.5, 0.0));
            gizmos.circle_2d(aim_tip, 3.0, Color::rgb(1.0, 0.5, 0.0));
        }

        // ── Health bar (axis-aligned, above ship) ─────────────────────────────
        let bar_half = 20.0;
        let bar_y_offset = 18.0;
        let bar_start = pos + Vec2::new(-bar_half, bar_y_offset);
        let bar_end_full = pos + Vec2::new(bar_half, bar_y_offset);
        let bar_end_hp = bar_start + Vec2::new(bar_half * 2.0 * hp_frac, 0.0);
        // Background (dark red track)
        gizmos.line_2d(bar_start, bar_end_full, Color::rgba(0.4, 0.0, 0.0, 0.8));
        // Fill (green at full HP → red at zero)
        if hp_frac > 0.0 {
            let fill_color = Color::rgb(1.0 - hp_frac, hp_frac, 0.0);
            gizmos.line_2d(bar_start, bar_end_hp, fill_color);
        }
    }

    // ── Projectiles ───────────────────────────────────────────────────────────
    let proj_color = Color::rgb(1.0, 0.9, 0.2); // yellow
    for transform in q_projectiles.iter() {
        let p = transform.translation.truncate();
        gizmos.circle_2d(p, 3.0, proj_color);
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
    let Ok(player_transform) = q_player.get_single() else {
        return;
    };
    let Ok(mut cam) = q_camera.get_single_mut() else {
        return;
    };

    cam.translation.x = player_transform.translation.x;
    cam.translation.y = player_transform.translation.y;
}
