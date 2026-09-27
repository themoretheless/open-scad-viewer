# Device-index scatter qualification

Date: 2026-09-27. This extends the [reduction/vector checkpoint](tensor-reductions-2026-09-27.md).

## Contract and implementation

`TensorScatterBackend` provides f32/u32 Replace, Add, Multiply, Min and Max.
It returns updated values with the base shape and a device-resident scalar
`invalid_count`. Inputs remain unchanged. Updates may broadcast to the shape
that gathering the same indices would produce. Invalid indices leave the base
unchanged and contribute once to the count, independently of other dimensions.

Replace deterministically selects the last logical index when indices repeat.
This refers to row-major index order, including for transposed or broadcast
index storage. Other operations fold the base with every valid update. u32
addition and multiplication wrap; f32 arithmetic allows parallel ordering and
requires finite inputs and intermediate values.

WGSL and CUDA initialize a distinct output from the base, elect Replace owners
with an unsigned maximum, then allow only winning updates to write. Other modes
use unsigned atomics where available and bitwise compare-and-swap for the
remaining folds. MLX uses native scatter reductions; Replace elects owners and
gathers the final winning slices. The algorithms scale with the base, indices
and expanded updates, without comparing every index to every output.

CUDA f32 Add deliberately uses compare-and-swap to keep the module's arithmetic
policy: native global `atom.add.f32` flushes subnormal values and results.
[NVIDIA's CUDA 12.8 PTX documentation](https://docs.nvidia.com/cuda/archive/12.8.0/parallel-thread-execution/index.html#parallel-synchronization-and-communication-instructions-atom).
NVIDIA execution remains required to validate the emitted kernels numerically.

WGSL prepares complete multi-step operations before appending them to a reusable
program. Every replay resets owner scratch and invalid counts and copies the
current base. Output offsets, alias checks and foreign-runtime checks apply to
both values and counts. CUDA supports input/update aliases by copying the base
into fresh output storage. MLX sanitizes invalid indices before native indexing
and uses neutral updates so invalid writes cannot affect the result.

## Shared reference fixtures

The common fixture uses independent serial coordinate references for all five
modes and both types. It covers strided base/index/update tensors, broadcast
updates, each axis of a 3D tensor, scalar indices, empty base/target/index
dimensions, values above 2^24 and u32::MAX, overflow, deterministic duplicate
resolution across 65,539 repeated indices, and untouched base storage.

A resident scatter → gather → scan chain checks composition and both invalid
counts. CPU shape tests separately validate broadcast rejection, invalid axes,
u32 count limits and expanded-update size overflow. The shape tests and
conformance code also compile for wasm32.

## Local results

| Check | Result |
| --- | --- |
| CPU shape/layout/indexing/reduction/scatter contracts | 17 tests pass |
| Full platform/compute/math/raster regression | 240 tests pass, zero ignored, GPU/timestamps/subgroups required |
| WGSL tensor integration | 27 actual Metal tests pass, including four new scatter tests |
| MLX | 22 actual GPU integration tests plus two loader/error tests pass |
| CUDA host contracts | Eight tests and the compile doctest pass |
| CUDA source compilation | All 27 entries compile for compute_70/80/90/120 with NVRTC 12.8.93; parameter-width ABI matches |
| Required CUDA execution | Fails explicitly because the driver/device is unavailable on this host |
| Static checks | Strict Clippy/rustdoc, architecture rules, WASM library/test compilation pass |

The current CUDA source and all four emitted PTX hashes were verified against
the [new compiler report](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-scatter/report.json).
The f32 scatter body in compute_80 PTX uses CAS with ordinary `add.rn.f32` and
`mul.rn.f32`, without `ftz` or native float atomic addition. This establishes
the emitted instruction policy, while numerical execution remains unverified.
The previous 19- and 23-kernel snapshots are preserved.

WGSL-specific fixtures add repeated programs with changed owners/indices,
4,097 contending updates, offset sentinels, cross-type aliases and transactional
rejection of an expanded update shape with 2^32 elements. The shared index-count
helper is also used by gather; its existing tests pass after the extraction.
MLX fixtures add mixed updates, native unsigned folds, duplicate ordering,
empty dimensions, dtype validation and foreign-runtime rejection.

### Reproduction and evidence

```sh
COMPUTE_REQUIRE_GPU=1 COMPUTE_REQUIRE_TIMESTAMPS=1 COMPUTE_REQUIRE_SUBGROUPS=1 \
  cargo test --offline --locked --manifest-path crates/Cargo.toml \
  -p gpu-compute -p compute-core -p raster-core -p osv-math \
  --features osv-math/gpu -- --test-threads=1

python3 scripts/qualify-tensor-backends.py --offline \
  --backend wgsl --backend mlx --output crates/target/tensor-scatter-qualification
```

The runner now includes scatter tests. Add `--backend cuda` on a provisioned
NVIDIA host; it is a required hardware gate and cannot pass by skipping CUDA.

- [Full regression output](tensor-scatter-2026-09-27/gpu-regression.txt).
- [WGSL scatter and indexing output](../../crates/compute-core/benchmarks/tensor-scatter-metal-tests.txt).
- [MLX native output](../../crates/compute-mlx/qualification/scatter-metal.txt).
- [CUDA source compilation and historical reports](../../crates/compute-cuda/qualification/README.md).

## Qualification boundaries

No throughput claim is based on these correctness tests. Atomic retry work grows
under contention, and f32 fold order is intentionally not bitwise deterministic.
Only Replace's duplicate selection has a deterministic last-index contract.

This host has an Apple GPU. CUDA source compilation cannot prove CUDA numerical
execution, cuBLAS results or Tensor Core selection. Browser execution, remote CI
and deployment packages remain outside the local qualification.
The existing workspace warnings for the `wgsl_export` binary name and duplicate
example output names remain; strict crate-level Clippy and rustdoc pass.
Changed and new Rust files satisfy rustfmt. The whole-package format check still
reports pre-existing differences in five untouched compute-core files:
`examples/bench.rs`, `examples/chain.rs`, `src/error.rs`, `src/program.rs` and
`tests/kernels.rs`.

Native f16/bf16 storage, casting, CUDA/MLX graph reuse, remaining numerical
operations and domain integration remain in the [active design](../design/tensor-backends-2026-09-27.md).
