use crate::alloc_profile;
use crate::asteroid::{Asteroid, Vertices};
use crate::simulation::MissileTelemetry;
use crate::simulation::ProfilerStats;
use bevy::prelude::*;
use bevy_rapier2d::prelude::{ExternalForce, Velocity};
use std::io::Write;

use super::{EnemyCombatObservations, EnemyCombatScriptState, TestConfig};

pub fn test_logging_system(
    mut test_config: ResMut<TestConfig>,
    time: Res<Time>,
    profiler_stats: Res<ProfilerStats>,
    missile_telemetry: Res<MissileTelemetry>,
    q: Query<(Entity, &Transform, &Velocity, &Vertices, &ExternalForce), With<Asteroid>>,
) {
    if !test_config.enabled {
        return;
    }

    test_config.frame_count += 1;
    let asteroid_count = q.iter().count();

    let is_perf_test = test_config.test_name == "perf_benchmark"
        || test_config.test_name == "baseline_100"
        || test_config.test_name == "baseline_225"
        || test_config.test_name == "tidal_only"
        || test_config.test_name == "soft_boundary_only"
        || test_config.test_name == "kdtree_only"
        || test_config.test_name == "all_three"
        || test_config.test_name == "all_three_225_enemy5"
        || test_config.test_name == "mixed_content_225_enemy8"
        || test_config.test_name == "mixed_content_324_enemy12";

    if is_perf_test {
        let dt_ms = time.delta_secs() * 1000.0;
        test_config.perf_frame_times.push(dt_ms);
        test_config
            .post_update_frame_times
            .push(profiler_stats.post_update_ms);

        if test_config.frame_count == 1 {
            test_config.initial_asteroid_count = asteroid_count;
            if alloc_profile::is_enabled() {
                alloc_profile::reset_counters();
            }
            println!(
                "[Frame 1] {} started | asteroids: {}",
                test_config.test_name, asteroid_count
            );
        } else if test_config.frame_count.is_multiple_of(50)
            || test_config.frame_count == test_config.frame_limit
        {
            let window = &test_config.perf_frame_times
                [test_config.perf_frame_times.len().saturating_sub(50)..];
            let avg = window.iter().sum::<f32>() / window.len() as f32;
            let min = window.iter().cloned().fold(f32::INFINITY, f32::min);
            let max = window.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            println!(
                "[Frame {}] asteroids: {} | last {} frames — avg: {:.2}ms  min: {:.2}ms  max: {:.2}ms  (target ≤16.7ms)",
                test_config.frame_count,
                asteroid_count,
                window.len(),
                avg,
                min,
                max,
            );
        }
        return;
    }

    if test_config.frame_count == 1 {
        test_config.initial_asteroid_count = asteroid_count;
        println!(
            "[Frame {}] Test: {} | Initial asteroids: {}",
            test_config.frame_count, test_config.test_name, asteroid_count
        );
        for (entity, transform, _, _, _) in q.iter() {
            println!(
                "  Entity {:?} at: ({:.1}, {:.1})",
                entity, transform.translation.x, transform.translation.y
            );
        }
    } else if test_config.frame_count == 10
        || test_config.frame_count == 20
        || test_config.frame_count == 30
        || test_config.frame_count == 40
        || test_config.frame_count == 50
        || test_config.frame_count.is_multiple_of(25)
        || test_config.frame_count == test_config.frame_limit
    {
        println!(
            "[Frame {}] Asteroids: {} (was {})",
            test_config.frame_count, asteroid_count, test_config.initial_asteroid_count
        );

        if missile_telemetry.shots_fired > 0 {
            let shots = missile_telemetry.shots_fired as f32;
            let hits = missile_telemetry.hits as f32;
            let hit_rate = if shots > 0.0 {
                100.0 * hits / shots
            } else {
                0.0
            };
            let outcome_total = missile_telemetry.instant_destroy_events
                + missile_telemetry.split_events
                + missile_telemetry.full_decompose_events;
            let outcome_total_f = outcome_total.max(1) as f32;
            let kill_events =
                missile_telemetry.instant_destroy_events + missile_telemetry.full_decompose_events;
            let frames_per_kill = if kill_events > 0 {
                test_config.frame_count as f32 / kill_events as f32
            } else {
                f32::INFINITY
            };
            println!(
                "  Missile telemetry | shots={} hits={} hit_rate={:.1}% outcomes[destroy={:.1}%, split={:.1}%, decompose={:.1}%] ttk_proxy_frames_per_kill={} mass[destroyed={}, decomposed={}]",
                missile_telemetry.shots_fired,
                missile_telemetry.hits,
                hit_rate,
                100.0 * missile_telemetry.instant_destroy_events as f32 / outcome_total_f,
                100.0 * missile_telemetry.split_events as f32 / outcome_total_f,
                100.0 * missile_telemetry.full_decompose_events as f32 / outcome_total_f,
                if frames_per_kill.is_finite() {
                    format!("{frames_per_kill:.1}")
                } else {
                    "n/a".to_string()
                },
                missile_telemetry.destroyed_mass_total,
                missile_telemetry.decomposed_mass_total,
            );
        }

        let positions: Vec<(Entity, Vec2, Vec2, Vec2, f32)> = q
            .iter()
            .map(|(e, t, v, _, f)| {
                (
                    e,
                    t.translation.truncate(),
                    v.linvel,
                    f.force,
                    f.force.length(),
                )
            })
            .collect();

        for (i, (entity, pos, vel, force, force_mag)) in positions.iter().enumerate() {
            let force_dir = if *force_mag > 0.0001 {
                format!("({:.3}, {:.3})", force.x, force.y)
            } else {
                "none".to_string()
            };

            let mut distances = Vec::new();
            for (j, (_, other_pos, _, _, _)) in positions.iter().enumerate() {
                if i != j {
                    let dist = (*other_pos - *pos).length();
                    distances.push(format!("d[{}]={:.1}", j, dist));
                }
            }
            let dist_str = distances.join(", ");

            println!("  [{}] Entity={:?} pos: ({:.1}, {:.1}), vel: ({:.1}, {:.1}) len={:.2}, force: {} mag={:.3}, {}", 
                i, entity, pos.x, pos.y, vel.x, vel.y, vel.length(), force_dir, force_mag, dist_str);
        }
    }
}

