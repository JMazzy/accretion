# Accretion Backlog

Planned features, improvements, and known limitations. Completed items are removed; see [FEATURES.md](FEATURES.md) and [CHANGELOG.md](CHANGELOG.md) for implemented history.

Last updated: February 27, 2026.

## Planning Notes

- Priority order: **P0 (next)** → **P1 (after P0)** → **P2 (longer horizon)**.
- Dependency notation: `depends on ...` indicates blocked tasks.
- Scope guidance: each checkbox should be shippable in one focused implementation cycle with tests/docs updates.

## P0 — Next Implementation Candidates

### Recommended Execution Sequence (second pass)

1. **Scenario baseline refresh: Field**
	- Execute `Scenario pass: Field refresh` first to establish new spawn/distribution baseline and seeded variability patterns reused by later scenario tasks.
2. **Scenario gravity anchor: Orbit**
	- Execute `Scenario pass: Orbit identity boost` second so stronger planetoid defaults are established before balancing controls/weapons around live gameplay feel.
3. **Outer-flow scenarios: Comet then Shower**
	- Execute `Scenario pass: Comet refresh`, then `Scenario pass: Shower redesign` using Comet flow as the template.
4. **Player control foundation (KB/Mouse)**
	- Execute `Control scheme overhaul: KB/Mouse strafe + cursor-facing ship` before gamepad parity and tractor fine-tuning.
5. **Gamepad parity pass**
	- Execute `Gamepad parity overhaul for all new mechanics` after KB/Mouse foundation is stable.
6. **Weapon aim + cone tuning**
	- Execute `Ion cannon retune + aim parity with primary weapon`, then `Tractor beam retune: 30° total narrow cone aligned to primary aim` to align with updated aiming/control model.
7. **Stabilization + verification checkpoint**
	- Run `cargo fmt`, `cargo clippy -- -D warnings`, `./test_all.sh`, plus scenario-focused manual checks after steps 1–6.
8. **Visual deformation track**
	- Execute `Concave deformation: damage model + rendering`, then `Concave deformation: collider/physics strategy` once gameplay/control/scenario tuning is locked.

### Content Improvements

- [x] **Scenario pass: Field refresh (remove planet + richer clustered distribution)**
	- Remove the small planet from `Field` scenario.
	- Replace single broad noise patch with finer-scale clustered distribution (multiple nearby patches).
	- Add per-run random seed so `Field` starts differ each time.
	- Increase variation in asteroid sizes, initial rotations, and initial speeds.
	- Acceptance: `Field` has no planet, shows multiple asteroid clusters, and exhibits clearly varied asteroid starts across repeated launches.
	- Initial plan:
		- Adjust Field scenario spawner in `src/asteroid.rs` to drop planet spawn and revise noise sampling scale.
		- Add seeded randomness source for scenario generation.
		- Tune spread ranges in `assets/physics.toml` / config constants where applicable.
		- Validate via repeated `Field` starts + test-mode sanity run.

- [x] **Scenario pass: Orbit identity boost (stronger well + noisier orbital starts)**
	- Keep planet in `Orbit`, but make planetoid objects larger and denser by default to strengthen gravity well.
	- Raise initial orbital speeds as needed to remain playable under stronger gravity.
	- Replace clean orbital bands with wider randomized ranges (size, rotation, orbital radius), while keeping the scene mostly stable.
	- Acceptance: `Orbit` is visibly distinct from `Field`, with stronger central attraction and more varied but still coherent orbital motion.
	- Initial plan:
		- Update planetoid defaults in `src/constants.rs` and `PhysicsConfig::default` in `src/config.rs`.
		- Retune orbit spawn velocities/radius jitter in Orbit scenario spawner.
		- Run orbit scenario manual checks for stability and gameplay readability.

- [x] **Scenario pass: Comet refresh (outer-start inward flow + larger average bodies)**
	- Increase size/shape variety for Comet scenario asteroids.
	- Spawn asteroids in/near soft-boundary region with gentle inward initial velocity.
	- Preserve existing comet concept, but bias to farther-out and larger-average asteroids than `Field`.
	- Acceptance: comets begin near outer region, accelerate inward naturally, and do not immediately eject out of bounds en masse.
	- Initial plan:
		- Rework Comet spawn radius/velocity envelopes in scenario spawner.
		- Add explicit average-size bias relative to Field defaults.
		- Verify early-frame trajectory behavior with logging in test mode.

