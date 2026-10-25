# Active SDF qualification — continuation contract

User explicitly requested waiting for naturally free CPU/GPU and completing large-grid/Auto/4-KiB full-workflow paired measurements. Never stop/resume another task. Do not conclude merely because the machine is busy. Preserve the dirty repository; no commit/push requested.

## Running worker

Repository: `/Users/themoretheless/Documents/Sources/open-scad-viewer`.

Command already running:

```
python3 docs/qualification/metal-sdf-large-auto-final-2026-10-21/measure.py /Users/themoretheless/Documents/Sources/open-scad-viewer/output/qualification/metal-sdf-large-auto-final-20261021/sdf_core-2b34184339a3f640
```

Current definitive series: `run-1792625264885333000/` in this directory.
Live stdout: `/tmp/sdf-final-measure.log`. Persistent telemetry: the series' `telemetry.jsonl`. Check the exact Python process with `ps`; do not start a duplicate. Its own fixture children are recognized by descendant ownership. The process was started by the app exec tool (session 91760; PID 83254) and can continue after the foreground turn; no stdin or terminal interaction is required.

The worker waits indefinitely without signalling discovered processes. It admits a window after 15 s without competing compilers/tests/explicit benchmarks, 5 s AGX <=10%, and 5-second mean CPU idle >=80%. It watches every 0.5 s, rejects entire windows containing competitors or post-exit GPU activity >10%, and preserves every attempt. Only its own explicitly created child can be terminated on timeout/cancellation.

There are eight cases: sphere GPU64, sphere Auto64 (CPU), tetra Auto48 (CPU), tetra Auto49/64 (GPU), closed 84/86-triangle bipyramids Auto32, and 84→86→84 Auto32. Aligned input sizes 4048→4120→4048 bracket 4096; fallback stays cached after shrink. Correctness asserts nonempty meshes, exact coordinate-bit/index parity, backend selection and cache state. Transition times cover the three full calls, including first allocation and growth; forced cache clear/pipeline construction are outside timing. A timing row is the full three-call sequence for that case, not one call.

Each mesh/cross sample averages 32 complete workflows; other cases average 8. Warm-up is 3000 ms; all cases retain 48 ABBA/BAAB blocks. Mesh84/mesh86/cross execute first in both phases. Wall start/end timestamps are recorded per block; observer polling cost is recorded in telemetry. All 24 AA-only windows must pass before any AB: 3 windows/case. Full phase has 24 windows, each AB followed by AA. AA median ratio [0.97,1.03], relative 95% CI width <=1.06. A clean numerical AA failure stops, rather than retrying until favorable. Environmental failures wait/retry. See protocol.md.

Earlier calibration run-* folders here are diagnostic only: initial unnecessarily strict CPU gates were corrected before restarting this definitive series. No AB was taken in calibration. The previous definitive series `run-1792580799783489000` failed a clean mesh84 A/A width gate after 15 accepted controls; it is retired and none of its windows count toward this fresh series. See aa-failure-diagnosis.md/json. Source/binary/runner hashes remained unchanged, so the rejection is numerical variability, not a production regression. No AB occurred. Sibling large-auto directories retain previous interrupted/contaminated work; exclude all of them from final estimates.

## Frozen code and tests

Executable is a byte-for-byte frozen copy outside Cargo outputs; already launched successfully. Original build command was `cargo test --manifest-path crates/Cargo.toml -p sdf-core --features gpu --release --lib --no-run --offline --locked`. Normal GPU tests passed: 15 passed, 2 ignored. Four observation/noninterference runner tests passed (`runner-tests-v2.log`); the new native log is `native-tests-v2.log`. Freeze/runner/source hashes and full source snapshots are in the series; the worker checks them before/after windows. Current project included unrelated new SDF variants before this build; do not revert them. The new test fixture adds larger samples and Auto64, and now fixed 32-call mesh averaging plus block timestamps; production SDF upload code is unchanged.

