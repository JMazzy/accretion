use crate::config::PhysicsConfig;
use crate::enemy::Enemy;
use crate::enemy::{EnemyProjectile, EnemySpawnState};
use crate::menu::{SelectedGameMode, SelectedScenario};
use crate::mining::OrePickup;
use crate::particles::Particle;
use crate::player::state::{Missile, Projectile};
use crate::player::PlayerHealth;
use bevy::prelude::*;

/// Static campaign mission descriptor used by the foundation mission-loader.
#[derive(Debug, Clone)]
pub struct CampaignMissionDefinition {
    pub mission_id: u32,
    pub map_scenario: SelectedScenario,
    pub wave_count: u32,
    pub reward_ore: u32,
    pub next_mission_id: Option<u32>,
}

/// Campaign mission catalog resource (A2 foundation).
#[derive(Resource, Debug, Clone)]
pub struct CampaignMissionCatalog {
    pub missions: Vec<CampaignMissionDefinition>,
}

impl CampaignMissionCatalog {
    pub fn mission_by_id(&self, mission_id: u32) -> Option<&CampaignMissionDefinition> {
        self.missions.iter().find(|m| m.mission_id == mission_id)
    }

    pub fn first_mission(&self) -> Option<&CampaignMissionDefinition> {
        self.missions.first()
    }
}

impl Default for CampaignMissionCatalog {
    fn default() -> Self {
        Self {
            missions: vec![
                CampaignMissionDefinition {
                    mission_id: 1,
                    map_scenario: SelectedScenario::Field,
                    wave_count: 3,
                    reward_ore: 20,
                    next_mission_id: Some(2),
                },
                CampaignMissionDefinition {
                    mission_id: 2,
                    map_scenario: SelectedScenario::Comets,
                    wave_count: 4,
                    reward_ore: 35,
                    next_mission_id: Some(3),
                },
                CampaignMissionDefinition {
                    mission_id: 3,
                    map_scenario: SelectedScenario::Orbit,
                    wave_count: 5,
                    reward_ore: 50,
                    next_mission_id: None,
                },
            ],
        }
    }
}

/// Minimal campaign runtime session state used by the mode scaffold.
#[derive(Resource, Debug, Clone, Default)]
pub struct CampaignSession {
    /// Whether the current `Playing` session is using campaign mode.
    pub active: bool,
    /// 1-indexed mission index for the current campaign run.
    pub mission_index: u32,
    /// Scenario used for the currently loaded mission map.
    pub map_scenario: SelectedScenario,
    /// Number of waves configured for the loaded mission.
    pub wave_count: u32,
    /// Ore reward granted on mission completion.
    pub reward_ore: u32,
    /// Next mission id if progression is available.
    pub next_mission_id: Option<u32>,
    /// Monotonic counter for campaign run starts in this process.
    pub run_counter: u64,
}

/// Runtime phase of a campaign mission wave loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CampaignWavePhase {
    #[default]
    Inactive,
    Warmup,
    ActiveWave,
    InterWaveBreak,
    Complete,
}

/// Campaign wave director runtime state (A3 foundation).
#[derive(Resource, Debug, Clone, Default)]
pub struct CampaignWaveDirector {
    pub phase: CampaignWavePhase,
    pub current_wave: u32,
    pub total_waves: u32,
    pub phase_timer_secs: f32,
    pub target_spawns_this_wave: u32,
    pub spawned_this_wave: u32,
    pub max_concurrent_enemies: u32,
    pub spawn_cooldown_secs: f32,
}

/// Runtime progression state for moving between campaign missions.
#[derive(Resource, Debug, Clone, Default)]
pub struct CampaignProgressionState {
    pub pending_advance: bool,
    pub advance_timer_secs: f32,
    pub mission_failed: bool,
}

