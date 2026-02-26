//! Player-specific rendering: ship mesh fill, wireframe outline, health bar,
//! aim indicator, projectile mesh fills, and the camera follow system.
//!
//! ## Layer model (player / projectile)
//!
//! | Layer                  | Technology | Default | Toggle                     |
//! |------------------------|------------|---------|----------------------------|
//! | Ship filled polygon    | `Mesh2d`   | ON      | hidden in `wireframe_only` |
//! | Ship wireframe outline | `Mesh2d`   | OFF     | `show_ship_outline`        |
//! | Aim direction indicator| `Mesh2d`   | OFF     | `show_aim_indicator`       |
//! | Health bar             | `Mesh2d`   | always  | —                          |
//! | Projectile filled disc | `Mesh2d`   | ON      | hidden in `wireframe_only` |
//! | Projectile outline     | `Mesh2d`   | OFF     | `show_projectile_outline`  |

use super::state::{AimDirection, Missile, Player, PlayerHealth, Projectile};
use crate::asteroid_rendering::{filled_polygon_mesh, polygon_outline_mesh, ring_mesh};
use crate::rendering::OverlayState;
use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use bevy_mesh::{Indices, PrimitiveTopology};
use bevy_rapier2d::prelude::Velocity;

// ── Player UI entity registry ─────────────────────────────────────────────────

/// Tracks the world-space `Mesh2d` entities that float above the player ship.
///
/// Stored as a `Resource` (not as child entities) so that world-space
/// positioning is trivial without needing to invert the parent's rotation.
/// When the player is despawned, [`cleanup_player_ui_system`] reads
/// this resource, despawns all stored entities, and resets the fields.
#[derive(Resource, Default)]
pub struct PlayerUiEntities {
    /// Dark-red background track of the health bar.
    pub health_bar_bg: Option<Entity>,
    /// Coloured HP fill of the health bar.
    pub health_bar_fill: Option<Entity>,
    /// Orange aim-direction arrow.
    pub aim_indicator: Option<Entity>,
}

// ── ECS component markers ─────────────────────────────────────────────────────

/// Marker for the health-bar background rectangle entity.
#[derive(Component)]
pub struct HealthBarBg;

/// Marker + material handle for the health-bar fill rectangle entity.
/// The material handle is stored here so the fill colour can be updated
/// in-place without round-tripping through a resource.
#[derive(Component)]
pub struct HealthBarFill(pub Handle<ColorMaterial>);

/// Marker for the aim-direction arrow entity.
#[derive(Component)]
pub struct AimIndicatorMesh;

/// Marker for retained projectile outline meshes.
#[derive(Component)]
pub struct ProjectileOutlineMesh;

/// Marker for retained missile outline meshes.
#[derive(Component)]
pub struct MissileOutlineMesh;

/// Marker + material handle for retained ship-outline mesh.
#[derive(Component)]
pub struct ShipOutlineMesh(pub Handle<ColorMaterial>);

/// Marker for retained ship nose-line mesh.
#[derive(Component)]
pub struct ShipNoseMesh;

// ── Mesh geometry helpers ─────────────────────────────────────────────────────

/// A unit square centred at the origin (−0.5 to +0.5 on both axes).
/// Scale and translate via [`Transform`] to get desired position and size.
fn unit_rect_mesh() -> Mesh {
    let positions: Vec<[f32; 3]> = vec![
        [-0.5, -0.5, 0.0],
        [0.5, -0.5, 0.0],
        [0.5, 0.5, 0.0],
        [-0.5, 0.5, 0.0],
    ];
    let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0]; 4];
    let uvs: Vec<[f32; 2]> = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));
    mesh
}

/// A triangular arrow pointing in local +Y (tip at Y=35, base width 5 units).
/// Rotate via [`Transform`] to point in any world-space direction.
fn aim_arrow_mesh() -> Mesh {
    let positions: Vec<[f32; 3]> = vec![[0.0, 35.0, 0.0], [-2.5, 0.0, 0.0], [2.5, 0.0, 0.0]];
    let normals: Vec<[f32; 3]> = vec![[0.0, 0.0, 1.0]; 3];
    let uvs: Vec<[f32; 2]> = vec![[0.5, 1.0], [0.0, 0.0], [1.0, 0.0]];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(vec![0, 1, 2]));
    mesh
}
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