If the process is gone, inspect decision.json and logs first. For a frozen-source change, compare with the archived sources and rebuild current code only before restarting a fresh complete qualification; do not mislabel an old executable with new source hashes. Keep all evidence. For another clean AA failure, correlate block wall timestamps with telemetry and observer_poll_ns; do not blindly raise repetitions again or infer an external client/clock change from aggregate AGX utilization alone. Investigate the actual cause before changing anything; do not loosen numerical gates to obtain a result. A new sampling scheme requires a fresh AA-first series.

## Finishing

If decision.json has completed=true, aa_first_passed=true and unchanged flags true, run:

```
python3 docs/qualification/metal-sdf-large-auto-final-2026-10-21/analyze.py docs/qualification/metal-sdf-large-auto-final-2026-10-21/run-1792592542365850000
```

This requires all 48 accepted clean/stable windows, writes summary.csv and analysis.json, and independently bootstraps AB-minus-AA log ratios. Additional conservative adjustment was fixed in analysis-plan.json before any AB. A gain requires raw AND AA-adjusted lower 95% bounds >1 in ALL THREE full windows. CPU/fallback identical paths (sphere-auto-64, tetra-auto-48, mesh86-auto-32) are negative controls, never improvements. Apply the same conservative test to regressions. The worker's raw confirmed_* lists are preliminary; use analyze.py and review raw data before conclusions.

Investigate confirmed regressions, implement only justified changes and repeat proper pairs. An inconclusive large-grid interval is not a demonstrated regression and does not erase the previously proven small-workload batching benefit. Do not change Auto thresholds or claim large-grid/FPS gains without evidence. Preserve synchronization. Complete focused tests after any production edits, outside measurement windows.

Write final README.md here with actual paired latency ranges/CIs, accepted/rejected windows, branch/4-KiB/cache correctness, source/binary hashes and limitations. Optionally compress source snapshots/telemetry after measurements to avoid hundreds of duplicate source files in Git; preserve evidence. Avoid archiving work during timing. Report completion in Russian with a clickable absolute file link. No fresh noise report while the worker is simply waiting.

## Automatic continuation

App heartbeat id: `sdf`, every 10 minutes on THIS thread. It exists to finish analysis/recovery when the worker completes or needs attention. Never create another heartbeat or another worker for unchanged waiting. After ACTUAL completion, read its configuration and update this existing heartbeat to PAUSED, preserving all other fields. The user's request authorizes this continuation; unrelated tasks must remain untouched.

## Latest human request and reporting precision

The user reiterated: finish the new A/A then full A/B, check whether increased mesh averaging helped, never touch other jobs or weaken gates, retain only proven improvements. The existing worker and heartbeat remain authorized; do not replace them merely because CPU/GPU is busy.

When reporting the averaging check, compare raw A/A bias/CI widths and all three repeat outcomes for mesh84/mesh86/cross against the retired failed 8-call series. The new protocol ALSO increased warm-up from 1500 to 3000 ms and changed case order. A passing new series establishes that the combined protocol is stable; it does not uniquely attribute causality to averaging alone. State this distinction rather than claiming an isolated averaging benefit without a matched fixed-warm-up experiment. Similarly, 84 vs 86 triangles changes geometry cost as well as crossing the padded input boundary: optimize conclusions must use A/B within the SAME field/grid, not absolute timings across those cases.

## Observer revision after second A/A failure

The retired 32-average series `run-1792586808782974000` failed its first clean mesh84 A/A: ratio 0.9836073, CI [0.9592616,1.0221198], relative width 1.06553 >1.06; zero accepted windows and no AB. See aa-32-failure-diagnosis.md/json. The active series above restarts all AA with the SAME frozen binary, sources and sampling. Only observer collection changed: one ps traversal plus read-only proc_pidpath, preserving compiler/test/explicit-benchmark classification, and full cumulative process CPU telemetry. Measured collection overhead fell ~95→57 ms; this does NOT prove observer causality or SDF benefit. If it fails again, use per-process CPU deltas plus block timestamps; do not blindly retry or increase averaging. Live process inspection verified exactly one observer. Sources match the frozen prior snapshot. Four runner safety tests passed.

