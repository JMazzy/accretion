//! Player input and movement systems.
//!
//! ## Pipeline (runs in order every `Update` frame)
//!
//! 1. [`player_intent_clear_system`] — resets `PlayerIntent` and `ExternalForce` to zero.
//! 2. [`keyboard_to_intent_system`] — translates WASD/rotation keys into `PlayerIntent` fields.
//! 3. [`gamepad_to_intent_system`] — translates gamepad left-stick + B-button into `PlayerIntent`.
//! 4. [`apply_player_intent_system`] — converts `PlayerIntent` into `ExternalForce` / `Velocity`.
//!
//! The **input abstraction layer** (`PlayerIntent`) makes the movement logic fully
//! testable: tests populate the resource directly and run only `apply_player_intent_system`.
//!
//! Also contains helper systems that are not part of the core thrust pipeline:
//! - [`gamepad_connection_system`] — tracks which gamepad is preferred
//! - [`aim_snap_system`] — snaps aim to ship forward after idle period
//! - [`player_oob_damping_system`] — soft boundary enforcement

use super::state::{
    AimDirection, AimIdleTimer, Player, PlayerIntent, PreferredGamepad, TractorBeamLevel,
};
use crate::asteroid::{Asteroid, AsteroidSize, Planet};
use crate::config::PhysicsConfig;
use bevy::input::gamepad::{GamepadAxis, GamepadButton, GamepadConnection, GamepadConnectionEvent};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

// ── Step 1: Clear ─────────────────────────────────────────────────────────────

/// Clear `ExternalForce` and `PlayerIntent` to zero at the start of every frame.
///
/// Must run before any system that writes to `PlayerIntent` or accumulates
/// forces.  Running both resets here ensures a single ordered dependency.
pub fn player_intent_clear_system(
    mut q: Query<&mut ExternalForce, With<Player>>,
    mut intent: ResMut<PlayerIntent>,
) {
    if let Ok(mut force) = q.single_mut() {
        force.force = Vec2::ZERO;
        force.torque = 0.0;
    }
    *intent = PlayerIntent::default();
}

// ── Step 2a: Keyboard → Intent ────────────────────────────────────────────────

/// Translate WASD / rotation keys into [`PlayerIntent`].
///
/// - **W** → `thrust_forward = 1.0`
/// - **S** → `thrust_reverse = 1.0`
/// - **A** → `angvel = Some(+ROTATION_SPEED)` (CCW)
/// - **D** → `angvel = Some(−ROTATION_SPEED)` (CW)
///
/// Additive: safe to run alongside gamepad intent system because each field is
/// overwritten, not accumulated (both sources can't be active simultaneously in
/// normal play).
pub fn keyboard_to_intent_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut intent: ResMut<PlayerIntent>,
    config: Res<PhysicsConfig>,
) {
    if keys.pressed(KeyCode::KeyW) {
        intent.thrust_forward = 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        intent.thrust_reverse = 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        intent.angvel = Some(config.rotation_speed);
    } else if keys.pressed(KeyCode::KeyD) {
        intent.angvel = Some(-config.rotation_speed);
    }
}

// ── Step 2b: Gamepad connection ────────────────────────────────────────────────

/// Track gamepad connect / disconnect events and update [`PreferredGamepad`].
///
/// The most-recently-connected gamepad is always preferred, ensuring that
/// non-gamepad HID devices (e.g. RGB LED controllers on Linux) that connect
/// first are superseded by the real gamepad.
pub fn gamepad_connection_system(
    mut events: MessageReader<GamepadConnectionEvent>,
    mut preferred: ResMut<PreferredGamepad>,
) {
    for event in events.read() {
        match &event.connection {
            GamepadConnection::Connected { .. } => {
                preferred.0 = Some(event.gamepad);
                info!(
                    "[gamepad] Gamepad {:?} connected (now preferred)",
                    event.gamepad
                );
            }
            GamepadConnection::Disconnected => {
                info!("[gamepad] Gamepad {:?} disconnected", event.gamepad);
                if preferred.0 == Some(event.gamepad) {
                    preferred.0 = None;
                }
            }
        }
    }
}

