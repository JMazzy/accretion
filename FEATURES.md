# Accretion Features & User Controls

## Runtime Controls

### Keyboard + Mouse (Twin-Stick)

| Input                       | Action                                                                         |
| --------------------------- | ------------------------------------------------------------------------------ |
| **W**                       | Thrust forward (ship-facing direction)                                         |
| **S**                       | Thrust backward                                                                |
| **A**                       | Rotate ship left                                                               |
| **D**                       | Rotate ship right                                                              |
| **Space** or **Left-click**  | Fire projectile toward mouse cursor (auto-repeats at cooldown rate while held) |
| **X** or **Right-click**     | Fire missile toward mouse cursor (limited ammo; single shot per press)         |
| **Mouse wheel**             | Zoom in / out                                                                  |
| **ESC**                     | Pause / resume simulation; opens in-game pause menu                            |
| **Pause menu Save buttons** | Save current run to slot 1/2/3                                                 |

- **Aiming is decoupled from movement**: the ship faces the direction you steer, but projectiles travel toward the mouse cursor regardless of ship heading.
- An **orange aim indicator** (line + dot) extends from the ship in the current fire direction. It is shown by default and can be hidden via the debug panel (*Aim Indicator* toggle).
- **Aim idle snap**: if no mouse movement, gamepad left stick, or right stick input is received for 1 second, the aim direction automatically resets to the ship's forward (+Y).

### Gamepad (Twin-Stick)

| Input           | Action                                                                 |
| --------------- | ---------------------------------------------------------------------- |
| **Left stick**  | Rotate ship toward stick direction at fixed speed, then thrust forward |
| **Right stick** | Aim and auto-fire projectiles in stick direction                       |
| **West button** | Fire missile in current aim direction (X on Xbox, Square on PS)       |
| **B button**    | Brake — applies strong velocity damping each frame while held          |

- **Left stick movement**: the ship rotates at a fixed angular speed until aligned with the stick direction, then applies forward thrust proportional to stick magnitude. Thrust is suppressed while rotating sharply (above the heading correction threshold in `src/constants.rs`) to avoid fighting the turn.
- **B button brake**: while held, multiplies both linear and angular velocity by `GAMEPAD_BRAKE_DAMPING` every frame, bringing the ship to a near-stop in roughly half a second at 60 fps. Forward thrust is independent and can still be applied simultaneously via the left stick.
- **Right stick auto-fire**: once the right stick exceeds `GAMEPAD_FIRE_THRESHOLD` deflection, projectiles auto-fire at the fire cooldown rate. Pulling the stick further does not change fire rate.
- **Dead zones**: left stick below `GAMEPAD_LEFT_DEADZONE`, right stick below `GAMEPAD_RIGHT_DEADZONE` are ignored to prevent drift.

### Initial World

- **100 asteroids** spawn at startup, distributed across a `SIM_WIDTH`×`SIM_HEIGHT` (4000×4000) unit simulation area (see `src/constants.rs`)
- A **`PLAYER_BUFFER_RADIUS`** exclusion zone around the player start (origin) keeps the starting area clear
- **Noise-based clustering**: positions are sampled from a hash-based 2D noise function so asteroids naturally form groups; cluster density and size are controlled by `noise_frequency` in `src/asteroid.rs`
- Random shapes (triangles, squares, pentagons, hexagons, **heptagons, octagons**) and sizes (`ASTEROID_SIZE_SCALE_MIN`–`ASTEROID_SIZE_SCALE_MAX`×), random initial velocities
- **Vertex jitter**: each spawned polygon has per-vertex random offsets applied (amplitude proportional to `size_scale × 0.8`) so asteroids appear worn and irregular rather than perfectly geometric
- One **planetoid** (16-sided near-circle, unit size `PLANETOID_UNIT_SIZE`) spawns at a fixed offset from the origin and participates in full N-body gravity and merging like any other asteroid

### Camera Controls

