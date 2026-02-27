use bevy::prelude::*;

/// Test configuration
#[derive(Resource)]
pub struct TestConfig {
    pub enabled: bool,
    pub test_name: String,
    pub frame_limit: u32,
    pub frame_count: u32,
    pub initial_asteroid_count: usize,
    /// Per-frame delta times (seconds) recorded for perf_benchmark test
    pub perf_frame_times: Vec<f32>,
    /// For orbit_pair test: set to true once the orbiting body's velocity has been
    /// calibrated from the actual Rapier mass read back by [`ReadMassProperties`].
    pub velocity_calibrated: bool,
    /// For orbit_pair test: orbital radius (world units) recorded after calibration.
    pub orbit_initial_dist: f32,
    /// For orbit_pair test: most-recent orbital radius, updated each frame.
    pub orbit_final_dist: f32,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            test_name: String::new(),
            frame_limit: 100,
            frame_count: 0,
            initial_asteroid_count: 0,
            perf_frame_times: Vec::new(),
            velocity_calibrated: false,
            orbit_initial_dist: 0.0,
            orbit_final_dist: 0.0,
        }
    }
}

/// Tags the large central body in the `orbit_pair` test scenario.
#[derive(Component)]
pub struct OrbitCentralBody;

/// Tags the small orbiting body in the `orbit_pair` test scenario.
#[derive(Component)]
pub struct OrbitTestBody;

/// Marks the single deterministic enemy used by the scripted enemy combat test.
#[derive(Component)]
pub struct ScriptEnemyTarget;

/// Marks the deterministic asteroid target used by the scripted enemy combat test.
#[derive(Component)]
pub struct ScriptAsteroidTarget;

/// Internal state machine for scripted enemy combat playback.
#[derive(Resource, Default)]
pub struct EnemyCombatScriptState {
    pub player_shot_spawned: bool,
    pub enemy_shot_player_spawned: bool,
    pub enemy_shot_asteroid_spawned: bool,
}

/// One-way observation flags for scripted enemy combat verification.
#[derive(Resource, Default)]
pub struct EnemyCombatObservations {
    pub enemy_damage_observed: bool,
    pub player_damage_observed: bool,
    pub asteroid_hit_observed: bool,
    pub particles_observed: bool,
    pub enemy_damage_first_frame: Option<u32>,
    pub player_damage_first_frame: Option<u32>,
    pub asteroid_hit_first_frame: Option<u32>,
    pub particles_first_frame: Option<u32>,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct TestMarker(pub usize); // Initial index for tracking
