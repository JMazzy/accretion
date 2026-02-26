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
//! ## Wireframe-Only Mode
//!
//! Each asteroid carries an [`AsteroidRenderHandles`] component with both a
//! filled-polygon mesh handle and a polygon-outline mesh handle.  Toggling
//! `OverlayState::wireframe_only` swaps the active `Mesh2d` handle between
//! the two — no gizmo calls and no per-frame CPU work once the toggle fires.
//!
//! The semi-transparent gizmo overlay (`show_wireframes`) is preserved as an
//! additive debug option and continues to use immediate-mode gizmos.

use crate::asteroid::{Asteroid, Planet, Vertices};
use crate::rendering::OverlayState;
use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use bevy_mesh::{Indices, PrimitiveTopology};

// ── Retained render handles ───────────────────────────────────────────────────

/// Both mesh/material variants stored per asteroid so `wireframe_only` can
/// swap between them with a single handle swap — no geometry rebuilds needed.
#[derive(Component)]
pub struct AsteroidRenderHandles {
    pub fill_mesh: Handle<Mesh>,
    pub fill_material: Handle<ColorMaterial>,
    pub outline_mesh: Handle<Mesh>,
    pub outline_material: Handle<ColorMaterial>,
}

// ── Spawn-time mesh attachment ────────────────────────────────────────────────

/// Attach both a filled `Mesh2d` polygon and a polygon-outline `Mesh2d` to
/// every newly spawned asteroid.
///
/// Uses [`Added<Asteroid>`] so this only executes for entities that appeared
/// since the previous frame — there is no per-frame overhead for existing
/// asteroids.
///
/// The entity's `Transform` (position + rotation) is managed entirely by
/// Rapier.  Because `Mesh2d` renders in the entity's local space and the
/// asteroid vertices are already stored in local space, the rotation is
/// applied automatically and correctly without any extra math here.
///
/// Both mesh/material handles are stored in [`AsteroidRenderHandles`] so
/// `sync_asteroid_render_mode_system` can swap between them instantly on
/// `wireframe_only` toggle with no per-frame CPU cost.
pub fn attach_asteroid_mesh_system(
    mut commands: Commands,
    query: Query<(Entity, &Vertices, Option<&Planet>), Added<Asteroid>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    overlay: Res<OverlayState>,
) {
    for (entity, vertices, is_planet) in query.iter() {
        if vertices.0.len() < 3 {
            continue;
        }

        // ── Filled polygon mesh ───────────────────────────────────────────────
        let fill_mesh = meshes.add(filled_polygon_mesh(&vertices.0));
        let fill_color = if is_planet.is_some() {
            Color::srgb(0.55, 0.25, 0.85)
        } else {
            rock_color(entity.index())
        };
        let fill_material = materials.add(ColorMaterial::from_color(fill_color));

        // ── Polygon outline mesh (used in wireframe_only mode) ────────────────
        // 0.4-unit half-width gives a crisp but thin outline at typical zoom levels.
        let outline_mesh = meshes.add(polygon_outline_mesh(&vertices.0, 0.4));
        let outline_material = materials.add(ColorMaterial::from_color(Color::WHITE));

        // Start in whichever mode is current (fill vs wireframe).
        let (active_mesh, active_material) = if overlay.wireframe_only {
            (outline_mesh.clone(), outline_material.clone())
        } else {
            (fill_mesh.clone(), fill_material.clone())
        };

        commands.entity(entity).insert((
            Mesh2d(active_mesh),
            MeshMaterial2d(active_material),
            AsteroidRenderHandles {
                fill_mesh,
                fill_material,
                outline_mesh,
                outline_material,
            },
        ));
    }
}

/// Swap every asteroid's active `Mesh2d` between the fill and outline variants
/// whenever `OverlayState::wireframe_only` changes.
///
/// Because both variants are pre-generated at spawn time this is a pure
/// handle-swap with zero mesh rebuilds — it runs only when the flag changes,
/// not every frame.
pub fn sync_asteroid_render_mode_system(
    overlay: Res<OverlayState>,
    mut query: Query<
        (
            &mut Mesh2d,
            &mut MeshMaterial2d<ColorMaterial>,
            &AsteroidRenderHandles,
        ),
        With<Asteroid>,
    >,
) {
    if !overlay.is_changed() {
        return;
    }
    for (mut mesh, mut material, handles) in query.iter_mut() {
        if overlay.wireframe_only {
            *mesh = Mesh2d(handles.outline_mesh.clone());
            *material = MeshMaterial2d(handles.outline_material.clone());
        } else {
            *mesh = Mesh2d(handles.fill_mesh.clone());
            *material = MeshMaterial2d(handles.fill_material.clone());
        }
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

/// Build a retained polygon-outline mesh from a convex polygon.
///
/// Each edge `(vᵢ, vᵢ₊₁)` is extruded into a skinny quad of half-width
/// `half_width` (world units).  The resulting mesh is a `TriangleList` and
/// is compatible with the standard `Mesh2d` + `ColorMaterial` pipeline.
///
/// Used to render asteroid wireframe outlines as GPU-retained geometry
/// instead of per-frame gizmo calls.
pub fn polygon_outline_mesh(vertices: &[Vec2], half_width: f32) -> Mesh {
    let n = vertices.len();
    debug_assert!(n >= 2, "outline needs ≥ 2 vertices");

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n * 4);
    let mut indices: Vec<u32> = Vec::with_capacity(n * 6);

    for i in 0..n {
        let v1 = vertices[i];
        let v2 = vertices[(i + 1) % n];
        let dir = (v2 - v1).normalize_or_zero();
        let perp = Vec2::new(-dir.y, dir.x); // perpendicular, outward

        // Four corners of the edge quad
        let a = v1 + perp * half_width;
        let b = v2 + perp * half_width;
        let c = v2 - perp * half_width;
        let d = v1 - perp * half_width;

        let base = (i * 4) as u32;
        positions.push([a.x, a.y, 0.0]);
        positions.push([b.x, b.y, 0.0]);
        positions.push([c.x, c.y, 0.0]);
        positions.push([d.x, d.y, 0.0]);

        // Two triangles: (a,b,c) and (a,c,d)
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let vcount = positions.len();
    let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0]; vcount];
    let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0]; vcount];

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

/// Build a ring (annulus) mesh centred at the origin.
///
/// Used for the cull-boundary indicator and any circular outline.
/// The ring is `thickness` world-units wide; the outer edge lies at
/// `radius + thickness/2` and the inner edge at `radius − thickness/2`.
///
/// `segments` controls smoothness; 128 is sufficient for a 2000-unit radius
/// at the maximum zoom-out level.
pub fn ring_mesh(radius: f32, thickness: f32, segments: usize) -> Mesh {
    let r_outer = radius + thickness * 0.5;
    let r_inner = (radius - thickness * 0.5).max(0.0);

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(segments * 4);
    let mut indices: Vec<u32> = Vec::with_capacity(segments * 6);

    for i in 0..segments {
        let theta_a = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let theta_b = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
        let (sa, ca) = theta_a.sin_cos();
        let (sb, cb) = theta_b.sin_cos();

        let base = (i * 4) as u32;
        positions.push([ca * r_outer, sa * r_outer, 0.0]); // outer-a
        positions.push([cb * r_outer, sb * r_outer, 0.0]); // outer-b
        positions.push([cb * r_inner, sb * r_inner, 0.0]); // inner-b
        positions.push([ca * r_inner, sa * r_inner, 0.0]); // inner-a

        // Two triangles forming one ring quad
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let vcount = positions.len();
    let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0]; vcount];
    let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0]; vcount];

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
