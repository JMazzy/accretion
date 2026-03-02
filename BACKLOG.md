# Accretion Backlog

Planned features, improvements, and known limitations. Completed items are removed; see [FEATURES.md](FEATURES.md) and [CHANGELOG.md](CHANGELOG.md) for implemented history.

Last updated: March 2, 2026.

## Planning Notes

- Priority order: **P0 (next)** → **P1 (after P0)** → **P2 (longer horizon)**.
- Dependency notation: `depends on ...` indicates blocked tasks.
- Scope guidance: each checkbox should be shippable in one focused implementation cycle with tests/docs updates.

## Immediate Execution Plan (Foundation Sprint)

Goal: deliver a playable campaign foundation (progression + win/fail loop) while completing the next visual quality step for initial asteroid generation.

### Track A — Campaign Foundation (MVP)

- [x] **A1: Mode/state scaffolding (`Campaign` vs `Practice`)**
	- Add explicit mode/state model so current scenario loop runs as `Practice` and new flow can run as `Campaign`.
	- Wire menu entry points and state transitions without changing existing practice behavior.
	- Likely touchpoints: `src/menu/`, `src/main.rs`, `src/menu/types.rs`, `FEATURES.md`.
	- Acceptance: both modes launch from menu; practice regression tests/behavior remain intact.
	- Concrete implementation plan:
		1. Add `SelectedGameMode` resource (`Practice`/`Campaign`) with sane default.
		2. Add menu buttons for `Campaign` and `Practice` with explicit routing.
		3. Set `SelectedGameMode` at all entry points (`main menu`, `scenario select`, `load game`).
		4. Make world-spawn transition logic mode-aware (`Campaign` path isolated from `Practice` path).
		5. Add/adjust transition wiring for `MainMenu -> Playing` (campaign bootstrap).
		6. Validate no regression in existing practice/load workflows.

- [x] **A2: Mission data model + runtime resource** `depends on A1`
	- Introduce mission descriptor types (mission id, map/scenario source, wave count, reward, next mission id).
	- Add campaign runtime resource tracking active mission, wave index, and mission state.
	- Likely touchpoints: `src/test_mode.rs` or new `src/campaign.rs`, `src/main.rs`, `ARCHITECTURE.md`.
	- Acceptance: campaign boot can load mission-1 definition deterministically from data.

- [x] **A3: Wave director v1 (spawn/break/escalation loop)** `depends on A2`
	- Implement minimal wave state machine: `Warmup -> ActiveWave -> InterWaveBreak -> Complete`.
	- Connect to enemy spawning controls so each wave raises pressure (count/tier/cadence).
	- Likely touchpoints: `src/enemy.rs`, new `src/campaign.rs`, `src/simulation.rs`.
	- Acceptance: mission runs 3+ waves with clear breaks and increasing difficulty.

- [x] **A4: Mission win/fail + progression transitions** `depends on A3`
	- Win: all waves cleared (boss-gate can be layered later).
	- Fail: player death/lives exhaustion exits mission with deterministic state reset/continue options.
	- Likely touchpoints: `src/menu/`, `src/player/state.rs`, `src/campaign.rs`.
	- Acceptance: end-of-mission transitions advance to next mission or fail screen correctly.

- [x] **A5: Campaign save slots v1 (name + mission progress)** `depends on A4`
	- Add campaign slot schema and persistence for slot name, mission index, and campaign progression state.
	- Keep slot data isolated from practice saves.
	- Likely touchpoints: `src/save.rs`, `saves/`, `src/menu/load_game.rs`, `src/menu/main_menu.rs`.
	- Acceptance: create/load/resume campaign slot with persistent mission progression.

- [x] **A6: Campaign foundation validation + docs update** `depends on A5`
	- Add targeted tests for mission state transitions and save/load roundtrip.
	- Update docs for mode split and campaign loop semantics.
	- Acceptance: `cargo check`, `cargo clippy -- -D warnings`, `cargo test` pass; `ARCHITECTURE.md`/`FEATURES.md`/`CHANGELOG.md` updated.