#### Zoom (Mouse Wheel)

- **Scroll up**: Zoom out (smaller scale, larger viewport)
- **Scroll down**: Zoom in (larger scale, smaller viewport)
- **Zoom range**: `MIN_ZOOM`× (full simulation circle visible) to `MAX_ZOOM`× (detail magnification)
- **Smooth scaling**: ±`ZOOM_SPEED` scale units per scroll event
- **Camera follows the player** automatically; no manual pan

### Coordinate System

- **Screen (0,0)**: Top-left corner
- **World (0,0)**: Center of simulation
- **X-axis**: Right (positive)
- **Y-axis**: Up (positive)

## Lives, Respawn & Game Over

### Lives System

- The player starts each session with **3 lives** (displayed as ♥ hearts in the HUD, below the score).
- Each time the ship is destroyed one heart is consumed and a respawn countdown begins.
- The `player_lives` count (default `PLAYER_LIVES = 3`) and all timing constants can be tuned in `assets/physics.toml` without recompilation, and changes hot-reload at runtime.

### Respawn

- After destruction the HUD shows **"RESPAWNING IN X.Xs…"** counting down `respawn_delay_secs` (default 2.5 s).
- The ship re-spawns at the **world origin** (simulation centre) with full HP.
- A post-respawn **invincibility window** (`respawn_invincibility_secs`, default 4.0 s) protects the ship long enough to orient and escape any nearby asteroids.

### Game Over

- When the final life is lost the simulation freezes and a **full-screen Game Over overlay** appears, showing the current score.
- **PLAY AGAIN** (button or **Enter**): resets lives to 3 and returns to the existing world (asteroids remain intact).
- **QUIT** (button): exits the application.

## Save / Load

### Save Slots

- The game supports **three manual save slots** (`saves/slot_1.toml`, `saves/slot_2.toml`, `saves/slot_3.toml`).
- While paused, use **SAVE 1 / SAVE 2 / SAVE 3** buttons to write the current run to a slot.
- Save files are TOML and include a schema version for compatibility checks.

### Loading

- From the main menu, click **LOAD GAME** to open the slot picker.
- Slot buttons show save metadata per slot: scenario and save timestamp (`saved: unix ...`) when loadable.
- Corrupt or unreadable slot files are shown as **SLOT N (CORRUPT)** and are not presented as load-ready.
- Loading restores the saved scenario, asteroid world snapshot, player state, and progression resources (score/lives/ore/ammo/upgrades).

## Ore Pickups

### Drops

- Destroying a small asteroid (bullet: size ≤ 1, missile: size ≤ 3) spawns a **green diamond** ore pickup that drifts away from the impact point with a small random scatter velocity.
- Each ore pickup expires automatically after `ORE_LIFETIME_SECS` (25 s) if not collected.

### Collection

- The player collects ore by flying over it — the ore sensor fires a `CollisionEvent::Started` when it overlaps the player ship.
- The total collected count is shown in a **green "Ore: N" HUD row** (row 4, below the missile ammo display).

### Ore Magnet

- The ore magnet is **upgrade-driven** and starts intentionally weak at Level 1 (internal level 0).
- Ore pickups within `ore_magnet_radius` (base 250 u) of the player are automatically attracted toward the ship.
- The pull uses a velocity lerp each frame: ore `linvel` smoothly transitions toward a vector pointing at the player at `ore_magnet_strength` u/s (base 40 u/s).
- Each ore-magnet upgrade level increases radius by +50 u and strength by +16 u/s (up to Level 10).
- Ore outside the magnet radius drifts freely under its initial scatter velocity.
- Base constants are runtime-tunable via `assets/physics.toml` and hot-reload while the game is running.

### Spending Ore

Ore has two consumable uses, replacing the old passive regeneration systems:

