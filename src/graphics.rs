use crate::particle::Particle;
use crate::rigid_body::{RigidBodyColor, RigidBodyGroup};
use bevy::prelude::*;

/// System: add sprites to rigid bodies on formation
pub fn rigid_body_rendering_system(
    mut commands: Commands,
    rigid_query: Query<(Entity, &RigidBodyColor), (With<RigidBodyGroup>, Without<Sprite>)>,
) {
    for (entity, color) in rigid_query.iter() {
        commands.entity(entity).insert(Sprite {
            color: color.0,
            custom_size: Some(Vec2::splat(24.0)),
            ..Default::default()
        });
    }
}

/// Debug system to monitor simulation state
pub fn debug_simulation_state(
    particle_query: Query<&Transform, With<Particle>>,
    rigid_query: Query<Entity, With<RigidBodyGroup>>,
) {
    let particle_count = particle_query.iter().count();
    let rigid_count = rigid_query.iter().count();
    
    if particle_count > 0 || rigid_count > 0 {
        eprintln!("[SIM] Particles: {}, Rigid Bodies: {}", particle_count, rigid_count);
        
        // Show first particle position for debugging
        if let Some(transform) = particle_query.iter().next() {
            let pos = transform.translation;
            eprintln!("[CAM] First particle at: ({:.1}, {:.1}, {:.1})", pos.x, pos.y, pos.z);
        }
    }
}

/// Setup camera for 2D rendering
pub fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    eprintln!("[SETUP] Camera initialized at origin (0, 0)");
}
