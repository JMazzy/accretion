//! Player input and movement systems.
//!
//! Handles all input sources that move or orient the ship:
//! - WASD keyboard thrust and rotation ([`player_control_system`])
//! - Gamepad left-stick twin-stick movement ([`gamepad_movement_system`])
//! - Gamepad connect / disconnect tracking ([`gamepad_connection_system`])
//!
//! Also contains [`player_oob_damping_system`] which enforces the soft
//! out-of-bounds boundary.

use super::state::{AimDirection, AimIdleTimer, Player, PreferredGamepad};
use crate::constants::{
    AIM_IDLE_SNAP_SECS, GAMEPAD_BRAKE_DAMPING, GAMEPAD_HEADING_SNAP_THRESHOLD,
    GAMEPAD_LEFT_DEADZONE, OOB_DAMPING, OOB_RADIUS, OOB_RAMP_WIDTH, REVERSE_FORCE, ROTATION_SPEED,
    THRUST_FORCE,
};
use bevy::input::gamepad::{
    GamepadAxis, GamepadAxisType, GamepadButton, GamepadButtonType, GamepadConnection,
    GamepadConnectionEvent,
};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

// ── Force reset ───────────────────────────────────────────────────────────────

/// Clear `ExternalForce` to zero at the start of every frame.
///
/// Must run before any input system accumulates forces.  If multiple input
/// sources (keyboard + gamepad) are simultaneously active their contributions
/// are safely added because this reset happens first.
pub fn player_force_reset_system(mut q: Query<&mut ExternalForce, With<Player>>) {
    if let Ok(mut force) = q.get_single_mut() {
        force.force = Vec2::ZERO;
        force.torque = 0.0;
    }
}

// ── Keyboard control ──────────────────────────────────────────────────────────

/// Apply WASD keyboard thrust and A/D rotation to the player ship.
///
/// - **W** — forward thrust along the ship's local +Y axis
/// - **S** — reverse thrust (weaker than forward)
/// - **A / D** — direct angular velocity override for snappy rotation;
///   releasing both keys lets Rapier angular damping slow the spin naturally
pub fn player_control_system(
    mut q: Query<(&Transform, &mut ExternalForce, &mut Velocity), With<Player>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let Ok((transform, mut force, mut velocity)) = q.get_single_mut() else {
        return;
    };

    let forward = transform.rotation.mul_vec3(Vec3::Y).truncate();

    if keys.pressed(KeyCode::KeyW) {
        force.force += forward * THRUST_FORCE;
    }
    if keys.pressed(KeyCode::KeyS) {
        force.force -= forward * REVERSE_FORCE;
    }

    if keys.pressed(KeyCode::KeyA) {
        velocity.angvel = ROTATION_SPEED;
    } else if keys.pressed(KeyCode::KeyD) {
        velocity.angvel = -ROTATION_SPEED;
    }
    // If neither A nor D, angular damping handles slow-down
}

// ── Gamepad connection ─────────────────────────────────────────────────────────

/// Track gamepad connect / disconnect events and update [`PreferredGamepad`].
///
/// The most-recently-connected gamepad is always preferred, ensuring that
/// non-gamepad HID devices (e.g. RGB LED controllers on Linux) that connect
/// first are superseded by the real gamepad.
pub fn gamepad_connection_system(
    mut events: EventReader<GamepadConnectionEvent>,
    mut preferred: ResMut<PreferredGamepad>,
) {
    for event in events.read() {
        match &event.connection {
            GamepadConnection::Connected(_info) => {
                preferred.0 = Some(event.gamepad);
                info!(
                    "[gamepad] Gamepad {} connected (now preferred)",
                    event.gamepad.id
                );
            }
            GamepadConnection::Disconnected => {
                info!("[gamepad] Gamepad {} disconnected", event.gamepad.id);
                if preferred.0 == Some(event.gamepad) {
                    preferred.0 = None;
                }
            }
        }
    }
}

// ── Gamepad movement ───────────────────────────────────────────────────────────

/// Twin-stick gamepad movement using the left stick and B button.
///
/// **Left stick behaviour**:
/// 1. The ship rotates at `ROTATION_SPEED` rad/s toward the stick direction.
/// 2. Once aligned within `GAMEPAD_HEADING_SNAP_THRESHOLD`, rotation stops.
/// 3. Forward thrust is applied proportional to stick magnitude at all times.
///
/// **B button (East)**: active brake — applies `GAMEPAD_BRAKE_DAMPING` to both
/// linear and angular velocity every frame while held.
pub fn gamepad_movement_system(
    mut q: Query<(&Transform, &mut ExternalForce, &mut Velocity), With<Player>>,
    preferred: Res<PreferredGamepad>,
    axes: Res<Axis<GamepadAxis>>,
    buttons: Res<ButtonInput<GamepadButton>>,
    mut idle: ResMut<AimIdleTimer>,
) {
    let Ok((transform, mut force, mut velocity)) = q.get_single_mut() else {
        return;
    };

    let Some(gamepad) = preferred.0 else {
        return;
    };

    // ── Brake (B / East button) ────────────────────────────────────────────────
    if buttons.pressed(GamepadButton::new(gamepad, GamepadButtonType::East)) {
        velocity.linvel *= GAMEPAD_BRAKE_DAMPING;
        velocity.angvel *= GAMEPAD_BRAKE_DAMPING;
    }

    let lx = axes
        .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickX))
        .unwrap_or(0.0);
    let ly = axes
        .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickY))
        .unwrap_or(0.0);
    let left_stick = Vec2::new(lx, ly);

    if left_stick.length() < GAMEPAD_LEFT_DEADZONE {
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

    velocity.angvel = if angle_diff.abs() > GAMEPAD_HEADING_SNAP_THRESHOLD {
        ROTATION_SPEED * angle_diff.signum()
    } else {
        0.0
    };

    let forward = transform.rotation.mul_vec3(Vec3::Y).truncate();
    force.force += forward * THRUST_FORCE * left_stick.length();
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
) {
    idle.secs += time.delta_seconds();
    if idle.secs >= AIM_IDLE_SNAP_SECS {
        if let Ok(transform) = q_player.get_single() {
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
pub fn player_oob_damping_system(mut q: Query<(&Transform, &mut Velocity), With<Player>>) {
    let Ok((transform, mut velocity)) = q.get_single_mut() else {
        return;
    };

    let dist = transform.translation.truncate().length();
    if dist > OOB_RADIUS {
        let exceed = (dist - OOB_RADIUS).min(OOB_RAMP_WIDTH) / OOB_RAMP_WIDTH;
        let factor = 1.0 - exceed * (1.0 - OOB_DAMPING);
        velocity.linvel *= factor;
        velocity.angvel *= factor;
    }
}
