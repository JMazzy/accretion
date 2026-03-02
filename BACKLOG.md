# Accretion Backlog

Planned features, improvements, and known limitations. Completed items are removed; see [FEATURES.md](FEATURES.md) and [CHANGELOG.md](CHANGELOG.md) for implemented history.

Last updated: March 1, 2026.

## Planning Notes

- Priority order: **P0 (next)** → **P1 (after P0)** → **P2 (longer horizon)**.
- Dependency notation: `depends on ...` indicates blocked tasks.
- Scope guidance: each checkbox should be shippable in one focused implementation cycle with tests/docs updates.

## P0 — Next Implementation Candidates

### Enemy Enhancements

- [ ] **Enemy scaling model pass (campaign-aware levels)**
	- Tie enemy stat/tier scaling to mission and wave progression curves.
	- Acceptance: measurable difficulty increase across mission waves without spike regressions.

- [ ] **Enemy variety set: silhouettes + movement + attack patterns** `depends on Enemy scaling model pass (campaign-aware levels)`
	- Expand enemy roster with additional hull shapes and at least one new movement and attack behavior.
	- Acceptance: wave composition includes at least two tactically distinct enemy archetypes.

- [ ] **Enemy formation behavior** `depends on Enemy variety set: silhouettes + movement + attack patterns`
	- Add formation-capable enemy group behavior (spawn + maintain + break conditions).
	- Acceptance: at least one wave spawns enemies in a stable formation pattern.

- [ ] **Enemy ore-drop scaling by level** `depends on Enemy scaling model pass (campaign-aware levels)`
	- Award ore on enemy defeat based on enemy tier/mission context.
	- Acceptance: higher-tier enemies drop more ore and drops are reflected in campaign economy.

### Upgrade Enhancements

- [ ] **Campaign loadout selection (primary/secondary/tool)**
	- Add pre-mission loadout selection: one primary, one secondary, one tool.
	- Initial supported set: primary `blaster`; secondary `missile`/`ion cannon`; tool `ore magnet` (tractor beam TBD).
	- Acceptance: selected loadout is visible in HUD/state and applied during mission runtime.

- [ ] **Between-mission upgrade/shop flow for campaign** `depends on Campaign loadout selection (primary/secondary/tool)`
	- Add intermission upgrade step where ore can be spent before next mission starts.
	- Preserve current any-time ore shop behavior in practice mode.
	- Acceptance: campaign enforces between-mission upgrade cadence; practice remains immediate-access.

- [ ] **Campaign-scoped upgrade persistence** `depends on Between-mission upgrade/shop flow for campaign`
	- Persist weapon/tool levels and selected loadout per campaign save slot.
	- Ensure no upgrade progression leaks across different saves.
	- Acceptance: upgrades carry across missions in same campaign slot and reset for new slots.

### Boss Progression

- [ ] **Boss ships: framework** `depends on Enemy scaling model pass (campaign-aware levels)`
	- Boss entity type, health pool, weak-point/damage gating model.
	- Intro/outro flow and baseline reward integration.
	- Acceptance: one boss can spawn and be defeated end-to-end within a campaign mission.

- [ ] **Boss ships: attack pattern set** `depends on Boss ships: framework`
	- Multi-phase behavior (at least two phases) with readable telegraphs.
	- Balance pass for projectile density and survivability.
	- Acceptance: boss fight has distinct phase transitions and no soft-locks.

- [ ] **Mission end boss-gate integration** `depends on Boss ships: attack pattern set; depends on Campaign-scoped upgrade persistence`
	- Make boss defeat the final gate for mission completion.
	- Integrate rewards/transition to next mission on boss death.
	- Acceptance: mission cannot complete until boss is defeated; defeat cleanly advances progression.

## P1 — Next Queue

### Enemy Enhancements

- [ ] **Enemy HUD health bars**
	- Add compact world-space health bars for enemies, style-aligned with player readability goals.
	- Acceptance: enemy remaining HP is visible and updates correctly under damage.

### Upgrade Enhancements

- [ ] **Split blaster upgrade tracks (chip size vs destroy threshold)**
	- Decouple current blaster progression so chip power and destroy threshold can be tuned independently.
	- Keep existing balance defaults via migration mapping for old saves.
	- Acceptance: two independently upgradable stats exist and are reflected in combat behavior.

### Visual Features

- [ ] **Post-processing: collision bloom pass**
	- Add bloom trigger/intensity mapping for high-energy collisions.
	- Acceptance: visible bloom on major impacts without overwhelming scene readability.

- [ ] **Post-processing: invincibility aberration pass**
	- Add chromatic aberration only during player invincibility windows.
	- Acceptance: effect is temporally bounded and clearly communicates invulnerability state.

### Performance Program 2

- [ ] **Performance pass v2 (post-v1 hardening + scale test)**
	- Re-run profiling after v1 optimizations and target the next bottleneck at higher scale (e.g., larger asteroid counts / heavier contact density).
	- Use [PERFORMANCE_V1_CLOSEOUT.md](PERFORMANCE_V1_CLOSEOUT.md) as the baseline reference for v2 comparisons.
	- Initial candidate from v1 closeout: reduce mixed-content allocation churn in formation/contact and projectile-heavy update paths.
	- Extend benchmark comparison table in docs with v1 vs v2 deltas.
	- Acceptance: second measurable frame-time improvement without stability regressions.

## P2 - Multiplayer

### Multiplayer

- [ ] **Local multiplayer: shared-world co-op MVP**
	- Two player entities, independent input mappings, shared asteroid world.
	- Basic camera and HUD strategy for dual-player readability.
	- Acceptance: two local players can play simultaneously without control conflicts.

- [ ] **Local multiplayer: PvP ruleset** `depends on Local multiplayer: shared-world co-op MVP`
	- Friendly-fire, scoring, and win-condition rule variants.
	- Acceptance: a complete PvP match loop can start, progress, and end cleanly.

### Performance Program 3

- [ ] **Replay/playback: capture format + recorder**
	- Define compact session log schema (input + key state snapshots + metadata).
	- Write record pipeline with bounded memory/disk behavior.
	- Acceptance: a full session can be recorded to disk reproducibly.

- [ ] **Replay/playback: deterministic playback runner** `depends on Replay/playback: capture format + recorder`
	- Add playback mode that consumes recorded logs and drives simulation.
	- Acceptance: playback reaches expected end-state within tolerance on repeated runs.

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