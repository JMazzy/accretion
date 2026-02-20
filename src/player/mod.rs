//! Player module: ship entity, input handling, combat, and rendering.
//!
//! ## Sub-module layout
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`state`] | ECS components (`Player`, `PlayerHealth`, `Projectile`) and Bevy resources (`AimDirection`, `PreferredGamepad`, `PlayerFireCooldown`) |
//! | [`control`] | Input systems: WASD thrust, A/D rotation, gamepad left-stick movement, out-of-bounds damping |
//! | [`combat`] | Projectile firing, lifetime management, player-asteroid damage, asteroid splitting/chipping |
//! | [`rendering`] | Ship gizmo outline, health bar, aim indicator, projectile circles, camera follow |
//!
//! All public items are re-exported at this level so that the rest of the crate
//! can continue to use flat `crate::player::*` imports without knowing the
//! sub-module layout.

pub mod combat;
pub mod control;
pub mod rendering;
pub mod state;

// ── Flat re-exports (backward-compatible API surface) ─────────────────────────

pub use combat::{
    despawn_old_projectiles_system, player_collision_damage_system, projectile_asteroid_hit_system,
    projectile_fire_system,
};
pub use control::{
    aim_snap_system, apply_player_intent_system, gamepad_connection_system,
    gamepad_to_intent_system, keyboard_to_intent_system, player_intent_clear_system,
    player_oob_damping_system,
};
pub use rendering::{
    attach_player_ship_mesh_system, attach_projectile_mesh_system, camera_follow_system,
    player_gizmo_system, sync_player_and_projectile_mesh_visibility_system,
};
pub use state::{
    AimDirection, AimIdleTimer, Player, PlayerFireCooldown, PlayerHealth, PlayerIntent,
    PlayerScore, PreferredGamepad,
};

// ── Ship spawn ─────────────────────────────────────────────────────────────────

use crate::config::PhysicsConfig;
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

/// Spawn the player's ship entity at the world origin.
///
/// The ship uses a ball collider (`PLAYER_COLLIDER_RADIUS`) rather than a convex
/// polygon collider — this simplifies physics interactions and is visually
/// indistinguishable at normal zoom levels since the ship shape is drawn via gizmos.
///
/// Collision groups:
/// - `GROUP_2` — ship belongs to this group
/// - collides with `GROUP_1` (asteroids) only; not with `GROUP_3` (projectiles)
pub fn spawn_player(mut commands: Commands, config: Res<PhysicsConfig>) {
    commands.spawn((
        Player,
        PlayerHealth::default(),
        // Physics
        RigidBody::Dynamic,
        Collider::ball(config.player_collider_radius),
        Velocity::zero(),
        ExternalForce::default(),
        Damping {
            linear_damping: config.player_linear_damping,
            angular_damping: config.player_angular_damping,
        },
        Restitution::coefficient(config.player_restitution),
        CollisionGroups::new(
            bevy_rapier2d::geometry::Group::GROUP_2,
            bevy_rapier2d::geometry::Group::GROUP_1,
        ),
        ActiveEvents::COLLISION_EVENTS,
        // Transform / visibility
        Transform::from_translation(Vec3::ZERO),
        Visibility::default(),
    ));

    println!("✓ Player ship spawned at origin");
}
