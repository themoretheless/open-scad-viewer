# CUDA binary64 indexing, scans and scatter — 2026-09-27

Implemented eager binary64 indexing without narrowing values, scan totals,
recursive carries or scatter arithmetic. Native NVIDIA numerical qualification
remains missing on this Apple host. The required CUDA tests fail explicitly;
ordinary test-harness successes that skip unavailable devices are not GPU proof.

## Implemented contracts

- `TensorF64IndexBackend`: six comparisons, u32-mask selection, inclusive or
  exclusive prefix sums in either direction, gather and stable compaction.
- `TensorF64ScatterBackend`: Replace, Add, Multiply, Min and Max. Replace uses
  existing last-logical-index owner election; other modes include the base and
  every valid update. Min/max order signed zeros as `-0 < +0`.
- Values stay f64; masks, indices, counts and elected owners remain u32. Invalid
  indices are counted once each, including when another dimension makes the
  value output empty. Compaction returns full capacity plus a resident count.
- CUDA specializes existing traversal templates and the same multilevel scan
  implementation. Every catalog slot for f64 is loaded explicitly. Native f32,
  u32 and low operations retain their existing kernel entries.
- Scatter folds use 64-bit integer CAS, double arithmetic and integer comparison
  for retry termination. Copy, select, gather, compact and Replace preserve raw
  binary64 payloads. Arithmetic inputs and intermediates must remain finite;
  floating accumulation order can vary across runs/backends.

This supports the migration toward a canonical f64 geometry API. Lower tensor
precisions remain explicit workload choices; the geometry contract must not be
silently narrowed to satisfy backend limitations. The existing geometry callers
have not all been migrated yet.

## Evidence

Exact commands, environments and exit codes: [checks.json](tensor-float64-indexing-2026-09-27/checks.json).
Log trailing whitespace is normalized for storage.

| Check | Result | What it proves |
| --- | --- | --- |
| NVRTC 12.8.93 on compute_70/80/90/120 | PASS, 67 entry points per target | Production CUDA source compiles; all pointer/scalar parameter widths match the launch ABI |
| CUDA host unit tests | PASS, 156 | Includes eight-byte copy/select/scan/gather bounds, plus existing shared planners and prepared-program validation |
| Shared tensor contract tests | PASS | Shape/layout contracts; no native f64 numerical execution |
| CUDA f64 integration, required device | FAIL, 2 base f64 + 3 indexing tests | NVIDIA device unavailable; numerical assertions were not executed |
| CUDA integration without required flags | Unavailable native tests skip | Compilation and portable test execution only |
| Strict all-target Clippy and rustdoc | PASS | tensor-core, compute-core, compute-cuda, compute-mlx, gpu-compute, osv-math/tensor-cuda |
| WASM check | PASS | compute-core compilation only |
| GPU architecture check | PASS | Repository dependency policy |

Cargo still emits the existing raster-core manifest warning for `wgsl_export`.
The changed Rust code passes Clippy with `-D warnings`.

The new fixtures cover f32-collapsed coordinate differences, NaN payload and
signed-zero selection/copy, strided and offset inputs/indices, zero input axes,
invalid counts for empty outputs, stable compaction, resident operation chains,
65,537-element two-row scans requiring recursive carries, every direction and
inclusion mode, repeated scatter indices, 4,097 competing updates, owner errors
for each input, unchanged bases and independent results across repeated calls.
These fixtures compile here; passing on NVIDIA remains a required next gate.

The [NVRTC report](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-float64-indexing/report.json)
records the pinned wheel/image, source hashes, options, all entry-point argument
types and generated PTX hashes. Raw PTX remains local and gitignored. Reports and
compiler logs are tracked. Reproduce:

```sh
bash crates/compute-cuda/qualification/qualify-nvrtc.sh crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-float64-indexing 70 80 90 120
COMPUTE_REQUIRE_CUDA=1 cargo test --locked --manifest-path crates/Cargo.toml -p compute-cuda --test float64 --test float64_indexing --no-fail-fast -- --nocapture
```

No WGSL/MLX implementation changed in this step; native regression evidence from
the [base f64 change](tensor-float64-2026-09-27.md) remains separately dated.
This report does not claim a new hardware run for those backends.

## Remaining requirements

NVIDIA execution/performance, f64 stable statistics, attention, convolution,
prepared/Graph slots, complete domain-call migration, and software binary64 for
WGSL/MLX remain open. The full shader/CUDA/tensor/MLX goal is not complete.
