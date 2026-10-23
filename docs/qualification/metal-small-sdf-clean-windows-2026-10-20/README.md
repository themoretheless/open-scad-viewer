<!-- noninterference-2026-10-21 -->
Current runners observe competing jobs and never suspend them. A busy or contaminated window cannot qualify. Earlier measurements retain their recorded runner hashes; their raw results have not been rewritten.

# SDF batching: clean per-case measurement windows, 2026-10-20

**Accepted and retained.** The SDF sampler now uses the common `write_small_parts`
primitive for a retained aligned input arena up to 4 KiB on Apple Silicon Metal.
Four primitive-field uploads or five mesh-field uploads become one immediate
queue write. Larger cached inputs and other backends retain the old update path.
No GPU wait or readback synchronization was removed.

Previous evidence: ../metal-small-sdf-isolated-2026-10-20 and
../metal-small-sdf-quiescent-2026-10-20. An unrelated offscreen ray-query benchmark
was identified after the first attempt (GPU utilization 66% at inspection).
Suspending it and its Python launcher produced stable AA controls. Long full
runs still overlapped independently resumed/new Cargo workers; those runs are
retained as environmentally contaminated, not accepted on their favorable times.

This protocol changes only window segmentation: select one existing field/grid
case per process, with the same 500 ms warm-up, 48 ABBA/BAAB blocks, 64 COMPLETE
polygonize_accelerated calls per sample. It does not shorten timed samples or
remove any CPU/GPU operation. The public workflow includes validation, flattening,
uploads, dispatch, copy/map/readback, CPU extraction and final mesh allocation/drop.
Correctness/first-capacity warm-up remains outside replay timing on both paths.

The compiled fixture's only change is a case selector outside timing. Sphere and
signed tetrahedron, each on 4^3/8^3 cell grids, remain the four fixed cases. The
same device/queue and QoS are used within each comparison. No CPU fallback.

Rules fixed before this series:

- Pause only same-user builds/tests, explicit offscreen benchmarks and their
  launchers/descendants; preserve pre-existing stopped state. Never kill them.
- A watchdog and finally restore exact PID/start-time/path identities on every
  exit, within an eight-minute lease. Own ancestors and benchmark are excluded.
- Require three seconds of process quiescence before each short case window.
  Monitor every 0.5 seconds. If another build/test starts or resumes, retain the
  entire contaminated window, suspend the competing work and retry that case
  at most four times. Selection uses only process activity, never its speed.
- Run THREE complete AA-only series (four cases each) BEFORE timed A/B. In all
  twelve windows, AA median must be within 3% of 1 and relative 95% CI width <=6%.
- Then run three complete A/B + AA series. Require those same control bounds
  and all twelve paired A/B 95% intervals wholly above 1 to retain integration.
- No trimming or favorable-run selection. Keep every attempt and its reason.

Only the paused CPU/GPU work is quiesced; desktop applications and dynamic clocks
remain. Other active chats may resume processes, hence per-window environment
qualification. Process monitoring adds the same small overhead to both paths.

Tiny grids normally choose CPU under Auto. These measurements explicitly select
Metal GPU; they cannot establish default Auto or larger-grid end-to-end gains.


## Results

Three AA-only series passed before the first timed A/B case. All twelve clean
full-workflow comparisons then passed, with all twelve 95% paired intervals
wholly above 1. Both calibration and full-run controls passed: across 24 AA
windows, median ratios were 0.9916–1.0091, with maximum relative interval width
2.37%. One environmentally contaminated attempt was retained and retried; no
attempt was rejected for its measured speed.

| Field / cells | Paired speedup across 3 clean runs | Lowest 95% CI lower bound | Baseline -> batch, us/call |
| --- | --- | --- | --- |
| Sphere / 4^3 | 1.044–1.058x | 1.0345 | 170.35 -> 163.80 |
| Sphere / 8^3 | 1.037–1.049x | 1.0299 | 206.62 -> 197.72 |
| Signed tetrahedron / 4^3 | 1.053–1.065x | 1.0454 | 191.55 -> 183.18 |
| Signed tetrahedron / 8^3 | 1.051–1.063x | 1.0384 | 199.96 -> 190.34 |

Absolute columns are medians of the three run medians; ratios are paired block
medians and need not equal the ratio of those aggregate absolute columns.
The paired ratios imply 3.5–6.1% less time for the complete public call in these
GPU replay cases. This does not qualify larger grids or Auto mode performance.
The unrelated matcher candidate remains rejected and is not reintroduced.

## Verification and process restoration

- 14 sdf-core native release tests passed; the timing fixture is normally ignored
  and was explicitly invoked by the qualification runner.
- 7 compute-core shared-upload/small-update tests passed with GPU required.
- compute-core + sdf-core/gpu wasm32 check passed.
- Raw f32 values and final mesh bits/indices match the old path. Tests cover cache
  shrink, >4 KiB fallback/reuse, byte-update alignment/range/size validation before
  mutation, empty updates, exact 4096-byte payloads, aliases and pending snapshots.
- The measured binary and all recorded source/lockfile hashes still matched
  after qualification, before subsequent builds (`source-verification.json`).
- The clean-window experiment held eleven process identities for 228.8 seconds.
  All surviving processes stopped by it were resumed; exact-identity inspection
  found none still stopped (`resume-verification.json`). Previously stopped
  processes were preserved. The watchdog also exited normally.

See `summary.csv`, `trials.json`, every raw attempt CSV, `decision.json`, the pause
journal/events and native/wasm logs. Earlier failed environment-control attempts
remain in the adjacent `isolated` and `quiescent` evidence directories. Their
contaminated favorable results were not counted toward acceptance here.

## Reproduce

The production patch is `retained-change.patch`, with the new fixture archived as
`small_batch_tests.rs`. Build sdf-core lib tests in release with `--features gpu`,
then invoke `qualify.py <absolute-built-test-executable>` from a fresh evidence
copy/directory. The runner refuses to overwrite an existing journal. It pauses
competing same-user builds/tests and explicit offscreen benchmarks only for its
bounded qualification lease; use it only for an authorized quiet measurement
window. No build runs while its calibration/comparisons are active.