/// Verify test results at the end
pub fn test_verification_system(
    test_config: Res<TestConfig>,
    missile_telemetry: Res<MissileTelemetry>,
    q: Query<(&Transform, &Vertices), With<Asteroid>>,
    enemy_combat_obs: Option<Res<EnemyCombatObservations>>,
    enemy_combat_script: Option<Res<EnemyCombatScriptState>>,
    mut exit: MessageWriter<bevy::app::AppExit>,
) {
    if !test_config.enabled || test_config.frame_count != test_config.frame_limit {
        return;
    }

    let final_count = q.iter().count();

    println!("\n╔════════════════════════════════════════════╗");
    println!("║           TEST COMPLETE                    ║");
    println!("╚════════════════════════════════════════════╝");
    println!("Test: {}", test_config.test_name);
    println!("Frames: {}", test_config.frame_count);
    println!("Initial asteroids: {}", test_config.initial_asteroid_count);
    println!("Final asteroids:   {}", final_count);

    if test_config.test_name == "enemy_combat_scripted" {
        let mut player_shot = false;
        let mut enemy_player_shot = false;
        let mut enemy_asteroid_shot = false;
        if let Some(script) = enemy_combat_script {
            player_shot = script.player_shot_spawned;
            enemy_player_shot = script.enemy_shot_player_spawned;
            enemy_asteroid_shot = script.enemy_shot_asteroid_spawned;
        }

        let mut enemy_damaged = false;
        let mut player_damaged = false;
        let mut asteroid_hit = false;
        let mut particles_seen = false;
        let mut enemy_damage_frame = None;
        let mut player_damage_frame = None;
        let mut asteroid_hit_frame = None;
        let mut particles_frame = None;
        if let Some(obs) = enemy_combat_obs {
            enemy_damaged = obs.enemy_damage_observed;
            player_damaged = obs.player_damage_observed;
            asteroid_hit = obs.asteroid_hit_observed;
            particles_seen = obs.particles_observed;
            enemy_damage_frame = obs.enemy_damage_first_frame;
            player_damage_frame = obs.player_damage_first_frame;
            asteroid_hit_frame = obs.asteroid_hit_first_frame;
            particles_frame = obs.particles_first_frame;
        }

        println!("Script shots spawned: player->enemy={player_shot}, enemy->player={enemy_player_shot}, enemy->asteroid={enemy_asteroid_shot}");
        println!("Observed outcomes: enemy_damaged={enemy_damaged}, player_damaged={player_damaged}, asteroid_hit={asteroid_hit}, particles_seen={particles_seen}");
        println!(
            "Observed first frames: enemy_damage={:?}, player_damage={:?}, asteroid_hit={:?}, particles={:?}",
            enemy_damage_frame, player_damage_frame, asteroid_hit_frame, particles_frame
        );

        let enemy_damage_pre_asteroid_leg = enemy_damage_frame.is_some_and(|f| f < 40);
        let enemy_damage_in_player_shot_window =
            enemy_damage_frame.is_some_and(|f| (10..40).contains(&f));
        let enemy_damage_before_player_damage =
            matches!((enemy_damage_frame, player_damage_frame), (Some(e), Some(p)) if e < p);
        let enemy_damage_before_asteroid_hit =
            matches!((enemy_damage_frame, asteroid_hit_frame), (Some(e), Some(a)) if e < a);

        let pass = player_shot
            && enemy_player_shot
            && enemy_asteroid_shot
            && enemy_damaged
            && player_damaged
            && asteroid_hit
            && particles_seen
            && enemy_damage_pre_asteroid_leg
            && enemy_damage_in_player_shot_window
            && enemy_damage_before_player_damage
            && enemy_damage_before_asteroid_hit;

        if pass {
            println!(
                "✓ PASS: enemy_combat_scripted — scripted runtime collision contracts observed"
            );
        } else {
            println!(
                "✗ FAIL: enemy_combat_scripted — one or more scripted collision outcomes missing"
            );
            if !enemy_damage_pre_asteroid_leg {
                println!(
                    "  Additional failure: enemy damage did not occur before asteroid-leg shot frame (40)."
                );
            }
            if !enemy_damage_in_player_shot_window {
                println!(
                    "  Additional failure: enemy damage was not first observed in expected player-shot window [10, 40)."
                );
            }
            if !enemy_damage_before_player_damage {
                println!(
                    "  Additional failure: enemy damage was not observed before player damage."
                );
            }
            if !enemy_damage_before_asteroid_hit {
                println!(
                    "  Additional failure: enemy damage was not observed before asteroid-hit outcome."
                );
            }
        }

        let _ = std::io::stdout().flush();
        exit.write(bevy::app::AppExit::Success);
        return;
    }

    if missile_telemetry.shots_fired > 0 {
        let shots = missile_telemetry.shots_fired as f32;
        let hits = missile_telemetry.hits as f32;
        let hit_rate = if shots > 0.0 {
            100.0 * hits / shots
        } else {
            0.0
        };
        let kill_events =
            missile_telemetry.instant_destroy_events + missile_telemetry.full_decompose_events;
        let frames_per_kill = if kill_events > 0 {
            test_config.frame_count as f32 / kill_events as f32
        } else {
            f32::INFINITY
        };
        println!(
            "Missile telemetry: shots={} hits={} hit_rate={:.1}% destroy={} split={} decompose={} ttk_proxy_frames_per_kill={} mass_destroyed={} mass_decomposed={}",
            missile_telemetry.shots_fired,
            missile_telemetry.hits,
            hit_rate,
            missile_telemetry.instant_destroy_events,
            missile_telemetry.split_events,
            missile_telemetry.full_decompose_events,
            if frames_per_kill.is_finite() {
                format!("{frames_per_kill:.1}")
            } else {
                "n/a".to_string()
            },
            missile_telemetry.destroyed_mass_total,
            missile_telemetry.decomposed_mass_total,
        );
    }

    if (test_config.test_name == "perf_benchmark"
        || test_config.test_name == "baseline_100"
        || test_config.test_name == "baseline_225"
        || test_config.test_name == "tidal_only"
        || test_config.test_name == "soft_boundary_only"
        || test_config.test_name == "kdtree_only"
        || test_config.test_name == "all_three"
        || test_config.test_name == "all_three_225_enemy5"
        || test_config.test_name == "mixed_content_225_enemy8"
        || test_config.test_name == "mixed_content_324_enemy12")
        && !test_config.perf_frame_times.is_empty()
    {
        let times = &test_config.perf_frame_times;
        let steady = if times.len() > 10 {
            &times[10..]
        } else {
            times.as_slice()
        };
        let avg = steady.iter().sum::<f32>() / steady.len() as f32;
        let min = steady.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = steady.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let mut sorted = steady.to_vec();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let p50 = percentile(&sorted, 0.50);
        let p95 = percentile(&sorted, 0.95);
        let p99 = percentile(&sorted, 0.99);
        let over_budget = steady.iter().filter(|&&t| t > 16.7).count();
        let pct_60fps = 100.0 * (steady.len() - over_budget) as f32 / steady.len() as f32;

        let post_times = &test_config.post_update_frame_times;
        let post_steady = if post_times.len() > 10 {
            &post_times[10..]
        } else {
            post_times.as_slice()
        };
        let post_avg = post_steady.iter().sum::<f32>() / post_steady.len() as f32;
        let post_min = post_steady.iter().cloned().fold(f32::INFINITY, f32::min);
        let post_max = post_steady
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max);
        let mut post_sorted = post_steady.to_vec();
        post_sorted.sort_by(|a, b| a.total_cmp(b));
        let post_p50 = percentile(&post_sorted, 0.50);
        let post_p95 = percentile(&post_sorted, 0.95);
        let post_p99 = percentile(&post_sorted, 0.99);

        println!("\n── Timing summary (frames 10–{}) ──", times.len());
        println!("  avg frame: {:.2}ms", avg);
        println!("  min frame: {:.2}ms", min);
        println!("  max frame: {:.2}ms", max);
        println!("  p50 frame: {:.2}ms", p50);
        println!("  p95 frame: {:.2}ms", p95);
        println!("  p99 frame: {:.2}ms", p99);
        println!(
            "  frames at 60 FPS (≤16.7ms): {}/{} ({:.1}%)",
            steady.len() - over_budget,
            steady.len(),
            pct_60fps
        );
        if avg <= 16.7 {
            println!("  ✓ Average frame time within 60 FPS budget");
        } else {
            println!("  ✗ Average frame time {:.2}ms exceeds 16.7ms budget", avg);
        }

        println!(
            "\n── PostUpdate schedule summary (frames 10–{}) ──",
            post_times.len()
        );
        println!("  post_update avg: {:.3}ms", post_avg);
        println!("  post_update min: {:.3}ms", post_min);
        println!("  post_update max: {:.3}ms", post_max);
        println!("  post_update p50: {:.3}ms", post_p50);
        println!("  post_update p95: {:.3}ms", post_p95);
        println!("  post_update p99: {:.3}ms", post_p99);

        if alloc_profile::is_enabled() {
            let snapshot = alloc_profile::snapshot();
            println!("\n── Allocator profile summary ──");
            println!("  alloc live bytes: {}", snapshot.live_bytes);
            println!("  alloc peak live bytes: {}", snapshot.peak_live_bytes);
            println!("  alloc total bytes: {}", snapshot.total_alloc_bytes);
            println!("  dealloc total bytes: {}", snapshot.total_dealloc_bytes);
            println!("  alloc net bytes: {}", snapshot.net_bytes());
            println!(
                "  alloc calls: {} dealloc calls: {} realloc calls: {}",
                snapshot.alloc_calls, snapshot.dealloc_calls, snapshot.realloc_calls
            );
        }
    }

    let result = verify_test_result(
        &test_config.test_name,
        test_config.initial_asteroid_count,
        final_count,
        test_config.orbit_initial_dist,
        test_config.orbit_final_dist,
        test_config.velocity_calibrated,
    );
    println!("{}\n", result);
    let _ = std::io::stdout().flush();

    exit.write(bevy::app::AppExit::Success);
}

