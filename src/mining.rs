//! Ore pickup system: spawns ore drops when asteroids are destroyed; collected
//! by the player on contact.
//!
//! ## Flow
//!
//! 1. `spawn_ore_drop()` is called by the combat system whenever an asteroid is
//!    terminally destroyed (size 0–1 bullet hit, or missile hit on size ≤ 3).
//! 2. The ore entity drifts with the parent asteroid's velocity plus a small
//!    random scatter, and rotates slowly for visibility.
//! 3. `ore_collection_system` listens for `CollisionEvent::Started`; when the
//!    player overlaps an ore sensor, the ore entity is despawned and
//!    [`PlayerOre::count`] is incremented.
//! 4. Ore entities older than [`ORE_LIFETIME_SECS`] are automatically despawned.
//! 5. Ore can be spent via the in-game **Ore Shop** (Tab key, or Pause → Ore Shop).
//!
//! ## Collision groups
//!
//! | Layer | Group  | Collides with |
//! |-------|--------|---------------|
//! | Ore   | GROUP_4 | GROUP_2 (player only) |
//!
//! Using a dedicated group keeps ore events completely separate from the
//! existing asteroid (GROUP_1) ↔ player (GROUP_2) channel.  The player
//! `CollisionGroups` filter is broadened to `GROUP_1 | GROUP_4` so the
//! player-ore sensor events fire correctly.

use crate::menu::GameState;
use crate::player::Player;
use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use bevy_mesh::{Indices, PrimitiveTopology};
use bevy_rapier2d::prelude::*;
use rand::Rng;

/// How long an ore pickup lingers in space before auto-despawning (seconds).
const ORE_LIFETIME_SECS: f32 = 25.0;

/// Visual half-width and half-height of the diamond mesh (world units).
const ORE_HALF_W: f32 = 3.5;
const ORE_HALF_H: f32 = 5.5;

/// Radius of the pickup sensor — larger than the visual for forgiving collection.
const ORE_COLLIDER_RADIUS: f32 = 8.0;

// ── Components & Resources ────────────────────────────────────────────────────

/// Marker component for ore pickup entities.
#[derive(Component)]
pub struct OrePickup;

/// Seconds this ore entity has been alive.
#[derive(Component)]
struct OreAge(f32);

/// The player's total accumulated ore.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct PlayerOre {
    pub count: u32,
}

// ══════════════════════════════════════════════════════════════════════════════
// Ore Magnet (Affinity) Upgrade Level
// ══════════════════════════════════════════════════════════════════════════════

/// Ore magnet upgrade level (affinity for ore collection).
///
/// Higher levels increase both the magnet's pull radius and velocity (strength).
/// Base values are intentionally weak (40 u/s at level 0) to make upgrades feel
/// impactful and rewarding.
///
/// Example progression (internal level → radius / strength):
/// - Level 0 (base): 250 u radius, 40 u/s pull
/// - Level 1: 300 u radius, 56 u/s pull
/// - Level 5: 500 u radius, 120 u/s pull
/// - Level 9 (max): 700 u radius, 184 u/s pull
#[derive(Resource, Debug, Clone, Default)]
pub struct OreAffinityLevel {
    /// Internal 0-indexed level (0 = Level 1 / base, 9 = Level 10 / max).
    pub level: u32,
}

impl OreAffinityLevel {
    /// Maximum internal level value (inclusive).
    pub const MAX: u32 = crate::constants::ORE_AFFINITY_MAX_LEVEL - 1;

    /// Human-readable display level (1-indexed).
    #[inline]
    pub fn display_level(&self) -> u32 {
        self.level + 1
    }

    /// Magnet pull radius at the current level (world units).
    #[inline]
    pub fn radius_at_level(&self) -> f32 {
        crate::constants::ORE_MAGNET_BASE_RADIUS + (self.level as f32 * 50.0)
    }