### Track B — Procedural Shape Pass (spawn-time visual quality)

- [ ] **B1: Spawn-shape tuning config (runtime knobs)**
	- Add config fields for spawn irregularity controls (radial jitter, edge subdivision chance, noise frequency/amplitude bounds).
	- Load from `assets/physics.toml` with sensible defaults/fallbacks.
	- Likely touchpoints: `src/constants.rs`, `src/config.rs`, `assets/physics.toml`, `ARCHITECTURE.md`.
	- Acceptance: spawn-shape behavior is fully tuneable without recompilation.
	- Concrete implementation plan:
		1. Add new constants for spawn-shape knobs in `src/constants.rs`.
		2. Mirror knobs into `PhysicsConfig` + `Default` mapping in `src/config.rs`.
		3. Add the knobs to `assets/physics.toml` with comments and initial values.
		4. Wire at least one existing spawn-shape path to consume new knobs (initial jitter pass).
		5. Validate config hot-reload/parse path still works with new fields.
		6. Follow with B2 to apply knobs across full procedural shape pipeline.

- [ ] **B2: Deterministic irregular polygon generation** `depends on B1`
	- Extend asteroid spawn geometry generation with deterministic per-asteroid noise/jitter (seeded RNG path already used in scenarios).
	- Preserve local-space vertex assumptions and area/mass normalization.
	- Likely touchpoints: `src/asteroid.rs`.
	- Acceptance: initial asteroids display broader silhouettes while preserving mass-density invariants.

- [ ] **B3: Scenario coverage + guardrails** `depends on B2`
	- Apply shape pass consistently to field/orbit/comets/shower and any direct spawn helper paths.
	- Maintain collider stability via existing Option A strategy (stable physics collider path).
	- Acceptance: all major spawn paths produce crater-ready, irregular initial shapes with no collider fallbacks spike.

- [ ] **B4: Regression checks + performance sanity** `depends on B3`
	- Add/extend tests for generated polygon validity (>=3 verts, finite coords, positive area, convex-hull collider success where required).
	- Do quick perf sanity at representative spawn counts to avoid excessive vertex growth.
	- Acceptance: tests pass and no obvious frame-time regression in baseline scenarios.

### Recommended execution order

- [ ] **Run B1 → B2 in parallel with A1 → A2, then finish A3 → A6, and B3 → B4 before boss/loadout expansions**
	- Rationale: unlock campaign core loop quickly while visual spawn improvements land early and stabilize before broader content scaling.

## P0 — Next Implementation Candidates

### Visual Features

- [ ] **Procedural asteroid shape pass (spawn-time noise) for all scenarios**
	- Extend spawn generators to produce richer irregular silhouettes from the start (not only through combat damage).
	- Keep physics stable by continuing to use the established collider strategy.
	- Acceptance: fresh spawns show visibly broader shape variety without collider regressions.

### Mission Progression

- [ ] **Campaign mode framework + mode split**
	- Add `Campaign` as the primary progression mode and relabel current scenario flow as `Practice`.
	- Route menu and runtime state transitions so both modes are selectable and isolated.
	- Acceptance: player can launch either mode from menu; practice behavior remains unchanged.

- [ ] **Campaign mission definition + map binding** `depends on Campaign mode framework + mode split`
	- Define mission descriptor format (mission id, map/scenario seed, wave profile, rewards, next mission).
	- Reuse existing scenario generators as initial mission-map backends.
	- Acceptance: campaign can start mission 1 from data and load the correct map profile.

- [ ] **Wave director with inter-wave downtime** `depends on Campaign mission definition + map binding`
	- Add wave scheduler with clear states: warmup → active wave → break → next wave.
	- Scale enemy count/tier/composition per wave for increasing challenge.
	- Acceptance: one mission runs multiple escalating waves with timed breaks for ore collection.