/// Creates a rocket-shaped mesh oriented along local +Y (upward).
///
/// The rocket has a pointed nose, cylindrical body, and two triangular fins.
/// Apply rotation via [`Transform`] to orient it in the direction of travel.
fn rocket_mesh(body_width: f32, body_length: f32, nose_length: f32, fin_size: f32) -> Mesh {
    let verts = vec![
        // Nose tip (top)
        Vec2::new(0.0, body_length / 2.0 + nose_length),
        // Body outline (left side going down, then right side going up)
        // Left edge of nose base
        Vec2::new(-body_width / 2.0, body_length / 2.0),
        // Left fin outer point
        Vec2::new(-body_width / 2.0 - fin_size, -body_length / 2.0),
        // Left fin inner (back of body)
        Vec2::new(-body_width / 2.0, -body_length / 2.0 + fin_size),
        // Center back (between fins)
        Vec2::new(0.0, -body_length / 2.0),
        // Right fin inner
        Vec2::new(body_width / 2.0, -body_length / 2.0 + fin_size),
        // Right fin outer point
        Vec2::new(body_width / 2.0 + fin_size, -body_length / 2.0),
        // Right edge of nose base
        Vec2::new(body_width / 2.0, body_length / 2.0),
    ];

    filled_polygon_mesh(&verts)
}

/// Creates an elongated capsule mesh oriented along local +Y (upward).
///
/// The capsule is `length` units tall with rounded ends of `radius`.
/// Apply rotation via [`Transform`] to orient it in any direction.
fn elongated_projectile_mesh(radius: f32, length: f32, segments: usize) -> Mesh {
    let half_segments = segments / 2;
    let mut verts = Vec::new();

    // Top semicircle (angles from 0 to π)
    for i in 0..=half_segments {
        let angle = (i as f32) * std::f32::consts::PI / (half_segments as f32);
        let x = angle.cos() * radius;
        let y = angle.sin() * radius + length / 2.0;
        verts.push(Vec2::new(x, y));
    }

    // Bottom semicircle (angles from π to 2π)
    for i in 0..=half_segments {
        let angle =
            std::f32::consts::PI + (i as f32) * std::f32::consts::PI / (half_segments as f32);
        let x = angle.cos() * radius;
        let y = angle.sin() * radius - length / 2.0;
        verts.push(Vec2::new(x, y));
    }

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
        let verts = ship_vertices();
        let mesh_handle = meshes.add(filled_polygon_mesh(&verts));
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

        let outline_visibility = if overlay.show_ship_outline || overlay.wireframe_only {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let outline_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.2, 0.8, 1.0)));
        let outline = commands
            .spawn((
                Mesh2d(meshes.add(polygon_outline_mesh(&verts, 0.45))),
                MeshMaterial2d(outline_mat.clone()),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.02)),
                outline_visibility,
                ShipOutlineMesh(outline_mat),
            ))
            .id();

        let nose = commands
            .spawn((
                Mesh2d(meshes.add(unit_rect_mesh())),
                MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::WHITE))),
                Transform {
                    translation: Vec3::new(0.0, 6.0, 0.03),
                    rotation: Quat::IDENTITY,
                    scale: Vec3::new(0.9, 12.0, 1.0),
                },
                outline_visibility,
                ShipNoseMesh,
            ))
            .id();

        commands.entity(entity).add_child(outline);
        commands.entity(entity).add_child(nose);
    }
}

/// Attach a filled elongated `Mesh2d` to every newly-fired projectile.
///
/// Runs once per projectile entity (via [`Added<Projectile>`]).
/// The mesh is oriented along +Y; a separate system rotates it to match velocity.
pub fn attach_projectile_mesh_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Velocity, &mut Transform), Added<Projectile>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    overlay: Res<OverlayState>,
) {
    const PROJ_RADIUS: f32 = 2.0;
    const PROJ_LENGTH: f32 = 10.0;
    const OUTLINE_RADIUS: f32 = 3.0;
    const OUTLINE_THICKNESS: f32 = 0.8;
    let mat_handle = materials.add(ColorMaterial::from_color(Color::srgb(1.0, 0.85, 0.1)));
    for (entity, velocity, mut transform) in query.iter_mut() {
        let mesh_handle = meshes.add(elongated_projectile_mesh(PROJ_RADIUS, PROJ_LENGTH, 16));
        let visibility = if overlay.wireframe_only {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };

        // Orient the projectile mesh in the direction of travel
        let direction = velocity.linvel.normalize_or_zero();
        let angle = direction.y.atan2(direction.x) - std::f32::consts::FRAC_PI_2;
        transform.rotation = Quat::from_rotation_z(angle);

        commands.entity(entity).insert((
            Mesh2d(mesh_handle),
            MeshMaterial2d(mat_handle.clone()),
            visibility,
        ));

        let outline_visibility = if overlay.show_projectile_outline || overlay.wireframe_only {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let outline = commands
            .spawn((
                Mesh2d(meshes.add(ring_mesh(OUTLINE_RADIUS, OUTLINE_THICKNESS, 24))),
                MeshMaterial2d(
                    materials.add(ColorMaterial::from_color(Color::srgb(1.0, 0.9, 0.2))),
                ),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.02)),
                outline_visibility,
                ProjectileOutlineMesh,
            ))
            .id();
        commands.entity(entity).add_child(outline);
    }
}