- [x] **Scenario pass: Shower redesign (small-body outer shower)**
	- Rebase Shower to be Comet-like outer soft-boundary spawn pattern.
	- Bias toward many small asteroids (not uniform size) with inward-start behavior.
	- Acceptance: Shower feels like dense small-body inward rain and is behaviorally distinct from Comet by smaller average mass.
	- Initial plan:
		- Reuse Comet spawn flow with Shower-specific size distribution.
		- Tune spawn count/velocity caps for readability and performance.
		- Validate with scenario-specific test-mode run and quick perf sanity.

### Weapon Enhancements

- [x] **Ion cannon retune + aim parity with primary weapon**
	- Increase ion shot visual/collider size.
	- Reduce ion cannon cooldown.
	- Change ion firing direction source to match primary weapon aiming direction.
	- Acceptance: ion shot is larger, fires more frequently, and follows the same aim vector behavior as primary blaster.
	- Initial plan:
		- Update ion constants/config and fire system in `src/player/ion_cannon.rs`.
		- Ensure gamepad and mouse aim paths feed ion direction consistently.
		- Re-run enemy stun behavior sanity checks.

- [x] **Tractor beam retune: 30° total narrow cone aligned to primary aim**
	- Replace current tractor effective area with narrow 30° total cone.
	- Ensure cone is aligned to primary weapon aiming direction (not ship-forward fallback unless aim absent).
	- Acceptance: tractor acquisition only occurs inside 30° aim cone and tracks active aim direction consistently.
	- Initial plan:
		- Update cone-dot threshold derivation and aim source in `src/player/control.rs`.
		- Add/adjust tests for cone gating and aim alignment behavior.

### Player Control Enhancements

- [ ] **Control scheme overhaul: KB/Mouse strafe + cursor-facing ship**
	- Ship heading follows mouse cursor direction.
	- Keep `W/S` as forward/back thrust.
	- Change `A/D` to strafe left/right.
	- Keep blaster/missile/ion fire bindings unchanged.
	- Tractor behavior: `Q` toggles hold mode; while engaged, `E` pulls/holds (stop before ship collision), `R` throws outward and disengages.
	- Acceptance: KB/mouse controls match specified mapping and tractor interaction flow works end-to-end.
	- Initial plan:
		- Refactor `player/control` intent model to separate facing, thrust, and strafe axes.
		- Add tractor toggle state machine resource and pull/throw action systems.
		- Update HUD/help text and FEATURES docs for new controls.

- [ ] **Gamepad parity overhaul for all new mechanics** `depends on Control scheme overhaul: KB/Mouse strafe + cursor-facing ship`
	- Right stick controls facing direction.
	- Left stick controls omnidirectional strafe (lower authority than thrust).
	- `RT/LT` provide forward/reverse thrust (analog where available).
	- Buttons: `A` blaster, `B` missile, `Y` ion.
	- Tractor behavior: `X` toggles hold mode; while engaged, `LB` pulls/holds, `RB` throws and disengages.
	- Acceptance: gamepad supports all current weapon + tractor features with behavior parity to KB/mouse where intended.
	- Initial plan:
		- Expand gamepad input mapping in `src/player/control.rs` + combat systems.
		- Ensure analog triggers map to variable thrust in intent application.
		- Add scripted/gamepad-focused verification cases and manual controller sanity pass.

### Visual Features

- [ ] **Concave deformation: damage model + rendering**
	- Per-vertex damage accumulation and inward displacement model.
	- Visual crack/deformation feedback linked to impact intensity.
	- Acceptance: repeated non-lethal hits visibly deform asteroid silhouettes.

- [ ] **Concave deformation: collider/physics strategy** `depends on Concave deformation: damage model + rendering`
	- Decide and implement safe collider approximation strategy (convex decomposition or fallback hull).
	- Validate performance and contact stability.
	- Acceptance: deformed asteroids remain physically stable and performant.