| Action | Key | Gamepad | Cost | Effect |
|--------|-----|---------|------|--------|
| Heal | `H` | DPad Up | 1 ore | Restore `ore_heal_amount` HP (default 30), capped at max HP |
| Restock missile | `M` | DPad Down | 1 ore | +1 missile, capped at `missile_ammo_max` |

- Ore is **not spent** if the corresponding stat is already full.
- The ore HUD row shows `[H] heal  [M] ammo  | Wpn: Lv.N` so current weapon level and spending hints are always visible.
- Passive HP regen and passive missile recharge have been **removed**; ore spending is the only way to replenish them.

### Primary Weapon Upgrades

The primary projectile weapon can be upgraded up to **Level 10** using ore, accessed from the **pause menu → UPGRADES**.

| Level | Fully destroys size… | Ore cost to reach |
|-------|---------------------|-------------------|
| 1 (default) | ≤ 1 | — |
| 2 | ≤ 2 | 10 |
| 3 | ≤ 3 | 15 |
| … | … | … |
| 10 | ≤ 10 | 55 |

- **Above threshold**: any asteroid larger than the current destroy-size is *chipped* — one vertex is removed and a size-1 fragment is ejected. No single hit can destroy more than half the target.
- **Ore reward scaling**: fully-destroying a size-N asteroid drops N ore (vs. 1 before), so higher-level play generates proportionally more upgrade fuel.
- **Shop UI**: opened from pause menu → UPGRADES. Shows current level, size range, ore balance, and upgrade cost. The buy button greys out when unaffordable or at max level.
- **Session-only**: upgrades reset when returning to the main menu (saves are not yet implemented).

### Secondary Weapon Upgrades (Missiles)

Missiles have their own ore-based upgrade progression up to **Level 10**, purchased from the same **pause menu → UPGRADES** flow.

- **Base behaviour (Level 1 / internal 0)**: missiles fully destroy small targets and chip larger targets.
- **Upgrade scaling**: each level increases missile impact power so larger asteroids can be fully destroyed, and chips remove more size-1 units on heavy targets.
- **Costs**: level cost scales linearly by upgrade tier (same progression shape as other ore upgrades).
- **HUD/shop visibility**: current levels and upgrade affordability are shown in the upgrade UI.
- **Session-only**: missile upgrades reset when returning to the main menu (until save/load exists).

## Visual Feedback

### Score HUD

A permanent top-left HUD (amber text, always visible) shows the player's running score:

```
Score: 42  (30 hits, 12 destroyed)
```

| Event                  | Points |
| ---------------------- | ------ |
| Bullet/missile hits asteroid | +1 × multiplier |
| Asteroid fully destroyed (size 0–1) by bullet | +5 × multiplier |
| Asteroid fully destroyed (size ≤ 3) by missile | +10 × multiplier |

**Hit-streak multiplier** — consecutive hits without missing build a streak; the multiplier increases at thresholds (×2 at 5, ×3 at 10, ×4 at 20, ×5 at 40). Missing a shot or dying resets the streak.

**Missile ammo** — starts at 5; replenished by spending ore (`M` key / DPad Down, 1 ore = 1 missile). HUD row 3 shows current ammo (`M M M - -`).

### On-Screen Statistics Display

Located in top-left corner (follows camera pan):

```text
Live: XX | Culled: YY | Merged: ZZ
```

- **Live**: Number of asteroids currently in simulation (within `CULL_DISTANCE` boundary)
- **Culled**: Total number of asteroids removed by culling system
- **Merged**: Total number of merge events (N asteroids → 1 counts as 1 merge)
- **Updates**: Every frame in real-time

### Physics Inspector Overlay

- A new **Physics Inspector** debug overlay can be toggled from the in-game debug panel.
- Shows:
  - Active contact pair count
  - Player entity ID, position, velocity, and contact count
  - A small sample of asteroid entity IDs with position/velocity/contact counts
- Useful for live debugging of contact/velocity behavior without restarting in test mode.

### Spatial Grid Overlay

