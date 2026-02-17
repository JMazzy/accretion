use bevy::prelude::*;
use crate::particle::{Particle, ParticleColor};
use crate::rigid_body::{RigidBodyGroup, RigidBodyColor};

/// System: spawn a sprite for each particle (if not present)
pub fn particle_rendering_system(
    mut commands: Commands,
    particle_query: Query<(Entity, &ParticleColor), (With<Particle>, Without<Sprite>)>,
    rigid_query: Query<(Entity, &RigidBodyColor), (With<RigidBodyGroup>, Without<Sprite>)>,
) {
    for (entity, color) in particle_query.iter() {
        commands.entity(entity).insert(Sprite {
            color: color.0,
            custom_size: Some(Vec2::splat(4.0)),
            ..Default::default()
        });
    }
    
    for (entity, color) in rigid_query.iter() {
        commands.entity(entity).insert(Sprite {
            color: color.0,
            custom_size: Some(Vec2::splat(20.0)),
            ..Default::default()
        });
    }
}

/// Setup camera for 2D rendering
pub fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