// ── Step 2c: Gamepad → Intent ─────────────────────────────────────────────────

/// Translate gamepad left-stick and B-button into [`PlayerIntent`].
///
/// **Left stick**:
/// 1. Sets `angvel` to steer toward the stick heading.
/// 2. Sets `thrust_forward` proportional to stick magnitude.
///
/// **B button (East)**: sets `intent.brake = true`.
///
/// Does nothing when no gamepad is connected ([`PreferredGamepad`] is `None`).
pub fn gamepad_to_intent_system(
    q_transform: Query<&Transform, With<Player>>,
    preferred: Res<PreferredGamepad>,
    gamepads: Query<&Gamepad>,
    mut intent: ResMut<PlayerIntent>,
    mut idle: ResMut<AimIdleTimer>,
    config: Res<PhysicsConfig>,
) {
    let Ok(transform) = q_transform.single() else {
        return;
    };

    let Some(gamepad_entity) = preferred.0 else {
        return;
    };

    let Ok(gamepad) = gamepads.get(gamepad_entity) else {
        return;
    };

    if gamepad.pressed(GamepadButton::East) {
        intent.brake = true;
    }

    let lx = gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
    let ly = gamepad.get(GamepadAxis::LeftStickY).unwrap_or(0.0);
    let left_stick = Vec2::new(lx, ly);

    if left_stick.length() < config.gamepad_left_deadzone {
        return;
    }

    // Left stick is active — prevent aim-direction idle snap.
    idle.secs = 0.0;

    // atan2(-lx, ly) maps: stick (0,1)→0°, (1,0)→−90°, (−1,0)→+90°
    let target_angle = (-lx).atan2(ly);
    let current_angle = transform.rotation.to_euler(EulerRot::ZYX).0;

    let mut angle_diff = target_angle - current_angle;
    while angle_diff > std::f32::consts::PI {
        angle_diff -= std::f32::consts::TAU;
    }
    while angle_diff < -std::f32::consts::PI {
        angle_diff += std::f32::consts::TAU;
    }

    intent.angvel = Some(
        if angle_diff.abs() > config.gamepad_heading_snap_threshold {
            config.rotation_speed * angle_diff.signum()
        } else {
            0.0
        },
    );

    intent.thrust_forward = left_stick.length().min(1.0);
}

// ── Step 3: Apply intent → physics ───────────────────────────────────────────

/// Convert [`PlayerIntent`] into `ExternalForce` and `Velocity` on the ship.
///
/// This is the **only** system that writes physics outputs; all input systems
/// only write to `PlayerIntent`.  This separation is what makes thrust testable:
/// tests populate `PlayerIntent` directly and call this system in isolation.
///
/// | Intent field        | Physics effect                                        |
/// |---------------------|-------------------------------------------------------|
/// | `thrust_forward`    | `force += local_forward * THRUST_FORCE * thrust_forward` |
/// | `thrust_reverse`    | `force -= local_forward * REVERSE_FORCE * thrust_reverse` |
/// | `angvel = Some(v)`  | `velocity.angvel = v`                                |
/// | `angvel = None`     | angular velocity left to Rapier damping               |
/// | `brake = true`      | `linvel *= GAMEPAD_BRAKE_DAMPING`; `angvel *= …`     |
pub fn apply_player_intent_system(
    mut q: Query<(&Transform, &mut ExternalForce, &mut Velocity), With<Player>>,
    intent: Res<PlayerIntent>,
    config: Res<PhysicsConfig>,
) {
    let Ok((transform, mut force, mut velocity)) = q.single_mut() else {
        return;
    };

    let forward = transform.rotation.mul_vec3(Vec3::Y).truncate();

    if intent.thrust_forward > 0.0 {
        force.force += forward * config.thrust_force * intent.thrust_forward;
    }
    if intent.thrust_reverse > 0.0 {
        force.force -= forward * config.reverse_force * intent.thrust_reverse;
    }
    if let Some(av) = intent.angvel {
        velocity.angvel = av;
    }
    if intent.brake {
        velocity.linvel *= config.gamepad_brake_damping;
        velocity.angvel *= config.gamepad_brake_damping;
    }
}

