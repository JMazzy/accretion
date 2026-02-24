# Accretion Backlog

Planned features, improvements, and known limitations. Mark items as `[x]` when complete.

## Base Gameplay

- [x] **Main menu / splash screen** — Settings, start game
- [x] **Pause + in-game menu** — ESC pauses and shows menu (replaces debug options menu)
- [x] **Player respawn** — lives system, respawn mechanic, healing damage system
- [x] **Custom scenarios** - Scenarios & Saves screen added with two built-in scenarios: "Field" and "Orbit"
- [x] **Quit to main menu** - quit option on pause menu goes back to the main menu rather than quitting the game
- [ ] **Save/load system** — Save/load persistent game files
- [x] **More/better scenarios** - "Orbit" object size more variable (rings 2+3 now spawn mixed polygons with per-body orbital velocities), "Comets" scenario (20 large fast-moving boulders), "Shower" scenario (250 unit triangles, near-zero velocity)
- [ ] **Asteroid mining** — Ore drops, currency system, ship upgrades
- [x] **Score system enhancements** — Point multipliers (e.g. by hit streak without misses)
- [x] **New Name** - Both "particle" and "grav-sim" were placeholder names from early in development. The game is now named **Accretion**.

## Physics

- [x] **Density** - There should be a predictable relationship between mass and the size of the object on screen. Use a *density* value to make volume (or more accurately *area* since it's 2d) look more consistent when calculating the size of the created polygons based on its mass. Density can vary based on the asteroid. Invariant `vertices.area == AsteroidSize / density` now enforced at every spawn site; asteroids no longer visually resize on first hit.
- [ ] **Planets** - A new object type with the same gravity system as asteroids.Important differences - large, high mass, nearly circular, no merging or splitting, shooting does not increase score, fixed in place relative to the simulation area to give a steady frame of reference. Visually distinct (rendered purple as a placeholder). Update "Orbit" scenario to use a planet instead of the current planetoid.
- [ ] **Concave asteroid deformation** — Asteroid shapes are currently limited to be convex, but it would look better if allowed to be concave; concave craters approximated by hull; Track per-vertex damage; move vertices inward and recompute hull
- [ ] **Performance** — Determine next steps for performance improvements; implement the most impactful one and add the rest to the backlog.

## Combat Gameplay

- [x] **Secondary weapon** — missiles (limited shots, fragments asteroids, destroys larger asteroids (size <= 3) right away for more points)
- [ ] **Enemy ships** — Computer-controlled enemies
- [ ] **Boss ships** — Computer-controlled boss encounters (depends on having enemy ships first)
- [ ] **Special Weapons** — tractor (grab/pull/push asteroids), ion cannon (disable enemy ships, depends on having enemy ships first)
- [ ] **Local multiplayer** — Co-op and PvP modes

## Visual & Rendering

- [ ] **Remove Gizmos** - Remove remaining usage of Gizmos in favor of `Mesh2d` for everything
- [x] **Particle effects** — Impact dust, merge vortex, debris trails
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