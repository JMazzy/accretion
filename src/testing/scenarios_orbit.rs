use crate::asteroid::{spawn_asteroid_with_vertices, AsteroidSize};
use crate::config::PhysicsConfig;
use bevy::prelude::*;
use bevy_rapier2d::prelude::{ReadMassProperties, Velocity};

use super::{OrbitCentralBody, OrbitTestBody, TestConfig};

/// Spawn scenario for the `orbit_pair` test.
pub fn spawn_test_orbit_pair(
    mut commands: Commands,
    mut test_config: ResMut<TestConfig>,
    config: Res<PhysicsConfig>,
) {
    test_config.test_name = "orbit_pair".to_string();
    test_config.frame_limit = 1500;
    test_config.velocity_calibrated = false;
    test_config.orbit_initial_dist = 0.0;
    test_config.orbit_final_dist = 0.0;

    let central_mass: u32 = 2_000_000;
    let central_radius = 10.0_f32;
    let central_verts: Vec<Vec2> = (0..16)
        .map(|i| {
            let angle = std::f32::consts::TAU * i as f32 / 16.0;
            Vec2::new(central_radius * angle.cos(), central_radius * angle.sin())
        })
        .collect();
    let central_entity = spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::ZERO,
        &central_verts,
        Color::srgb(0.9, 0.5, 0.1),
        central_mass,
    );
    commands
        .entity(central_entity)
        .insert((ReadMassProperties::default(), OrbitCentralBody));

    let orbital_radius = 200.0_f32;
    let side = config.triangle_base_side;
    let h = side * 3.0_f32.sqrt() / 2.0;
    let tri_verts = vec![
        Vec2::new(0.0, h / 2.0),
        Vec2::new(-side / 2.0, -h / 2.0),
        Vec2::new(side / 2.0, -h / 2.0),
    ];
    let orbit_entity = spawn_asteroid_with_vertices(
        &mut commands,
        Vec2::new(orbital_radius, 0.0),
        &tri_verts,
        Color::srgb(0.4, 0.9, 0.4),
        1,
    );
    commands
        .entity(orbit_entity)
        .insert((ReadMassProperties::default(), OrbitTestBody));

    println!(
        "✓ Spawned orbit_pair test: central at (0,0) r={central_radius} mass={central_mass}, \
         orbiter at ({orbital_radius},0) side={side}"
    );
    println!(
        "  Expected Rapier mass of triangle ≈ {:.4} (√3/4·{side}²)",
        3.0_f32.sqrt() / 4.0 * side * side
    );
}

/// Calibrates orbital velocity from actual Rapier mass and tracks orbit radius.
#[allow(clippy::type_complexity)]
pub fn orbit_pair_calibrate_and_track_system(
    mut test_config: ResMut<TestConfig>,
    config: Res<PhysicsConfig>,
    central_q: Query<(&Transform, &AsteroidSize), (With<OrbitCentralBody>, Without<OrbitTestBody>)>,
    mut orbit_q: Query<
        (&Transform, &ReadMassProperties, &mut Velocity),
        (With<OrbitTestBody>, Without<OrbitCentralBody>),
    >,
) {
    if !test_config.enabled || test_config.test_name != "orbit_pair" {
        return;
    }

    let Ok((central_tf, central_size)) = central_q.single() else {
        return;
    };
    let Ok((orbit_tf, mass_props, mut orbit_vel)) = orbit_q.single_mut() else {
        return;
    };

    let central_pos = central_tf.translation.truncate();
    let orbit_pos = orbit_tf.translation.truncate();
    let current_dist = (orbit_pos - central_pos).length();

    if !test_config.velocity_calibrated && test_config.frame_count >= 2 {
        let m_rapier = mass_props.mass;
        let m_central = central_size.0 as f32;
        let g = config.gravity_const;

        let v_mag = (g * m_central / (current_dist * m_rapier)).sqrt();
        let radial = (orbit_pos - central_pos).normalize_or_zero();
        let tangent = Vec2::new(-radial.y, radial.x);
        orbit_vel.linvel = tangent * v_mag;

        test_config.orbit_initial_dist = current_dist;
        test_config.velocity_calibrated = true;

        let period_s = std::f32::consts::TAU * current_dist / v_mag;
        println!(
            "[Orbit calibration] frame={} G={g}  M_central={m_central}  \
             m_rapier={m_rapier:.4}  r={current_dist:.1}  v={v_mag:.4} u/s",
            test_config.frame_count
        );
        println!(
            "[Orbit calibration] Expect period ≈ {period_s:.1}s = {:.0} frames at 60fps",
            period_s * 60.0
        );
    }

    if test_config.velocity_calibrated {
        test_config.orbit_final_dist = current_dist;
    }
}
