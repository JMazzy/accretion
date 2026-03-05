//! Player components and resources.
//!
//! All ECS components and Bevy resources that describe player state live here.
//! Systems that mutate this state are in the sibling modules:
//! - [`super::control`] — input + movement
//! - [`super::combat`] — projectile firing + damage
//! - [`super::rendering`] — gizmo drawing + camera

use crate::config::PhysicsConfig;
use crate::constants::{
    INVINCIBILITY_DURATION, ION_CANNON_BASE_STUN_SECS, ION_CANNON_MAX_LEVEL,
    ION_CANNON_STUN_SECS_PER_LEVEL, ION_CANNON_UPGRADE_BASE_COST, MISSILE_AMMO_MAX, PLAYER_LIVES,
    PLAYER_MAX_HP, PRIMARY_WEAPON_MAX_LEVEL, SECONDARY_WEAPON_MAX_LEVEL,
    SECONDARY_WEAPON_UPGRADE_BASE_COST, TRACTOR_BEAM_MAX_LEVEL, TRACTOR_BEAM_UPGRADE_BASE_COST,
    WEAPON_UPGRADE_BASE_COST,
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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
    /// Seconds since the last damage event; used to gate passive HP regeneration.
    pub time_since_damage: f32,
}

impl Default for PlayerHealth {
    fn default() -> Self {
        Self {
            hp: PLAYER_MAX_HP,
            max_hp: PLAYER_MAX_HP,
            inv_timer: 0.0,
            time_since_damage: 0.0,
        }
    }
}

/// Per-projectile state attached to each fired round.
#[derive(Component, Default)]
pub struct Projectile {
    /// Seconds since this projectile was spawned.
    pub age: f32,
    /// Distance travelled since spawn (world units).
    pub distance_traveled: f32,
    /// Set to `true` when the projectile has already hit an asteroid so the
    /// lifetime system knows not to count its expiry as a missed shot.
    pub was_hit: bool,
}

/// Per-missile state attached to each fired missile.
///
/// Missiles are fired with `X` / right-click and have different destruction
/// rules from normal projectiles (see `combat::missile_asteroid_hit_system`).
#[derive(Component, Default)]
pub struct Missile {
    /// Seconds since this missile was spawned.
    pub age: f32,
    /// Distance travelled since spawn (world units).
    pub distance_traveled: f32,
    /// Time accumulator used by the trail emission system.
    pub trail_emit_timer: f32,
}

// ── Resources ──────────────────────────────────────────────────────────────────

/// Tracks available missile ammo.
#[derive(Resource, Debug, Clone)]
pub struct MissileAmmo {
    /// Missiles currently available to fire.
    pub count: u32,
}

/// Supported primary-weapon choices for campaign loadout selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CampaignPrimaryWeapon {
    #[default]
    Blaster,
}

impl CampaignPrimaryWeapon {
    #[inline]
    pub fn label(self) -> &'static str {
        match self {
            Self::Blaster => "BLASTER",
        }
    }
}

/// Supported secondary-weapon choices for campaign loadout selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CampaignSecondaryWeapon {
    #[default]
    Missile,
    IonCannon,
}

impl CampaignSecondaryWeapon {
    #[inline]
    pub fn label(self) -> &'static str {
        match self {
            Self::Missile => "MISSILE",
            Self::IonCannon => "ION CANNON",
        }
    }

    #[inline]
    pub fn toggled(self) -> Self {
        match self {
            Self::Missile => Self::IonCannon,
            Self::IonCannon => Self::Missile,
        }
    }
}

/// Active campaign loadout applied at runtime.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignLoadout {
    pub primary: CampaignPrimaryWeapon,
    pub secondary: CampaignSecondaryWeapon,
}

impl Default for CampaignLoadout {
    fn default() -> Self {
        Self {
            primary: CampaignPrimaryWeapon::Blaster,
            secondary: CampaignSecondaryWeapon::Missile,
        }
    }
}

