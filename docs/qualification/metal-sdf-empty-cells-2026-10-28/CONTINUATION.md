# SDF bottleneck qualification continuation

Current runner: run-1793135914010755000. App exec session13363. Log /tmp/sdf-empty-measure.log. Exactly one observer; do not start a duplicate. Frozen binary recorded in binary.txt and environment.json.

New task: identify full polygonize_accelerated bottleneck at large grids and promote only AA-adjusted full-path proven optimization. Experimental same-sign cube skip is test-only, production unchanged. All16 native tests passed,4 ignored;4 observer safety tests passed.

Read protocol.md. Worker first waits naturally for idle CPU/GPU and collects accepted stage profile, then21 AA-only and21 full AB+AA windows, preserving contaminated attempts and ending on clean numeric failure. Never signal other processes. If sources drift, preserve edits, investigate and restart whole matrix from fresh frozen binary.

When finished run analyze.py on this run, review accepted profile medians to establish dominant cost, all raw/adjusted three-repeat intervals and GPU8/CPU8 negative controls. These negative controls are intentionally below candidate's >=32 threshold; large CPU cases ARE affected, hence not negative controls. All negative control intervals should include one. Profile stage costs are diagnostic and not paired gain evidence.

Only after confirmed improvement and no confirmed regression promote production skip_empty from false to grid.cells.iter().all(|&n|n>=32); keep test reference switch. Rebuild and verify production/candidate equivalence and required tests. If unproven do not enable. Preserve raw data/history, finish report with limitations. Do not claim bottleneck or speedup before measurements. No commit/push requested.

Existing heartbeat sdf is ACTIVE every10min on this chat, updated to this new qualification; suppress unchanged waiting updates, pause only after final measured decision and production verification. No duplicate observer.

## Recovery 2026-10-28, latest state
Previous observer disappeared, termination provenance unknown, with zero profile/AA/AB samples. Old telemetry and interruption.json preserved. Five externally changed Cargo/GPU runtime files required fresh build; local wgpu, openscad-gpu AND naga sources now included in frozen snapshots. First rebuilt GPU tests failed on MSL trailing commas generated from two legal WGSL call trailing commas. Removed ONLY those two redundant commas in sdf-core/src/wgsl.rs (no arithmetic/shader/synchronization changes); current GPU-required tests:16 passed,4 ignored. Preserve recovery-native-tests.log containing failure and recovery-native-tests-v2.log containing success. This compatibility fix is not a performance improvement.

CURRENT series run-1793175191840089000, frozen binary sdf_core-889da10b952c8af3, session35375. Persistent stdout docs/qualification/metal-sdf-empty-cells-2026-10-28/worker-recovery.log (NOT /tmp). Prior series supplies no final estimates. One worker, all original gates. Read current pointer here before any historic paths.

## Recovery v3 — latest, supersedes every historical process/path
Previous run-1793175191840089000 correctly ended before any profile or numerical window when 8 GPU runtime/compiler sources changed externally. source-drift.json preserves file list. No timings reused; preserve all foreign edits. Current source rebuilt offline/locked/release; GPU-required native tests16 passed,4 ignored (recovery-native-tests-v3.log).
CURRENT series run-1793184064541571000, frozen sdf_core-10240261d0727074, session36251. Log docs/qualification/metal-sdf-empty-cells-2026-10-28/worker-recovery-v3.log. Exactly one observer launched after verified predecessor exit. Same profile, all21AA-first then21full, same gates.

## Recovery v5 — CURRENT, supersedes all older pointers
Prior executable under output/ disappeared externally before any profile or AA/AB. Preserve interruption.json and worker-recovery-v3.log; no estimates reused. Frozen binaries now live outside generated output at /Users/themoretheless/.codex/qualification/metal-sdf-empty-cells-20261028/. Observer finalization now records missing evidence as unchanged=false instead of raising before decision.json (5 safety tests pass).

Runtime packages were renamed externally to gpu-runtime/gpu-shader. Preserve all external work; source snapshot list follows current names, all source file types. Current rebuilt native GPU tests initially found builtin distance mistakenly emitted as escaped user identifier; fixed translate_call in gpu-shader/src/back/msl.rs to emit metal::distance for the builtin while retaining escaped local/user helper identifiers. Generated shader regression1passed; GPU-required SDF tests16passed,4ignored. This is a required correctness repair, NOT the candidate performance improvement. Failed and passing logs v4/v5 retained.

CURRENT series run-1793196787614973000, frozen binary sdf_core-c28777ea73af5560 (binary-recovery-v5.txt gives absolute path), exec session60346. Log docs/qualification/metal-sdf-empty-cells-2026-10-28/worker-recovery-v5.log. Exactly one worker. All profile+21AA-first+21full gates unchanged. Candidate production still disabled.
