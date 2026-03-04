# Accretion Backlog

Planned features, improvements, and known limitations. Completed items are removed; see [FEATURES.md](FEATURES.md) and [CHANGELOG.md](CHANGELOG.md) for implemented history.

Last updated: March 4, 2026.

## Planning Notes

- Priority order: **P0 (next)** → **P1 (after P0)** → **P2 (longer horizon)**.
- Dependency notation: `depends on ...` indicates blocked tasks.
- Scope guidance: each checkbox should be shippable in one focused implementation cycle with tests/docs updates.

## P0 — Next Implementation Candidates

### Upgrade Enhancements

- [ ] **Between-mission upgrade/shop flow for campaign**
    - In campaign mode, add intermission upgrade step where ore is spent before the next mission starts.
    - This replaces any-time ore shop access in campaign mode.
    - Preserve current any-time ore shop behavior in practice mode.
    - Acceptance: campaign enforces between-mission upgrade cadence; practice remains immediate-access.

- [ ] **Campaign-scoped upgrade persistence** `depends on Between-mission upgrade/shop flow for campaign`
    - Persist weapon levels and selected loadout per campaign save slot.
    - Ensure no progression leaks across different campaign slots.
    - Acceptance: upgrades/loadout carry across missions in the same slot and reset for new slots.

### Boss Progression

- [ ] **Boss ships: framework**
    - Add boss entity type, health model, and weak-point/damage-gating foundation.
    - Add intro/outro flow and baseline reward integration.
    - Acceptance: one boss can spawn and be defeated end-to-end in a campaign mission.

- [ ] **Boss ships: attack pattern set** `depends on Boss ships: framework`
    - Add at least two readable phases with telegraphed behavior changes.
    - Balance projectile density and survivability.
    - Acceptance: boss fight shows clear phase transitions and no soft-locks.

- [ ] **Mission end boss-gate integration** `depends on Boss ships: attack pattern set; depends on Campaign-scoped upgrade persistence`
    - Make boss defeat the final gate for mission completion.
    - Integrate rewards and mission transition on boss death.
    - Acceptance: mission cannot complete before boss defeat; post-boss progression advances cleanly.

## P1 — Next Queue

### Enemy Enhancements

- [ ] **Enemy HUD health bars**
    - Add compact world-space health bars for enemies, style-aligned with player readability goals.
    - Acceptance: enemy remaining HP is visible and updates correctly under damage.

### Upgrade Enhancements

- [ ] **Split primary upgrade tracks (chip vs destroy threshold)**
    - Decouple primary progression so chip power and destroy threshold can be tuned independently.
    - Keep existing balance defaults via migration mapping for old saves.
    - Acceptance: two independently upgradable stats exist and are reflected in combat behavior.

- [ ] **Primary fire-rate upgrade track** `depends on Split primary upgrade tracks (chip vs destroy threshold)`
    - Add fire-rate (inverse cooldown) as a separate upgradeable stat.
    - Ensure fire-rate scaling integrates with existing HUD, shop costs, and save schema.
    - Acceptance: fire-rate can be upgraded independently and affects runtime cooldown behavior.

- [ ] **Primary weapon roster framework (blaster/mining laser/plasma rifle)** `depends on Split primary upgrade tracks (chip vs destroy threshold); depends on Primary fire-rate upgrade track`
    - Introduce primary-weapon type abstraction and selection wiring.
    - Keep `blaster` behavior as baseline reference implementation.
    - Acceptance: loadout/system can route behavior by primary type with no regression for blaster.

- [ ] **Mining laser implementation** `depends on Primary weapon roster framework (blaster/mining laser/plasma rifle)`
    - Add ore-focused weapon behavior:
        - chips two small asteroids per shot,
        - chip/destroy scaling tracks like blaster,
        - lower enemy-ship damage,
        - slightly faster baseline cooldown,
        - red-orange, longer, thinner projectile visuals.
    - Acceptance: mining laser is functionally and visually distinct, with ore-leaning tradeoffs.

- [ ] **Plasma rifle implementation** `depends on Primary weapon roster framework (blaster/mining laser/plasma rifle)`
    - Add combat-focused weapon behavior:
        - chip-size scaling retained,
        - destroy threshold fixed to unit-size ore conversion only,
        - sub-chip asteroids fragment into multiple unit asteroids,
        - higher enemy-ship damage,
        - slightly slower baseline cooldown,
        - yellow-green, shorter, wider projectile visuals with light flight particles.
    - Acceptance: plasma rifle is functionally and visually distinct, with combat-leaning tradeoffs.

- [ ] **Primary weapon DPS normalization pass** `depends on Mining laser implementation; depends on Plasma rifle implementation`
    - Add internal mining/combat/overall DPS comparisons across primary types and levels.
    - Keep same-level overall DPS roughly comparable while preserving role tradeoffs.
    - Ensure DPS scales with upgrades.
    - Acceptance: weapon advantages/disadvantages remain clear without one type dominating all scenarios.

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