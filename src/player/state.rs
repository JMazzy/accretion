//! Player components and resources.
//!
//! All ECS components and Bevy resources that describe player state live here.
//! Systems that mutate this state are in the sibling modules:
//! - [`super::control`] — input + movement
//! - [`super::combat`] — projectile firing + damage
//! - [`super::rendering`] — gizmo drawing + camera

use crate::constants::{INVINCIBILITY_DURATION, PLAYER_MAX_HP};
use bevy::input::gamepad::Gamepad;
use bevy::prelude::*;

// ── Components ─────────────────────────────────────────────────────────────────

/// Marker component for the player ship entity.
#[derive(Component)]
pub struct Player;

/// Tracks current HP and the remaining invincibility window after a hit.
///
/// HP depletes when the player collides with an asteroid faster than
/// `DAMAGE_SPEED_THRESHOLD`.  Invincibility frames prevent rapid damage
/// stacking from a single sustained contact.
#[derive(Component)]
pub struct PlayerHealth {
    pub hp: f32,
    pub max_hp: f32,
    /// Seconds of invincibility remaining; decremented each frame.
    pub inv_timer: f32,
}

impl Default for PlayerHealth {
    fn default() -> Self {
        Self {
            hp: PLAYER_MAX_HP,
            max_hp: PLAYER_MAX_HP,
            inv_timer: 0.0,
        }
    }
}

/// Per-projectile state attached to each fired round.
#[derive(Component)]
pub struct Projectile {
    /// Seconds since this projectile was spawned.
    pub age: f32,
}

// ── Resources ──────────────────────────────────────────────────────────────────

/// Enforces a minimum interval between consecutive shots.
#[derive(Resource, Default)]
pub struct PlayerFireCooldown {
    /// Remaining cooldown in seconds; decremented each frame, clamped to 0.
    pub timer: f32,
}

/// World-space unit vector representing the player's current aim direction.
///
/// Updated every frame by `mouse_aim_system` (cursor offset from screen centre)
/// or by `projectile_fire_system` (gamepad right stick).
/// Falls back to the ship's local +Y (forward) direction when no explicit aim
/// source is active.
#[derive(Resource, Clone, Copy)]
pub struct AimDirection(pub Vec2);

impl Default for AimDirection {
    fn default() -> Self {
        Self(Vec2::Y) // ship starts pointing up
    }
}

/// Tracks the most recently connected gamepad so that accidental HID devices
/// (e.g. RGB LED controllers exposed as joysticks on Linux) don't hijack input.
///
/// Updated by `gamepad_connection_system`.  Always prefers the *last* connected
/// gamepad; cleared when that gamepad disconnects.
#[derive(Resource, Default)]
pub struct PreferredGamepad(pub Option<Gamepad>);

// ── Invincibility helper ───────────────────────────────────────────────────────

// Helper methods are public API; suppress dead_code until they're wired into systems.
#[allow(dead_code)]
impl PlayerHealth {
    /// Grant a full invincibility window (used immediately after taking damage).
    #[inline]
    pub fn grant_invincibility(&mut self) {
        self.inv_timer = INVINCIBILITY_DURATION;
    }

    /// Returns `true` while the invincibility window is active.
    #[inline]
    pub fn is_invincible(&self) -> bool {
        self.inv_timer > 0.0
    }
}
