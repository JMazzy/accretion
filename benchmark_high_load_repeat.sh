#!/bin/bash

# Repeated high-load benchmark runner with summary stats.
# Writes all logs to repo-local artifacts.

set -euo pipefail

REPEATS="${1:-3}"
DATE_TAG="$(date +%F)"
OUT_DIR="artifacts/perf/${DATE_TAG}/high_load_repeat"
mkdir -p "$OUT_DIR"
export OUT_DIR

SCENARIOS=(
  "baseline_225"
  "all_three_225_enemy5"
  "mixed_content_225_enemy8"
    "mixed_content_324_enemy12"
)

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║  Accretion High-Load Repeat Benchmark                          ║"
echo "║  Runs: ${REPEATS} per scenario | output: ${OUT_DIR}                "
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

echo "Building release binary..."
cargo build --release > /dev/null 2>&1 || cargo build --release
echo ""

for scenario in "${SCENARIOS[@]}"; do
  for run in $(seq 1 "$REPEATS"); do
    out_file="$OUT_DIR/${scenario}_r${run}.log"
    echo "=== ${scenario} :: run ${run}/${REPEATS} ==="
    timeout 360s env ACCRETION_TEST="$scenario" cargo run --release > "$out_file" 2>&1 || true
    grep -E "avg frame|min frame|max frame|p50 frame|p95 frame|p99 frame|post_update (avg|min|max|p50|p95|p99)|frames at 60 FPS|PASS:" "$out_file" | tail -15 || echo "(no timing summary captured)"
    echo "log: $out_file"
    echo ""
  done
done

echo "=== Aggregate summary ==="
python3 - << 'PY'
import glob
import os
import re
import statistics

out_dir = os.environ.get("OUT_DIR")
scenarios = [
    "baseline_225",
    "all_three_225_enemy5",
    "mixed_content_225_enemy8",
    "mixed_content_324_enemy12",
]

avg_re = re.compile(r"avg frame:\s*([0-9.]+)ms")
p50_re = re.compile(r"p50 frame:\s*([0-9.]+)ms")
p95_re = re.compile(r"p95 frame:\s*([0-9.]+)ms")
p99_re = re.compile(r"p99 frame:\s*([0-9.]+)ms")
post_p50_re = re.compile(r"post_update p50:\s*([0-9.]+)ms")
post_p95_re = re.compile(r"post_update p95:\s*([0-9.]+)ms")
post_p99_re = re.compile(r"post_update p99:\s*([0-9.]+)ms")
fps_re = re.compile(r"frames at 60 FPS .*\(([0-9.]+)%\)")

def percentile(values, p):
    if not values:
        return None
    vals = sorted(values)
    if len(vals) == 1:
        return vals[0]
    rank = (len(vals) - 1) * p
    low = int(rank)
    high = min(low + 1, len(vals) - 1)
    frac = rank - low
    return vals[low] * (1.0 - frac) + vals[high] * frac

for scenario in scenarios:
    logs = sorted(glob.glob(os.path.join(out_dir, f"{scenario}_r*.log")))
    avg_ms = []
    p50_ms = []
    p95_ms = []
    p99_ms = []
    post_p50_ms = []
    post_p95_ms = []
    post_p99_ms = []
    fps_pct = []

    for path in logs:
        with open(path, "r", encoding="utf-8", errors="ignore") as f:
            text = f.read()
        m_avg = avg_re.search(text)
        m_p50 = p50_re.search(text)
        m_p95 = p95_re.search(text)
        m_p99 = p99_re.search(text)
        m_post_p50 = post_p50_re.search(text)
        m_post_p95 = post_p95_re.search(text)
        m_post_p99 = post_p99_re.search(text)
        m_fps = fps_re.search(text)
        if m_avg:
            avg_ms.append(float(m_avg.group(1)))
        if m_p50:
            p50_ms.append(float(m_p50.group(1)))
        if m_p95:
            p95_ms.append(float(m_p95.group(1)))
        if m_p99:
            p99_ms.append(float(m_p99.group(1)))
        if m_post_p50:
            post_p50_ms.append(float(m_post_p50.group(1)))
        if m_post_p95:
            post_p95_ms.append(float(m_post_p95.group(1)))
        if m_post_p99:
            post_p99_ms.append(float(m_post_p99.group(1)))
        if m_fps:
            fps_pct.append(float(m_fps.group(1)))

    if not avg_ms:
        print(f"{scenario}: no parsable timing summaries")
        continue

    med = statistics.median(avg_ms)
    p95 = percentile(avg_ms, 0.95)
    fps_med = statistics.median(fps_pct) if fps_pct else None
    p50_med = statistics.median(p50_ms) if p50_ms else None
    p95_med = statistics.median(p95_ms) if p95_ms else None
    p99_med = statistics.median(p99_ms) if p99_ms else None
    post_p50_med = statistics.median(post_p50_ms) if post_p50_ms else None
    post_p95_med = statistics.median(post_p95_ms) if post_p95_ms else None
    post_p99_med = statistics.median(post_p99_ms) if post_p99_ms else None
    runs = len(avg_ms)

    line = f"{scenario}: runs={runs} avg_ms{{median={med:.2f}, p95={p95:.2f}}}"
    if p50_med is not None and p95_med is not None and p99_med is not None:
        line += f" frame_ms{{p50_med={p50_med:.2f}, p95_med={p95_med:.2f}, p99_med={p99_med:.2f}}}"
    if post_p50_med is not None and post_p95_med is not None and post_p99_med is not None:
        line += f" post_update_ms{{p50_med={post_p50_med:.3f}, p95_med={post_p95_med:.3f}, p99_med={post_p99_med:.3f}}}"
    if fps_med is not None:
        line += f" 60fps_pct{{median={fps_med:.1f}}}"
    print(line)
PY

echo ""
echo "Done. Raw logs + summary inputs are in $OUT_DIR"
