# Accretion Backlog

Planned features, improvements, and known limitations. Completed items are removed; see [FEATURES.md](FEATURES.md) and [CHANGELOG.md](CHANGELOG.md) for implemented history.

Last updated: March 4, 2026.

## Planning Notes

- Priority order: **P0 (next)** → **P1 (after P0)** → **P2 (longer horizon)**.
- Dependency notation: `depends on ...` indicates blocked tasks.
- Scope guidance: each checkbox should be shippable in one focused implementation cycle with tests/docs updates.

## P0 — Next Implementation Candidates

- [ ] **Split primary: sub-chip fragmentation completion**
    - Finish the remaining split-primary behavior rule: if destroy track is too low to convert a sub-chip-sized asteroid directly to ore, fragment it into multiple unit asteroids.
    - Keep existing split-track progression and persistence behavior unchanged.
    - Acceptance: sub-chip edge cases follow the new fragmentation rule without regressions in normal chip/destroy flow.

- [ ] **Primary fire-rate upgrade track**
    - Add fire-rate (inverse cooldown) as a separate upgradeable stat.
    - Integrate fire-rate scaling with HUD, shop costs, and save schema.
    - Acceptance: fire-rate can be upgraded independently and changes runtime primary cooldown behavior.

- [ ] **Primary weapon roster framework (blaster/mining laser/plasma rifle) — foundation slice**
    - Introduce primary-weapon type abstraction and runtime routing while keeping `blaster` as baseline.
    - Keep behavior parity for existing blaster sessions.
    - Acceptance: loadout/system can route by primary type with no regression for blaster-only play.

## P1 — Next Queue

### Upgrade Enhancements

Priority order (high → low):

- [ ] **Primary weapon roster framework (blaster/mining laser/plasma rifle) — full integration** `depends on Primary weapon roster framework (blaster/mining laser/plasma rifle) — foundation slice; depends on Primary fire-rate upgrade track`
    - Complete selection wiring + full gameplay integration for primary type routing.
    - Preserve blaster parity while unlocking additional primary types.
    - Acceptance: weapon type routing is fully integrated across loadout, runtime systems, and persistence.

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

- [ ] **Performance pass v2 (post-v1 hardening + scale test)**
    - Re-run profiling after v1 optimizations and target the next bottleneck at higher scale (e.g., larger asteroid counts / heavier contact density).
    - Use [PERFORMANCE_V1_CLOSEOUT.md](PERFORMANCE_V1_CLOSEOUT.md) as the baseline reference for v2 comparisons.
    - Initial candidate from v1 closeout: reduce mixed-content allocation churn in formation/contact and projectile-heavy update paths.
    - Extend benchmark comparison table in docs with v1 vs v2 deltas.
    - Acceptance: second measurable frame-time improvement without stability regressions.

### Visual Features

- [ ] **Post-processing: collision bloom pass**
    - Add bloom trigger/intensity mapping for high-energy collisions.
    - Acceptance: visible bloom on major impacts without overwhelming scene readability.

- [ ] **Post-processing: invincibility aberration pass**
    - Add chromatic aberration only during player invincibility windows.
    - Acceptance: effect is temporally bounded and clearly communicates invulnerability state.

## P2 - Multiplayer

Priority order (high → low):

- [ ] **Local multiplayer: shared-world co-op MVP**
    - Two player entities, independent input mappings, shared asteroid world.
    - Basic camera and HUD strategy for dual-player readability.
    - Acceptance: two local players can play simultaneously without control conflicts.

- [ ] **Local multiplayer: PvP ruleset** `depends on Local multiplayer: shared-world co-op MVP`
    - Friendly-fire, scoring, and win-condition rule variants.
    - Acceptance: a complete PvP match loop can start, progress, and end cleanly.

- [ ] **Replay/playback: capture format + recorder**
    - Define compact session log schema (input + key state snapshots + metadata).
    - Write record pipeline with bounded memory/disk behavior.
    - Acceptance: a full session can be recorded to disk reproducibly.

- [ ] **Replay/playback: deterministic playback runner** `depends on Replay/playback: capture format + recorder`
    - Add playback mode that consumes recorded logs and drives simulation.
    - Acceptance: playback reaches expected end-state within tolerance on repeated runs.

- [ ] **Bevy upgrade path planning (0.18+)**
    - Capture migration risk list (API breaks, Rapier compatibility, schedule changes).
    - Define stepwise branch plan with rollback points.
    - Acceptance: written migration plan with test matrix and owner sequence.

- [ ] **Performance pass v3 (post-v2 hardening + scale test)**
    - Re-run profiling after v2 optimizations and target the next bottleneck at higher scale (e.g., larger asteroid counts / heavier contact density).
    - Extend benchmark comparison table in docs with v1 vs v2 vs v3 deltas.
    - Acceptance: measurable frame-time improvement without stability regressions.

- [ ] **Bevy upgrade execution (0.18+)** `depends on Bevy upgrade path planning (0.18+)`
    - Update dependencies, compile fixes, and behavioral parity validation.
    - Acceptance: passes `cargo check`, `cargo clippy -- -D warnings`, `cargo build --release`, and key runtime sanity checks.