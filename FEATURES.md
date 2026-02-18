# GRAV-SIM Features & User Controls

## Runtime Controls

### Asteroid Spawning

- **Left-click**: Spawn a small triangle asteroid at cursor position
- **Accuracy**: Click position correctly tracks with camera pan and zoom
- **No automatic spawning**: Simulation starts empty; user drives all spawning

### Camera Controls

#### Pan (Arrow Keys)

- **Up/Down arrows**: Pan camera vertically (±5 units/frame)
- **Left/Right arrows**: Pan camera horizontally (±5 units/frame)
- **Pan bounds**: Constrained to ±600 units from origin
  - Ensures simulation area stays visible and culling boundary remains partially in view

#### Zoom (Mouse Wheel)

- **Scroll up**: Zoom out (smaller scale, larger viewport)
- **Scroll down**: Zoom in (larger scale, smaller viewport)
- **Zoom range**: 0.5x (full simulation circle visible) to 8.0x (detail magnification)
- **Smooth scaling**: ±0.1 scale units per scroll event

### Coordinate System

- **Screen (0,0)**: Top-left corner
- **World (0,0)**: Center of simulation
- **X-axis**: Right (positive)
- **Y-axis**: Up (positive)

## Visual Feedback

### On-Screen Statistics Display

Located in top-left corner (follows camera pan):

```text
Live: XX | Culled: YY | Merged: ZZ
```

- **Live**: Number of asteroids currently in simulation (within 1000-unit boundary)
- **Culled**: Total number of asteroids removed by culling system
- **Merged**: Total number of merge events (N asteroids → 1 counts as 1 merge)
- **Updates**: Every frame in real-time

### Culling Boundary Visualization

- **Visual**: Yellow circle with 1000-unit radius at origin
- **Purpose**: Shows edge where asteroids will be auto-removed
- **Follows Camera**: Rendered in world-space, moves with pan
- **Color**: RGB(1.0, 1.0, 0.0) - Bright yellow for visibility

### Asteroid Rendering

- **Small asteroids**: White wireframe outline (triangle or polygon vertices)
- **Rotation**: Vertices rotate with physics-based angular velocity
- **Color distinction**: All asteroids rendered identically as wireframes
- **Size scaling**: Composite asteroids appear larger due to wider vertex spread

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
  - Counts live asteroids within 1000-unit boundary
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
