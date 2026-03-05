use super::*;

/// Despawn all simulation entities and reset per-session resources so the game
/// is completely clean when the player returns to the main menu.
///
/// Runs on `OnTransition { Paused → MainMenu }` (after `OnExit(Paused)` has
/// already removed the pause overlay).
///
/// The Rapier physics pipeline is explicitly disabled here as a safeguard
/// against parry2d BVH "key not present" panics: `step_simulation` must not
/// run with a live pipeline while entity handles are being flushed from
/// Rapier's internal data structures.  `resume_physics` is called again on
/// the `ScenarioSelect → Playing` transition when a new session begins.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn cleanup_game_world(
    mut commands: Commands,
    asteroids: Query<Entity, With<crate::asteroid::Asteroid>>,
    players_and_enemies: Query<
        Entity,
        Or<(
            With<crate::player::Player>,
            With<crate::enemy::Enemy>,
            With<crate::enemy::Boss>,
        )>,
    >,
    projectiles: Query<
        Entity,
        Or<(
            With<crate::player::state::Projectile>,
            With<crate::player::state::Missile>,
            With<crate::enemy::EnemyProjectile>,
        )>,
    >,
    particles: Query<Entity, With<crate::particles::Particle>>,
    ore_pickups: Query<Entity, With<crate::mining::OrePickup>>,
    hud: Query<
        Entity,
        Or<(
            With<crate::rendering::HudScoreDisplay>,
            With<crate::rendering::StatsTextDisplay>,
            With<crate::rendering::PhysicsInspectorDisplay>,
            With<crate::rendering::ProfilerDisplay>,
            With<crate::rendering::DebugPanel>,
            With<crate::rendering::LivesHudDisplay>,
            With<crate::rendering::MissileHudDisplay>,
            With<crate::rendering::BoundaryRing>,
            With<crate::rendering::WireframeOverlayLayer>,
            With<crate::rendering::ForceVectorLayer>,
            With<crate::rendering::VelocityArrowLayer>,
            With<crate::rendering::SpatialGridLayer>,
            With<crate::rendering::OreHudDisplay>,
        )>,
    >,
    player_ui: Query<
        Entity,
        Or<(
            With<crate::player::rendering::HealthBarBg>,
            With<crate::player::rendering::HealthBarFill>,
            With<crate::player::rendering::AimIndicatorMesh>,
        )>,
    >,
    mut player_ui_res: ResMut<crate::player::PlayerUiEntities>,
    mut score: ResMut<PlayerScore>,
    mut lives: ResMut<PlayerLives>,
    mut overlay: ResMut<crate::rendering::OverlayState>,
    mut sim_stats: ResMut<crate::simulation::SimulationStats>,
    mut ore: ResMut<crate::mining::PlayerOre>,
    mut campaign_session: ResMut<crate::campaign::CampaignSession>,
    mut rapier_config: Query<&mut RapierConfiguration>,
) {
    for e in asteroids
        .iter()
        .chain(players_and_enemies.iter())
        .chain(projectiles.iter())
        .chain(particles.iter())
        .chain(ore_pickups.iter())
        .chain(hud.iter())
        .chain(player_ui.iter())
    {
        commands.entity(e).despawn();
    }
    *player_ui_res = crate::player::PlayerUiEntities::default();
    *score = PlayerScore::default();
    lives.reset();
    *overlay = crate::rendering::OverlayState::default();
    *sim_stats = crate::simulation::SimulationStats::default();
    *ore = crate::mining::PlayerOre::default();
    *campaign_session = crate::campaign::CampaignSession::default();
    commands.insert_resource(crate::campaign::CampaignWaveDirector::default());
    commands.insert_resource(crate::campaign::CampaignProgressionState::default());
    // Reset upgrades so a new session starts fresh.
    commands.insert_resource(PrimaryWeaponUpgradeTracks::from_legacy_level(0));
    commands.insert_resource(SecondaryWeaponLevel::default());
    commands.insert_resource(OreAffinityLevel::default());
    commands.insert_resource(TractorBeamLevel::default());
    // Keep the physics pipeline disabled until a new session begins.
    // resume_physics is called on OnTransition { ScenarioSelect → Playing }.
    for mut cfg in rapier_config.iter_mut() {
        cfg.physics_pipeline_active = false;
    }
}