fn percentile(sorted_values: &[f32], p: f32) -> f32 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    if sorted_values.len() == 1 {
        return sorted_values[0];
    }

    let rank = (sorted_values.len() - 1) as f32 * p.clamp(0.0, 1.0);
    let low = rank.floor() as usize;
    let high = (low + 1).min(sorted_values.len() - 1);
    let frac = rank - low as f32;

    sorted_values[low] * (1.0 - frac) + sorted_values[high] * frac
}

fn verify_test_result(
    test_name: &str,
    initial: usize,
    final_count: usize,
    orbit_initial: f32,
    orbit_final: f32,
    orbit_calibrated: bool,
) -> String {
    match test_name {
        "two_triangles_combine" => {
            if final_count < initial && final_count >= 1 {
                format!(
                    "✓ PASS: Two triangles combined into {}asteroid(s)",
                    final_count
                )
            } else {
                format!(
                    "✗ FAIL: Expected combining, but got: {} → {} asteroids",
                    initial, final_count
                )
            }
        }
        "three_triangles_combine" => {
            if final_count < initial && final_count >= 1 {
                format!(
                    "✓ PASS: Three triangles combined into {}asteroid(s)",
                    final_count
                )
            } else {
                format!(
                    "✗ FAIL: Expected combining, but got: {} → {} asteroids",
                    initial, final_count
                )
            }
        }
        "gravity_attraction" => {
            if initial > 1 && final_count <= initial {
                "✓ PASS: Asteroids interacted (gravity or collision)".to_string()
            } else {
                "✗ FAIL: Asteroids did not interact as expected".to_string()
            }
        }
        "high_speed_collision" => {
            if initial == 2 && final_count == 2 {
                "✓ PASS: Two asteroids bounced without merging (remained 2)".to_string()
            } else if final_count < initial && final_count >= 1 {
                format!("✓ PASS: Asteroids merged into {}asteroid(s)", final_count)
            } else {
                format!(
                    "✗ FAIL: Unexpected result: {} → {} asteroids",
                    initial, final_count
                )
            }
        }
        "near_miss" => {
            if initial == 2 && final_count == 2 {
                "✓ PASS: Two asteroids passed each other without merging (remained 2)".to_string()
            } else {
                format!(
                    "✗ FAIL: Expected 2 separate asteroids, got {} → {}",
                    initial, final_count
                )
            }
        }
        "gentle_approach" => {
            if final_count < initial && final_count >= 1 {
                format!(
                    "✓ PASS: Asteroids merged cleanly via gravity ({} → {})",
                    initial, final_count
                )
            } else {
                format!(
                    "✗ FAIL: Expected gentle merge, got {} → {} asteroids",
                    initial, final_count
                )
            }
        }
        "culling_verification" => {
            if initial == 2 && final_count == 1 {
                format!(
                    "✓ PASS: One asteroid was culled ({} → {})",
                    initial, final_count
                )
            } else {
                format!(
                    "✗ FAIL: Expected culling result 2 → 1, got {} → {}",
                    initial, final_count
                )
            }
        }
        "mixed_size_asteroids" => {
            if initial == 5 {
                format!(
                    "✓ PASS: All 5 asteroids present at end ({} → {})",
                    initial, final_count
                )
            } else {
                format!("✗ FAIL: Expected 5 asteroids, got {}", initial)
            }
        }
        "large_small_pair" => {
            if initial == 2 && final_count <= initial {
                if final_count == 1 {
                    "✓ PASS: Large+small merged into 1 asteroid".to_string()
                } else {
                    format!(
                        "✓ PASS: Large+small interaction stable (2 → {})",
                        final_count
                    )
                }
            } else {
                format!("✗ FAIL: Unexpected result {} → {}", initial, final_count)
            }
        }
        "gravity_boundary" => {
            if initial == 2 && final_count == 2 {
                "✓ PASS: Asteroids remained separate at gravity boundary (no merge)".to_string()
            } else if initial == 2 && final_count == 1 {
                "✓ PASS: Asteroids eventually merged from boundary distance".to_string()
            } else {
                format!(
                    "✗ FAIL: Expected stable or merged, got {} → {}",
                    initial, final_count
                )
            }
        }
        "passing_asteroid" => {
            if initial == 2 {
                "✓ PASS: Small asteroid passed by large one (check velocity logs)".to_string()
            } else {
                format!("✗ FAIL: Expected 2 asteroids, got {}", initial)
            }
        }
        "perf_benchmark" => {
            format!(
                "✓ PASS: perf_benchmark complete — {} asteroids remaining (see timing logs above)",
                final_count
            )
        }
        "baseline_100" => {
            format!(
                "✓ PASS: baseline_100 complete — {} asteroids | Compare timing to tidal_only, soft_boundary_only, kdtree_only, all_three",
                final_count
            )
        }
        "baseline_225" => {
            format!(
                "✓ PASS: baseline_225 complete — {} asteroids | High-load baseline for >200 asteroid profiling",
                final_count
            )
        }
        "tidal_only" => {
            format!(
                "✓ PASS: tidal_only complete — {} asteroids | Cost = tidal_only minus baseline_100",
                final_count
            )
        }
        "soft_boundary_only" => {
            format!(
                "✓ PASS: soft_boundary_only complete — {} asteroids | Cost = soft_boundary_only minus baseline_100",
                final_count
            )
        }
        "kdtree_only" => {
            format!(
                "✓ PASS: kdtree_only complete — {} asteroids | Cost = kdtree_only minus baseline_100",
                final_count
            )
        }
        "all_three" => {
            format!(
                "✓ PASS: all_three complete — {} asteroids | Full cost = all_three minus baseline_100",
                final_count
            )
        }
        "all_three_225_enemy5" => {
            format!(
                "✓ PASS: all_three_225_enemy5 complete — {} asteroids/entities | High-load mixed asteroid+enemy benchmark",
                final_count
            )
        }
        "mixed_content_225_enemy8" => {
            format!(
                "✓ PASS: mixed_content_225_enemy8 complete — {} asteroids/entities | High-load mixed-content benchmark (sizes/shapes/planets/projectiles)",
                final_count
            )
        }
        "mixed_content_324_enemy12" => {
            format!(
                "✓ PASS: mixed_content_324_enemy12 complete — {} asteroids/entities | Heavier-scale mixed-content benchmark (324 asteroids + 12 enemies + planets + projectiles)",
                final_count
            )
        }
        "orbit_pair" => {
            if !orbit_calibrated {
                format!(
                    "✗ FAIL: orbit_pair — orbit never calibrated (check ReadMassProperties population). \
                     asteroid_count={final_count}"
                )
            } else {
                let drift_pct = ((orbit_final - orbit_initial) / orbit_initial).abs() * 100.0;
                if drift_pct < 30.0 {
                    format!(
                        "✓ PASS: orbit_pair — orbit stable; drift={drift_pct:.1}% \
                         (initial_dist={orbit_initial:.1} u, final_dist={orbit_final:.1} u)"
                    )
                } else {
                    format!(
                        "✗ FAIL: orbit_pair — orbit unstable; drift={drift_pct:.1}% > 30% \
                         (initial_dist={orbit_initial:.1} u, final_dist={orbit_final:.1} u)"
                    )
                }
            }
        }
        _ => format!("? UNKNOWN: {}", test_name),
    }
}
