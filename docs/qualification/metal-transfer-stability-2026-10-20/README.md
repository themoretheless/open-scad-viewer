# Controlled transfer measurements and decoder fix

## Findings and retained changes

The previous single-call measurements mixed CPU copy cost with allocation,
encoding, GPU completion and scheduling. The residual time was much noisier
than the copy being optimized: in the prior 16 MiB records its standard
deviation reached milliseconds and its range reached tens of milliseconds
(`previous-noise.json`). Full verification reads between samples also changed
allocation/cache state. Those records cannot justify choosing size thresholds.

A second issue was real **context-dependent decoder code generation**. Under
this workspace's release profile (`opt-level = "s"`, LTO, one codegen unit),
the production generic f32 decoder compiled to a scalar `ldr s0` / `str s0`
loop. An apparently equivalent standalone typed decoder compiled to a
`memcpy` call. The first diagnostic harness also contained a recursive enum
decoder that changed optimization decisions. Timing either standalone model
was insufficient to decide how the public project API performed.

The final source change makes the production f32/u32 decode a single explicit
bulk copy into newly allocated, correctly aligned typed storage on LE hosts.
No byte-Vec allocation is reinterpreted as a differently aligned Vec. The
trait is sealed to f32/u32, every bit pattern is valid, length is exposed only
after copying, and BE retains scalar little-endian decoding. A new regression
checks unaligned source bytes, independent ownership, empty input, trailing
partial words, random payloads, signed zero, infinities and NaN payloads.
Mapping, snapshot copies, cancellation and GPU synchronization are unchanged.

The before/after decoder comparison uses **the same benchmark source and two
actual project builds**, differing only in the three files recorded by
`decoder-change.patch`. Their reconstructed before/after source hashes were
verified against `decoder-ab-builds.json`. Scalar-loop and memcpy disassemblies,
including symbol binding evidence, are retained in `decoder-*-assembly.txt`,
`typed-decoder-assembly.txt` and `*-copy-symbol.txt`.

| Decision | Evidence |
|---|---|
| Keep borrowed-byte updates | 1–16 MiB CPU speedup reproduced; full-call timing can still be dominated by GPU/host wait noise |
| Keep direct generated updates | Reproduced CPU improvement; 16 MiB full-call speedup in all three controlled runs |
| Reject buffered generated-update alternative | A temporary converted Vec was consistently weaker than direct staging on CPU; full-call evidence was inconsistent |
| Keep mapped RGBA row decoding | CPU and 1–16 MiB full-call improvements reproduced for both padded and aligned rows |
| Reject extra aligned-RGBA bulk shortcut | No repeatable incremental advantage over the retained row path; no special branch added |
| Replace context-sensitive f32/u32 decoder with explicit bulk copy | Confirmed through actual API calls and counterbalanced before/after build comparison |
| Do not claim 4 KiB full-call speedup | Its changes are comparable with identical-code AA controls; no threshold or extra fast path inferred |

All selection statements are scoped to these experiments on M4 Max. Removing
small temporary allocations remains a structural benefit; it is not counted
as a measured small-buffer latency win. No universal FPS or frame-tail claim
is made.

## Matched conditions and controls

All measurements used Apple M4 Max / Metal on AC power, the same Rust toolchain
and release settings, with user-initiated QoS applied only to the benchmark
thread. GPU source data, byte count, prepared affine program and buffer usage
are identical between each pair. CPU batches and GPU call latency are separate.
No stdout or verification read occurs between timed samples. Correctness
checks precede warm-up. Each case warms for 150 ms and records 24 blocks,
alternating ABBA and BAAB; each block averages two A and two B samples.

- CPU read experiments hold one actual GPU-written mapping for both paths;
  they contain no GPU submit, wait or staging allocation inside timed samples.
- CPU update experiments batch uploads and drain the queue outside CPU timing.
- Full update experiments use the same prepared affine kernel and completion
  wait for both paths. Full reads compare both fresh and reusable staging.
- Identical-code `*_aa` controls measure the noise of allocation/wait/scheduling
  rather than treating it as a candidate improvement.
- Three serial processes repeat every phase. `summary.json` reports every
  case, paired log-ratio medians and percentile bootstrap 95% intervals using
  2,000 resamples and fixed seed 712. No sample or slow run is trimmed.

The machine was **not idle**: a separate long-running geometry test and desktop
applications remained active. They were not stopped. Metadata records top CPU
loads, AC power and available thermal reports. `pmset` exposed no temperature
or clock measurements; no fixed-frequency or idle-host assumption is made.
Some AA cases also show nonzero apparent changes, especially at 4 KiB. Cases
whose effect is indistinguishable from those controls remain inconclusive.

### Updates and RGBA, 16 MiB

