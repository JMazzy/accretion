# GRAV-SIM Backlog

Planned features, improvements, and known limitations. Mark items as `[x]` when complete.

## Gameplay

- [x] **Main menu / splash screen** — Settings, start game
- [x] **Pause + in-game menu** — ESC pauses and shows menu (replaces debug options menu)
- [ ] **Player respawn** — lives system, respawn mechanic, healing damage system
- [ ] **Save/load system** — Save games, load, snapshots, custom scenarios
- [ ] **Asteroid mining** — Ore drops, currency system, ship upgrades
- [ ] **Multiple weapons** — Slicing laser (slice in half), ablative laser (quickly chip off pieces), missiles (multi-split), tractor (grab/pull/push)
- [ ] **Enemy ships** — Computer-controlled enemies
- [ ] **Boss ships** — Computer-controlled boss encounters
- [ ] **Weapon** — ion cannon (disable enemy ships)
- [ ] **Score system enhancements** — Point multipliers (e.g. by hit streak without misses)
- [ ] **Local multiplayer** — Co-op and PvP modes
- [ ] **New Name** - Both "particle" and "grav-sim" were placeholder names from early in development. The game needs a proper name. Actual name TBD.

## Physics

- [ ] **Concave asteroid deformation** — Asteroid shapes are currently limited to be convex, but it would look better if allowed to be concave; concave craters approximated by hull; Track per-vertex damage; move vertices inward and recompute hull
- [ ] **Multi-frame contact resolution** — Large simultaneous merges may need multiple frames

## Visual & Rendering

- [ ] **Particle effects** — Impact dust, merge vortex, debris trails
- [ ] **Post-processing** — Bloom on collisions; chromatic aberration on invincibility frames

## Developer Tooling

- [ ] **Hot-reload constants** — Watch `assets/physics.toml`; apply changes instantly
- [ ] **Physics inspector overlay** — Show entity IDs, velocities, contacts; toggle in-game
- [ ] **Profiler integration** — Frame-time breakdown (physics, rendering, ECS)
- [ ] **Replay/playback system** — Record and replay sessions
- [ ] **Debug grid visualization** — Draw spatial partition cells
- [ ] **Golden test baselines** — Frame-log snapshots in `tests/golden/`; diff on runs

## Upgrade Bevy versions When Available

- [ ] **Bevy upgrade path** — Currently on Bevy 0.17 + bevy_rapier2d 0.32; 0.18+ will require migration