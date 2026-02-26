# Accretion Backlog

Planned features, improvements, and known limitations. Completed items are removed; see [FEATURES.md](FEATURES.md) and [CHANGELOG.md](CHANGELOG.md) for implemented history.

Last updated: February 25, 2026.

## Essential Features

- [ ] **Enemy ships** — Computer-controlled enemies which fire their own weapon and take damage from player weapons and asteroid collisions. Should have multiple "levels" which equate to health and attack power.
- [ ] **Boss ships** — Large enemies with more powerful attacks (depends on having enemy ships first)
- [ ] **Tractor Beam** — A new weapon/action which can grab, pull, and push asteroids. Strength/upgrade leveling is based on the strength of the pull (i.e. higher levels are effective against larger and faster moving asteroids)
- [ ] **Ion Cannon** — A new weapon which disables or stuns enemy ships (depends on having enemy ships first). Strength/upgrade leveling increases minimum size effected (level 1 only effects level 1 enemies, level 2 effects level 1-2 enemies, etc.), as well as length of time lower level enemies are stunned.

## Enhancements

- [ ] **Remove Gizmos** — Remove remaining usage of Gizmos in favor of `Mesh2d` for everything
- [ ] **Concave asteroid deformation** — Asteroid shapes are currently limited to be convex; track per-vertex damage; move vertices inward and recompute hull
- [ ] **Planets** — A new object type with the same gravity system as asteroids, but otherwise very different. Important differences: larger, higher mass, nearly circular, no merging or splitting, shooting does not increase score, fixed in place relative to the simulation area to give a steady frame of reference. Visually distinct (rendered purple as a placeholder). Scenarios have 0–1 planet (not common). Update "Orbit" scenario to use a planet instead of the current planetoid.
- [ ] **Performance** — Determine next steps for performance improvements; implement the most impactful one and add the rest to the backlog.
- [ ] **Local multiplayer** — Co-op and PvP modes
- [ ] **Post-processing** — Bloom on collisions; chromatic aberration on invincibility frames

## Developer Tooling

- [ ] **Profiler integration** — Frame-time breakdown (physics, rendering, ECS)
- [ ] **Replay/playback system** — Record and replay sessions
- [ ] **Debug grid visualization** — Draw spatial partition cells
- [ ] **Golden test baselines** — Frame-log snapshots in `tests/golden/`; diff on runs

## Upgrade Bevy versions When Available

- [ ] **Bevy upgrade path** — Currently on Bevy 0.17 + bevy_rapier2d 0.32; 0.18+ will require migration