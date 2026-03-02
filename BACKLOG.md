# Accretion Backlog

Planned features, improvements, and known limitations. Completed items are removed; see [FEATURES.md](FEATURES.md) and [CHANGELOG.md](CHANGELOG.md) for implemented history.

Last updated: March 1, 2026.

## Planning Notes

- Priority order: **P0 (next)** → **P1 (after P0)** → **P2 (longer horizon)**.
- Dependency notation: `depends on ...` indicates blocked tasks.
- Scope guidance: each checkbox should be shippable in one focused implementation cycle with tests/docs updates.

## P0 — Next Implementation Candidates

### Visual Features

- [X] **Concave deformation: damage model + rendering** ✅ Complete (March 1, 2026)
	- Crater-based deformation with local edge subdivision and inward crater shaping.
	- Repeated non-lethal hits add crater detail instead of flattening/removing vertices.
	- Acceptance: repeated non-lethal hits produce stable crater-like dents without collapsing to triangles.
	- Implementation: asteroid visuals now regenerate from `BaseVertices` + accumulated `CraterData`; impacts add craters (position/depth/radius) with bounded count, then rebuild deformed local vertices.

- [X] **Concave deformation: collider/physics strategy** ✅ Complete (March 1, 2026)
	- Option A strategy implemented: visual concavity with stable convex physics collider.
	- Collider derives from `BaseVertices` (undeformed convex hull), while render mesh uses crater-deformed `Vertices`.
	- Acceptance: deformed asteroids remain physically stable/performance-safe with no decomposition overhead.
	- Implementation: removed decomposition path for deformation handling; convex hull colliders now stay stable while crater visuals update independently.

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