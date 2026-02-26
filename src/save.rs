use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use serde::{Deserialize, Serialize};

use crate::asteroid::{self, Asteroid, AsteroidSize, Vertices};
use crate::config::PhysicsConfig;
use crate::menu::{GameState, SelectedScenario};
use crate::mining::{OreAffinityLevel, PlayerOre};
use crate::player::state::{
    MissileAmmo, PlayerHealth, PlayerLives, PlayerScore, PrimaryWeaponLevel, SecondaryWeaponLevel,
    TractorBeamLevel,
};
use crate::player::Player;

pub const SAVE_SLOT_COUNT: u8 = 3;
const SAVE_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct SaveSlotMetadata {
    pub slot: u8,
    pub exists: bool,
    pub loadable: bool,
    pub scenario: Option<SaveScenario>,
    pub saved_at_unix: Option<u64>,
    pub status: String,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct SaveSlotRequest {
    pub slot: u8,
}

#[derive(Resource, Default, Debug, Clone)]
pub struct PendingLoadedSnapshot(pub Option<SaveSnapshot>);

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum SaveScenario {
    Field,
    Orbit,
    Comets,
    Shower,
}

impl From<SelectedScenario> for SaveScenario {
    fn from(value: SelectedScenario) -> Self {
        match value {
            SelectedScenario::Field => Self::Field,
            SelectedScenario::Orbit => Self::Orbit,
            SelectedScenario::Comets => Self::Comets,
            SelectedScenario::Shower => Self::Shower,
        }
    }
}

impl From<SaveScenario> for SelectedScenario {
    fn from(value: SaveScenario) -> Self {
        match value {
            SaveScenario::Field => Self::Field,
            SaveScenario::Orbit => Self::Orbit,
            SaveScenario::Comets => Self::Comets,
            SaveScenario::Shower => Self::Shower,
        }
    }
}

impl SaveScenario {
    #[inline]
    pub fn label(self) -> &'static str {
        match self {
            SaveScenario::Field => "FIELD",
            SaveScenario::Orbit => "ORBIT",
            SaveScenario::Comets => "COMETS",
            SaveScenario::Shower => "SHOWER",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaveSnapshot {
    pub version: u32,
    pub saved_at_unix: u64,
    pub scenario: SaveScenario,
    pub player: Option<PlayerSnapshot>,
    pub asteroids: Vec<AsteroidSnapshot>,
    pub resources: ResourceSnapshot,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResourceSnapshot {
    pub score_hits: u32,
    pub score_destroyed: u32,
    pub score_streak: u32,
    pub score_points: u32,
    pub lives_remaining: i32,
    pub lives_respawn_timer: Option<f32>,
    pub ore_count: u32,
    pub missile_ammo: u32,
    pub primary_weapon_level: u32,
    pub secondary_weapon_level: u32,
    pub ore_affinity_level: u32,
    pub tractor_beam_level: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerSnapshot {
    pub pos: [f32; 2],
    pub rot: f32,
    pub linvel: [f32; 2],
    pub angvel: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub inv_timer: f32,
    pub time_since_damage: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AsteroidSnapshot {
    pub pos: [f32; 2],
    pub rot: f32,
    pub linvel: [f32; 2],
    pub angvel: f32,
    pub size: u32,
    pub vertices: Vec<[f32; 2]>,
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingLoadedSnapshot>()
            .add_message::<SaveSlotRequest>()
            .add_systems(
                Update,
                handle_save_slot_requests_system.run_if(in_state(GameState::Paused)),
            );
    }
}

fn save_dir() -> PathBuf {
    PathBuf::from("saves")
}

fn slot_path(slot: u8) -> PathBuf {
    save_dir().join(format!("slot_{slot}.toml"))
}

pub fn slot_exists(slot: u8) -> bool {
    if !(1..=SAVE_SLOT_COUNT).contains(&slot) {
        return false;
    }
    slot_path(slot).exists()
}

pub fn load_slot(slot: u8) -> Result<SaveSnapshot, String> {
    if !(1..=SAVE_SLOT_COUNT).contains(&slot) {
        return Err(format!("invalid slot {slot}"));
    }

    let path = slot_path(slot);
    let contents = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;

    parse_snapshot_with_migration(&contents)
}

pub fn slot_metadata(slot: u8) -> SaveSlotMetadata {
    if !(1..=SAVE_SLOT_COUNT).contains(&slot) {
        return SaveSlotMetadata {
            slot,
            exists: false,
            loadable: false,
            scenario: None,
            saved_at_unix: None,
            status: "INVALID SLOT".to_string(),
        };
    }

    if !slot_exists(slot) {
        return SaveSlotMetadata {
            slot,
            exists: false,
            loadable: false,
            scenario: None,
            saved_at_unix: None,
            status: "EMPTY".to_string(),
        };
    }

    match load_slot(slot) {
        Ok(snapshot) => SaveSlotMetadata {
            slot,
            exists: true,
            loadable: true,
            scenario: Some(snapshot.scenario),
            saved_at_unix: Some(snapshot.saved_at_unix),
            status: "READY".to_string(),
        },
        Err(_) => SaveSlotMetadata {
            slot,
            exists: true,
            loadable: false,
            scenario: None,
            saved_at_unix: None,
            status: "CORRUPT".to_string(),
        },
    }
}

pub fn slot_loadable(slot: u8) -> bool {
    slot_metadata(slot).loadable
}

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn parse_snapshot_with_migration(contents: &str) -> Result<SaveSnapshot, String> {
    let mut value: toml::Value =
        toml::from_str(contents).map_err(|err| format!("failed to parse save TOML: {err}"))?;

    migrate_snapshot_value(&mut value)?;

    value
        .try_into::<SaveSnapshot>()
        .map_err(|err| format!("failed to decode migrated save snapshot: {err}"))
}

fn migrate_snapshot_value(value: &mut toml::Value) -> Result<(), String> {
    let table = value
        .as_table_mut()
        .ok_or_else(|| "save file root must be a TOML table".to_string())?;

    if !table.contains_key("version") {
        table.insert(
            "version".to_string(),
            toml::Value::Integer(SAVE_VERSION as i64),
        );
    }

    if !table.contains_key("saved_at_unix") {
        table.insert("saved_at_unix".to_string(), toml::Value::Integer(0));
    }

    if let Some(resources) = table
        .get_mut("resources")
        .and_then(toml::Value::as_table_mut)
    {
        if !resources.contains_key("tractor_beam_level") {
            resources.insert("tractor_beam_level".to_string(), toml::Value::Integer(0));
        }
    }

    let version = table
        .get("version")
        .and_then(toml::Value::as_integer)
        .ok_or_else(|| "save version is missing or invalid".to_string())?;

    if version != SAVE_VERSION as i64 {
        return Err(format!(
            "unsupported save version {} (expected {})",
            version, SAVE_VERSION
        ));
    }

    Ok(())
}

fn write_slot(slot: u8, snapshot: &SaveSnapshot) -> Result<(), String> {
    if !(1..=SAVE_SLOT_COUNT).contains(&slot) {
        return Err(format!("invalid slot {slot}"));
    }

    fs::create_dir_all(save_dir()).map_err(|err| format!("failed to create save dir: {err}"))?;

    let serialized = toml::to_string_pretty(snapshot)
        .map_err(|err| format!("failed to serialize save TOML: {err}"))?;

    let path = slot_path(slot);
    fs::write(&path, serialized).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

#[allow(clippy::too_many_arguments)]
pub fn handle_save_slot_requests_system(
    mut requests: MessageReader<SaveSlotRequest>,
    scenario: Res<SelectedScenario>,
    score: Res<PlayerScore>,
    lives: Res<PlayerLives>,
    ore: Res<PlayerOre>,
    ammo: Res<MissileAmmo>,
    primary_level: Res<PrimaryWeaponLevel>,
    secondary_level: Res<SecondaryWeaponLevel>,
    affinity_level: Res<OreAffinityLevel>,
    tractor_level: Res<TractorBeamLevel>,
    q_player: Query<(&Transform, &Velocity, &PlayerHealth), With<Player>>,
    q_asteroids: Query<(&Transform, &Velocity, &AsteroidSize, &Vertices), With<Asteroid>>,
) {
    for request in requests.read() {
        let player_snapshot = q_player
            .single()
            .ok()
            .map(|(transform, vel, hp)| PlayerSnapshot {
                pos: [transform.translation.x, transform.translation.y],
                rot: transform.rotation.to_euler(EulerRot::XYZ).2,
                linvel: [vel.linvel.x, vel.linvel.y],
                angvel: vel.angvel,
                hp: hp.hp,
                max_hp: hp.max_hp,
                inv_timer: hp.inv_timer,
                time_since_damage: hp.time_since_damage,
            });

        let asteroids = q_asteroids
            .iter()
            .map(|(transform, vel, size, vertices)| AsteroidSnapshot {
                pos: [transform.translation.x, transform.translation.y],
                rot: transform.rotation.to_euler(EulerRot::XYZ).2,
                linvel: [vel.linvel.x, vel.linvel.y],
                angvel: vel.angvel,
                size: size.0,
                vertices: vertices.0.iter().map(|v| [v.x, v.y]).collect(),
            })
            .collect();

        let snapshot = SaveSnapshot {
            version: SAVE_VERSION,
            saved_at_unix: current_unix_timestamp(),
            scenario: SaveScenario::from(*scenario),
            player: player_snapshot,
            asteroids,
            resources: ResourceSnapshot {
                score_hits: score.hits,
                score_destroyed: score.destroyed,
                score_streak: score.streak,
                score_points: score.points,
                lives_remaining: lives.remaining,
                lives_respawn_timer: lives.respawn_timer,
                ore_count: ore.count,
                missile_ammo: ammo.count,
                primary_weapon_level: primary_level.level,
                secondary_weapon_level: secondary_level.level,
                ore_affinity_level: affinity_level.level,
                tractor_beam_level: tractor_level.level,
            },
        };

        match write_slot(request.slot, &snapshot) {
            Ok(()) => {
                info!("Saved game to slot {}", request.slot);
            }
            Err(err) => {
                error!("Failed to save game to slot {}: {}", request.slot, err);
            }
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn apply_pending_loaded_snapshot_system(
    mut commands: Commands,
    mut pending: ResMut<PendingLoadedSnapshot>,
    config: Res<PhysicsConfig>,
    mut selected_scenario: ResMut<SelectedScenario>,
    mut score: ResMut<PlayerScore>,
    mut lives: ResMut<PlayerLives>,
    mut ore: ResMut<PlayerOre>,
    mut ammo: ResMut<MissileAmmo>,
    mut primary_level: ResMut<PrimaryWeaponLevel>,
    mut secondary_level: ResMut<SecondaryWeaponLevel>,
    mut affinity_level: ResMut<OreAffinityLevel>,
    mut tractor_level: ResMut<TractorBeamLevel>,
) {
    let Some(snapshot) = pending.0.take() else {
        warn!("No pending save snapshot found on load transition");
        return;
    };

    *selected_scenario = SelectedScenario::from(snapshot.scenario);

    *score = PlayerScore {
        hits: snapshot.resources.score_hits,
        destroyed: snapshot.resources.score_destroyed,
        streak: snapshot.resources.score_streak,
        points: snapshot.resources.score_points,
    };
    *lives = PlayerLives {
        remaining: snapshot.resources.lives_remaining,
        respawn_timer: snapshot.resources.lives_respawn_timer,
    };
    *ore = PlayerOre {
        count: snapshot.resources.ore_count,
    };
    ammo.count = snapshot.resources.missile_ammo;
    primary_level.level = snapshot
        .resources
        .primary_weapon_level
        .min(PrimaryWeaponLevel::MAX);
    secondary_level.level = snapshot
        .resources
        .secondary_weapon_level
        .min(SecondaryWeaponLevel::MAX);
    affinity_level.level = snapshot
        .resources
        .ore_affinity_level
        .min(OreAffinityLevel::MAX);
    tractor_level.level = snapshot
        .resources
        .tractor_beam_level
        .min(TractorBeamLevel::MAX);

    for asteroid in snapshot.asteroids {
        if asteroid.vertices.len() < 3 {
            continue;
        }

        let hull: Vec<Vec2> = asteroid
            .vertices
            .iter()
            .map(|v| Vec2::new(v[0], v[1]))
            .collect();

        let entity = asteroid::spawn_asteroid_with_vertices(
            &mut commands,
            Vec2::new(asteroid.pos[0], asteroid.pos[1]),
            &hull,
            Color::WHITE,
            asteroid.size,
        );

        let transform = Transform {
            translation: Vec3::new(asteroid.pos[0], asteroid.pos[1], 0.05),
            rotation: Quat::from_rotation_z(asteroid.rot),
            scale: Vec3::ONE,
        };

        commands.entity(entity).insert((
            transform,
            GlobalTransform::from(transform),
            Velocity {
                linvel: Vec2::new(asteroid.linvel[0], asteroid.linvel[1]),
                angvel: asteroid.angvel,
            },
        ));
    }

    if let Some(player) = snapshot.player {
        commands.spawn((
            Player,
            PlayerHealth {
                hp: player.hp,
                max_hp: player.max_hp,
                inv_timer: player.inv_timer,
                time_since_damage: player.time_since_damage,
            },
            RigidBody::Dynamic,
            Collider::ball(config.player_collider_radius),
            Velocity {
                linvel: Vec2::new(player.linvel[0], player.linvel[1]),
                angvel: player.angvel,
            },
            ExternalForce::default(),
            Damping {
                linear_damping: config.player_linear_damping,
                angular_damping: config.player_angular_damping,
            },
            Restitution::coefficient(config.player_restitution),
            CollisionGroups::new(
                bevy_rapier2d::geometry::Group::GROUP_2,
                bevy_rapier2d::geometry::Group::GROUP_1 | bevy_rapier2d::geometry::Group::GROUP_4,
            ),
            ActiveEvents::COLLISION_EVENTS,
            Transform {
                translation: Vec3::new(player.pos[0], player.pos[1], 0.0),
                rotation: Quat::from_rotation_z(player.rot),
                scale: Vec3::ONE,
            },
            Visibility::default(),
        ));
    }

    info!("Loaded snapshot successfully");
}
