#!/bin/bash

# Feature Isolation Benchmark Script
#
# This script measures the individual performance cost of:
# 1. Tidal Torque (realistic spin from differential gravity)
# 2. Soft Boundary (gentle spring force at the boundary)
# 3. KD-tree Spatial Index (redesigned from flat grid)
#
# Each test runs 100 asteroids for 300 frames (5 seconds @ 60fps).
# The KD-tree is already embedded, so "kdtree_only" test uses it.
# The tidal_only and soft_boundary_only tests toggle those features.

set -e

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║  GRAV-SIM Feature Isolation Performance Benchmark              ║"
echo "║  Measures: Tidal Torque, Soft Boundary, KD-tree               ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

# Build in release mode
echo "Building in release mode (optimized)..."
cargo build --release > /dev/null 2>&1 || cargo build --release
echo ""

# Test 1: BASELINE
echo "═ Test 1: BASELINE (original world size, NO new features)"
echo "  Config: tidal_torque_scale = 0.0, soft_boundary_strength = 0.0"
echo "  Running BASELINE_100 test..."
cat > assets/physics.toml << 'EOF'
sim_width = 3000.0
sim_height = 2000.0
spawn_grid_margin = 150.0
player_buffer_radius = 100.0
gravity_const = 10.0
min_gravity_dist = 5.0
max_gravity_dist = 1000.0
velocity_threshold_locking = 5.0
hull_extent_base = 60.0
hull_extent_per_member = 20.0
restitution_small = 0.0
friction_asteroid = 1.0
tidal_torque_scale = 0.0
cull_distance = 1000.0
soft_boundary_radius = 900.0
soft_boundary_strength = 0.0
hard_cull_distance = 1250.0
EOF

GRAV_SIM_TEST=baseline_100 cargo run --release 2>&1 | tail -30
echo ""

# Test 2: TIDAL TORQUE ONLY
echo "═ Test 2: TIDAL TORQUE ONLY (baseline + tidal enabled)"
echo "  Config: tidal_torque_scale = 1.0, soft_boundary_strength = 0.0"
echo "  Running TIDAL_ONLY test..."
cat > assets/physics.toml << 'EOF'
sim_width = 3000.0
sim_height = 2000.0
spawn_grid_margin = 150.0
player_buffer_radius = 100.0
gravity_const = 10.0
min_gravity_dist = 5.0
max_gravity_dist = 1000.0
velocity_threshold_locking = 5.0
hull_extent_base = 60.0
hull_extent_per_member = 20.0
restitution_small = 0.0
friction_asteroid = 1.0
tidal_torque_scale = 1.0
cull_distance = 1000.0
soft_boundary_radius = 900.0
soft_boundary_strength = 0.0
hard_cull_distance = 1250.0
EOF

GRAV_SIM_TEST=tidal_only cargo run --release 2>&1 | tail -30
echo ""

# Test 3: SOFT BOUNDARY ONLY
echo "═ Test 3: SOFT BOUNDARY ONLY (baseline + soft boundary enabled)"
echo "  Config: tidal_torque_scale = 0.0, soft_boundary_strength = 2.0"
echo "  Running SOFT_BOUNDARY_ONLY test..."
cat > assets/physics.toml << 'EOF'
sim_width = 3000.0
sim_height = 2000.0
spawn_grid_margin = 150.0
player_buffer_radius = 100.0
gravity_const = 10.0
min_gravity_dist = 5.0
max_gravity_dist = 1000.0
velocity_threshold_locking = 5.0
hull_extent_base = 60.0
hull_extent_per_member = 20.0
restitution_small = 0.0
friction_asteroid = 1.0
tidal_torque_scale = 0.0
cull_distance = 1000.0
soft_boundary_radius = 900.0
soft_boundary_strength = 2.0
hard_cull_distance = 1250.0
EOF

GRAV_SIM_TEST=soft_boundary_only cargo run --release 2>&1 | tail -30
echo ""

# Test 4: KD-TREE ONLY (no tidal, no soft boundary)
echo "═ Test 4: KD-TREE ONLY (baseline + KD-tree in use)"
echo "  Config: tidal_torque_scale = 0.0, soft_boundary_strength = 0.0"
echo "  Note: KD-tree is already in use; this measures its standalone cost"
echo "  Running KDTREE_ONLY test..."
cat > assets/physics.toml << 'EOF'
sim_width = 3000.0
sim_height = 2000.0
spawn_grid_margin = 150.0
player_buffer_radius = 100.0
gravity_const = 10.0
min_gravity_dist = 5.0
max_gravity_dist = 1000.0
velocity_threshold_locking = 5.0
hull_extent_base = 60.0
hull_extent_per_member = 20.0
restitution_small = 0.0
friction_asteroid = 1.0
tidal_torque_scale = 0.0
cull_distance = 1000.0
soft_boundary_radius = 900.0
soft_boundary_strength = 0.0
hard_cull_distance = 1250.0
EOF

GRAV_SIM_TEST=kdtree_only cargo run --release 2>&1 | tail -30
echo ""

# Test 5: ALL THREE FEATURES
echo "═ Test 5: ALL THREE FEATURES (full current implementation)"
echo "  Config: tidal_torque_scale = 1.0, soft_boundary_strength = 2.0"
echo "  Running ALL_THREE test..."
cat > assets/physics.toml << 'EOF'
sim_width = 3000.0
sim_height = 2000.0
spawn_grid_margin = 150.0
player_buffer_radius = 100.0
gravity_const = 10.0
min_gravity_dist = 5.0
max_gravity_dist = 1000.0
velocity_threshold_locking = 5.0
hull_extent_base = 60.0
hull_extent_per_member = 20.0
restitution_small = 0.0
friction_asteroid = 1.0
tidal_torque_scale = 1.0
cull_distance = 1000.0
soft_boundary_radius = 900.0
soft_boundary_strength = 2.0
hard_cull_distance = 1250.0
EOF

GRAV_SIM_TEST=all_three cargo run --release 2>&1 | tail -30
echo ""

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║  BENCHMARK COMPLETE                                            ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""
echo "Summary:"
echo "--------"
echo "Compare 'avg frame' times across the five tests:"
echo ""
echo "1. baseline_100         = reference (0ms cost)"
echo "2. tidal_only           = baseline + tidal torque cost"
echo "3. soft_boundary_only   = baseline + soft boundary cost"
echo "4. kdtree_only          = baseline + KD-tree cost"
echo "5. all_three            = total cost of all three features"
echo ""
echo "Feature cost = (individual test avg) - (baseline avg)"
echo ""
echo "The feature with the largest cost is the primary slowdown."
echo ""
echo "Restoring original physics.toml..."
git checkout assets/physics.toml 2>/dev/null || echo "  (keeping current config)"