    /// Magnet pull strength at the current level (velocity magnitude, u/s).
    #[inline]
    pub fn strength_at_level(&self) -> f32 {
        crate::constants::ORE_MAGNET_BASE_STRENGTH + (self.level as f32 * 16.0)
    }

    /// Whether the magnet can be upgraded further.
    #[inline]
    pub fn is_maxed(&self) -> bool {
        self.level >= Self::MAX
    }

    /// Ore cost to buy the next upgrade level.
    /// Returns `None` when already at max level.
    #[inline]
    pub fn cost_for_next_level(&self) -> Option<u32> {
        if self.is_maxed() {
            None
        } else {
            // next_level (1-indexed) × base cost: 5, 10, 15, …, 50
            Some(crate::constants::ORE_AFFINITY_UPGRADE_BASE_COST * (self.level + 2))
        }
    }

    /// Returns `true` when the player has enough ore to afford the next upgrade.
    #[inline]
    pub fn can_afford_next(&self, ore: u32) -> bool {
        self.cost_for_next_level().is_some_and(|cost| ore >= cost)
    }

    /// Spend ore and increment the level.  Returns the amount spent, or `None`
    /// if maxed-out or the player cannot afford it.
    pub fn try_upgrade(&mut self, ore: &mut u32) -> Option<u32> {
        let cost = self.cost_for_next_level()?;
        if *ore < cost {
            return None;
        }
        *ore -= cost;
        self.level += 1;
        Some(cost)
    }
}

/// Shared mesh handle for all ore diamond visuals (created once at startup).
#[derive(Resource)]
struct OreMesh(Handle<Mesh>);

// ── Plugin ────────────────────────────────────────────────────────────────────

pub struct MiningPlugin;

impl Plugin for MiningPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerOre>()
            .init_resource::<OreAffinityLevel>()
            .add_systems(Startup, setup_ore_mesh)
            .add_systems(
                Update,
                (
                    attach_ore_mesh_system,
                    ore_lifetime_system,
                    ore_magnet_system,
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                // Run alongside the other hit systems that read CollisionEvents.
                PostUpdate,
                ore_collection_system.run_if(in_state(GameState::Playing)),
            );
    }
}

// ── Startup ───────────────────────────────────────────────────────────────────

fn setup_ore_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let handle = meshes.add(diamond_mesh(ORE_HALF_W, ORE_HALF_H));
    commands.insert_resource(OreMesh(handle));
}

// ── Spawn helper ──────────────────────────────────────────────────────────────

/// Spawn an ore pickup at `pos` with `base_vel` plus a small random scatter.
///
/// Called by the combat system on terminal asteroid destruction.
pub fn spawn_ore_drop(commands: &mut Commands, pos: Vec2, base_vel: Vec2) {
    let mut rng = rand::thread_rng();
    let scatter = Vec2::new(rng.gen_range(-18.0..18.0), rng.gen_range(-18.0..18.0));
    commands.spawn((
        OrePickup,
        OreAge(0.0),
        Transform::from_translation(pos.extend(0.2)), // Z slightly above asteroids
        Visibility::default(),
        RigidBody::KinematicVelocityBased,
        Velocity {
            linvel: base_vel + scatter,
            angvel: rng.gen_range(1.2..2.8),
        },
        Collider::ball(ORE_COLLIDER_RADIUS),
        Sensor,
        CollisionGroups::new(
            bevy_rapier2d::geometry::Group::GROUP_4,
            bevy_rapier2d::geometry::Group::GROUP_2,
        ),
        // Kinematic body detecting a dynamic body (the player).
        ActiveCollisionTypes::DYNAMIC_KINEMATIC,
        ActiveEvents::COLLISION_EVENTS,
    ));
}

// ── Systems ───────────────────────────────────────────────────────────────────