- A new **Spatial Grid** debug overlay can be toggled from the in-game debug panel.
- Renders KD-tree split-cell lines used by the `SpatialGrid` neighbor index.
- Uses world-space bounds that match the active simulation area (`CULL_DISTANCE`) for consistent cell context.

### Culling Boundary Visualization

- **Visual**: Yellow circle with `CULL_DISTANCE` radius at origin
- **Purpose**: Shows edge where asteroids will be auto-removed
- **Follows Camera**: Rendered in world-space, moves with pan
- **Color**: RGB(1.0, 1.0, 0.0) - Bright yellow for visibility

### Asteroid Rendering

- **Filled polygon mesh** (`Mesh2d`): every asteroid is drawn as a GPU-retained filled polygon with a rocky grey-brown tint derived from its entity index — no per-frame CPU rebuild.
- **Wireframe overlay** (optional, debug panel): translucent white edges can be drawn on top of the fill via the *Wireframe Outlines* toggle.
- **Wireframe-only mode** (debug panel): hides all fills; asteroids (and ship + projectiles) render as white gizmo wireframes only.
- **Rotation**: the `Mesh2d` is attached to the Rapier-managed `Transform`, so mesh rotation is automatic.
- Composite asteroids appear larger due to wider vertex spread.

### Weapon Rendering

- **Primary weapon projectiles**: elongated capsule shape (10 units long, 2u radius) in bright yellow, oriented in the direction of travel to resemble a sci-fi "plasma pulse" or "blaster bolt".
- **Missiles**: rocket-shaped polygon (6u wide body, 12u long, 6u pointed nose, 4u fins) in orange, oriented in the direction of travel with a distinctive rocket silhouette.
- **Missile movement profile**: missiles now spawn at a lower initial speed, then accelerate continuously in flight until reaching their configured max speed.
- **Missile trail particles**: short-lived orange exhaust particles emit continuously from the missile tail opposite movement direction while missiles are in flight.
- **Both weapons**: meshes are rotated automatically on spawn to align with velocity direction; the orientation is fixed for the lifetime of the projectile/missile.

## Simulation Statistics

### Tracked Metrics

The `SimulationStats` resource tracks:

```rust
pub struct SimulationStats {
    pub live_count: usize,       // Asteroids in bounds
    pub culled_total: usize,     // Cumulative removed
    pub merged_total: usize,     // Cumulative merges
}
```

### Counting System

- **`stats_counting_system`**: Runs every frame
  - Counts live asteroids within `CULL_DISTANCE` boundary
  - Tracks culled asteroids (those beyond boundary)
  - Tracks merge events when clusters form
  - Output: Updates on-screen display and console logging

### Data Accuracy

- Counts update BEFORE culling to catch removals accurately
- Merge counter increments when N asteroids → 1 composite
- Display updates in real-time (every frame)

## Implementation Details

### Statistics Text Rendering

- **Component**: `Text2dBundle` entity for on-screen rendering
- **Font**: Bevy default system font (no external assets required)
- **Position**: Fixed to camera (top-left, follows pan)
- **Color**: Cyan text for high visibility
- **Update frequency**: Every frame

### Camera Management

The `CameraState` resource manages:

```rust
pub struct CameraState {
    pub pan_x: f32,             // X offset from origin
    pub pan_y: f32,             // Y offset from origin
    pub zoom: f32,              // Scale factor (1.0 = default)
}
```

### Click Input Coordinate Conversion

Clicking correctly spawns asteroids using transformed coordinates:

```text
screen_pos → apply_zoom → add_pan_offset → world_pos
```

**Formula**:

```text
norm_x = (cursor_x - window.width/2) * zoom
norm_y = -(cursor_y - window.height/2) * zoom
world_x = norm_x + pan_x
world_y = norm_y + pan_y
```

This ensures accurate spawning regardless of camera state.

## Player Ship Systems

### Player Health & Damage

The player ship has a health pool that depletes when struck by asteroids at high relative speeds.