Observer recovery: waiting-only `run-1792591602581769000` ended before any sample on unresolved libproc identity. Targeted state/comm ps fallback now handles this exceptional path; exited/zombie/stopped processes are skipped, unresolved live identities still fail closed. Four safety tests passed. Current worker was verified as the only observer. No gates, sources or binary changed.

## Current sampling revision (takes precedence over historical descriptions)

Reduced-observer series `run-1792591888080760000` also failed first clean AA: ratio 1.0199924, CI [0.9823954,1.0439447], width >1.06. No AB; see aa-observer-revision-diagnosis.md. New frozen binary above averages adjacent single-workflow ABBA cycles within each block, instead of averaging 32/8 contiguous workflows in each position. Number of complete calls, 48 blocks, three repeats, warm-up, exact timers, backend/cache correctness and all gates unchanged. Fresh ALL-AA-first series. Normal tests: 15 passed, 2 ignored (native-tests-v3.log); four runner safety tests passed. Production code unchanged. Do not restart unchanged numerical failures. If another failure occurs, correlate per-process CPU deltas/block timing and consider whether this machine can qualify the requested precision; never retry for favorable controls.

## Latest human notification requirement

Continue waiting and finish all remaining full A/B windows. Send ONLY the final result after AA-adjusted confidence intervals and negative controls have been reviewed. Suppress intermediate progress, accepted-window-count and unchanged waiting reports, including on heartbeat turns. Existing worker and heartbeat remain authorized and active; no duplicate processes, no relaxed gates, no signals to other tasks. Pause automation only after actual completion.

## Timeout recovery (current active series)

Prior worker exited on its OWN fixture 180-second timeout in full-cross-auto-32-2-attempt-7, during observed competing work. See prior timeout-diagnosis.json; no numerical CSV from timeout and no acceptance. Retain prior decision, telemetry and worker.log. Current series `run-1792614936723125000` resumes the unchanged accepted prefix (24 AA and 7 full windows) after verifying binary/source hashes and prior unchanged flags. Copies original CSV/log evidence unchanged; prior telemetry remains in predecessor directory. Runner now supports --resume-from ONLY for this timeout outcome, never clean numerical failures. Same 180-second limit and all gates. Four safety tests passed. Current runner is exactly one observer, waiting naturally. Source/binary/sampling unchanged; no compilation performed. Analyze current series only after full completion; its environment.json records resumed_from.

## Source drift restart (latest, supersedes timeout continuation)

Resumed worker correctly stopped before next measurement because 17 frozen dependency/source files changed externally (compute/gpu/math/planar/sdf/tensor). Prior binary/runner unchanged; source unchanged flag false. Preserve all foreign edits. Retire prior 24 AA +7 AB estimates from final qualification; do NOT reuse with rebuilt code. Built current code with same release/offline/locked command, froze sdf_core-2b34184339a3f640; 15 native tests passed, 2 ignored (native-tests-v4.log). Fresh series `run-1792625264885333000` starts ALL AA from zero, then full AB under the same sampling and every original gate. No production edits by this task. Exactly one worker PID 83254 verified. Analyze this CURRENT series after completion, not historical command path above. User requires silence until final adjusted/negative-control review.

## Completed 2026-10-22
All 24 AA-first and 24 full AB windows completed in run-1792625264885333000. analyze.py completed; adjusted analysis confirms no new gains and no reproducible regressions. All negative-control intervals include one. README.md is the final report; analysis.json overrides the runner raw-only decision. Preserve prior qualified small-workload batching. No further measurement worker needed.