Ranges below are the median paired A/B ratios in three final controlled runs;
above one favors the retained implementation. Full read numbers use fresh
staging. Complete intervals, other sizes, reuse cases and AA controls are in
`summary.json` and `summary.txt`.

| Retained path | CPU speedup | Full-call speedup |
|---|---:|---:|
| Borrow ready scalar bytes | 1.16–1.22× | 1.06–1.22×; one run's interval includes no improvement |
| Generate directly in queue staging | 1.16–1.19× | 1.09–1.12× |
| RGBA, padded rows | 1.69–1.71× | 1.20–1.26× |
| RGBA, aligned rows | 1.92–1.99× | 1.09–1.14× |

The recurring large f32 regression is explained by the actual scalar decoder,
not by snapshot synchronization. The update/RGBA regressions from exploratory
runs were not reproduced as a systematic effect under the matched controls.
Host/GPU wait variance still prevents a blanket end-to-end speedup statement
for small updates. Existing queue ordering and independent snapshots are kept.

## Actual f32 before/after builds

Standalone decoder results are not used to qualify this fix. Two binaries run
the same public `try_read_f32` benchmark. Three outer blocks use build orders
ABBA, BAAB, ABBA. Each process has warm-up and internal identical-code AA
controls. No compiler/build/test process runs concurrently with these timing
processes. Every process exits successfully and checks complete results.

The table uses only `current_aa` samples: **both halves call the real public
API in that build**. Each process contributes its median, then the two A
processes and two B processes are averaged for its outer block. These are
three observed build comparisons, not a claimed confidence interval over
all workloads. `decoder-ab-summary.json` retains the individual process values.

| Payload | Outer block | Before → after, ms | Speedup |
|---|---:|---:|---:|
| 1 MiB | 1 | 0.292 → 0.201 | 1.45× |
| 1 MiB | 2 | 0.269 → 0.225 | 1.20× |
| 1 MiB | 3 | 0.287 → 0.193 | 1.49× |
| 16 MiB | 1 | 2.596 → 1.966 | 1.32× |
| 16 MiB | 2 | 2.619 → 1.544 | 1.70× |
| 16 MiB | 3 | 2.747 → 1.491 | 1.84× |

4 KiB changed direction between outer blocks (1.06×, 0.99×, 1.03×); no full-call
speedup is claimed there. The bulk implementation does not add a mapping,
GPU copy, queue wait, or new borrowed lifetime.

## Evidence layout

- `cpu-{1,2,3}.csv`, `full-{1,2,3}.csv`: final controlled update/RGBA experiments
  and typed decoder reference experiments, before changing the production
  decoder. The reference f32 timings must not be substituted for API timings.
- `production-{1,2,3}.csv`: actual API baseline showing why a standalone typed
  decoder was an inadequate model of the then-current production decoder.
- `decoder-ab-*.csv`, `decoder-ab-index.json`: counterbalanced builds with the
  final decoder fix. `A` is scalar production, `B` is explicit bulk production.
- `diagnostic-*`: all preliminary experiments, including the misleading bulk
  candidate model. They are preserved separately and excluded from final
  qualification because the model changed codegen, not because they were slow.
- `*-environment.json`: timing/return codes, build hashes and host conditions.
- `decoder-change.patch`: exact, hash-verified source delta for this fix only.

The earlier exploratory report in `../metal-update-readback-2026-10-20/` is
preserved and marked as superseded for timing conclusions.

## Verification and reproduction

Final native validation: **202 passed, zero failures across 39 executables**;
wasm compilation also passed. Logs are `native-tests.log` and `wasm-check.log`.
Benchmark preflight validates complete affine results,
scalar arrays and RGBA row output before warm-up for each candidate/control.

```sh
cargo build --manifest-path crates/Cargo.toml --release -p compute-core --example bench_transfer_stability
python3 docs/qualification/metal-transfer-stability-2026-10-20/run_stability.py --output /tmp/metal-transfer-recheck
python3 docs/qualification/metal-transfer-stability-2026-10-20/analyze_stability.py --input /tmp/metal-transfer-recheck
COMPUTE_REQUIRE_GPU=1 cargo test --manifest-path crates/Cargo.toml -p gpu-compute -p compute-core -p raster-core --tests -- --test-threads=1
cargo check --manifest-path crates/Cargo.toml -p gpu-compute -p compute-core -p raster-core --target wasm32-unknown-unknown
```

For the two-build experiment, build baseline and candidate in an isolated
checkout using `decoder-change.patch`, leaving the benchmark source identical.
Preserve their binaries separately and run the public-API phase in ABBA/BAAB
build order. The measured binary/source hashes are in `decoder-ab-builds.json`;
the before/after results are in `decoder-ab-summary.json`.