| Property                 | Description                                                     |
| ------------------------ | --------------------------------------------------------------- |
| `PLAYER_MAX_HP`          | Full health at spawn                                            |
| `DAMAGE_SPEED_THRESHOLD` | Minimum relative speed before damage is dealt                   |
| `INVINCIBILITY_DURATION` | Immunity period after each hit to prevent rapid damage stacking |

**Damage formula**: `damage = (relative_speed − DAMAGE_SPEED_THRESHOLD) × 0.5` — slow grazes deal no damage; high-speed impacts deal proportionally more.

**Visual feedback**: The ship body is rendered as a dark-teal **filled polygon mesh** (`Mesh2d`) that rotates with physics transforms. The optional wireframe outline (toggled via the debug panel) shifts colour from cyan (full health) to red as HP decreases. A pixel-wide health bar floats above the ship showing the current HP fraction (green → red as health drops). The outline and health bar are always centred on the ship regardless of camera state.

**Ship destruction**: When HP reaches 0 the player entity is despawned. There is currently no respawn mechanic.

### Asteroid Destruction (Projectile Hits)

Projectiles interact with asteroids based on the target's `AsteroidSize` unit count:

| Size | Effect |
|---|---|
| 0–1 | **Destroy** — asteroid fully despawned |
| 2–3 | **Scatter** — despawns and spawns `N` unit fragments at evenly-spaced angles with random velocity jitter |
| 4–8 | **Split** — cut roughly in half along the projectile's impact axis; each half retains its velocity plus a separation impulse |
| ≥ 9 | **Chip** — removes the vertex closest to the impact point; spawns one unit fragment; original asteroid loses one mass unit |

In all cases the projectile is despawned on contact.

### Asteroid Destruction (Missile Hits)

Missiles deal heavier damage than bullets:

| Size | Effect |
|---|---|
| ≤ 3 | **Instant destroy** — full despawn, double bonus points |
| 4–8 | **Full scatter** — despawns and ejects all `n` unit fragments with large random velocity jitter |
| ≥ 9 | **Burst chip** — scatters 4 unit fragments; original asteroid shrinks by 3 mass units (min 1) |

| Size (units) | Behaviour                                                                                                                          |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| 0–1          | **Destroy** — asteroid removed immediately                                                                                         |
| 2–3          | **Scatter** — despawns and spawns `N` unit fragments at evenly-spaced angles with random velocity jitter                           |
| 4–8          | **Split** — cut roughly in half along the projectile's impact axis; each half retains its velocity plus a separation impulse       |
| ≥9           | **Chip** — the hull vertex closest to impact is removed; a single unit fragment is ejected outward; the asteroid shrinks by 1 unit |

In all cases the projectile is despawned on contact.
**Mass → shape rules (split and chip only):** Fragments produced by splitting or chipping must have a minimum number of polygon sides matching their mass. Merged composites are exempt.

| Fragment mass | Min shape | Min vertices |
| ------------- | --------- | ------------ |
| 1             | triangle  | 3            |
| 2–4           | square    | 4            |
| 5             | pentagon  | 5            |
| ≥6            | hexagon   | 6            |

If the geometric split produces fewer vertices than the minimum for that mass (e.g. a triangular half from a size-4 asteroid), the fragment is replaced with the canonical regular polygon centred at the computed split position. Fragments may have _more_ sides than the minimum — the raw hull is kept whenever it already meets or exceeds the requirement.
**Split geometry**: For the 4–8 case the split plane passes through the asteroid centroid and is aligned with the projectile trajectory direction, so the two halves separate naturally along the incoming fire direction.

**Chip geometry**: The remaining asteroid recomputes its convex hull after removing the impacted vertex, so the outline incrementally shrinks with each chip hit.

### Out-of-Bounds Behaviour

The player ship is not culled like asteroids, but experiences increasing velocity damping when outside the `OOB_RADIUS` boundary:

