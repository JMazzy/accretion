//! Integration tests that execute ACCRETION_TEST scenarios via the real binary.
//!
//! These tests are marked `#[ignore]` because they are slower than unit tests
//! and spin up the full app in release mode. Run specific scenarios with:
//! `cargo test --test physics_scenarios_integration scenario_two_triangles -- --ignored --nocapture --test-threads=1`

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static RUN_LOCK: Mutex<()> = Mutex::new(());
static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

fn log_root() -> &'static PathBuf {
    LOG_DIR.get_or_init(|| {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("artifacts")
            .join("test_logs")
            .join("integration")
            .join(format!("run_{timestamp}"));
        fs::create_dir_all(&root).expect("failed to create integration test log directory");
        root
    })
}

fn tail_lines(input: &str, count: usize) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let start = lines.len().saturating_sub(count);
    lines[start..].join("\n")
}

fn run_scenario_and_assert_pass(scenario: &str, expected_fragment: &str) {
    let _guard = RUN_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let command = format!("timeout 120 env ACCRETION_TEST={scenario} cargo run --release 2>&1");

    let output = Command::new("bash")
        .arg("-lc")
        .arg(command)
        .output()
        .expect("failed to execute scenario command");

    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(&output.stdout));
    combined.push_str(&String::from_utf8_lossy(&output.stderr));

    let log_path = log_root().join(format!("{scenario}.log"));
    fs::write(&log_path, &combined).expect("failed writing scenario log");

    let marker = combined
        .lines()
        .filter(|line| {
            line.contains("✓ PASS")
                || line.contains("✗ FAIL")
                || line.contains("PASS:")
                || line.contains("FAIL:")
        })
        .next_back();

    let status_code = output.status.code().unwrap_or(-1);

    if status_code == 124 {
        panic!(
            "scenario '{scenario}' timed out (120s). log={}\n{}",
            log_path.display(),
            tail_lines(&combined, 80)
        );
    }

    let marker = marker.unwrap_or_else(|| {
        panic!(
            "scenario '{scenario}' produced no PASS/FAIL marker. exit_code={status_code} log={}\n{}",
            log_path.display(),
            tail_lines(&combined, 100)
        )
    });

    assert!(
        marker.contains("✓ PASS"),
        "scenario '{scenario}' failed: {marker}. log={}\n{}",
        log_path.display(),
        tail_lines(&combined, 120)
    );

    assert!(
        marker.contains(expected_fragment),
        "scenario '{scenario}' PASS marker did not contain expected fragment '{expected_fragment}'. marker='{marker}'. log={}",
        log_path.display()
    );
}

#[test]
#[ignore = "slow integration: runs release binary scenario"]
fn scenario_two_triangles() {
    run_scenario_and_assert_pass("two_triangles", "Two triangles combined");
}

#[test]
#[ignore = "slow integration: runs release binary scenario"]
fn scenario_three_triangles() {
    run_scenario_and_assert_pass("three_triangles", "Three triangles combined");
}

#[test]
#[ignore = "slow integration: runs release binary scenario"]
fn scenario_gentle_approach() {
    run_scenario_and_assert_pass("gentle_approach", "Asteroids merged cleanly via gravity");
}

#[test]
#[ignore = "slow integration: runs release binary scenario"]
fn scenario_high_speed_collision() {
    run_scenario_and_assert_pass("high_speed_collision", "PASS");
}

#[test]
#[ignore = "slow integration: runs release binary scenario"]
fn scenario_near_miss() {
    run_scenario_and_assert_pass("near_miss", "passed each other without merging");
}

#[test]
#[ignore = "slow integration: runs release binary scenario"]
fn scenario_gravity() {
    run_scenario_and_assert_pass("gravity", "Asteroids interacted");
}

#[test]
#[ignore = "slow integration: runs release binary scenario"]
fn scenario_culling_verification() {
    run_scenario_and_assert_pass("culling_verification", "One asteroid was culled");
}

#[test]
#[ignore = "slow integration: runs release binary scenario"]
fn scenario_large_small_pair() {
    run_scenario_and_assert_pass("large_small_pair", "Large+small interaction stable");
}

#[test]
#[ignore = "slow integration: runs release binary scenario"]
fn scenario_gravity_boundary() {
    run_scenario_and_assert_pass("gravity_boundary", "Asteroids remained separate");
}

#[test]
#[ignore = "slow integration: runs release binary scenario"]
fn scenario_mixed_size_asteroids() {
    run_scenario_and_assert_pass("mixed_size_asteroids", "All 5 asteroids present");
}