impl Default for MissileAmmo {
    fn default() -> Self {
        Self {
            count: MISSILE_AMMO_MAX,
        }
    }
}

/// Enforces a minimum interval between consecutive missile shots.
#[derive(Resource, Default)]
pub struct MissileCooldown {
    /// Remaining cooldown in seconds; decremented each frame, clamped to 0.
    pub timer: f32,
}

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
pub struct PreferredGamepad(pub Option<Entity>);

/// Tracks how long (seconds) since any active aim input was last received.
///
/// Reset to 0.0 whenever the mouse cursor moves, the gamepad left stick is
/// active, or the right stick is active.  When the timer exceeds
/// `AIM_IDLE_SNAP_SECS` the aim direction is snapped back to the ship's
/// local forward (+Y).
#[derive(Resource, Default)]
pub struct AimIdleTimer {
    /// Seconds since the last active aim input.
    pub secs: f32,
    /// Last known cursor screen position; used to detect mouse movement.
    pub last_cursor: Option<Vec2>,
}

/// Latched tractor hold-mode state.
///
/// Toggled by keyboard/gamepad input and consumed by the tractor-beam force
/// system to decide whether pull/throw actions are currently active.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct TractorHoldState {
    pub engaged: bool,
}

/// Persistent tractor-beam capture state.
///
/// Stores which asteroid is currently captured (if any) and the maintained
/// hold distance from the player while tractor mode is engaged.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct TractorCaptureState {
    pub target: Option<Entity>,
    pub hold_distance: f32,
}

/// Cooldown timer after using tractor throw.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct TractorThrowCooldown {
    pub timer_secs: f32,
}

/// Multiplier tier thresholds (streak → multiplier).
///
/// | Streak | Multiplier |
/// |--------|------------|
/// | 0–4    | ×1         |
/// | 5–9    | ×2         |
/// | 10–19  | ×3         |
/// | 20–39  | ×4         |
/// | 40+    | ×5         |
pub fn streak_to_multiplier(streak: u32) -> u32 {
    match streak {
        0..=4 => 1,
        5..=9 => 2,
        10..=19 => 3,
        20..=39 => 4,
        _ => 5,
    }
}

/// Tracks the player's gameplay score.
///
/// - `hits`: Raw hit count (each projectile–asteroid contact = 1).
/// - `destroyed`: Asteroids fully eliminated (size 0–1, no fragments).
/// - `streak`: Consecutive hits without a miss; resets on miss or death.
/// - `points`: Accumulated score (multiplier-weighted hits and destroys).
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct PlayerScore {
    pub hits: u32,
    pub destroyed: u32,
    pub streak: u32,
    pub points: u32,
}

impl PlayerScore {
    /// Total score (points accumulated with multipliers applied).
    #[inline]
    pub fn total(self) -> u32 {
        self.points
    }

    /// Active scoring multiplier derived from the current streak.
    #[inline]
    pub fn multiplier(self) -> u32 {
        streak_to_multiplier(self.streak)
    }
}

/// Tracks the player's current lives and pending respawn state.
///
/// - `remaining`: lives left, including the current one. Starts at `PLAYER_LIVES`.
///   Decremented on each death; reaching 0 triggers a game-over.
/// - `respawn_timer`: when `Some(t)`, counts down `t` seconds before
///   re-spawning the player ship.  `None` means the player is alive.
#[derive(Resource, Debug, Clone)]
pub struct PlayerLives {
    /// Lives remaining (including the current life).
    pub remaining: i32,
    /// Active respawn countdown (seconds); `None` while the ship is alive.
    pub respawn_timer: Option<f32>,
}

impl Default for PlayerLives {
    fn default() -> Self {
        Self {
            remaining: PLAYER_LIVES,
            respawn_timer: None,
        }
    }
}

impl PlayerLives {
    /// Reset to full lives with no pending respawn (used on game-over restart).
    pub fn reset(&mut self) {
        self.remaining = PLAYER_LIVES;
        self.respawn_timer = None;
    }
}