- **OOB radius**: `OOB_RADIUS` from origin (matches asteroid cull boundary)
- **Damping factor**: `OOB_DAMPING` (velocity scaled per frame), ramped smoothly over `OOB_RAMP_WIDTH` from 0% at the boundary to full effect beyond
- **Effect**: Gentle drag that discourages escaping the simulation; the player can still re-enter under thrust

## Pause Menu

Press **ESC** during gameplay to pause the simulation. The pause menu appears as a semi-transparent overlay:

| Button            | Action                                                  |
| ----------------- | ------------------------------------------------------- |
| **RESUME**        | Resume simulation (also triggered by pressing ESC again)|
| **DEBUG OVERLAYS**| Toggle the floating debug overlay panel (top-right)     |
| **QUIT**          | Exit the application                                    |

While paused, Rapier's physics pipeline is fully disabled — all asteroids, velocities, and forces are frozen in place until the game is resumed.

## Debug Overlay Panel

Open from the pause menu (**DEBUG OVERLAYS** button). Appears in the top-right corner.

| Toggle                | Default | Description                                                      |
| --------------------- | ------- | ---------------------------------------------------------------- |
| Culling Boundary      | OFF     | Yellow circle showing the `CULL_DISTANCE` boundary              |
| Wireframe Outlines    | OFF     | Translucent polygon edges over asteroid fills                    |
| Force Vectors         | OFF     | Red force direction arrows per asteroid (hidden at high count)   |
| Velocity Arrows       | OFF     | Cyan velocity arrows per asteroid                                |
| Wireframe-Only Mode   | OFF     | Hide all `Mesh2d` fills; render everything as gizmo wireframes  |
| Aim Indicator         | OFF     | Orange line + dot showing current fire direction                 |
| Ship Outline          | OFF     | HP-tinted polygon edges + nose indicator over the ship fill     |
| Projectile Outline    | OFF     | Yellow gizmo circles over projectile disc fills                  |
| Spatial Grid          | OFF     | KD-tree split-cell lines for spatial partition debugging         |
| Stats Overlay         | OFF     | Live/Culled/Merged/Split/Destroyed simulation counters           |
| Physics Inspector     | OFF     | Entity IDs, velocities, and active contact counts                |

## UI/UX Notes

### Viewport Design

- **Simulation origin**: (0,0) at center of screen initially
- **Live zone**: Within `CULL_DISTANCE` radius of origin — asteroids inside count as "live"
- **Soft boundary**: Beyond `SOFT_BOUNDARY_RADIUS` a gentle inward spring force nudges asteroids back toward centre
- **Hard-cull zone**: Beyond `HARD_CULL_DISTANCE` asteroids are permanently removed as a safety net
- **Camera follows the player** — no manual pan; zoom in/out with mouse wheel

### Zoom Levels Explained

- **0.5x (min)**: Full simulation circle visible
  - Use for high-level overview, cluster observation
- **1.0x (default)**: 1200×680 simulation window in world units
  - Use for standard gameplay
- **4.0x (max)**: ~300 unit detail zone
  - Use for examining asteroid structures, collision dynamics

### Statistics as Feedback

The real-time display helps verify:

- **Spawning success**: Live count increases when clicking
- **Merge events**: Merged counter increments when asteroids combine
- **Culling behavior**: Culled counter increases as asteroids leave boundary
- **Physics stability**: Observe consistency in merge events over time

## Performance Considerations

### Asteroids Rendered

- Dynamically scales from 1 to 1000+ asteroids
- Gizmo rendering efficient for wireframe
- No performance degradation observed up to 100+ asteroids

### Physics Complexity

- N-body gravity: O(n²) force calculations
- Culling: O(n) position checks
- Merging: O(n) cluster detection via flood-fill
- System stable over 500+ frame simulations

### Memory Management

- Culling system automatically removes off-screen asteroids
- Merged asteroids despawned after composite formation
- Stats tracking minimal overhead