/// Attach a filled rocket-shaped `Mesh2d` to every newly-fired missile.
///
/// Missiles are rendered as an orange rocket with a pointed nose and fins,
/// oriented in the direction of travel.
pub fn attach_missile_mesh_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Velocity, &mut Transform), Added<Missile>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    overlay: Res<OverlayState>,
) {
    const BODY_WIDTH: f32 = 6.0;
    const BODY_LENGTH: f32 = 12.0;
    const NOSE_LENGTH: f32 = 6.0;
    const FIN_SIZE: f32 = 4.0;
    const OUTLINE_RADIUS: f32 = 5.5;
    const OUTLINE_THICKNESS: f32 = 1.0;

    let mat_handle = materials.add(ColorMaterial::from_color(Color::srgb(1.0, 0.45, 0.05)));
    for (entity, velocity, mut transform) in query.iter_mut() {
        let mesh_handle = meshes.add(rocket_mesh(BODY_WIDTH, BODY_LENGTH, NOSE_LENGTH, FIN_SIZE));
        let visibility = if overlay.wireframe_only {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };

        // Orient the rocket mesh in the direction of travel
        let direction = velocity.linvel.normalize_or_zero();
        let angle = direction.y.atan2(direction.x) - std::f32::consts::FRAC_PI_2;
        transform.rotation = Quat::from_rotation_z(angle);

        commands.entity(entity).insert((
            Mesh2d(mesh_handle),
            MeshMaterial2d(mat_handle.clone()),
            visibility,
        ));

        let outline_visibility = if overlay.show_projectile_outline || overlay.wireframe_only {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        let outline = commands
            .spawn((
                Mesh2d(meshes.add(ring_mesh(OUTLINE_RADIUS, OUTLINE_THICKNESS, 28))),
                MeshMaterial2d(
                    materials.add(ColorMaterial::from_color(Color::srgb(1.0, 0.45, 0.05))),
                ),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.02)),
                outline_visibility,
                MissileOutlineMesh,
            ))
            .id();
        commands.entity(entity).add_child(outline);
    }
}

/// Spawn the health bar and aim indicator `Mesh2d` entities the first time
/// a [`Player`] entity appears, and register them in [`PlayerUiEntities`].
///
/// All three entities live in **world space** (not as children of the player)
/// so their positions are unaffected by the ship’s rotation.  A sync system
/// updates their transforms every frame.
///
/// Runs on [`Added<Player>`] — zero per-frame overhead for existing players.
pub fn attach_player_ui_system(
    mut commands: Commands,
    query: Query<Entity, Added<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut ui: ResMut<PlayerUiEntities>,
) {
    for _ in query.iter() {
        // ── Health bar background (dark-red track) ───────────────────────────
        let bg_mesh = meshes.add(unit_rect_mesh());
        let bg_mat = materials.add(ColorMaterial::from_color(Color::srgba(0.4, 0.0, 0.0, 0.85)));
        let bg_entity = commands
            .spawn((
                Mesh2d(bg_mesh),
                MeshMaterial2d(bg_mat),
                Transform::default(),
                HealthBarBg,
            ))
            .id();

        // ── Health bar fill (green→red) ────────────────────────────────────
        let fill_mesh = meshes.add(unit_rect_mesh());
        let fill_mat_handle = materials.add(ColorMaterial::from_color(Color::srgb(0.0, 1.0, 0.0)));
        let fill_entity = commands
            .spawn((
                Mesh2d(fill_mesh),
                MeshMaterial2d(fill_mat_handle.clone()),
                Transform::default(),
                HealthBarFill(fill_mat_handle),
            ))
            .id();

        // ── Aim direction arrow ──────────────────────────────────────────────
        let aim_mesh = meshes.add(aim_arrow_mesh());
        let aim_mat = materials.add(ColorMaterial::from_color(Color::srgb(1.0, 0.5, 0.0)));
        let aim_entity = commands
            .spawn((
                Mesh2d(aim_mesh),
                MeshMaterial2d(aim_mat),
                Transform::default(),
                Visibility::Hidden, // starts hidden; shown only when aim is active
                AimIndicatorMesh,
            ))
            .id();

        *ui = PlayerUiEntities {
            health_bar_bg: Some(bg_entity),
            health_bar_fill: Some(fill_entity),
            aim_indicator: Some(aim_entity),
        };
    }
}

