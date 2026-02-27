# Performance Pass v1 Closeout (Feb 27, 2026)

This report summarizes completed v1 profiling and optimization evidence used to scope v2 work.

## Scope

- Baseline and stress scenarios executed through test mode (`ACCRETION_TEST=...`) with repeated runs.
- Frame-tail metrics collected from repeated benchmark logs.
- PostUpdate schedule timings collected from profiler overlay summaries.
- Allocator evidence collected with `ACCRETION_ALLOC_PROFILE=1` (clean 5x sweeps).

## Consolidated v1 Summary

| Scenario | Frame p95 med (ms) | Frame p99 med (ms) | PostUpdate p95 med (ms) | Peak live med (B) | Total alloc med (B) | Calls med (alloc/dealloc/realloc) |
|---|---:|---:|---:|---:|---:|---:|
| `baseline_225` | 17.05 | 17.31 | 0.117 | 662,962 | 4,155,383 | 3,872 / 2,754 / 1,882 |
| `all_three_225_enemy5` | 16.94 | 17.07 | 0.121 | 729,504 | 4,591,119 | 5,984 / 4,827 / 2,438 |
| `mixed_content_225_enemy8` | 16.89 | 17.04 | 0.153 | 1,531,000 | 6,875,309 | 12,552 / 10,089 / 4,451 |
| `mixed_content_324_enemy12` | 16.92* | 17.10* | 0.150 | 1,321,926 | 7,734,395 | 10,351 / 8,169 / 3,952 |

\* Frame-tail medians for `mixed_content_324_enemy12` are from the repeated high-load set that introduced this scenario; allocator medians are from the clean 5x allocator sweep.

## Interpretation

- Frame tails are still clustered just above the 16.7ms budget in heavier scenarios.
- PostUpdate p95 remains low relative to frame budget, indicating it is not currently the dominant wall-time contributor.
- Allocation pressure and call volume rise sharply in mixed-content scenarios, especially `mixed_content_225_enemy8`.

## v2 Candidate (single highest-leverage target)

- **Target:** reduce mixed-content allocation churn in formation/contact and projectile-heavy update paths.
- **Why:** allocator medians and call counts scale faster than PostUpdate scheduler tails under stress, suggesting memory churn is a stronger v2 leverage point than raw PostUpdate scheduling overhead.

## Evidence Artifacts

- Repeated high-load frame + PostUpdate logs: `artifacts/perf/2026-02-27/high_load_repeat/`
- Allocator 5x sweep logs: `artifacts/perf/2026-02-27/alloc_profile_repeat/`