// ── Aim idle snap ─────────────────────────────────────────────────────────────

/// Snap the aim direction back to the ship's local forward when no aim input
/// has been received for [`AIM_IDLE_SNAP_SECS`] seconds.
///
/// Increments [`AimIdleTimer`] every frame.  When the threshold is crossed and
/// the player entity exists, `AimDirection` is overwritten with the ship's
/// world-space +Y.
pub fn aim_snap_system(
    q_player: Query<&Transform, With<Player>>,
    mut aim: ResMut<AimDirection>,
    mut idle: ResMut<AimIdleTimer>,
    time: Res<Time>,
    config: Res<PhysicsConfig>,
) {
    idle.secs += time.delta_secs();
    if idle.secs >= config.aim_idle_snap_secs {
        if let Ok(transform) = q_player.single() {
            aim.0 = transform.rotation.mul_vec3(Vec3::Y).truncate();
        }
    }
}

// ── Out-of-bounds damping ──────────────────────────────────────────────────────

/// Applies ramped velocity damping when the player drifts outside `OOB_RADIUS`.
///
/// The damping factor ramps smoothly from 0% at the boundary to a maximum of
/// `(1.0 − OOB_DAMPING) × 100%` at `OOB_RADIUS + OOB_RAMP_WIDTH`.
/// The player can always re-enter under thrust; they are never hard-stopped.
pub fn player_oob_damping_system(
    mut q: Query<(&Transform, &mut Velocity), With<Player>>,
    config: Res<PhysicsConfig>,
) {
    let Ok((transform, mut velocity)) = q.single_mut() else {
        return;
    };

    let dist = transform.translation.truncate().length();
    if dist > config.oob_radius {
        let exceed = (dist - config.oob_radius).min(config.oob_ramp_width) / config.oob_ramp_width;
        let factor = 1.0 - exceed * (1.0 - config.oob_damping);
        velocity.linvel *= factor;
        velocity.angvel *= factor;
    }
}