/// Update the health bar position, scale, and colour every frame.
///
/// The bar hovers at a fixed world-space offset (0, +18) above the player
/// regardless of the ship’s rotation.  The fill width and colour are
/// proportional to the remaining HP fraction.
#[allow(clippy::type_complexity)]
pub fn sync_player_health_bar_system(
    q_player: Query<(&Transform, &PlayerHealth), With<Player>>,
    ui: Res<PlayerUiEntities>,
    mut q_bg: Query<&mut Transform, (With<HealthBarBg>, Without<Player>, Without<HealthBarFill>)>,
    mut q_fill: Query<(&HealthBarFill, &mut Transform), (Without<Player>, Without<HealthBarBg>)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let Ok((ptrans, health)) = q_player.single() else {
        return;
    };
    let hp_frac = (health.hp / health.max_hp).clamp(0.0, 1.0);
    let pos = ptrans.translation.truncate();
    const BAR_HALF: f32 = 20.0;
    const BAR_HEIGHT: f32 = 2.0;
    const BAR_Y: f32 = 18.0;

    // Background track: full width, centred above player.
    if let Some(bg_ent) = ui.health_bar_bg {
        if let Ok(mut t) = q_bg.get_mut(bg_ent) {
            t.translation = Vec3::new(pos.x, pos.y + BAR_Y, 0.8);
            t.scale = Vec3::new(BAR_HALF * 2.0, BAR_HEIGHT, 1.0);
            t.rotation = Quat::IDENTITY;
        }
    }

    // Fill: width proportional to HP, anchored at the left edge.
    if let Some(fill_ent) = ui.health_bar_fill {
        if let Ok((fill_comp, mut t)) = q_fill.get_mut(fill_ent) {
            let fill_w = (BAR_HALF * 2.0 * hp_frac).max(0.01);
            let fill_x = pos.x - BAR_HALF + fill_w * 0.5;
            t.translation = Vec3::new(fill_x, pos.y + BAR_Y, 1.0);
            t.scale = Vec3::new(fill_w, BAR_HEIGHT, 1.0);
            t.rotation = Quat::IDENTITY;
            // Colour: green at full HP → red at empty.
            if let Some(mat) = materials.get_mut(&fill_comp.0) {
                mat.color = Color::srgb(1.0 - hp_frac, hp_frac, 0.0);
            }
        }
    }
}

/// Update the aim indicator arrow position, orientation, and visibility.
///
/// The arrow is a world-space entity centred at the player’s position and
/// rotated to point toward [`AimDirection`].  It is hidden when the aim
/// overlay is disabled or the aim vector is near-zero.
#[allow(clippy::type_complexity)]
pub fn sync_aim_indicator_system(
    q_player: Query<&Transform, With<Player>>,
    ui: Res<PlayerUiEntities>,
    mut q_aim: Query<(&mut Transform, &mut Visibility), (With<AimIndicatorMesh>, Without<Player>)>,
    aim: Res<AimDirection>,
    overlay: Res<OverlayState>,
) {
    let Ok(ptrans) = q_player.single() else {
        return;
    };
    let Some(aim_ent) = ui.aim_indicator else {
        return;
    };
    let Ok((mut t, mut vis)) = q_aim.get_mut(aim_ent) else {
        return;
    };

    if overlay.show_aim_indicator && aim.0.length_squared() > 0.01 {
        // The arrow mesh points in local +Y; rotate so +Y faces aim.0 in world space.
        // atan2(y,x) gives angle from +X; subtract PI/2 to align +Y instead of +X.
        let aim_angle = aim.0.y.atan2(aim.0.x);
        t.rotation = Quat::from_rotation_z(aim_angle - std::f32::consts::FRAC_PI_2);
        t.translation = ptrans.translation.with_z(2.0);
        t.scale = Vec3::ONE;
        *vis = Visibility::Visible;
    } else {
        *vis = Visibility::Hidden;
    }
}