fn configure_active_wave(director: &mut CampaignWaveDirector, config: &PhysicsConfig) {
    let wave_index = director.current_wave.saturating_sub(1);
    director.phase = CampaignWavePhase::ActiveWave;
    director.phase_timer_secs = 0.0;
    director.spawned_this_wave = 0;
    director.target_spawns_this_wave = (2 + wave_index * 2).min(20);
    director.max_concurrent_enemies = (1 + wave_index).min(config.enemy_max_count_cap.max(1));
    let wave_speedup = 1.0 + wave_index as f32 * 0.20;
    director.spawn_cooldown_secs = (config.enemy_spawn_base_cooldown / wave_speedup)
        .max(config.enemy_spawn_cooldown_min.max(0.25));
}

fn load_mission_into_session(
    session: &mut CampaignSession,
    catalog: &CampaignMissionCatalog,
    requested_mission: u32,
) {
    let mission = catalog
        .mission_by_id(requested_mission.max(1))
        .or_else(|| catalog.first_mission());

    if let Some(mission) = mission {
        session.mission_index = mission.mission_id;
        session.map_scenario = mission.map_scenario;
        session.wave_count = mission.wave_count;
        session.reward_ore = mission.reward_ore;
        session.next_mission_id = mission.next_mission_id;
    } else {
        session.mission_index = 1;
        session.map_scenario = SelectedScenario::Field;
        session.wave_count = 1;
        session.reward_ore = 0;
        session.next_mission_id = None;
    }
}

fn spawn_campaign_world_for_scenario(
    commands: &mut Commands,
    config: &PhysicsConfig,
    scenario: SelectedScenario,
) {
    match scenario {
        SelectedScenario::Field => crate::asteroid::spawn_initial_asteroids(commands, 100, config),
        SelectedScenario::Orbit => crate::asteroid::spawn_orbit_scenario(commands, config),
        SelectedScenario::Comets => crate::asteroid::spawn_comets_scenario(commands, config),
        SelectedScenario::Shower => crate::asteroid::spawn_shower_scenario(commands, config),
    }
}

/// Initialize campaign session state when entering gameplay.
pub fn bootstrap_campaign_session(
    mode: Res<SelectedGameMode>,
    catalog: Res<CampaignMissionCatalog>,
    mut session: ResMut<CampaignSession>,
) {
    match *mode {
        SelectedGameMode::Campaign => {
            session.active = true;
            let requested = session.mission_index.max(1);
            load_mission_into_session(&mut session, &catalog, requested);
            session.run_counter = session.run_counter.saturating_add(1);
        }
        SelectedGameMode::Practice => {
            session.active = false;
            session.mission_index = 0;
            session.map_scenario = SelectedScenario::Field;
            session.wave_count = 0;
            session.reward_ore = 0;
            session.next_mission_id = None;
        }
    }
}

/// Initialize campaign progression control state when entering gameplay.
pub fn bootstrap_campaign_progression_state(
    session: Res<CampaignSession>,
    mut progression: ResMut<CampaignProgressionState>,
) {
    progression.pending_advance = false;
    progression.advance_timer_secs = 0.0;
    progression.mission_failed = false;

    if !session.active {
        progression.mission_failed = false;
    }
}

/// Initialize wave director state when entering gameplay.
pub fn bootstrap_campaign_wave_director(
    config: Res<PhysicsConfig>,
    session: Res<CampaignSession>,
    mut director: ResMut<CampaignWaveDirector>,
) {
    if !session.active || session.wave_count == 0 {
        *director = CampaignWaveDirector::default();
        return;
    }

    director.phase = CampaignWavePhase::Warmup;
    director.current_wave = 1;
    director.total_waves = session.wave_count.max(1);
    director.phase_timer_secs = 2.0;
    director.target_spawns_this_wave = 0;
    director.spawned_this_wave = 0;
    director.max_concurrent_enemies = 1;
    director.spawn_cooldown_secs = config.enemy_spawn_base_cooldown;
}

