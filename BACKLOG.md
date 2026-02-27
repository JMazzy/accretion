# Accretion Backlog

Planned features, improvements, and known limitations. Completed items are removed; see [FEATURES.md](FEATURES.md) and [CHANGELOG.md](CHANGELOG.md) for implemented history.

Last updated: February 26, 2026.

## Planning Notes

- Priority order: **P0 (next)** → **P1 (after P0)** → **P2 (longer horizon)**.
- Dependency notation: `depends on ...` indicates blocked tasks.
- Scope guidance: each checkbox should be shippable in one focused implementation cycle with tests/docs updates.

## P0 — Next Implementation Candidates

### Gameplay Foundation (Combat + AI)

- [ ] **Ion Cannon MVP**
	- Add stun status effect for enemies only; level scaling controls minimum enemy tier affected.
	- Add upgrade progression and HUD feedback for stun duration/effectiveness.
	- Acceptance: cannon applies temporary disable correctly across enemy tiers.

## P1 — After Core Combat Lands

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

- [ ] **Concave deformation: damage model + rendering**
	- Per-vertex damage accumulation and inward displacement model.
	- Visual crack/deformation feedback linked to impact intensity.
	- Acceptance: repeated non-lethal hits visibly deform asteroid silhouettes.

- [ ] **Concave deformation: collider/physics strategy** `depends on Concave deformation: damage model + rendering`
	- Decide and implement safe collider approximation strategy (convex decomposition or fallback hull).
	- Validate performance and contact stability.
	- Acceptance: deformed asteroids remain physically stable and performant.

- [ ] **Post-processing: collision bloom pass**
	- Add bloom trigger/intensity mapping for high-energy collisions.
	- Acceptance: visible bloom on major impacts without overwhelming scene readability.

- [ ] **Post-processing: invincibility aberration pass**
	- Add chromatic aberration only during player invincibility windows.
	- Acceptance: effect is temporally bounded and clearly communicates invulnerability state.

### Multiplayer

- [ ] **Local multiplayer: shared-world co-op MVP**
	- Two player entities, independent input mappings, shared asteroid world.
	- Basic camera and HUD strategy for dual-player readability.
	- Acceptance: two local players can play simultaneously without control conflicts.

- [ ] **Local multiplayer: PvP ruleset** `depends on Local multiplayer: shared-world co-op MVP`
	- Friendly-fire, scoring, and win-condition rule variants.
	- Acceptance: a complete PvP match loop can start, progress, and end cleanly.

## P2 — Developer Quality and Maintainability

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

- [ ] **Performance pass v1 (guided by profiler)**
	- Use profiler overlay + benchmark scenarios to identify top 1–2 hot systems.
	- Implement highest-impact optimization and log measurable before/after metrics.
	- Acceptance: documented performance improvement in target scenario(s).

### Platform Maintenance

- [ ] **Bevy upgrade path planning (0.18+)**
	- Capture migration risk list (API breaks, Rapier compatibility, schedule changes).
	- Define stepwise branch plan with rollback points.
	- Acceptance: written migration plan with test matrix and owner sequence.

- [ ] **Bevy upgrade execution (0.18+)** `depends on Bevy upgrade path planning (0.18+)`
	- Update dependencies, compile fixes, and behavioral parity validation.
	- Acceptance: passes `cargo check`, `cargo clippy -- -D warnings`, `cargo build --release`, and key runtime sanity checks.