// ── Primary Weapon Upgrade ─────────────────────────────────────────────────────

/// Tracks the player's primary weapon upgrade level.
///
/// At level 0 (display: Level 1) the weapon fully destroys asteroids of size ≤ 1
/// and chips a single vertex off any larger target.  Each additional upgrade level
/// raises the "full-destroy threshold" by 1.  A level-10 weapon destroys size ≤ 10.
///
/// The "no more than half" design constraint is enforced implicitly: since the max
/// level is 10, full-destroy only applies to asteroids small enough to disappear
/// without leaving a sizable remnant.  Anything above the threshold always takes
/// exactly the chip path (1-unit fragment removed, asteroid shrinks by 1).
#[derive(Resource, Debug, Clone, Default)]
pub struct PrimaryWeaponLevel {
    /// Internal 0-indexed level (0 = Level 1 / base, 9 = Level 10 / max).
    pub level: u32,
}

#[allow(dead_code)]
impl PrimaryWeaponLevel {
    /// Maximum internal level value (inclusive).
    pub const MAX: u32 = PRIMARY_WEAPON_MAX_LEVEL - 1;

    /// Human-readable display level (1-indexed).
    #[inline]
    pub fn display_level(&self) -> u32 {
        self.level + 1
    }

    /// Largest asteroid size that this weapon fully destroys in one hit.
    #[inline]
    pub fn max_destroy_size(&self) -> u32 {
        self.level + 1
    }

    /// Whether the weapon can be upgraded further.
    #[inline]
    pub fn is_maxed(&self) -> bool {
        self.level >= Self::MAX
    }

    /// Ore cost to buy the next upgrade level.
    /// Returns `None` when already at max level.
    #[inline]
    pub fn cost_for_next_level(&self) -> Option<u32> {
        if self.is_maxed() {
            None
        } else {
            // next_level (1-indexed) × base cost: 5, 10, 15, …, 50
            Some(WEAPON_UPGRADE_BASE_COST * (self.level + 2))
        }
    }

    /// Returns `true` when the player has enough ore to afford the next upgrade.
    #[inline]
    pub fn can_afford_next(&self, ore: u32) -> bool {
        self.cost_for_next_level().is_some_and(|cost| ore >= cost)
    }

    /// Spend ore and increment the level.  Returns the amount spent, or `None`
    /// if maxed-out or the player cannot afford it.
    pub fn try_upgrade(&mut self, ore: &mut u32) -> Option<u32> {
        let cost = self.cost_for_next_level()?;
        if *ore < cost {
            return None;
        }
        *ore -= cost;
        self.level += 1;
        Some(cost)
    }
}

/// Primary weapon split-upgrade foundation model.
///
/// This is the data-layer slice for decoupling chip and destroy behavior.
/// Runtime combat now consumes this split-track model; [`PrimaryWeaponLevel`]
/// remains only for legacy serialization compatibility.
#[derive(Resource, Debug, Clone, Default)]
pub struct PrimaryWeaponUpgradeTracks {
    /// Internal 0-indexed level for chip power scaling.
    pub chip_level: u32,
    /// Internal 0-indexed level for full-destroy threshold scaling.
    pub destroy_level: u32,
}

impl PrimaryWeaponUpgradeTracks {
    /// Maximum internal level value (inclusive) for both tracks.
    pub const MAX: u32 = PRIMARY_WEAPON_MAX_LEVEL - 1;

    /// Build parity split-track values from legacy single-track progression.
    #[inline]
    pub fn from_legacy_level(level: u32) -> Self {
        let clamped = level.min(Self::MAX);
        Self {
            chip_level: clamped,
            destroy_level: clamped,
        }
    }

    /// Human-readable chip-track level (1-indexed).
    #[inline]
    pub fn chip_display_level(&self) -> u32 {
        self.chip_level.min(Self::MAX) + 1
    }