/// Apply tractor beam force to nearby asteroids while `E` is held.
///
/// - Hold `E` to pull asteroids toward the player.
/// - Hold `Alt` + `E` to push asteroids away from the player.
///
/// The beam only affects asteroids inside a forward cone around `AimDirection`
/// and below level-scaled mass/speed thresholds to keep interactions stable.
#[allow(clippy::type_complexity)]
pub fn tractor_beam_force_system(
    keys: Res<ButtonInput<KeyCode>>,
    q_player: Query<&Transform, With<Player>>,
    mut q_asteroids: Query<
        (&Transform, &Velocity, &AsteroidSize, &mut ExternalForce),
        (With<Asteroid>, Without<Planet>),
    >,
    aim: Res<AimDirection>,
    beam_level: Res<TractorBeamLevel>,
    config: Res<PhysicsConfig>,
) {
    if !keys.pressed(KeyCode::KeyE) {
        return;
    }

    let Ok(player_transform) = q_player.single() else {
        return;
    };

    let push_mode = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    let player_pos = player_transform.translation.truncate();
    let beam_dir = if aim.0.length_squared() > 1e-4 {
        aim.0.normalize_or_zero()
    } else {
        player_transform.rotation.mul_vec3(Vec3::Y).truncate()
    };

    let range = beam_level.range_at_level(&config);
    let min_dist = config.tractor_beam_min_distance;
    if range <= min_dist {
        return;
    }
    let max_size = beam_level.max_target_size_at_level(&config);
    let max_speed = beam_level.max_target_speed_at_level(&config);
    let force_base = beam_level.force_at_level(&config);
    let range_sq = range * range;
    let min_dist_sq = min_dist * min_dist;

    for (transform, velocity, size, mut external_force) in q_asteroids.iter_mut() {
        if size.0 > max_size || velocity.linvel.length() > max_speed {
            continue;
        }

        let asteroid_pos = transform.translation.truncate();
        let to_target = asteroid_pos - player_pos;
        let dist_sq = to_target.length_squared();
        if dist_sq < min_dist_sq || dist_sq > range_sq {
            continue;
        }

        let dist = dist_sq.sqrt();
        let target_dir = to_target / dist;
        if beam_dir.dot(target_dir) < config.tractor_beam_aim_cone_dot {
            continue;
        }

        let toward_player = (player_pos - asteroid_pos).normalize_or_zero();
        if toward_player == Vec2::ZERO {
            continue;
        }

        let dist_alpha = ((dist - min_dist) / (range - min_dist)).clamp(0.0, 1.0);
        let falloff = 1.0 - dist_alpha;
        if falloff <= 0.0 {
            continue;
        }

        let dir = if push_mode {
            -toward_player
        } else {
            toward_player
        };
        external_force.force += dir * (force_base * falloff);
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(deprecated)] // iter_entities deprecated in 0.17; accepted until query::<EntityRef> is stable in tests
mod tests {
    use super::*;
    use crate::constants::{REVERSE_FORCE, ROTATION_SPEED, THRUST_FORCE};

    // ── helpers ───────────────────────────────────────────────────────────────

    /// Build a minimal Bevy `App` with just the resources and systems needed to
    /// test the PlayerIntent → physics pipeline, without Rapier or rendering.
    fn build_test_app() -> App {
        let mut app = App::new();
        // Minimal plugin set: time + transforms (no window, no renderer, no physics).
        app.add_plugins(MinimalPlugins);
        // Resources required by apply_player_intent_system.
        app.insert_resource(PlayerIntent::default());
        app.insert_resource(PhysicsConfig::default());
        app
    }

    /// Spawn a player entity carrying the components queried by `apply_player_intent_system`.
    fn spawn_test_player(app: &mut App) {
        app.world_mut().spawn((
            Player,
            Transform::from_rotation(Quat::IDENTITY), // facing +Y
            ExternalForce::default(),
            Velocity::zero(),
        ));
    }

    /// Run only the apply step with the given intent.
    fn run_apply(app: &mut App, intent: PlayerIntent) {
        app.insert_resource(intent);
        app.add_systems(Update, apply_player_intent_system);
        app.update();
    }

    // ── apply_player_intent_system ────────────────────────────────────────────

    #[test]
    fn thrust_forward_sets_nonzero_force() {
        let mut app = build_test_app();
        spawn_test_player(&mut app);

        run_apply(
            &mut app,
            PlayerIntent {
                thrust_forward: 1.0,
                ..Default::default()
            },
        );

        let force = app
            .world()
            .iter_entities()
            .find(|e| e.contains::<Player>())
            .unwrap()
            .get::<ExternalForce>()
            .unwrap()
            .force;

        assert!(
            force.length() > 0.0,
            "expected non-zero force when thrust_forward=1.0, got {force:?}"
        );
    }

    #[test]
    fn thrust_forward_magnitude_matches_constant() {
        let mut app = build_test_app();
        spawn_test_player(&mut app);

        run_apply(
            &mut app,
            PlayerIntent {
                thrust_forward: 1.0,
                ..Default::default()
            },
        );

        let force = app
            .world()
            .iter_entities()
            .find(|e| e.contains::<Player>())
            .unwrap()
            .get::<ExternalForce>()
            .unwrap()
            .force;

        // Ship faces +Y (identity rotation), so force should be (0, THRUST_FORCE).
        assert!(
            (force.length() - THRUST_FORCE).abs() < 1e-4,
            "expected force magnitude {THRUST_FORCE}, got {}",
            force.length()
        );
    }

    #[test]
    fn thrust_forward_is_along_local_y() {
        let mut app = build_test_app();
        // Rotate ship 90° CW (−FRAC_PI_2) so local +Y points toward world +X.
        // In Bevy (right-hand Z): rotation_z(−π/2) maps (0,1,0) → (+1,0,0).
        app.world_mut().spawn((
            Player,
            Transform::from_rotation(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)),
            ExternalForce::default(),
            Velocity::zero(),
        ));

        run_apply(
            &mut app,
            PlayerIntent {
                thrust_forward: 1.0,
                ..Default::default()
            },
        );

        let force = app
            .world()
            .iter_entities()
            .find(|e| e.contains::<Player>())
            .unwrap()
            .get::<ExternalForce>()
            .unwrap()
            .force;

        // Local +Y in world space is world +X after 90° CCW rotation.
        assert!(
            force.x > 0.0 && force.y.abs() < 1e-4,
            "expected force along world +X after 90° ship rotation, got {force:?}"
        );
    }

    #[test]
    fn no_thrust_leaves_force_zero() {
        let mut app = build_test_app();
        spawn_test_player(&mut app);

        run_apply(&mut app, PlayerIntent::default());

        let force = app
            .world()
            .iter_entities()
            .find(|e| e.contains::<Player>())
            .unwrap()
            .get::<ExternalForce>()
            .unwrap()
            .force;

        assert_eq!(
            force,
            Vec2::ZERO,
            "expected zero force with no intent, got {force:?}"
        );
    }

    #[test]
    fn reverse_thrust_applies_negative_force() {
        let mut app = build_test_app();
        spawn_test_player(&mut app);

        run_apply(
            &mut app,
            PlayerIntent {
                thrust_reverse: 1.0,
                ..Default::default()
            },
        );

        let force = app
            .world()
            .iter_entities()
            .find(|e| e.contains::<Player>())
            .unwrap()
            .get::<ExternalForce>()
            .unwrap()
            .force;

        // Ship faces +Y; reverse force should be negative Y.
        assert!(
            force.y < 0.0,
            "expected negative Y force from reverse thrust, got {force:?}"
        );
        assert!(
            (force.length() - REVERSE_FORCE).abs() < 1e-4,
            "expected reverse force magnitude {REVERSE_FORCE}, got {}",
            force.length()
        );
    }

    #[test]
    fn angvel_override_sets_velocity() {
        let mut app = build_test_app();
        spawn_test_player(&mut app);

        run_apply(
            &mut app,
            PlayerIntent {
                angvel: Some(ROTATION_SPEED),
                ..Default::default()
            },
        );

        let angvel = app
            .world()
            .iter_entities()
            .find(|e| e.contains::<Player>())
            .unwrap()
            .get::<Velocity>()
            .unwrap()
            .angvel;

        assert!(
            (angvel - ROTATION_SPEED).abs() < 1e-4,
            "expected angvel {ROTATION_SPEED}, got {angvel}"
        );
    }

    #[test]
    fn no_angvel_intent_leaves_velocity_unchanged() {
        let mut app = build_test_app();
        // Start with non-zero angular velocity.
        app.world_mut().spawn((
            Player,
            Transform::from_rotation(Quat::IDENTITY),
            ExternalForce::default(),
            Velocity {
                linvel: Vec2::ZERO,
                angvel: 2.5,
            },
        ));

        run_apply(&mut app, PlayerIntent::default());

        let angvel = app
            .world()
            .iter_entities()
            .find(|e| e.contains::<Player>())
            .unwrap()
            .get::<Velocity>()
            .unwrap()
            .angvel;

        assert!(
            (angvel - 2.5).abs() < 1e-4,
            "expected angvel unchanged (2.5), got {angvel}"
        );
    }

    #[test]
    fn partial_thrust_scales_force() {
        let mut app = build_test_app();
        spawn_test_player(&mut app);

        run_apply(
            &mut app,
            PlayerIntent {
                thrust_forward: 0.5,
                ..Default::default()
            },
        );

        let force = app
            .world()
            .iter_entities()
            .find(|e| e.contains::<Player>())
            .unwrap()
            .get::<ExternalForce>()
            .unwrap()
            .force;

        let expected = THRUST_FORCE * 0.5;
        assert!(
            (force.length() - expected).abs() < 1e-4,
            "expected force magnitude {expected}, got {}",
            force.length()
        );
    }
}
