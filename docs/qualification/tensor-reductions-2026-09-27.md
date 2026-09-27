# Typed reductions and vector matmul qualification

This report preserves the reduction/vector checkpoint. The following
[scatter qualification](tensor-scatter-2026-09-27.md) records the next extension.

Date: 2026-09-27. This extends the [typed indexing checkpoint](tensor-indexing-2026-09-27.md).

## Implemented behavior

- `TensorReduceBackend` provides f32/u32 sum, product, min and max, plus f32
  mean, over arbitrary unique axes with optional dimension retention. Existing
  sum APIs delegate to the shared backend implementation.
- Empty axes preserve logical values. Empty contractions produce zero for sum
  and one for product. Min, max and mean reject empty contractions that would
  produce nonempty output. Empty output remains valid for every operation.
- u32 sums and products wrap modulo 2^32 without passing through f32. Floating
  inputs and intermediate arithmetic must remain finite; reduction order can
  differ across backends.
- `MatmulPlan` centralizes vector promotion, contraction validation and batch
  broadcasting. Vector × vector returns a scalar; matrix × vector and vector ×
  matrix remove the promoted dimension. Zero-length dot products return zero.
- WGSL preserves strided and broadcast views through metadata, records typed
  hierarchical reductions and supports preallocated outputs with offsets.
  CUDA uses typed resident reductions and the existing cuBLAS path with vector
  promotion. MLX constructs a native lazy graph. No intermediate tensor data
  is read back to implement these operations.

## Native MLX failure found and fixed

The shared vector fixtures exposed a native SIGSEGV in MLX 0.32.1 Metal for
empty-batch matrix/vector multiplication, including `[0,2,3] × [3]` and the
reverse orientation. Shape and precision validation now precede a native empty
array result when the output has no elements. This preserves the mathematical
result without invoking the crashing primitive or moving computation to CPU.

The regression retains both orientations and the common zero-contraction,
zero-stride, transposed and batched vector cases. The final required-MLX run
passes 19 tests: 17 actual GPU tests and two loader/error tests. See
[the MLX evidence](../../crates/compute-mlx/qualification/reduction-metal.txt).

## Common and backend checks

The shared fixture independently maps logical coordinates to output buckets.
It covers every subset of three strided axes, both keep-dimension modes, all
operations and both storage types, broadcast dimensions, empty contracts,
finite extrema, scalar identities, invalid axes and 131,077-element reductions.
Vector references use explicit dot products and checked expected result shapes.

WGSL-specific tests cover lengths around workgroup and hierarchy boundaries,
nonzero input/output offsets, sentinel storage, repeated input updates, alias
and owner rejection. A reusable program records vector/batched-matrix matmul,
matrix/vector matmul and mean in one submission with only final readbacks.

CPU contracts pass 14 tests. CUDA host contracts pass eight tests. All tensor
crates pass strict Clippy and rustdoc. Recorded WGSL APIs and common contracts
compile for wasm32; this does not qualify browser execution.

The final platform/compute/math/raster regression passes **235 tests**, with
zero failures and zero ignored tests. GPU execution, timestamp queries and
subgroups were required. This includes all 23 WGSL tensor integration tests,
shader validation and the existing domain algorithms. The final source policy
retains the specialized f32 sum kernel described below.

```sh
COMPUTE_REQUIRE_GPU=1 COMPUTE_REQUIRE_TIMESTAMPS=1 COMPUTE_REQUIRE_SUBGROUPS=1 \
  cargo test --offline --locked --manifest-path crates/Cargo.toml \
  -p gpu-compute -p compute-core -p raster-core -p osv-math \
  --features osv-math/gpu -- --test-threads=1
```

[Full regression output](tensor-reductions-2026-09-27/gpu-regression.txt).

Actual NVRTC 12.8.93 compilation also passes for the current production source:
all 23 entry points compile for compute_70/80/90/120, and emitted parameter
widths match the host launch ABI. The compiler runs in an isolated Linux
aarch64 container without a CUDA device. Current source hashes were checked
against the [new compiler report](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-reductions/report.json).
The earlier 19-kernel report remains an unchanged historical checkpoint.

## Existing sum performance

A controlled Metal comparison used the frozen original one-workgroup kernel,
the frozen preceding hierarchical kernel, and the current API. Each case used
resident input, identical output/readback, 200 ms warm-up and 31 rotated samples
for separate GPU timestamps and unprofiled host time. Every result was checked
against an f64 reference.

The generic reduction shader increased f32 sum GPU medians by 4.4–7.9% across
five cases. The common planner now retains the unchanged specialized sum kernel
for f32 sum; all other operations use typed generic kernels. The retained path
was within −0.6% to +1.4% of the frozen hierarchical kernel's paired GPU medians.
This preserves its existing schedule without duplicating shape validation or
allocation planning.

Host timing does not establish a general speedup: the broadcast case measured
216.917 versus 231.291 microseconds (+6.6%), despite almost equal GPU medians.
The cause of that host difference is not established. Other host medians were
within about 1%. The comparison is limited to these resident reduction cases;
it does not measure cold startup, complete application workloads or CUDA/MLX
performance. See [the benchmark details and raw samples](../../crates/compute-core/benchmarks/tensor-reductions.md).

## Qualification limits

This host has an Apple GPU and no NVIDIA device. CUDA source compilation and
host-side ABI/shape tests cannot establish CUDA numerical execution, cuBLAS
results or actual Tensor Core selection. Those require the mandatory NVIDIA
conformance and profiler run. No CUDA performance claim is made.

Remote CI and deployment packages have not been executed for this working tree.
The repository still reports its existing `wgsl_export` binary naming warning
and shared example output-name warnings when testing several crates together.
Scatter, native half-precision storage, graph reuse in CUDA/MLX and migration of
domain algorithms remain in the [active design](../design/tensor-backends-2026-09-27.md).
