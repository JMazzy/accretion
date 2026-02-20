//! Mesh2d-based filled polygon rendering for asteroids.
//!
//! Every `Asteroid` entity automatically receives a GPU-efficient filled
//! polygon mesh shortly after spawning, via the `attach_asteroid_mesh_system`
//! (which queries `Added<Asteroid>`).  This replaces the immediate-mode Gizmo
//! wireframe as the default visual layer.
//!
//! ## Why Mesh2d?
//!
//! Gizmos are immediate-mode: each frame every `gizmos.line_2d()` call
//! rebuilds line geometry on the CPU and re-uploads it to the GPU.  At
//! 500+ asteroids with force vectors this becomes a measurable bottleneck.
//!
//! `Mesh2d` uses retained GPU assets.  The mesh geometry is uploaded once at
//! spawn time and lives on the GPU until the entity is despawned.  Bevy
//! batches compatible `Mesh2d` + `ColorMaterial` draw calls into a single GPU
//! dispatch, scaling to thousands of entities with minimal CPU overhead.
//!
//! Gizmo wireframes are preserved as an optional debug overlay, toggled via
//! [`OverlayState`](crate::rendering::OverlayState).

use crate::asteroid::{Asteroid, Vertices};
use crate::rendering::OverlayState;
use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use bevy_mesh::{Indices, PrimitiveTopology};

// ── Spawn-time mesh attachment ────────────────────────────────────────────────

/// Attach a filled `Mesh2d` polygon to every newly spawned asteroid.
///
/// Uses [`Added<Asteroid>`] so this only executes for entities that appeared
/// since the previous frame — there is no per-frame overhead for existing
/// asteroids.
///
/// The entity's `Transform` (position + rotation) is managed entirely by
/// Rapier.  Because `Mesh2d` renders in the entity's local space and the
/// asteroid vertices are already stored in local space, the rotation is
/// applied automatically and correctly without any extra math here.
pub fn attach_asteroid_mesh_system(
    mut commands: Commands,
    query: Query<(Entity, &Vertices), Added<Asteroid>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    overlay: Res<OverlayState>,
) {
    for (entity, vertices) in query.iter() {
        if vertices.0.len() < 3 {
            continue;
        }

        let mesh_handle = meshes.add(filled_polygon_mesh(&vertices.0));
        let material_handle = materials.add(ColorMaterial::from_color(rock_color(entity.index())));

        // Respect wireframe-only mode: hide fills if the flag is already set.
        let visibility = if overlay.wireframe_only {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };

        commands.entity(entity).insert((
            Mesh2d(mesh_handle),
            MeshMaterial2d(material_handle),
            visibility,
        ));
    }
}

/// Propagate a `wireframe_only` toggle to all currently-alive asteroid meshes.
///
/// Only runs when `OverlayState` changes, so the per-frame cost is negligible.
pub fn sync_asteroid_mesh_visibility_system(
    overlay: Res<OverlayState>,
    mut query: Query<&mut Visibility, (With<Asteroid>, With<Mesh2d>)>,
) {
    if !overlay.is_changed() {
        return;
    }
    let vis = if overlay.wireframe_only {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    for mut visibility in query.iter_mut() {
        *visibility = vis;
    }
}

// ── Geometry helpers ──────────────────────────────────────────────────────────

/// Fan-triangulate a convex polygon into a renderable [`Mesh`].
///
/// Triangle fan from vertex 0: triangles `(0, i, i+1)` for `i ∈ 1..n-2`.
/// Valid for any convex polygon (all asteroid hulls are convex).
///
/// UVs are mapped from local-space coordinates so a future texture atlas can
/// be dropped in without a UV re-bake step.
pub fn filled_polygon_mesh(vertices: &[Vec2]) -> Mesh {
    let n = vertices.len();
    debug_assert!(n >= 3, "polygon must have ≥ 3 vertices");

    let positions: Vec<[f32; 3]> = vertices.iter().map(|v| [v.x, v.y, 0.0]).collect();
    let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0]; n];
    // Map ±50 world-unit local coords to roughly 0–1 UV range.
    let uvs: Vec<[f32; 2]> = vertices
        .iter()
        .map(|v| [(v.x / 100.0) + 0.5, (v.y / 100.0) + 0.5])
        .collect();

    let mut indices: Vec<u32> = Vec::with_capacity((n - 2) * 3);
    for i in 1..(n as u32 - 1) {
        indices.extend_from_slice(&[0, i, i + 1]);
    }

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

/// Generate a rocky grey-brown fill color seeded by the entity index.
///
/// Uses a multiplicative hash so every asteroid gets a deterministic but
/// visually distinct tone without an external noise library.
///
/// Palette: luminance 0.18–0.36 with a slight warm/cool tint variation.
fn rock_color(seed: u32) -> Color {
    // Knuth multiplicative hash → 0.0–1.0
    let h = seed.wrapping_mul(2_654_435_761).wrapping_add(0xDEAD_BEEF);
    let t = (h & 0xFFFF) as f32 / 65_535.0;

    let lum = 0.18 + t * 0.18;
    let r = (lum + t * 0.06).min(1.0);
    let g = (lum + t * 0.02).min(1.0);
    let b = (lum.max(0.14) - t * 0.03).max(0.0);
    Color::srgb(r, g, b)
}