/// Attach a filled diamond `Mesh2d` to every freshly-spawned ore pickup.
fn attach_ore_mesh_system(
    mut commands: Commands,
    query: Query<Entity, Added<OrePickup>>,
    ore_mesh: Res<OreMesh>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for entity in query.iter() {
        let mat = materials.add(ColorMaterial::from_color(Color::srgb(0.25, 0.95, 0.50)));
        commands
            .entity(entity)
            .insert((Mesh2d(ore_mesh.0.clone()), MeshMaterial2d(mat)));
    }
}

/// Pull ore pickups toward the player when they are within `ore_magnet_radius`.
///
/// Uses a velocity lerp so the attraction feels smooth rather than a hard snap:
/// each frame the ore's `linvel` is blended toward a target vector pointing
/// directly at the player at `ore_magnet_strength` u/s.  Outside the magnet
/// radius the ore velocity is left completely untouched.
fn ore_magnet_system(
    affinity_level: Res<OreAffinityLevel>,
    time: Res<Time>,
    q_player: Query<&Transform, With<Player>>,
    mut q_ore: Query<(&Transform, &mut Velocity), With<OrePickup>>,
) {
    let Ok(player_transform) = q_player.single() else {
        return;
    };
    let player_pos = player_transform.translation.truncate();
    let radius = affinity_level.radius_at_level();
    let radius_sq = radius * radius;
    let dt = time.delta_secs();
    // Lerp alpha: at 4 × dt the velocity rotates ~14% per frame (≈60 fps →
    // fully pointing at player in ~0.25 s), giving a smooth but responsive pull.
    let alpha = (dt * 4.0).min(1.0);

    for (ore_transform, mut vel) in q_ore.iter_mut() {
        let ore_pos = ore_transform.translation.truncate();
        let delta = player_pos - ore_pos;
        if delta.length_squared() > radius_sq {
            continue;
        }
        // direction is guaranteed non-zero: ore can't overlap the player sensor
        // without already triggering collection.
        let target_linvel = delta.normalize_or_zero() * affinity_level.strength_at_level();
        vel.linvel = vel.linvel.lerp(target_linvel, alpha);
    }
}

/// Tick ore age and despawn pickups that have exceeded their lifetime.
fn ore_lifetime_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut OreAge)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (entity, mut age) in query.iter_mut() {
        age.0 += dt;
        if age.0 >= ORE_LIFETIME_SECS {
            commands.entity(entity).despawn();
        }
    }
}

/// Collect ore when the player's sensor overlaps an ore pickup.
pub fn ore_collection_system(
    mut commands: Commands,
    mut collision_events: MessageReader<CollisionEvent>,
    q_ore: Query<Entity, With<OrePickup>>,
    q_player: Query<Entity, With<Player>>,
    mut ore: ResMut<PlayerOre>,
) {
    let Ok(player_entity) = q_player.single() else {
        return;
    };

    for event in collision_events.read() {
        let (e1, e2) = match event {
            CollisionEvent::Started(e1, e2, _) => (*e1, *e2),
            CollisionEvent::Stopped(..) => continue,
        };

        let ore_entity = if q_ore.contains(e1) && e2 == player_entity {
            e1
        } else if q_ore.contains(e2) && e1 == player_entity {
            e2
        } else {
            continue;
        };

        commands.entity(ore_entity).despawn();
        ore.count += 1;
    }
}

// ── Mesh helper ───────────────────────────────────────────────────────────────

/// Build a filled diamond (rhombus) mesh with the given half-extents.
fn diamond_mesh(half_w: f32, half_h: f32) -> Mesh {
    // 4 vertices: top, right, bottom, left
    let positions: Vec<[f32; 3]> = vec![
        [0.0, half_h, 0.0],
        [half_w, 0.0, 0.0],
        [0.0, -half_h, 0.0],
        [-half_w, 0.0, 0.0],
    ];
    // Two CCW triangles sharing the horizontal axis: [top,right,left] [right,bottom,left]
    let indices = Indices::U32(vec![0, 1, 3, 1, 2, 3]);
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(indices);
    mesh
}
