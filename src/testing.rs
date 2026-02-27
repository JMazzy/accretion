//! Testing utilities for the simulation

#[path = "testing/scenarios_core.rs"]
mod scenarios_core;
#[path = "testing/scenarios_orbit.rs"]
mod scenarios_orbit;
#[path = "testing/scenarios_performance.rs"]
mod scenarios_performance;
#[path = "testing/scripted_enemy_combat.rs"]
mod scripted_enemy_combat;
#[path = "testing/types.rs"]
mod types;
#[path = "testing/verification.rs"]
mod verification;

pub use scenarios_core::{
    spawn_test_culling_verification, spawn_test_gentle_approach, spawn_test_gravity,
    spawn_test_gravity_boundary, spawn_test_high_speed_collision, spawn_test_large_small_pair,
    spawn_test_mixed_size_asteroids, spawn_test_near_miss, spawn_test_passing_asteroid,
    spawn_test_three_triangles, spawn_test_two_triangles,
};
pub use scenarios_orbit::{orbit_pair_calibrate_and_track_system, spawn_test_orbit_pair};
pub use scenarios_performance::{
    spawn_test_all_three, spawn_test_baseline_100, spawn_test_kdtree_only,
    spawn_test_perf_benchmark, spawn_test_soft_boundary_only, spawn_test_tidal_only,
};
pub use scripted_enemy_combat::{
    enemy_combat_observer_system, enemy_combat_script_system, spawn_test_enemy_combat_scripted,
};
pub use types::{
    EnemyCombatObservations, EnemyCombatScriptState, OrbitCentralBody, OrbitTestBody,
    ScriptAsteroidTarget, ScriptEnemyTarget, TestConfig,
};
pub use verification::{test_logging_system, test_verification_system};
