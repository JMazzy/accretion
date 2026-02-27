#!/bin/bash

# High-load performance benchmark focused on playtest hotspots:
# - >200 asteroids
# - multiple enemy ships
#
# Logs are written inside the repository to avoid external temp paths.

set -euo pipefail

OUT_DIR="artifacts/perf/$(date +%F)"
mkdir -p "$OUT_DIR"

SCENARIOS=(
  "baseline_225"
  "all_three_225_enemy5"
  "mixed_content_225_enemy8"
  "mixed_content_324_enemy12"
)

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║  Accretion High-Load Performance Benchmark                     ║"
echo "║  Scenarios: >200 asteroids + enemies + mixed content           ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

echo "Building release binary..."
cargo build --release > /dev/null 2>&1 || cargo build --release
echo ""

for scenario in "${SCENARIOS[@]}"; do
  out_file="$OUT_DIR/${scenario}.log"
  echo "=== $scenario ==="
  timeout 360s env ACCRETION_TEST="$scenario" cargo run --release > "$out_file" 2>&1 || true
  grep -E "Timing summary|PostUpdate schedule summary|avg frame|min frame|max frame|post_update (avg|min|max|p50|p95|p99)|frames at 60 FPS|PASS:" "$out_file" | tail -16 || echo "(no timing summary captured)"
  echo "log: $out_file"
  echo ""
done

echo "Done. Raw logs are in $OUT_DIR"
