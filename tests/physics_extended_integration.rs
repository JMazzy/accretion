//! Extended integration tests that execute non-core ACCRETION_TEST scenarios via
//! the real binary.
//!
//! These are intentionally `#[ignore]` because they are slower/heavier and are
//! meant for targeted validation when related systems change.

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
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("artifacts")
            .join("test_logs")
            .join("integration_extended")
            .join(format!("run_{timestamp}"));
        fs::create_dir_all(&root).expect("failed to create extended integration log directory");
        root
    })
}

fn tail_lines(input: &str, count: usize) -> String {
    let lines: Vec<&str> = input.lines().collect();
    let start = lines.len().saturating_sub(count);
    lines[start..].join("\n")
}

fn run_scenario_and_assert_pass(scenario: &str, expected_fragment: &str, timeout_secs: u32) {
    let _guard = RUN_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let command =
        format!("timeout {timeout_secs} env ACCRETION_TEST={scenario} cargo run --release 2>&1");

    let output = Command::new("bash")
        .arg("-lc")
        .arg(command)
        .output()
        .expect("failed to execute scenario command");

    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(&output.stdout));
    combined.push_str(&String::from_utf8_lossy(&output.stderr));

    let log_path = log_root().join(format!("{scenario}.log"));
    fs::write(&log_path, &combined).expect("failed writing extended scenario log");

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
            "scenario '{scenario}' timed out ({timeout_secs}s). log={}\n{}",
            log_path.display(),
            tail_lines(&combined, 120)
        );
    }

    let marker = marker.unwrap_or_else(|| {
        panic!(
            "scenario '{scenario}' produced no PASS/FAIL marker. exit_code={status_code} log={}\n{}",
            log_path.display(),
            tail_lines(&combined, 120)
        )
    });

    assert!(
        marker.contains("✓ PASS"),
        "scenario '{scenario}' failed: {marker}. log={}\n{}",
        log_path.display(),
        tail_lines(&combined, 150)
    );

    assert!(
        marker.contains(expected_fragment),
        "scenario '{scenario}' PASS marker did not contain expected fragment '{expected_fragment}'. marker='{marker}'. log={}",
        log_path.display()
    );
}

#[test]
#[ignore = "slow integration: runs release binary extended scenario"]
fn scenario_orbit_pair() {
    run_scenario_and_assert_pass("orbit_pair", "orbit_pair — orbit stable", 180);
}

#[test]
#[ignore = "slow integration: runs release binary extended scenario"]
fn scenario_enemy_combat_scripted() {
    run_scenario_and_assert_pass(
        "enemy_combat_scripted",
        "enemy_combat_scripted — scripted runtime collision contracts observed",
        180,
    );
}

#[test]
#[ignore = "slow integration: runs release binary extended scenario"]
fn scenario_baseline_100() {
    run_scenario_and_assert_pass("baseline_100", "baseline_100 complete", 180);
}

#[test]
#[ignore = "slow integration: runs release binary extended scenario"]
fn scenario_mixed_content_225_enemy8() {
    run_scenario_and_assert_pass(
        "mixed_content_225_enemy8",
        "mixed_content_225_enemy8 complete",
        240,
    );
}