    /// Human-readable destroy-track level (1-indexed).
    #[inline]
    pub fn destroy_display_level(&self) -> u32 {
        self.destroy_level.min(Self::MAX) + 1
    }

    /// Current chip-size cap (mass units removed in one chip event).
    #[inline]
    pub fn max_chip_size(&self) -> u32 {
        self.chip_level.min(Self::MAX) + 1
    }

    /// Current full-destroy asteroid-size threshold.
    #[inline]
    pub fn max_destroy_size(&self) -> u32 {
        self.destroy_level.min(Self::MAX) + 1
    }

    #[inline]
    pub fn is_chip_maxed(&self) -> bool {
        self.chip_level >= Self::MAX
    }

    #[inline]
    pub fn is_destroy_maxed(&self) -> bool {
        self.destroy_level >= Self::MAX
    }

    #[inline]
    pub fn cost_for_next_chip_level(&self) -> Option<u32> {
        if self.is_chip_maxed() {
            None
        } else {
            Some(WEAPON_UPGRADE_BASE_COST * (self.chip_level + 2))
        }
    }

    #[inline]
    pub fn cost_for_next_destroy_level(&self) -> Option<u32> {
        if self.is_destroy_maxed() {
            None
        } else {
            Some(WEAPON_UPGRADE_BASE_COST * (self.destroy_level + 2))
        }
    }

    #[inline]
    pub fn can_afford_next_chip(&self, ore: u32) -> bool {
        self.cost_for_next_chip_level()
            .is_some_and(|cost| ore >= cost)
    }

    #[inline]
    pub fn can_afford_next_destroy(&self, ore: u32) -> bool {
        self.cost_for_next_destroy_level()
            .is_some_and(|cost| ore >= cost)
    }

    pub fn try_upgrade_chip(&mut self, ore: &mut u32) -> Option<u32> {
        let cost = self.cost_for_next_chip_level()?;
        if *ore < cost {
            return None;
        }
        *ore -= cost;
        self.chip_level += 1;
        Some(cost)
    }