/// Update campaign wave loop state machine while playing campaign mode.
pub fn campaign_wave_director_system(
    time: Res<Time>,
    session: Res<CampaignSession>,
    config: Res<PhysicsConfig>,
    q_enemies: Query<Entity, With<Enemy>>,
    mut director: ResMut<CampaignWaveDirector>,
) {
    if !session.active {
        return;
    }
    if director.phase == CampaignWavePhase::Inactive
        || director.phase == CampaignWavePhase::Complete
    {
        return;
    }

    let dt = time.delta_secs();
    director.phase_timer_secs = (director.phase_timer_secs - dt).max(0.0);
    let live_enemies = q_enemies.iter().count() as u32;

    match director.phase {
        CampaignWavePhase::Warmup => {
            if director.phase_timer_secs <= 0.0 {
                configure_active_wave(&mut director, &config);
            }
        }
        CampaignWavePhase::ActiveWave => {
            if director.spawned_this_wave >= director.target_spawns_this_wave && live_enemies == 0 {
                if director.current_wave >= director.total_waves {
                    director.phase = CampaignWavePhase::Complete;
                    director.phase_timer_secs = 0.0;
                } else {
                    director.phase = CampaignWavePhase::InterWaveBreak;
                    director.phase_timer_secs = 4.0;
                }
            }
        }
        CampaignWavePhase::InterWaveBreak => {
            if director.phase_timer_secs <= 0.0 {
                director.current_wave += 1;
                director.phase = CampaignWavePhase::Warmup;
                director.phase_timer_secs = 1.5;
            }
        }
        CampaignWavePhase::Complete | CampaignWavePhase::Inactive => {}
    }
}

/// Handle campaign mission completion/failure transitions during gameplay.
#[allow(clippy::too_many_arguments)]
pub fn campaign_progression_system(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<PhysicsConfig>,
    catalog: Res<CampaignMissionCatalog>,
    mut session: ResMut<CampaignSession>,
    mut progression: ResMut<CampaignProgressionState>,
    mut wave: ResMut<CampaignWaveDirector>,
    mut enemy_spawn: ResMut<EnemySpawnState>,
    mut q_player_health: Query<&mut PlayerHealth>,
    q_asteroids: Query<Entity, With<crate::asteroid::Asteroid>>,
    q_enemies: Query<Entity, With<Enemy>>,
    q_enemy_projectiles: Query<Entity, With<EnemyProjectile>>,
    q_projectiles: Query<Entity, With<Projectile>>,
    q_missiles: Query<Entity, With<Missile>>,
    q_particles: Query<Entity, With<Particle>>,
    q_ore: Query<Entity, With<OrePickup>>,
) {
    if !session.active {
        return;
    }

    if wave.phase != CampaignWavePhase::Complete {
        return;
    }

    if !progression.pending_advance {
        progression.pending_advance = true;
        progression.advance_timer_secs = 1.0;
        return;
    }

    progression.advance_timer_secs = (progression.advance_timer_secs - time.delta_secs()).max(0.0);
    if progression.advance_timer_secs > 0.0 {
        return;
    }

    let Some(next_mission) = session.next_mission_id else {
        progression.pending_advance = false;
        return;
    };

    load_mission_into_session(&mut session, &catalog, next_mission);

    for entity in q_asteroids
        .iter()
        .chain(q_enemies.iter())
        .chain(q_enemy_projectiles.iter())
        .chain(q_projectiles.iter())
        .chain(q_missiles.iter())
        .chain(q_particles.iter())
        .chain(q_ore.iter())
    {
        commands.entity(entity).despawn();
    }

    spawn_campaign_world_for_scenario(&mut commands, &config, session.map_scenario);

    if let Ok(mut player_health) = q_player_health.single_mut() {
        player_health.hp = player_health.max_hp;
        player_health.inv_timer = config.respawn_invincibility_secs;
        player_health.time_since_damage = 0.0;
    }

    wave.phase = CampaignWavePhase::Warmup;
    wave.current_wave = 1;
    wave.total_waves = session.wave_count.max(1);
    wave.phase_timer_secs = 1.5;
    wave.target_spawns_this_wave = 0;
    wave.spawned_this_wave = 0;
    wave.max_concurrent_enemies = 1;
    wave.spawn_cooldown_secs = config.enemy_spawn_base_cooldown;

    enemy_spawn.timer_secs = 0.0;
    enemy_spawn.session_elapsed_secs = 0.0;

    progression.pending_advance = false;
}

