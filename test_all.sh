#!/bin/bash

# Comprehensive test suite for the gravity-sim asteroid simulation
# Tests various scenarios to verify physics behavior after the gravity fix

set -e

echo "╔════════════════════════════════════════════════════════╗"
echo "║        GRAV-SIM Physics Test Suite                     ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

TESTS=(
    "two_triangles"
    "three_triangles"
    "gentle_approach"
    "high_speed_collision"
    "near_miss"
    "gravity"
    "culling_verification"
    "large_small_pair"
    "gravity_boundary"
    "mixed_size_asteroids"
)

TOTAL=0
PASSED=0
FAILED=0

for test in "${TESTS[@]}"; do
    echo "▶ Running test: $test"
    TOTAL=$((TOTAL + 1))
    
    RESULT=$(timeout 50 bash -c "GRAV_SIM_TEST=$test cargo run --release 2>&1" | grep -E "(PASS|FAIL)" | tail -1)
    echo "$RESULT"

    if echo "$RESULT" | grep -q "✓ PASS"; then
        PASSED=$((PASSED + 1))
    else
        FAILED=$((FAILED + 1))
    fi
    echo ""
done

echo "╔════════════════════════════════════════════════════════╗"
echo "║              TEST SUMMARY                             ║"
echo "╚════════════════════════════════════════════════════════╝"
echo "Total:  $TOTAL"
echo "Passed: $PASSED"
echo "Failed: $FAILED"

if [ $FAILED -eq 0 ]; then
    echo ""
    echo "✓ ALL TESTS PASSED!"
    exit 0
else
    echo ""
    echo "✗ SOME TESTS FAILED"
    exit 1
fi