    pub fn try_upgrade_destroy(&mut self, ore: &mut u32) -> Option<u32> {
        let cost = self.cost_for_next_destroy_level()?;
        if *ore < cost {
            return None;
        }
        *ore -= cost;
        self.destroy_level += 1;
        Some(cost)
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Secondary Weapon (Missile) Upgrade Level
// ══════════════════════════════════════════════════════════════════════════════

/// Missile weapon upgrade level.
///
/// Missiles become more destructive with upgrades:
/// - At level N (0-indexed), the missile destroys asteroids ≤ size `2 + N`
/// - For asteroids > size `2 + N`, it splits the asteroid into convex fragments
///
/// Example progression (internal level → destroy threshold):
/// - Level 0 (base): destroys 0–2
/// - Level 1: destroys 0–3
/// - Level 5: destroys 0–7
/// - Level 9 (max): destroys 0–11
#[derive(Resource, Debug, Clone, Default)]
pub struct SecondaryWeaponLevel {
    /// Internal 0-indexed level (0 = Level 1 / base, 9 = Level 10 / max).
    pub level: u32,
}

/// Tractor beam upgrade level.
///
/// Level scaling controls how aggressively the beam can interact with asteroids:
/// radius, force, max affected asteroid size, and max affected asteroid speed.
#[derive(Resource, Debug, Clone, Default)]
pub struct TractorBeamLevel {
    /// Internal 0-indexed level (0 = base).
    pub level: u32,
}

/// Ion cannon upgrade level.
///
/// The ion cannon emits a short EMP-like pulse that can temporarily disable
/// enemy ships. Higher levels increase stun duration and improve effectiveness
/// against stronger enemy tiers.
#[derive(Resource, Debug, Clone, Default)]
pub struct IonCannonLevel {
    /// Internal 0-indexed level (0 = Level 1 / base).
    pub level: u32,
}

impl TractorBeamLevel {
    /// Maximum internal level value (inclusive).
    pub const MAX: u32 = TRACTOR_BEAM_MAX_LEVEL - 1;

    /// Human-readable display level (1-indexed).
    #[inline]
    pub fn display_level(&self) -> u32 {
        self.level + 1
    }

    #[inline]
    pub fn range_at_level(&self, config: &PhysicsConfig) -> f32 {
        config.tractor_beam_range_base + self.level as f32 * config.tractor_beam_range_per_level
    }

    #[inline]
    pub fn force_at_level(&self, config: &PhysicsConfig) -> f32 {
        config.tractor_beam_force_base + self.level as f32 * config.tractor_beam_force_per_level
    }

    #[inline]
    pub fn max_target_size_at_level(&self, config: &PhysicsConfig) -> u32 {
        config.tractor_beam_max_target_size_base
            + self.level * config.tractor_beam_max_target_size_per_level
    }

    #[inline]
    pub fn max_target_speed_at_level(&self, config: &PhysicsConfig) -> f32 {
        config.tractor_beam_max_target_speed_base
            + self.level as f32 * config.tractor_beam_max_target_speed_per_level
    }

    #[inline]
    pub fn throw_cooldown_secs(&self, config: &PhysicsConfig) -> f32 {
        (config.tractor_throw_cooldown_base
            - self.level as f32 * config.tractor_throw_cooldown_per_level)
            .max(0.5)
    }

    /// Whether the tractor beam can be upgraded further.
    #[inline]
    pub fn is_maxed(&self) -> bool {
        self.level >= Self::MAX
    }

    /// Ore cost to buy the next upgrade level.
    /// Returns `None` when already at max level.
    #[inline]
    pub fn cost_for_next_level(&self) -> Option<u32> {
        if self.is_maxed() {
            None
        } else {
            Some(TRACTOR_BEAM_UPGRADE_BASE_COST * (self.level + 2))
        }
    }

    /// Returns `true` when the player has enough ore to afford the next upgrade.
    #[inline]
    pub fn can_afford_next(&self, ore: u32) -> bool {
        self.cost_for_next_level().is_some_and(|cost| ore >= cost)
    }

    /// Spend ore and increment the level. Returns the amount spent, or `None`
    /// if maxed-out or the player cannot afford it.
    pub fn try_upgrade(&mut self, ore: &mut u32) -> Option<u32> {
        let cost = self.cost_for_next_level()?;
        if *ore < cost {
            return None;
        }
        *ore -= cost;
        self.level += 1;
        Some(cost)
    }
}

impl SecondaryWeaponLevel {
    /// Maximum internal level value (inclusive).
    pub const MAX: u32 = SECONDARY_WEAPON_MAX_LEVEL - 1;

    /// Human-readable display level (1-indexed).
    #[inline]
    pub fn display_level(&self) -> u32 {
        self.level + 1
    }

    /// Largest asteroid size that this weapon fully destroys in one hit.
    #[inline]
    pub fn destroy_threshold(&self) -> u32 {
        2 + self.level
    }

    /// Split fragment count for impacts above [`Self::destroy_threshold`].
    ///
    /// Level mapping is 1-indexed for gameplay readability:
    /// - Level 1 → 2 pieces
    /// - Level 2 → 3 pieces
    /// - Level 3 → 4 pieces
    ///
    /// The result is clamped by `config.missile_split_max_pieces`.
    #[inline]
    pub fn split_piece_count(&self, config: &PhysicsConfig) -> u32 {
        (self.display_level() + 1)
            .max(2)
            .min(config.missile_split_max_pieces.max(2))
    }

    /// Whether this missile level should fully decompose an asteroid of `size`
    /// into unit fragments on impact.
    ///
    /// Rule: display level (1-indexed) must be at least the asteroid size.
    #[inline]
    pub fn can_fully_decompose_size(&self, size: u32) -> bool {
        self.display_level() >= size
    }

    /// Whether the weapon can be upgraded further.
    #[inline]
    pub fn is_maxed(&self) -> bool {
        self.level >= Self::MAX
    }

    /// Ore cost to buy the next upgrade level.
    /// Returns `None` when already at max level.
    #[inline]
    pub fn cost_for_next_level(&self) -> Option<u32> {
        if self.is_maxed() {
            None
        } else {
            // next_level (1-indexed) × base cost: 5, 10, 15, …, 50
            Some(SECONDARY_WEAPON_UPGRADE_BASE_COST * (self.level + 2))
        }
    }

    /// Returns `true` when the player has enough ore to afford the next upgrade.
    #[inline]
    pub fn can_afford_next(&self, ore: u32) -> bool {
        self.cost_for_next_level().is_some_and(|cost| ore >= cost)
    }

    /// Spend ore and increment the level.  Returns the amount spent, or `None`
    /// if maxed-out or the player cannot afford it.
    pub fn try_upgrade(&mut self, ore: &mut u32) -> Option<u32> {
        let cost = self.cost_for_next_level()?;
        if *ore < cost {
            return None;
        }
        *ore -= cost;
        self.level += 1;
        Some(cost)
    }
}

impl IonCannonLevel {
    /// Maximum internal level value (inclusive).
    pub const MAX: u32 = ION_CANNON_MAX_LEVEL - 1;

    /// Human-readable display level (1-indexed).
    #[inline]
    pub fn display_level(&self) -> u32 {
        self.level + 1
    }

    /// Stun duration in seconds at the current level.
    #[inline]
    pub fn stun_duration_secs(&self) -> f32 {
        ION_CANNON_BASE_STUN_SECS + self.level as f32 * ION_CANNON_STUN_SECS_PER_LEVEL
    }

    /// Highest enemy tier this level can affect.
    #[inline]
    pub fn max_enemy_tier_affected(&self) -> u32 {
        (1 + self.level / 2).min(4)
    }

    /// Whether the ion cannon can be upgraded further.
    #[inline]
    pub fn is_maxed(&self) -> bool {
        self.level >= Self::MAX
    }

    /// Ore cost to buy the next upgrade level.
    /// Returns `None` when already at max level.
    #[inline]
    pub fn cost_for_next_level(&self) -> Option<u32> {
        if self.is_maxed() {
            None
        } else {
            Some(ION_CANNON_UPGRADE_BASE_COST * (self.level + 2))
        }
    }

    /// Returns `true` when the player has enough ore to afford the next upgrade.
    #[inline]
    pub fn can_afford_next(&self, ore: u32) -> bool {
        self.cost_for_next_level().is_some_and(|cost| ore >= cost)
    }

    /// Spend ore and increment the level. Returns the amount spent, or `None`
    /// if maxed-out or the player cannot afford it.
    pub fn try_upgrade(&mut self, ore: &mut u32) -> Option<u32> {
        let cost = self.cost_for_next_level()?;
        if *ore < cost {
            return None;
        }
        *ore -= cost;
        self.level += 1;
        Some(cost)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missile_split_piece_count_scales_with_level() {
        let config = PhysicsConfig::default();
        let base = SecondaryWeaponLevel { level: 0 };
        let lvl_two = SecondaryWeaponLevel { level: 1 };
        let lvl_three = SecondaryWeaponLevel { level: 2 };

        assert_eq!(base.split_piece_count(&config), 2);
        assert_eq!(lvl_two.split_piece_count(&config), 3);
        assert_eq!(lvl_three.split_piece_count(&config), 4);
    }

    #[test]
    fn missile_split_piece_count_respects_config_clamp() {
        let mut config = PhysicsConfig::default();
        config.missile_split_max_pieces = 4;
        let high_level = SecondaryWeaponLevel { level: 9 };

        assert_eq!(high_level.split_piece_count(&config), 4);
    }

    #[test]
    fn missile_full_decompose_threshold_tracks_display_level() {
        let level_one = SecondaryWeaponLevel { level: 0 };
        let level_five = SecondaryWeaponLevel { level: 4 };

        assert!(level_one.can_fully_decompose_size(1));
        assert!(!level_one.can_fully_decompose_size(2));
        assert!(level_five.can_fully_decompose_size(5));
        assert!(!level_five.can_fully_decompose_size(6));
    }

    #[test]
    fn primary_split_tracks_from_legacy_preserves_parity() {
        let tracks = PrimaryWeaponUpgradeTracks::from_legacy_level(4);
        assert_eq!(tracks.chip_level, 4);
        assert_eq!(tracks.destroy_level, 4);
        assert_eq!(tracks.max_chip_size(), 5);
        assert_eq!(tracks.max_destroy_size(), 5);
    }

    #[test]
    fn primary_split_tracks_clamp_to_max_level() {
        let over = PrimaryWeaponUpgradeTracks::from_legacy_level(999);
        assert_eq!(over.chip_level, PrimaryWeaponUpgradeTracks::MAX);
        assert_eq!(over.destroy_level, PrimaryWeaponUpgradeTracks::MAX);
    }

    #[test]
    fn primary_split_tracks_support_independent_values() {
        let tracks = PrimaryWeaponUpgradeTracks {
            chip_level: 2,
            destroy_level: 6,
        };
        assert_eq!(tracks.chip_level, 2);
        assert_eq!(tracks.destroy_level, 6);
        assert_eq!(tracks.chip_display_level(), 3);
        assert_eq!(tracks.destroy_display_level(), 7);
    }

    #[test]
    fn primary_split_tracks_upgrade_helpers_spend_ore_per_track() {
        let mut tracks = PrimaryWeaponUpgradeTracks {
            chip_level: 0,
            destroy_level: 0,
        };
        let mut ore = 10;

        let chip_cost = tracks.try_upgrade_chip(&mut ore);
        assert_eq!(chip_cost, Some(10));
        assert_eq!(tracks.chip_level, 1);
        assert_eq!(tracks.destroy_level, 0);
        assert_eq!(ore, 0);

        let destroy_cost = tracks.try_upgrade_destroy(&mut ore);
        assert_eq!(destroy_cost, None);
        assert_eq!(tracks.destroy_level, 0);
    }
}

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

// ── Input Abstraction ──────────────────────────────────────────────────────────

/// Aggregated player intent for the current frame, derived from all input sources.
///
/// Input systems (keyboard, gamepad) write to this resource each frame after it
/// is cleared.  [`super::control::apply_player_intent_system`] reads it and
/// applies the corresponding physics forces.  Tests can populate this directly
/// to drive ship behaviour without a real input device.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub struct PlayerIntent {
    /// Forward thrust multiplier.  `1.0` applies full `THRUST_FORCE`; `0.0` means no thrust.
    pub thrust_forward: f32,
    /// Reverse thrust multiplier.  `1.0` applies full `REVERSE_FORCE`; `0.0` means no reverse.
    pub thrust_reverse: f32,
    /// Direct angular-velocity override in **rad/s**.
    ///
    /// `Some(value)` overwrites `Velocity::angvel`; `None` leaves the current
    /// angular velocity untouched (Rapier damping will slow it naturally).
    pub angvel: Option<f32>,
    /// Desired facing direction in world-space (unit vector).
    ///
    /// When present, the movement apply system steers angular velocity toward
    /// this direction and takes precedence over `angvel`.
    pub desired_facing: Option<Vec2>,
    /// Local left/right strafe axis: -1 = left, +1 = right.
    pub strafe_local: f32,
    /// World-space omnidirectional strafe input (typically gamepad left stick).
    /// Expected range per component is [-1, 1].
    pub strafe_world: Vec2,
    /// Active-brake flag: applies `GAMEPAD_BRAKE_DAMPING` to linvel/angvel while true.
    pub brake: bool,
}
