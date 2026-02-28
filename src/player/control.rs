//! Player input and movement systems.
//!
//! ## Pipeline (runs in order every `Update` frame)
//!
//! 1. [`player_intent_clear_system`] — resets `PlayerIntent` and `ExternalForce` to zero.
//! 2. [`keyboard_to_intent_system`] — translates KB/mouse thrust+strafe+facing into `PlayerIntent`.
//! 3. [`gamepad_to_intent_system`] — translates gamepad sticks/triggers into `PlayerIntent`.
//! 4. [`apply_player_intent_system`] — converts `PlayerIntent` into `ExternalForce` / `Velocity`.
//!
//! The **input abstraction layer** (`PlayerIntent`) makes the movement logic fully
//! testable: tests populate the resource directly and run only `apply_player_intent_system`.
//!
//! Also contains helper systems that are not part of the core thrust pipeline:
//! - [`gamepad_connection_system`] — tracks which gamepad is preferred
//! - [`aim_snap_system`] — snaps aim to ship forward after idle period

use super::state::{
    AimDirection, AimIdleTimer, Player, PlayerIntent, PreferredGamepad, TractorBeamLevel,
    TractorCaptureState, TractorHoldState, TractorThrowCooldown,
};
use crate::asteroid::{Asteroid, AsteroidSize, Planet};
use crate::config::PhysicsConfig;
use crate::particles::{
    spawn_ship_thrust_particles, spawn_tractor_beam_particles, TractorBeamVfxMode,
};
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

