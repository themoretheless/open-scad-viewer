<!-- noninterference-2026-10-21 -->
Current runners observe competing jobs and never suspend them. A busy or contaminated window cannot qualify. Earlier measurements retain their recorded runner hashes; their raw results have not been rewritten.

# SDF batching, full-workflow qualification with CPU/GPU tests suspended, 2026-10-20

The candidate is the previously rejected small-input SDF arena, restored from
`../metal-small-sdf-2026-10-20/final-candidate.patch`. The typed runtime and actual
SDF sampler share the byte `write_small_parts` primitive. Input ranges use storage
alignment, stay disjoint/read-only, and fit in 4096 bytes including padding.
Apple Silicon Metal only; larger inputs/other backends retain the old path.

## Protocol fixed before measurements

Compile and correctness checks first. Then temporarily SIGSTOP only same-user
build/test roots and their descendants, identified by executable identity and
Rust test paths; do not kill them or pause desktop applications. Record initial
states and exact PID/start-time/path identities, preserve already stopped states,
and resume only processes stopped by this experiment. A detached watchdog
restores them if the owner exits or the eight-minute lease expires. The main
runner also restores them in finally on success, failure or interruption.

Before each run require ten seconds without a newly appearing build/test process.
Monitor every 0.5 seconds while timing. Any newly detected competing work makes
the entire run environmentally contaminated, regardless of its timing result;
retain the raw run and retry at most once. Parent-first suspension stops build
workers from being replaced. Monitoring itself is lightweight and identical for
both paths; it does not control GPU clocks or other desktop applications.

1. Three AA-only calibration runs must pass **before any timed A/B run**.
2. Per full workflow case, AA median ratio must lie in [0.97,1.03] and its 95%
   paired bootstrap interval relative width must be <=6%, in every run.
3. Run three A/B + AA full-workflow runs, with the same AA acceptance rule.
4. Retain the integration only if all twelve A/B intervals (four cases x three
   runs) lie wholly above 1, and the full-run controls also pass.

The release ignored fixture calls real `polygonize_accelerated` with explicit GPU:
validation, flattening, parameter generation, uploads, compute dispatch, readback,
CPU extraction, final mesh allocation/drop. The test-only sampler override
selects baseline/candidate on the same device and cannot silently fall back to
CPU. Pipeline/fixture creation and first allocations are outside replay timing.
Sphere and signed tetrahedron on 4^3 and 8^3 cell grids. Inputs are deterministic;
mesh position bits/indices match before each case. Both paths use the original
submission/readback synchronization.

Per case: 500 ms warm-up, 48 alternating ABBA/BAAB blocks, 64 complete calls per
sample. Fixed thread QoS user-initiated. No timed IO or correctness comparisons.
Ratios use paired log medians; fixed-seed 4000-resample bootstrap. No trimming.
The phase selector changes only whether A/B cases are run; timing code is shared
by calibration and full phases. No own build/test runs during qualification
except the selected benchmark itself. Source and binary hashes are recorded.

Tiny grids normally use CPU under Auto; explicit GPU ensures matched Metal work.
No default Auto or large-grid speedup is inferred from these cases.


The preceding isolated attempt did not reach A/B: two AA attempts were
environmentally contaminated. An offscreen ray-query benchmark outside Cargo
paths was then identified, with GPU utilization 66% at inspection. This runner
also recognizes explicit benchmark/test command flags and freezes their
Python/shell launcher, preserving exact identities and resuming them afterward.
Ordinary desktop apps remain running. Zombies are ignored by process monitoring.
Criteria, compiled binary, warm-up, counts and workloads are unchanged.
