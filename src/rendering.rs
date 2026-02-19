//! Rendering systems: asteroid wireframes, force vectors, and the stats overlay.
//!
//! This module owns all Bevy-gizmo and UI-text rendering that was previously
//! scattered across `simulation.rs`.  Player-specific rendering (ship outline,
//! health bar, aim indicator, projectile circles) remains in
//! `player::rendering`.
//!
//! ## System Responsibilities
//!
//! | System | Schedule | Responsibility |
//! |--------|----------|----------------|
//! | `setup_stats_text` | Startup | Spawn the fixed-position UI text node |
//! | `gizmo_rendering_system` | Update | Draw asteroid wireframes + force vectors |
//! | `stats_display_system` | Update | Update the live/culled/merged text each frame |

use crate::asteroid::{Asteroid, Vertices};
use crate::constants::{
    CULL_DISTANCE, FORCE_VECTOR_DISPLAY_SCALE, FORCE_VECTOR_HIDE_THRESHOLD,
    FORCE_VECTOR_MIN_LENGTH, STATS_FONT_SIZE,
};
use crate::simulation::SimulationStats;
use bevy::prelude::*;
use bevy_rapier2d::prelude::ExternalForce;

// ── Stats overlay ─────────────────────────────────────────────────────────────

/// Marker component for the root node of the on-screen statistics display.
#[derive(Component)]
pub struct StatsTextDisplay;

/// Spawn the UI stats text node at startup.
///
/// Uses a `NodeBundle` + `TextBundle` hierarchy so the text is fixed to the
/// screen corner and unaffected by camera zoom or translation.
pub fn setup_stats_text(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Live: 0 | Culled: 0 | Merged: 0",
                TextStyle {
                    font: Handle::default(),
                    font_size: STATS_FONT_SIZE,
                    color: Color::rgb(0.0, 1.0, 1.0),
                },
            ));
        })
        .insert(StatsTextDisplay);
}

/// Update the stats text content each frame.
pub fn stats_display_system(
    stats: Res<SimulationStats>,
    parent_query: Query<&Children, With<StatsTextDisplay>>,
    mut text_query: Query<&mut Text>,
) {
    for children in parent_query.iter() {
        for &child in children.iter() {
            if let Ok(mut text) = text_query.get_mut(child) {
                text.sections[0].value = format!(
                    "Live: {} | Culled: {} | Merged: {}",
                    stats.live_count, stats.culled_total, stats.merged_total
                );
            }
        }
    }
}

// ── Asteroid wireframe rendering ──────────────────────────────────────────────

/// Draw asteroid polygon outlines and, at low counts, force-vector overlays.
///
/// Force-vector lines are skipped when `live_count ≥ FORCE_VECTOR_HIDE_THRESHOLD`
/// to reduce CPU gizmo overhead at high asteroid densities.
pub fn gizmo_rendering_system(
    mut gizmos: Gizmos,
    query: Query<(&Transform, &Vertices, &ExternalForce), With<Asteroid>>,
    stats: Res<SimulationStats>,
) {
    let draw_force_vectors = stats.live_count < FORCE_VECTOR_HIDE_THRESHOLD;

    for (transform, vertices, force) in query.iter() {
        if vertices.0.len() < 2 {
            continue;
        }

        let pos = transform.translation.truncate();
        let rotation = transform.rotation;
        let n = vertices.0.len();

        // Draw polygon outline with rotation applied
        for i in 0..n {
            let v1 = vertices.0[i];
            let v2 = vertices.0[(i + 1) % n];
            let p1 = pos + rotation.mul_vec3(v1.extend(0.0)).truncate();
            let p2 = pos + rotation.mul_vec3(v2.extend(0.0)).truncate();
            gizmos.line_2d(p1, p2, Color::WHITE);
        }

        // Force-vector overlay (red line from centre, proportional to magnitude)
        if draw_force_vectors {
            let force_vec = force.force * FORCE_VECTOR_DISPLAY_SCALE;
            if force_vec.length() > FORCE_VECTOR_MIN_LENGTH {
                gizmos.line_2d(pos, pos + force_vec, Color::rgb(1.0, 0.0, 0.0));
            }
        }
    }

    // Culling boundary circle (yellow) rendered at world origin regardless of camera
    gizmos.circle_2d(Vec2::ZERO, CULL_DISTANCE, Color::rgb(1.0, 1.0, 0.0));
}