/// Translate WASD + A/D strafe + cursor-facing into [`PlayerIntent`].
///
/// - **W** → `thrust_forward = 1.0`
/// - **S** → `thrust_reverse = 1.0`
/// - **A** → `strafe_local = -1.0`
/// - **D** → `strafe_local = +1.0`
/// - `desired_facing` follows current `AimDirection`
///
/// Additive: safe to run alongside gamepad intent system because each field is
/// overwritten, not accumulated (both sources can't be active simultaneously in
/// normal play).
pub fn keyboard_to_intent_system(
    keys: Res<ButtonInput<KeyCode>>,
    aim: Res<AimDirection>,
    mut intent: ResMut<PlayerIntent>,
) {
    if keys.pressed(KeyCode::KeyW) {
        intent.thrust_forward = 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        intent.thrust_reverse = 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        intent.strafe_local = -1.0;
    } else if keys.pressed(KeyCode::KeyD) {
        intent.strafe_local = 1.0;
    }
    if aim.0.length_squared() > 1e-6 {
        intent.desired_facing = Some(aim.0.normalize_or_zero());
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

/// Translate gamepad sticks/triggers into [`PlayerIntent`].
///
/// - Right stick sets desired facing direction.
/// - Left stick sets world-space strafe direction (omnidirectional, lower authority).
/// - RT/LT provide analog forward/reverse thrust where available.
///
/// Does nothing when no gamepad is connected ([`PreferredGamepad`] is `None`).
pub fn gamepad_to_intent_system(
    preferred: Res<PreferredGamepad>,
    gamepads: Query<&Gamepad>,
    mut intent: ResMut<PlayerIntent>,
    mut idle: ResMut<AimIdleTimer>,
    config: Res<PhysicsConfig>,
) {
    let Some(gamepad_entity) = preferred.0 else {
        return;
    };

    let Ok(gamepad) = gamepads.get(gamepad_entity) else {
        return;
    };

    let rt_analog = gamepad.get(GamepadButton::RightTrigger2).unwrap_or(0.0);
    let lt_analog = gamepad.get(GamepadButton::LeftTrigger2).unwrap_or(0.0);

    intent.thrust_forward = intent.thrust_forward.max(rt_analog.max(
        if gamepad.pressed(GamepadButton::RightTrigger2) {
            1.0
        } else {
            0.0
        },
    ));
    intent.thrust_reverse = intent.thrust_reverse.max(lt_analog.max(
        if gamepad.pressed(GamepadButton::LeftTrigger2) {
            1.0
        } else {
            0.0
        },
    ));

    let lx = gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
    let ly = gamepad.get(GamepadAxis::LeftStickY).unwrap_or(0.0);
    let left_stick = Vec2::new(lx, ly);

    if left_stick.length() >= config.gamepad_left_deadzone {
        // Left stick active — count as active aim-control time so snap doesn't trigger.
        idle.secs = 0.0;
        intent.strafe_world = left_stick.clamp_length_max(1.0);
    }

    let rx = gamepad.get(GamepadAxis::RightStickX).unwrap_or(0.0);
    let ry = gamepad.get(GamepadAxis::RightStickY).unwrap_or(0.0);
    let right_stick = Vec2::new(rx, ry);
    if right_stick.length() >= config.gamepad_right_deadzone {
        idle.secs = 0.0;
        intent.desired_facing = Some(right_stick.normalize_or_zero());
    }
}

/// Toggle tractor hold mode from keyboard/gamepad bindings.
///
/// - Keyboard: `Q` toggles hold mode.
/// - Gamepad: `West` (X / Square) toggles hold mode.
pub fn tractor_hold_toggle_system(
    keys: Res<ButtonInput<KeyCode>>,
    preferred: Res<PreferredGamepad>,
    gamepads: Query<&Gamepad>,
    mut state: ResMut<TractorHoldState>,
    cooldown: Res<TractorThrowCooldown>,
) {
    let kb_toggle = keys.just_pressed(KeyCode::KeyQ);
    let gp_toggle = preferred
        .0
        .and_then(|entity| gamepads.get(entity).ok())
        .is_some_and(|gp| gp.just_pressed(GamepadButton::West));

    if kb_toggle || gp_toggle {
        if state.engaged {
            state.engaged = false;
        } else if cooldown.timer_secs <= 0.0 {
            state.engaged = true;
        }
    }
}

pub fn tractor_throw_cooldown_tick_system(
    time: Res<Time>,
    mut cooldown: ResMut<TractorThrowCooldown>,
) {
    cooldown.timer_secs = (cooldown.timer_secs - time.delta_secs()).max(0.0);
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
/// | `strafe_local`      | `force += local_right * STRAFE_FORCE * strafe_local` |
/// | `strafe_world`      | `force += strafe_world * STRAFE_FORCE` |
/// | `desired_facing`    | `angvel` steers toward facing direction |
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
    let right = transform.rotation.mul_vec3(Vec3::X).truncate();

    if intent.thrust_forward > 0.0 {
        force.force += forward * config.thrust_force * intent.thrust_forward;
    }
    if intent.thrust_reverse > 0.0 {
        force.force -= forward * config.reverse_force * intent.thrust_reverse;
    }
    if intent.strafe_local.abs() > 0.0 {
        force.force += right * config.strafe_force * intent.strafe_local;
    }
    if intent.strafe_world.length_squared() > 0.0 {
        force.force += intent.strafe_world.clamp_length_max(1.0) * config.strafe_force;
    }

    if let Some(desired) = intent
        .desired_facing
        .filter(|dir| dir.length_squared() > 1e-6)
    {
        // Match the exact +Y-facing convention used by the aim indicator and
        // projectile orientation: atan2(y, x) - PI/2.
        let target_angle = desired.y.atan2(desired.x) - std::f32::consts::FRAC_PI_2;
        let current_angle = transform.rotation.to_euler(EulerRot::ZYX).0;
        let mut angle_diff = target_angle - current_angle;
        while angle_diff > std::f32::consts::PI {
            angle_diff -= std::f32::consts::TAU;
        }
        while angle_diff < -std::f32::consts::PI {
            angle_diff += std::f32::consts::TAU;
        }
        velocity.angvel = if angle_diff.abs() > config.gamepad_heading_snap_threshold {
            config.rotation_speed * angle_diff.signum()
        } else {
            0.0
        };
    } else if let Some(av) = intent.angvel {
        velocity.angvel = av;
    }
    if intent.brake {
        velocity.linvel *= config.gamepad_brake_damping;
        velocity.angvel *= config.gamepad_brake_damping;
    }
}

pub fn player_thrust_particles_system(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<PhysicsConfig>,
    intent: Res<PlayerIntent>,
    q: Query<(&Transform, &Velocity), With<Player>>,
    mut emit_timer: Local<f32>,
) {
    const THRUST_PARTICLE_INTERVAL_SECS: f32 = 0.028;

    let Ok((transform, velocity)) = q.single() else {
        return;
    };

    let forward = transform.rotation.mul_vec3(Vec3::Y).truncate();
    let right = transform.rotation.mul_vec3(Vec3::X).truncate();

    let mut exhaust_dir = Vec2::ZERO;
    if intent.thrust_forward > 0.0 {
        exhaust_dir -= forward * intent.thrust_forward;
    }
    if intent.thrust_reverse > 0.0 {
        exhaust_dir += forward * intent.thrust_reverse;
    }
    if intent.strafe_local.abs() > 0.0 {
        exhaust_dir -= right * intent.strafe_local;
    }
    if intent.strafe_world.length_squared() > 0.0 {
        exhaust_dir -= intent.strafe_world.clamp_length_max(1.0);
    }

    let thrust_intensity = exhaust_dir.length().clamp(0.0, 1.0);
    if thrust_intensity <= 1e-4 {
        *emit_timer = 0.0;
        return;
    }

    let emit_dir = exhaust_dir.normalize_or_zero();
    let spawn_pos = transform.translation.truncate()
        + emit_dir * (config.player_collider_radius + 1.5)
        + velocity.linvel.normalize_or_zero() * 0.75;

    *emit_timer += time.delta_secs();
    while *emit_timer >= THRUST_PARTICLE_INTERVAL_SECS {
        *emit_timer -= THRUST_PARTICLE_INTERVAL_SECS;
        spawn_ship_thrust_particles(
            &mut commands,
            spawn_pos,
            emit_dir,
            velocity.linvel,
            thrust_intensity,
        );
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

/// Apply tractor beam force to nearby asteroids while hold mode is engaged.
///
/// - Toggle hold mode with `Q` (keyboard) / `West` (`X`/`Square`) on gamepad.
/// - While engaged: hold `E` / `LB` to pull and hold near the ship.
/// - While engaged: press `R` / `RB` to throw outward and disengage.
///
/// The beam only affects asteroids inside a narrow cone around active
/// `AimDirection` (falls back to ship forward if aim is unavailable), and below
/// level-scaled mass/speed thresholds to keep interactions stable.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn tractor_beam_force_system(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    preferred: Res<PreferredGamepad>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    aim: Res<AimDirection>,
    mut hold_state: ResMut<TractorHoldState>,
    mut capture_state: ResMut<TractorCaptureState>,
    mut throw_cooldown: ResMut<TractorThrowCooldown>,
    mut particle_emit_cooldown: Local<f32>,
    mut was_engaged: Local<bool>,
    q_player: Query<(&Transform, &Velocity), (With<Player>, Without<Asteroid>)>,
    mut q_asteroids: ParamSet<(
        Query<(Entity, &Transform, &Velocity, &AsteroidSize), (With<Asteroid>, Without<Planet>)>,
        Query<
            (&Transform, &mut Velocity, &AsteroidSize, &mut ExternalForce),
            (With<Asteroid>, Without<Planet>),
        >,
    )>,
    beam_level: Res<TractorBeamLevel>,
    config: Res<PhysicsConfig>,
) {
    const TRACTOR_VFX_EMIT_INTERVAL_SECS: f32 = 0.05;
    const TRACTOR_PULL_RATE_UNITS_PER_SEC: f32 = 95.0;
    const TRACTOR_RELEASE_RANGE_MULTIPLIER: f32 = 1.8;
    const TRACTOR_FORCE_POSITION_FACTOR: f32 = 0.12;
    const TRACTOR_FORCE_VELOCITY_FACTOR: f32 = 0.30;

    let gamepad = preferred.0.and_then(|entity| gamepads.get(entity).ok());
    let pull_mode = keys.pressed(KeyCode::KeyE)
        || gamepad.is_some_and(|gp| gp.pressed(GamepadButton::LeftTrigger));
    let throw_mode = keys.just_pressed(KeyCode::KeyR)
        || gamepad.is_some_and(|gp| gp.just_pressed(GamepadButton::RightTrigger));

    let player_linvel = q_player
        .single()
        .map(|(_, velocity)| velocity.linvel)
        .unwrap_or(Vec2::ZERO);

    if !hold_state.engaged {
        if *was_engaged {
            if let Some(target_entity) = capture_state.target {
                if let Ok((_, mut velocity, _, mut external_force)) =
                    q_asteroids.p1().get_mut(target_entity)
                {
                    velocity.linvel = player_linvel;
                    external_force.force = Vec2::ZERO;
                }
            }
        }
        capture_state.target = None;
        capture_state.hold_distance = 0.0;
        *was_engaged = false;
        return;
    }

    *was_engaged = true;

    let Ok((player_transform, player_velocity)) = q_player.single() else {
        return;
    };

    let player_pos = player_transform.translation.truncate();
    let ship_forward = player_transform.rotation.mul_vec3(Vec3::Y).truncate();
    let beam_dir = if aim.0.length_squared() > 1e-6 {
        aim.0.normalize_or_zero()
    } else {
        ship_forward.normalize_or_zero()
    };
    if beam_dir.length_squared() <= 1e-6 {
        return;
    }

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
    let safe_hold_min = (config.player_collider_radius + 14.0).max(min_dist + 4.0);
    let hold_max = (range * 0.9).max(safe_hold_min + 1.0);

    let emit_particles = *particle_emit_cooldown <= 0.0;
    if emit_particles {
        *particle_emit_cooldown = TRACTOR_VFX_EMIT_INTERVAL_SECS;
    } else {
        *particle_emit_cooldown = (*particle_emit_cooldown - time.delta_secs()).max(0.0);
    }

    if let Some(entity) = capture_state.target {
        let keep_target = q_asteroids
            .p0()
            .get(entity)
            .is_ok_and(|(_, transform, _, size)| {
                let asteroid_pos = transform.translation.truncate();
                let dist = asteroid_pos.distance(player_pos);
                size.0 <= max_size && dist <= range * TRACTOR_RELEASE_RANGE_MULTIPLIER
            });
        if !keep_target {
            capture_state.target = None;
        }
    }

    if capture_state.target.is_none() {
        let mut best: Option<(Entity, f32)> = None;
        for (entity, transform, velocity, size) in q_asteroids.p0().iter() {
            if size.0 > max_size || velocity.linvel.length() > max_speed {
                continue;
            }

            let asteroid_pos = transform.translation.truncate();
            let to_target = asteroid_pos - player_pos;
            let dist_sq = to_target.length_squared();
            if dist_sq < min_dist_sq || dist_sq > range_sq {
                continue;
            }

            let target_dir = to_target.normalize_or_zero();
            if beam_dir.dot(target_dir) < config.tractor_beam_aim_cone_dot {
                continue;
            }

            if best.is_none_or(|(_, best_dist_sq)| dist_sq < best_dist_sq) {
                best = Some((entity, dist_sq));
            }
        }

        if let Some((entity, dist_sq)) = best {
            capture_state.target = Some(entity);
            capture_state.hold_distance = dist_sq.sqrt().clamp(safe_hold_min, hold_max);
        }
    }

    let Some(target_entity) = capture_state.target else {
        return;
    };

    let mut asteroid_mut = q_asteroids.p1();
    let Ok((transform, mut velocity, size, mut external_force)) =
        asteroid_mut.get_mut(target_entity)
    else {
        capture_state.target = None;
        return;
    };
    if size.0 > max_size {
        capture_state.target = None;
        return;
    }

    let asteroid_pos = transform.translation.truncate();
    let to_target = asteroid_pos - player_pos;
    let dist = to_target.length();
    if dist <= 1e-4 {
        capture_state.target = None;
        return;
    }

    if capture_state.hold_distance <= 0.0 {
        capture_state.hold_distance = dist.clamp(safe_hold_min, hold_max);
    }

    if throw_mode {
        let throw_dir = if beam_dir.length_squared() > 1e-6 {
            beam_dir
        } else {
            to_target.normalize_or_zero()
        };
        let throw_force = throw_dir * (force_base * 0.45);
        external_force.force += throw_force;
        velocity.linvel = player_velocity.linvel + throw_dir * (force_base * 0.010);

        if emit_particles {
            spawn_tractor_beam_particles(
                &mut commands,
                asteroid_pos,
                throw_dir,
                velocity.linvel,
                TractorBeamVfxMode::Push,
                1.0,
            );
        }

        capture_state.target = None;
        capture_state.hold_distance = 0.0;
        hold_state.engaged = false;
        throw_cooldown.timer_secs = beam_level.throw_cooldown_secs(&config);
        *was_engaged = false;
        return;
    }

    if pull_mode {
        capture_state.hold_distance = (capture_state.hold_distance
            - TRACTOR_PULL_RATE_UNITS_PER_SEC * time.delta_secs())
        .max(safe_hold_min);
    }
    capture_state.hold_distance = capture_state.hold_distance.clamp(safe_hold_min, hold_max);

    let desired_pos = player_pos + beam_dir * capture_state.hold_distance;
    let position_error = desired_pos - asteroid_pos;
    let relative_velocity = player_velocity.linvel - velocity.linvel;
    let position_force = position_error * (force_base * TRACTOR_FORCE_POSITION_FACTOR);
    let velocity_force = relative_velocity
        * (config.tractor_beam_freeze_velocity_damping * TRACTOR_FORCE_VELOCITY_FACTOR);

    let mut applied_force = position_force + velocity_force;
    if dist < safe_hold_min {
        let outward = to_target.normalize_or_zero();
        applied_force += outward * (force_base * 0.6);
    }
    external_force.force += applied_force;

    if emit_particles {
        let mode = if pull_mode {
            TractorBeamVfxMode::Pull
        } else {
            TractorBeamVfxMode::Freeze
        };
        let dir = if pull_mode {
            (player_pos - asteroid_pos).normalize_or_zero()
        } else {
            applied_force.normalize_or_zero()
        };
        let intensity = if pull_mode { 0.95 } else { 0.7 };

        spawn_tractor_beam_particles(
            &mut commands,
            asteroid_pos,
            dir,
            velocity.linvel,
            mode,
            intensity,
        );
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

    // ── tractor_beam_force_system ───────────────────────────────────────────

    fn build_tractor_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(PhysicsConfig::default());
        app.insert_resource(TractorBeamLevel::default());
        app.insert_resource(TractorHoldState::default());
        app.insert_resource(TractorCaptureState::default());
        app.insert_resource(TractorThrowCooldown::default());
        app.insert_resource(PreferredGamepad::default());
        app.insert_resource(AimDirection::default());
        app.add_systems(Update, tractor_beam_force_system);
        app
    }

    fn spawn_tractor_player(app: &mut App, transform: Transform, velocity: Velocity) {
        app.world_mut()
            .spawn((Player, transform, velocity, ExternalForce::default()));
    }

    fn spawn_tractor_asteroid(app: &mut App, pos: Vec2, velocity: Vec2) -> Entity {
        app.world_mut()
            .spawn((
                Asteroid,
                AsteroidSize(1),
                Transform::from_translation(pos.extend(0.0)),
                Velocity {
                    linvel: velocity,
                    angvel: 0.0,
                },
                ExternalForce::default(),
            ))
            .id()
    }

    fn run_tractor_once(app: &mut App) {
        app.update();
    }

    #[test]
    fn tractor_engage_captures_nearest_target() {
        let mut app = build_tractor_test_app();
        spawn_tractor_player(
            &mut app,
            Transform::from_rotation(Quat::IDENTITY),
            Velocity::zero(),
        );
        let near = spawn_tractor_asteroid(&mut app, Vec2::new(0.0, 120.0), Vec2::ZERO);
        let _far = spawn_tractor_asteroid(&mut app, Vec2::new(0.0, 220.0), Vec2::ZERO);

        app.world_mut().resource_mut::<TractorHoldState>().engaged = true;

        run_tractor_once(&mut app);

        let captured = app.world().resource::<TractorCaptureState>().target;
        assert_eq!(captured, Some(near), "expected nearest asteroid capture");
    }

    #[test]
    fn tractor_hold_emits_particles_without_pull_input() {
        let mut app = build_tractor_test_app();
        spawn_tractor_player(
            &mut app,
            Transform::from_rotation(Quat::IDENTITY),
            Velocity::zero(),
        );
        let _asteroid = spawn_tractor_asteroid(&mut app, Vec2::new(0.0, 120.0), Vec2::ZERO);

        app.world_mut().resource_mut::<TractorHoldState>().engaged = true;

        run_tractor_once(&mut app);

        let particle_count = {
            let world = app.world_mut();
            world
                .query::<&crate::particles::Particle>()
                .iter(world)
                .count()
        };
        assert!(
            particle_count > 0,
            "expected hold mode to emit particles, got {particle_count}"
        );
    }

    #[test]
    fn tractor_pull_force_moves_target_toward_ship() {
        let mut app = build_tractor_test_app();
        spawn_tractor_player(
            &mut app,
            Transform::from_rotation(Quat::IDENTITY),
            Velocity::zero(),
        );
        let asteroid = spawn_tractor_asteroid(&mut app, Vec2::new(0.0, 120.0), Vec2::ZERO);

        app.world_mut().resource_mut::<TractorHoldState>().engaged = true;
        app.world_mut().resource_mut::<TractorCaptureState>().target = Some(asteroid);
        app.world_mut()
            .resource_mut::<TractorCaptureState>()
            .hold_distance = 50.0;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyE);

        run_tractor_once(&mut app);

        let force = app.world().get::<ExternalForce>(asteroid).unwrap().force;
        assert!(
            force.y < 0.0,
            "expected pull force toward player/hold point, got {force:?}"
        );
    }

    #[test]
    fn tractor_throw_releases_capture_and_pushes_target() {
        let mut app = build_tractor_test_app();
        spawn_tractor_player(
            &mut app,
            Transform::from_rotation(Quat::IDENTITY),
            Velocity::zero(),
        );
        let asteroid = spawn_tractor_asteroid(&mut app, Vec2::new(0.0, 120.0), Vec2::ZERO);

        app.world_mut().resource_mut::<TractorHoldState>().engaged = true;
        app.world_mut().resource_mut::<TractorCaptureState>().target = Some(asteroid);
        app.world_mut()
            .resource_mut::<TractorCaptureState>()
            .hold_distance = 100.0;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyR);

        run_tractor_once(&mut app);

        let captured = app.world().resource::<TractorCaptureState>().target;
        let vel = app.world().get::<Velocity>(asteroid).unwrap().linvel;
        let engaged = app.world().resource::<TractorHoldState>().engaged;
        let cooldown = app.world().resource::<TractorThrowCooldown>().timer_secs;
        assert_eq!(captured, None, "expected throw to release captured target");
        assert!(!engaged, "expected throw to disengage tractor mode");
        assert!(cooldown > 0.0, "expected throw to start cooldown");
        assert!(
            vel.y > 0.0,
            "expected throw to push target outward, got {vel:?}"
        );
    }

    #[test]
    fn tractor_disengage_releases_target_at_player_velocity() {
        let mut app = build_tractor_test_app();
        let player_vel = Vec2::new(27.0, -11.0);
        spawn_tractor_player(
            &mut app,
            Transform::from_rotation(Quat::IDENTITY),
            Velocity {
                linvel: player_vel,
                angvel: 0.0,
            },
        );
        let asteroid = spawn_tractor_asteroid(&mut app, Vec2::new(0.0, 120.0), Vec2::ZERO);

        app.world_mut().resource_mut::<TractorHoldState>().engaged = true;
        app.world_mut().resource_mut::<TractorCaptureState>().target = Some(asteroid);
        app.world_mut()
            .resource_mut::<TractorCaptureState>()
            .hold_distance = 80.0;

        run_tractor_once(&mut app);

        app.world_mut().resource_mut::<TractorHoldState>().engaged = false;
        app.world_mut()
            .get_mut::<Velocity>(asteroid)
            .unwrap()
            .linvel = Vec2::new(-90.0, 45.0);

        run_tractor_once(&mut app);

        let capture = *app.world().resource::<TractorCaptureState>();
        let released_vel = app.world().get::<Velocity>(asteroid).unwrap().linvel;
        assert_eq!(capture.target, None, "expected capture clear on disengage");
        assert!(
            capture.hold_distance <= 0.0,
            "expected hold distance reset on disengage"
        );
        assert_eq!(
            released_vel, player_vel,
            "expected disengage release velocity to match player velocity"
        );
    }

    #[test]
    fn tractor_capture_uses_aim_direction() {
        let mut app = build_tractor_test_app();
        spawn_tractor_player(
            &mut app,
            Transform::from_rotation(Quat::IDENTITY),
            Velocity::zero(),
        );
        let aimed = spawn_tractor_asteroid(&mut app, Vec2::new(120.0, 0.0), Vec2::ZERO);
        let _other = spawn_tractor_asteroid(&mut app, Vec2::new(0.0, 120.0), Vec2::ZERO);

        app.world_mut().resource_mut::<TractorHoldState>().engaged = true;
        app.world_mut().insert_resource(AimDirection(Vec2::X));

        run_tractor_once(&mut app);

        let captured = app.world().resource::<TractorCaptureState>().target;
        assert_eq!(
            captured,
            Some(aimed),
            "expected aim-direction capture selection"
        );
    }
}
