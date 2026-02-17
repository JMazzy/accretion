# Quick Reference: Gravity Fix

## The Problem
Asteroids accelerated to extreme speeds (20 → 426 u/s) when passing near each other or bouncing at high speed.

## The Root Cause
Gravity was still being applied at very close range (< 2 units), causing energy injection and runaway acceleration.

## The Solution
**Simply skip applying gravity when asteroids are closer than 20 units apart.**

## Code Change

**Before (BROKEN)**:
```rust
let min_dist = 2.0;  // Clamping didn't prevent close-range force
let dist_sq = (dist * dist).max(min_dist * min_dist);
let force_mag = gravity_const / dist_sq;  // Force still applied!
```

**After (FIXED)**:
```rust
let min_gravity_dist = 20.0;  // Skip entirely at close range
if dist < min_gravity_dist || dist > max_gravity_dist {
    continue;  // No force at all - let Rapier physics handle it
}
let dist_sq = dist * dist;
let force_mag = gravity_const / dist_sq;  // Only applied at safe distance
```

## Test Results
```
✓ two_triangles        → Merged properly
✓ three_triangles      → Merged properly  
✓ gentle_approach      → Smooth gravity convergence
✓ high_speed_collision → Merged at collision
✓ near_miss            → 20→38 u/s (stable, was 20→426!)
✓ gravity              → Distant asteroids attract cleanly
```

## Physics Constants (Updated)
```rust
gravity_const     = 2.0     // Gentle mutual attraction
min_gravity_dist  = 20.0    // ← PRIMARY FIX
max_gravity_dist  = 300.0   // Prevents phantom forces
```

## Why This Works
1. **Below 20 units**: Rapier2D physics engine handles contact detection and collision response perfectly
2. **Adding gravity at close range**: Only causes energy injection and instability
3. **Disabling gravity in collision zone**: Lets Rapier do what it's designed for
4. **Above 20 units**: Gravity naturally attracting asteroids together - exactly what we want

## Impact
- **Performance**: Slightly improved (fewer force calculations)
- **Stability**: Much more stable physics
- **Realism**: Asteroids now behave with expected physics at all speeds
- **Compatibility**: No breaking changes, all existing tests pass