/// Reset active gameplay world/resources for a campaign retry after Game Over.
///
/// Unlike [`cleanup_game_world`], this preserves campaign slot progression/loadout
/// resources and HUD entities. It only clears active combat entities and
/// per-attempt counters so `GameOver -> Playing` can start a clean mission run.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn reset_campaign_retry_world(
    mut commands: Commands,
    mode: Res<SelectedGameMode>,
    asteroids: Query<Entity, With<crate::asteroid::Asteroid>>,
    players_and_enemies: Query<
        Entity,
        Or<(
            With<crate::player::Player>,
            With<crate::enemy::Enemy>,
            With<crate::enemy::Boss>,
        )>,
    >,
    projectiles: Query<
        Entity,
        Or<(
            With<crate::player::state::Projectile>,
            With<crate::player::state::Missile>,
            With<crate::player::ion_cannon::IonCannonShot>,
            With<crate::enemy::EnemyProjectile>,
        )>,
    >,
    particles: Query<Entity, With<crate::particles::Particle>>,
    ore_pickups: Query<Entity, With<crate::mining::OrePickup>>,
    player_ui: Query<
        Entity,
        Or<(
            With<crate::player::rendering::HealthBarBg>,
            With<crate::player::rendering::HealthBarFill>,
            With<crate::player::rendering::AimIndicatorMesh>,
        )>,
    >,
    mut player_ui_res: ResMut<crate::player::PlayerUiEntities>,
    mut score: ResMut<PlayerScore>,
    mut lives: ResMut<PlayerLives>,
    mut sim_stats: ResMut<crate::simulation::SimulationStats>,
    mut ore: ResMut<crate::mining::PlayerOre>,
    mut ammo: ResMut<crate::player::MissileAmmo>,
    mut enemy_spawn: ResMut<crate::enemy::EnemySpawnState>,
) {
    if *mode != SelectedGameMode::Campaign {
        return;
    }

    for e in asteroids
        .iter()
        .chain(players_and_enemies.iter())
        .chain(projectiles.iter())
        .chain(particles.iter())
        .chain(ore_pickups.iter())
        .chain(player_ui.iter())
    {
        commands.entity(e).despawn();
    }

    *player_ui_res = crate::player::PlayerUiEntities::default();
    *score = PlayerScore::default();
    lives.reset();
    *sim_stats = crate::simulation::SimulationStats::default();
    *ore = crate::mining::PlayerOre::default();
    *ammo = crate::player::MissileAmmo::default();
    *enemy_spawn = crate::enemy::EnemySpawnState::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::EnemySpawnState;
    use crate::mining::PlayerOre;
    use crate::player::{MissileAmmo, PlayerLives, PlayerScore, PlayerUiEntities};
    use crate::simulation::SimulationStats;

    #[test]
    fn campaign_retry_reset_clears_runtime_state_and_entities() {
        let mut world = World::new();
        world.insert_resource(SelectedGameMode::Campaign);
        world.insert_resource(PlayerScore {
            hits: 7,
            destroyed: 4,
            streak: 3,
            points: 250,
        });
        world.insert_resource(PlayerLives {
            remaining: 1,
            respawn_timer: Some(0.5),
        });
        world.insert_resource(SimulationStats {
            live_count: 10,
            culled_total: 2,
            merged_total: 3,
            split_total: 1,
            destroyed_total: 5,
        });
        world.insert_resource(PlayerOre { count: 42 });
        world.insert_resource(MissileAmmo { count: 1 });
        world.insert_resource(EnemySpawnState {
            timer_secs: 1.0,
            session_elapsed_secs: 99.0,
            total_spawned: 77,
        });
        world.insert_resource(PlayerUiEntities::default());

        world.spawn(crate::asteroid::Asteroid);
        world.spawn(crate::player::Player);
        world.spawn(crate::enemy::Enemy);
        world.spawn(crate::player::state::Projectile::default());
        world.spawn(crate::player::state::Missile::default());
        world.spawn(crate::player::ion_cannon::IonCannonShot {
            age: 0.0,
            distance_traveled: 0.0,
        });
        world.spawn(crate::particles::Particle {
            velocity: Vec2::ZERO,
            age: 0.0,
            lifetime: 1.0,
            r: 1.0,
            g: 1.0,
            b: 1.0,
            material: None,
        });
        world.spawn(crate::mining::OrePickup);
        let health_bar_bg = world.spawn(crate::player::rendering::HealthBarBg).id();
        let health_bar_fill = world
            .spawn(crate::player::rendering::HealthBarFill(Handle::<
                ColorMaterial,
            >::default(
            )))
            .id();
        let aim_indicator = world.spawn(crate::player::rendering::AimIndicatorMesh).id();

        {
            let mut ui = world.resource_mut::<PlayerUiEntities>();
            ui.health_bar_bg = Some(health_bar_bg);
            ui.health_bar_fill = Some(health_bar_fill);
            ui.aim_indicator = Some(aim_indicator);
        }

        let mut schedule = Schedule::default();
        schedule.add_systems(reset_campaign_retry_world);
        schedule.run(&mut world);

        let score = world.resource::<PlayerScore>();
        assert_eq!(score.points, 0);
        assert_eq!(score.hits, 0);

        let lives = world.resource::<PlayerLives>();
        assert_eq!(lives.respawn_timer, None);

        let ore = world.resource::<PlayerOre>();
        assert_eq!(ore.count, 0);

        let ammo = world.resource::<MissileAmmo>();
        assert_eq!(ammo.count, MissileAmmo::default().count);

        let spawn = world.resource::<EnemySpawnState>();
        assert_eq!(spawn.total_spawned, 0);

        let stats = world.resource::<SimulationStats>();
        assert_eq!(stats.culled_total, 0);
        assert_eq!(stats.destroyed_total, 0);

        let ui = world.resource::<PlayerUiEntities>();
        assert!(ui.health_bar_bg.is_none());
        assert!(ui.health_bar_fill.is_none());
        assert!(ui.aim_indicator.is_none());

        let asteroid_count = world
            .query_filtered::<Entity, With<crate::asteroid::Asteroid>>()
            .iter(&world)
            .count();
        let player_count = world
            .query_filtered::<Entity, With<crate::player::Player>>()
            .iter(&world)
            .count();
        let enemy_count = world
            .query_filtered::<Entity, With<crate::enemy::Enemy>>()
            .iter(&world)
            .count();
        let projectile_count = world
            .query_filtered::<Entity, With<crate::player::state::Projectile>>()
            .iter(&world)
            .count();
        let missile_count = world
            .query_filtered::<Entity, With<crate::player::state::Missile>>()
            .iter(&world)
            .count();
        let ion_count = world
            .query_filtered::<Entity, With<crate::player::ion_cannon::IonCannonShot>>()
            .iter(&world)
            .count();
        let particle_count = world
            .query_filtered::<Entity, With<crate::particles::Particle>>()
            .iter(&world)
            .count();
        let ore_pickup_count = world
            .query_filtered::<Entity, With<crate::mining::OrePickup>>()
            .iter(&world)
            .count();
        let ui_bg_count = world
            .query_filtered::<Entity, With<crate::player::rendering::HealthBarBg>>()
            .iter(&world)
            .count();
        let ui_fill_count = world
            .query_filtered::<Entity, With<crate::player::rendering::HealthBarFill>>()
            .iter(&world)
            .count();
        let ui_aim_count = world
            .query_filtered::<Entity, With<crate::player::rendering::AimIndicatorMesh>>()
            .iter(&world)
            .count();

        assert_eq!(asteroid_count, 0);
        assert_eq!(player_count, 0);
        assert_eq!(enemy_count, 0);
        assert_eq!(projectile_count, 0);
        assert_eq!(missile_count, 0);
        assert_eq!(ion_count, 0);
        assert_eq!(particle_count, 0);
        assert_eq!(ore_pickup_count, 0);
        assert_eq!(ui_bg_count, 0);
        assert_eq!(ui_fill_count, 0);
        assert_eq!(ui_aim_count, 0);
    }
}
