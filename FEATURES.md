# GRAV-SIM Features & User Controls

## Runtime Controls

### Keyboard + Mouse (Twin-Stick)

| Input                       | Action                                                                         |
| --------------------------- | ------------------------------------------------------------------------------ |
| **W**                       | Thrust forward (ship-facing direction)                                         |
| **S**                       | Thrust backward                                                                |
| **A**                       | Rotate ship left                                                               |
| **D**                       | Rotate ship right                                                              |
| **Space** or **Left-click** | Fire projectile toward mouse cursor (auto-repeats at cooldown rate while held) |
| **Mouse wheel**             | Zoom in / out                                                                  |

- **Aiming is decoupled from movement**: the ship faces the direction you steer, but projectiles travel toward the mouse cursor regardless of ship heading.
- An **orange aim indicator** (line + dot) extends from the ship in the current fire direction. It is shown by default and can be hidden via the debug panel (*Aim Indicator* toggle).
- **Aim idle snap**: if no mouse movement, gamepad left stick, or right stick input is received for 1 second, the aim direction automatically resets to the ship's forward (+Y).

### Gamepad (Twin-Stick)

| Input           | Action                                                                 |
| --------------- | ---------------------------------------------------------------------- |
| **Left stick**  | Rotate ship toward stick direction at fixed speed, then thrust forward |
| **Right stick** | Aim and auto-fire projectiles in stick direction                       |
| **B button**    | Brake — applies strong velocity damping each frame while held          |

- **Left stick movement**: the ship rotates at a fixed angular speed until aligned with the stick direction, then applies forward thrust proportional to stick magnitude. Thrust is suppressed while rotating sharply (above the heading correction threshold in `src/constants.rs`) to avoid fighting the turn.
- **B button brake**: while held, multiplies both linear and angular velocity by `GAMEPAD_BRAKE_DAMPING` every frame, bringing the ship to a near-stop in roughly half a second at 60 fps. Forward thrust is independent and can still be applied simultaneously via the left stick.
- **Right stick auto-fire**: once the right stick exceeds `GAMEPAD_FIRE_THRESHOLD` deflection, projectiles auto-fire at the fire cooldown rate. Pulling the stick further does not change fire rate.
- **Dead zones**: left stick below `GAMEPAD_LEFT_DEADZONE`, right stick below `GAMEPAD_RIGHT_DEADZONE` are ignored to prevent drift.

### Initial World

- Asteroids spawn at startup, distributed across a `SIM_WIDTH`×`SIM_HEIGHT` unit simulation area (see `src/constants.rs`)
- A **`PLAYER_BUFFER_RADIUS`** exclusion zone around the player start (origin) keeps the starting area clear
- Grid-based seeding prevents random clumping while maintaining variety
- Random shapes (triangles, squares, pentagons, hexagons, **heptagons, octagons**) and sizes (`ASTEROID_SIZE_SCALE_MIN`–`ASTEROID_SIZE_SCALE_MAX`×), random initial velocities
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

## Visual Feedback

### Score HUD

A permanent top-left HUD (amber text, always visible) shows the player's running score:

```
Score: 42  (30 hits, 12 destroyed)
```

| Event                  | Points |
| ---------------------- | ------ |
| Projectile hits asteroid| +1    |
| Asteroid fully destroyed (size 0–1) | +5 |

### On-Screen Statistics Display

Located in top-left corner (follows camera pan):

```text
Live: XX | Culled: YY | Merged: ZZ
```

- **Live**: Number of asteroids currently in simulation (within `CULL_DISTANCE` boundary)
- **Culled**: Total number of asteroids removed by culling system
- **Merged**: Total number of merge events (N asteroids → 1 counts as 1 merge)
- **Updates**: Every frame in real-time

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

## Debug Overlay Panel

Press **ESC** to open or close the debug overlay panel (top-right corner).

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
| Stats Overlay         | OFF     | Live/Culled/Merged/Split/Destroyed simulation counters           |

## UI/UX Notes

### Viewport Design

- **Simulation origin**: (0,0) at center of screen initially
- **Safe spawning area**: Within ±1000 units (inside yellow boundary)
- **Culling zone**: Beyond ±1000 units (asteroids removed automatically)
- **Pan limit**: Can't move camera >600 units from origin
  - Ensures you can always see both the center and part of the boundary

### Zoom Levels Explained

- **0.5x (min)**: Full simulation circle visible (~2000 units across)
  - Use for high-level overview, cluster observation
- **1.0x (default)**: 1200×680 simulation window in world units
  - Use for standard gameplay
- **4.0x-8.0x (max)**: 250-300 unit detail zone
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