## P1 — Next Queue

### Boss Progression

- [ ] **Boss ships: framework** `depends on Enemy ships: combat loop`
	- Boss entity type, health pool, weak-point/damage gating model.
	- Intro/outro flow and baseline reward integration.
	- Acceptance: one boss can spawn and be defeated end-to-end.

- [ ] **Boss ships: attack pattern set** `depends on Boss ships: framework`
	- Multi-phase behavior (at least two phases) with readable telegraphs.
	- Balance pass for projectile density and survivability.
	- Acceptance: boss fight has distinct phase transitions and no soft-locks.

### Visual Features

- [ ] **Post-processing: collision bloom pass**
	- Add bloom trigger/intensity mapping for high-energy collisions.
	- Acceptance: visible bloom on major impacts without overwhelming scene readability.

- [ ] **Post-processing: invincibility aberration pass**
	- Add chromatic aberration only during player invincibility windows.
	- Acceptance: effect is temporally bounded and clearly communicates invulnerability state.

### Performance Program

- [ ] **Performance pass v2 (post-v1 hardening + scale test)**
	- Re-run profiling after v1 optimizations and target the next bottleneck at higher scale (e.g., larger asteroid counts / heavier contact density).
	- Use [PERFORMANCE_V1_CLOSEOUT.md](PERFORMANCE_V1_CLOSEOUT.md) as the baseline reference for v2 comparisons.
	- Initial candidate from v1 closeout: reduce mixed-content allocation churn in formation/contact and projectile-heavy update paths.
	- Extend benchmark comparison table in docs with v1 vs v2 deltas.
	- Acceptance: second measurable frame-time improvement without stability regressions.

## P2 — Developer Quality and Maintainability

### Multiplayer

- [ ] **Local multiplayer: shared-world co-op MVP**
	- Two player entities, independent input mappings, shared asteroid world.
	- Basic camera and HUD strategy for dual-player readability.
	- Acceptance: two local players can play simultaneously without control conflicts.

- [ ] **Local multiplayer: PvP ruleset** `depends on Local multiplayer: shared-world co-op MVP`
	- Friendly-fire, scoring, and win-condition rule variants.
	- Acceptance: a complete PvP match loop can start, progress, and end cleanly.

### Tooling & Testing

- [ ] **Replay/playback: capture format + recorder**
	- Define compact session log schema (input + key state snapshots + metadata).
	- Write record pipeline with bounded memory/disk behavior.
	- Acceptance: a full session can be recorded to disk reproducibly.

- [ ] **Replay/playback: deterministic playback runner** `depends on Replay/playback: capture format + recorder`
	- Add playback mode that consumes recorded logs and drives simulation.
	- Acceptance: playback reaches expected end-state within tolerance on repeated runs.

- [ ] **Golden baselines: snapshot format + fixtures**
	- Standardize frame-log output format and create canonical fixtures under `tests/golden/`.
	- Acceptance: test harness can generate and load golden snapshots.

- [ ] **Golden baselines: diff + CI gate** `depends on Golden baselines: snapshot format + fixtures`
	- Add structured diff output (positions/velocities/counts) and CI failure conditions.
	- Acceptance: intentional physics changes require explicit golden update workflow.

### Performance Program

- [ ] **Performance pass v3 (post-v2 hardening + scale test)**
	- Re-run profiling after v2 optimizations and target the next bottleneck at higher scale (e.g., larger asteroid counts / heavier contact density).
	- Extend benchmark comparison table in docs with v1 vs v2 vs v3 deltas.
	- Acceptance: measurable frame-time improvement without stability regressions.

### Platform Maintenance

- [ ] **Bevy upgrade path planning (0.18+)**
	- Capture migration risk list (API breaks, Rapier compatibility, schedule changes).
	- Define stepwise branch plan with rollback points.
	- Acceptance: written migration plan with test matrix and owner sequence.

- [ ] **Bevy upgrade execution (0.18+)** `depends on Bevy upgrade path planning (0.18+)`
	- Update dependencies, compile fixes, and behavioral parity validation.
	- Acceptance: passes `cargo check`, `cargo clippy -- -D warnings`, `cargo build --release`, and key runtime sanity checks.