/// Mark campaign run as failed when entering GameOver.
pub fn mark_campaign_failure_on_game_over(
    session: Res<CampaignSession>,
    mut progression: ResMut<CampaignProgressionState>,
    mut wave: ResMut<CampaignWaveDirector>,
) {
    if !session.active {
        return;
    }
    progression.mission_failed = true;
    progression.pending_advance = false;
    progression.advance_timer_secs = 0.0;
    wave.phase = CampaignWavePhase::Inactive;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contains_mission_one() {
        let catalog = CampaignMissionCatalog::default();
        let mission = catalog
            .mission_by_id(1)
            .expect("default campaign catalog must include mission 1");
        assert_eq!(mission.mission_id, 1);
        assert!(mission.wave_count >= 1);
    }

    #[test]
    fn bootstrap_campaign_loads_requested_mission_from_catalog() {
        let mut world = World::new();
        world.insert_resource(SelectedGameMode::Campaign);
        world.insert_resource(CampaignMissionCatalog::default());
        world.insert_resource(CampaignSession {
            active: false,
            mission_index: 2,
            map_scenario: SelectedScenario::Field,
            wave_count: 0,
            reward_ore: 0,
            next_mission_id: None,
            run_counter: 0,
        });

        let mut schedule = Schedule::default();
        schedule.add_systems(bootstrap_campaign_session);
        schedule.run(&mut world);

        let session = world.resource::<CampaignSession>();
        assert!(session.active);
        assert_eq!(session.mission_index, 2);
        assert_eq!(session.map_scenario, SelectedScenario::Comets);
        assert!(session.wave_count >= 1);
        assert!(session.run_counter >= 1);
    }

    #[test]
    fn bootstrap_wave_director_starts_at_warmup_for_campaign() {
        let mut world = World::new();
        world.insert_resource(PhysicsConfig::default());
        world.insert_resource(CampaignSession {
            active: true,
            mission_index: 1,
            map_scenario: SelectedScenario::Field,
            wave_count: 3,
            reward_ore: 20,
            next_mission_id: Some(2),
            run_counter: 1,
        });
        world.insert_resource(CampaignWaveDirector::default());

        let mut schedule = Schedule::default();
        schedule.add_systems(bootstrap_campaign_wave_director);
        schedule.run(&mut world);

        let director = world.resource::<CampaignWaveDirector>();
        assert_eq!(director.phase, CampaignWavePhase::Warmup);
        assert_eq!(director.current_wave, 1);
        assert_eq!(director.total_waves, 3);
    }

    #[test]
    fn wave_director_transitions_to_complete_on_last_wave_clear() {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        world.insert_resource(PhysicsConfig::default());
        world.insert_resource(CampaignSession {
            active: true,
            mission_index: 1,
            map_scenario: SelectedScenario::Field,
            wave_count: 1,
            reward_ore: 20,
            next_mission_id: None,
            run_counter: 1,
        });
        world.insert_resource(CampaignWaveDirector {
            phase: CampaignWavePhase::ActiveWave,
            current_wave: 1,
            total_waves: 1,
            phase_timer_secs: 0.0,
            target_spawns_this_wave: 2,
            spawned_this_wave: 2,
            max_concurrent_enemies: 1,
            spawn_cooldown_secs: 1.0,
        });

        let mut schedule = Schedule::default();
        schedule.add_systems(campaign_wave_director_system);
        schedule.run(&mut world);

        let director = world.resource::<CampaignWaveDirector>();
        assert_eq!(director.phase, CampaignWavePhase::Complete);
    }
}