/// Despawn the floating health bar and aim indicator entities when the player
/// is removed (i.e., when they are destroyed).
///
/// Uses [`RemovedComponents<Player>`] which fires in the frame after the
/// player entity is despawned, by which point the entity IDs in
/// [`PlayerUiEntities`] are safe to despawn via `Commands`.
pub fn cleanup_player_ui_system(
    mut removed: RemovedComponents<Player>,
    mut ui: ResMut<PlayerUiEntities>,
    mut commands: Commands,
) {
    for _ in removed.read() {
        if let Some(e) = ui.health_bar_bg.take() {
            commands.entity(e).despawn();
        }
        if let Some(e) = ui.health_bar_fill.take() {
            commands.entity(e).despawn();
        }
        if let Some(e) = ui.aim_indicator.take() {
            commands.entity(e).despawn();
        }
    }
}

/// Update projectile mesh rotation to match velocity direction.
///
/// Projectiles are elongated along their local +Y axis; this system rotates
/// each projectile so +Y points in the direction of travel.
pub fn sync_projectile_rotation_system(
    mut query: Query<(&Velocity, &mut Transform), With<Projectile>>,
) {
    for (velocity, mut transform) in query.iter_mut() {
        if velocity.linvel.length_squared() > 0.01 {
            let direction = velocity.linvel.normalize();
            let angle = direction.y.atan2(direction.x) - std::f32::consts::FRAC_PI_2;
            transform.rotation = Quat::from_rotation_z(angle);
        }
    }
}

/// Propagate the `wireframe_only` flag to all live ship, projectile, and missile meshes.
///
/// Only runs when [`OverlayState`] changes, so the per-frame cost is negligible.
#[allow(clippy::type_complexity)]
pub fn sync_player_and_projectile_mesh_visibility_system(
    overlay: Res<OverlayState>,
    mut q_ship: Query<
        &mut Visibility,
        (
            With<Player>,
            With<Mesh2d>,
            Without<Projectile>,
            Without<Missile>,
        ),
    >,
    mut q_proj: Query<
        &mut Visibility,
        (
            With<Projectile>,
            With<Mesh2d>,
            Without<Player>,
            Without<Missile>,
        ),
    >,
    mut q_missile: Query<
        &mut Visibility,
        (
            With<Missile>,
            With<Mesh2d>,
            Without<Player>,
            Without<Projectile>,
        ),
    >,
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
    for mut v in q_missile.iter_mut() {
        *v = vis;
    }
}

/// Show/hide retained projectile and missile outlines from overlay toggles.
#[allow(clippy::type_complexity)]
pub fn sync_projectile_outline_visibility_system(
    overlay: Res<OverlayState>,
    mut outlines: ParamSet<(
        Query<&mut Visibility, With<ProjectileOutlineMesh>>,
        Query<&mut Visibility, With<MissileOutlineMesh>>,
    )>,
) {
    if !overlay.is_changed() {
        return;
    }

    let vis = if overlay.show_projectile_outline || overlay.wireframe_only {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    for mut v in outlines.p0().iter_mut() {
        *v = vis;
    }
    for mut v in outlines.p1().iter_mut() {
        *v = vis;
    }
}

/// Show/hide retained ship outline meshes and update HP-based tint.
#[allow(clippy::type_complexity)]
pub fn sync_ship_outline_visibility_and_color_system(
    q_player: Query<&PlayerHealth, With<Player>>,
    overlay: Res<OverlayState>,
    mut ship_outline: ParamSet<(
        Query<(&ShipOutlineMesh, &mut Visibility)>,
        Query<&mut Visibility, With<ShipNoseMesh>>,
    )>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let Ok(health) = q_player.single() else {
        return;
    };

    let hp_frac = (health.hp / health.max_hp).clamp(0.0, 1.0);
    let ship_color = Color::srgb(1.0 - hp_frac * 0.8, hp_frac * 0.6 + 0.2, hp_frac);
    let vis = if overlay.show_ship_outline || overlay.wireframe_only {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    for (outline, mut visibility) in ship_outline.p0().iter_mut() {
        *visibility = vis;
        if let Some(mat) = materials.get_mut(&outline.0) {
            mat.color = ship_color;
        }
    }
    for mut visibility in ship_outline.p1().iter_mut() {
        *visibility = vis;
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
