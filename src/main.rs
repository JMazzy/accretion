use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
mod particle;
mod simulation;
mod rigid_body;
mod graphics;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Particle Simulation".into(),
                        resolution: (1200.0, 680.0).into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(50.0))
        .insert_resource(RapierConfiguration {
            gravity: Vec2::ZERO,
            ..Default::default()
        })
        .add_plugins(simulation::SimulationPlugin)
        .add_systems(Startup, graphics::setup_camera)
        .add_systems(Update, graphics::particle_rendering_system)
        .run();
}
