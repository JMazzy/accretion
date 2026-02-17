# Asteroid Merging System - Complete Fix Summary

## Problem
Asteroids that were touching were not merging/combining into composite structures, despite the formation system detecting contacts and attempting to spawn merged composites.

## Root Causes Found & Fixed

### 1. **System Scheduling Issue**
- **Problem**: `asteroid_formation_system` ran in `Update` schedule, but Rapier physics updates in `FixedUpdate`
- **Impact**: Contact manifolds weren't populated when the formation system queried them
- **Solution**: Moved `asteroid_formation_system` to `PostUpdate` schedule so it runs AFTER physics updates complete
- **Status**: ✅ FIXED

### 2. **Test Verification Timing**
- **Problem**: Test verification systems also ran in `Update`, checking asteroid count before formation merging
- **Impact**: Tests counted asteroids before despawn commands executed
- **Solution**: Moved `test_logging_system` and `test_verification_system` to `PostUpdate` with explicit ordering after `asteroid_formation_system`
- **Status**: ✅ FIXED

### 3. **Convex Hull Computation Bug (Critical)**
- **Problem**: `compute_convex_hull()` was computing hull from asteroid **CENTER positions** instead of actual **vertex positions**
- **Impact**: Two touching asteroids produced a degenerate 2-vertex hull (line segment), rejecting merge as "hull too small"
- **Solution**: 
  - Refactored `asteroid_formation_system` to include `&Vertices` in cluster data
  - Created `compute_convex_hull_from_points()` helper function
  - Collected ALL vertices from ALL cluster members in world-space
  - Rotated vertices by transform rotation to get accurate world positions
  - Computed hull from complete set of vertices
  - Converted hull back to local-space relative to center
- **Status**: ✅ FIXED

### 4. **Overlapping Asteroid Separation**
- **Problem**: Test asteroids spawned with overlapping positions caused Rapier to apply strong separation impulses
- **Impact**: Asteroids accelerated apart instead of merging
- **Solution**: Spawned test asteroids with edges precisely touching but not overlapping
- **Status**: ✅ FIXED

## Test Results

### Two Triangles Test
```
Initial:  2 triangles (at centers ±3.0)
Final:    1 composite hexagon
Result:   ✓ PASS
```

### Three Triangles Test  
```
Initial:  3 triangles (in touching cluster)
Final:    1 composite polygon
Result:   ✓ PASS
```

## Code Changes

### Modified Files
1. **src/simulation.rs**
   - Added `Vertices` component to formation query
   - Moved `asteroid_formation_system` to `PostUpdate`
   - Refactored cluster merging to collect and compose all vertices

2. **src/asteroid.rs**
   - Added `compute_convex_hull_from_points()` function

3. **src/main.rs**
   - Moved test systems to `PostUpdate` with explicit ordering

4. **src/testing.rs**
   - Updated `spawn_test_two_triangles()` to position asteroids correctly
   - Updated `spawn_test_three_triangles()` to position asteroids correctly

## Physics Behavior Verified
- ✅ N-body gravity attracting asteroids (gravity_const=2.0)
- ✅ Inelastic collisions (restitution=0.0, friction=1.0)
- ✅ Contact detection via Rapier manifolds
- ✅ Composite hull formation via convex hull algorithm
- ✅ Velocity inheritance on composite formation
- ✅ Proper local/world space vertex handling

## Architecture Impact
The fix improves the ECS system ordering:
- Physics updates → Contact manifolds populate
- Formation system runs → Detects contacts, merges asteroids
- Test verification → Counts final asteroid count

This ensures proper data consistency and eliminates race conditions between physics and logic systems.
