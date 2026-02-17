//! Particle ECS components and spawn system for Bevy

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

/// Marker for a particle entity
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Particle;

/// Particle color (RGBA)
#[derive(Component, Debug, Clone, Copy)]
pub struct ParticleColor(pub Color);

/// Group ID for locked/grouped particles
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupId(pub u32);

/// Whether this particle is locked to others
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Locked(pub bool);

/// Count of nearby particles for environmental damping calculation
#[derive(Component, Debug, Clone, Copy)]
pub struct NeighborCount(pub usize);

/// Tracks if particle has come to rest
#[derive(Component, Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Resting(pub bool);

/// Marker for rigid body entities formed from particle groups
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub struct RigidBodyMarker;

/// Spawns a single particle at the given position
pub fn spawn_particle(commands: &mut Commands, position: Vec2, color: Color, group_id: u32) {
    commands.spawn((
        Particle,
        ParticleColor(color),
        GroupId(group_id),
        Locked(false),
        NeighborCount(0),
        Resting(false),
        RigidBody::Dynamic,
        Collider::ball(2.0),
        Restitution::coefficient(0.5),
        Transform::from_translation(position.extend(0.0)),
        GlobalTransform::default(),
        Velocity::zero(),
        Damping {
            linear_damping: 0.0,
            angular_damping: 0.0,
        },
        ExternalForce {
            force: Vec2::ZERO,
            torque: 0.0,
        },
        Sleeping::disabled(),
    ));
}

// ...existing code...
// (Obsolete impls and trailing code removed for ECS migration)
