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
        Or<(With<crate::player::Player>, With<crate::enemy::Enemy>)>,
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
    commands.insert_resource(PrimaryWeaponLevel::default());
    commands.insert_resource(SecondaryWeaponLevel::default());
    commands.insert_resource(OreAffinityLevel::default());
    commands.insert_resource(TractorBeamLevel::default());
    // Keep the physics pipeline disabled until a new session begins.
    // resume_physics is called on OnTransition { ScenarioSelect → Playing }.
    for mut cfg in rapier_config.iter_mut() {
        cfg.physics_pipeline_active = false;
    }
}