- [ ] **Mission completion + failure + progression rules** `depends on Wave director with inter-wave downtime`
	- Win condition: all configured waves (and mission boss, when present) are defeated.
	- Failure condition: standard player death/lives rules terminate mission attempt.
	- Acceptance: mission end transitions to next mission or mission-failed screen with deterministic outcomes.

- [ ] **Campaign save slots + naming + resume** `depends on Mission completion + failure + progression rules`
	- Add dedicated campaign slots with user-provided save names and persisted mission index/progression state.
	- Keep campaign progression isolated per slot (no global cross-save carryover).
	- Acceptance: player can create, rename, save, and resume campaign runs from the menu.

### Upgrade Enhancements

- [ ] **Campaign loadout selection (primary/secondary/tool)** `depends on Campaign mode framework + mode split`
	- Add pre-mission loadout selection: one primary, one secondary, one tool.
	- Initial supported set: primary `blaster`; secondary `missile`/`ion cannon`; tool `ore magnet` (tractor beam TBD).
	- Acceptance: selected loadout is visible in HUD/state and applied during mission runtime.

- [ ] **Between-mission upgrade/shop flow for campaign** `depends on Mission completion + failure + progression rules`
	- Add intermission upgrade step where ore can be spent before next mission starts.
	- Preserve current any-time ore shop behavior in practice mode.
	- Acceptance: campaign enforces between-mission upgrade cadence; practice remains immediate-access.

- [ ] **Campaign-scoped upgrade persistence** `depends on Campaign save slots + naming + resume`
	- Persist weapon/tool levels and selected loadout per campaign save slot.
	- Ensure no upgrade progression leaks across different saves.
	- Acceptance: upgrades carry across missions in same campaign slot and reset for new slots.

- [ ] **Split blaster upgrade tracks (chip size vs destroy threshold)**
	- Decouple current blaster progression so chip power and destroy threshold can be tuned independently.
	- Keep existing balance defaults via migration mapping for old saves.
	- Acceptance: two independently upgradable stats exist and are reflected in combat behavior.

### Boss Progression

- [ ] **Boss ships: framework**
	- Boss entity type, health pool, weak-point/damage gating model.
	- Intro/outro flow and baseline reward integration.
	- Acceptance: one boss can spawn and be defeated end-to-end within a campaign mission.

- [ ] **Boss ships: attack pattern set** `depends on Boss ships: framework`
	- Multi-phase behavior (at least two phases) with readable telegraphs.
	- Balance pass for projectile density and survivability.
	- Acceptance: boss fight has distinct phase transitions and no soft-locks.

- [ ] **Mission end boss-gate integration** `depends on Boss ships: attack pattern set; depends on Mission completion + failure + progression rules`
	- Make boss defeat the final gate for mission completion.
	- Integrate rewards/transition to next mission on boss death.
	- Acceptance: mission cannot complete until boss is defeated; defeat cleanly advances progression.

### Enemy Enhancements

- [ ] **Enemy scaling model pass (campaign-aware levels)** `depends on Wave director with inter-wave downtime`
	- Tie enemy stat/tier scaling to mission and wave progression curves.
	- Acceptance: measurable difficulty increase across mission waves without spike regressions.

- [ ] **Enemy HUD health bars**
	- Add compact world-space health bars for enemies, style-aligned with player readability goals.
	- Acceptance: enemy remaining HP is visible and updates correctly under damage.

- [ ] **Enemy variety set: silhouettes + movement + attack patterns**
	- Expand enemy roster with additional hull shapes and at least one new movement and attack behavior.
	- Acceptance: wave composition includes at least two tactically distinct enemy archetypes.

- [ ] **Enemy formation behavior** `depends on Enemy variety set: silhouettes + movement + attack patterns`
	- Add formation-capable enemy group behavior (spawn + maintain + break conditions).
	- Acceptance: at least one wave spawns enemies in a stable formation pattern.

- [ ] **Enemy ore-drop scaling by level**
	- Award ore on enemy defeat based on enemy tier/mission context.
	- Acceptance: higher-tier enemies drop more ore and drops are reflected in campaign economy.

## P1 — Next Queue

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