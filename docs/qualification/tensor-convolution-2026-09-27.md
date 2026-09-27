# Forward convolution across tensor backends

Date: 2026-09-27. Grouped 1D/2D/3D cross-correlation now runs through the same
f32 and low-storage contracts on WGSL, CUDA and MLX. Native WGSL/Metal and
MLX/Metal passed; CUDA compiled with NVRTC but still needs NVIDIA execution.
This phase does not implement f64 tensors or replace existing f64 geometry.

## Contract and implementation

[ConvPlan](../../crates/tensor-core/src/convolution.rs) validates input
`[N,C,*spatial]`, weights `[O,C/groups,*kernel]`, positive stride/dilation,
asymmetric before/after zero padding, groups and checked output geometry.
Inputs and weights retain their channel order; kernel coordinates are not
reversed. Bias can be a separate resident broadcast addition. Empty outputs
are valid. Zero channels and padding-only windows produce zeros. All owners,
dtypes and logical geometry are checked before empty shortcuts.

`TensorConvBackend::conv` returns f32. `TensorLowConvBackend::conv_low_f32`
reads f16/BF16 storage directly into f32 accumulation; `conv_low` applies one
final nearest-even cast. No full operand expansion or im2col tensor is needed.
Finite intermediate arithmetic is required. Parallel reduction and FMA may
change rounding. Normal products of BF16 subnormals with large finite partners
must survive; other f32 underflow remains backend-dependent.

- [WGSL](../../crates/compute-core/src/tensor/convolution.rs) and
  [MLX Metal](../../crates/compute-mlx/src/metal/convolution.metal) split each
  output contraction over 256 lanes, reduce 1 KiB of shared partials and reuse
  workgroups beyond the grid limit. Low loads protect tiny BF16 products.
- [CUDA](../../crates/compute-cuda/src/convolution.cu) uses one thread per
  output and shared templated traversal for native f32/u16 input. Existing
  NVRTC options disable FTZ and FMA contraction. No cuDNN or Tensor Core
  instruction use is claimed for this direct implementation.
- WGSL exposes `tensor_conv`, `tensor_conv_low_f32`, `tensor_conv_low` and
  `_into` variants in reusable `ComputeProgram`. Output aliases and malformed
  views are rejected before recording; odd low output offsets preserve their
  neighboring halfwords. Replay rewrites zero results too.
- CUDA/MLX expose eager or lazy resident calls. Their prepared/compiled
  program builders do not yet record convolution. MLX uses its existing custom
  Metal ABI and kernel cache, with no new FFI dependency.

WGSL restricts descriptor coordinates and contraction counts to u32; CUDA
restricts active coordinates to u32 while retaining u64 physical addresses.
MLX carries options in unsigned 64-bit metadata and uses native view strides.
Backend allocation limits also apply. These are limits of the implementations,
not a measured peak-memory guarantee. No performance improvement is claimed.

## Verification

[checks.json](tensor-convolution-2026-09-27/checks.json) records exact commands,
mandatory-backend variables, exits and elapsed times. The
[native report](tensor-convolution-2026-09-27/native/report.json) includes the
full shared tensor and existing resident geometry regression on WGSL and MLX.
[host.json](tensor-convolution-2026-09-27/host.json) records the host/toolchain.

The shared [f64 oracle](../../crates/tensor-core/src/conformance/convolution.rs)
derives coordinates and output shapes independently of ConvPlan. Exactly
representable fixtures cover all ranks, grouped/depthwise channels, asymmetric
padding, stride, dilation, permutations, broadcasts, empty dimensions and
contractions around 256 lanes. An inexact f32 dot uses a sum-of-absolute-products
error bound. Low fixtures independently check final rounding, cancellation and
both signs/orders of tiny-times-large products.

| Check | Evidence and scope |
| --- | --- |
| Shared contracts and oracle | [CPU log](tensor-convolution-2026-09-27/native/contracts.txt); seven new CPU tests |
| WGSL convolution | [Native suite](tensor-convolution-2026-09-27/native/wgsl.txt); four new entries include shared f32/low fixtures, strided/offset replay, preflight and 65,537-output grid reuse |
| WGSL kernel validation | [Unit log](tensor-convolution-2026-09-27/wgsl-unit.txt); f32/f16/BF16 shader specializations validate with Naga |
| MLX convolution | [Native suite](tensor-convolution-2026-09-27/native/mlx.txt); five new native entries plus one CPU metadata test, including huge unsigned stride/dilation and padding-only windows |
| CUDA host | [Host suite](tensor-convolution-2026-09-27/cuda-host.txt); three new metadata tests and compiled native fixtures |
| Required CUDA convolution | [Required log](tensor-convolution-2026-09-27/cuda-required.txt); three failures reporting absent CUDA driver/device, zero CUDA execution passes |
| Strict Clippy and rustdoc | [Clippy](tensor-convolution-2026-09-27/clippy.txt), [Rustdoc](tensor-convolution-2026-09-27/rustdoc.txt) |
| WASM library and architecture | [Compile-only](tensor-convolution-2026-09-27/wasm-check.txt), [Dependencies](tensor-convolution-2026-09-27/architecture.txt); no browser execution proof |

The first MLX run caught a Metal reserved identifier (`kernel`); renaming it
to `kernel_volume` fixed compilation. The historical failure is retained in
[diagnostics](tensor-convolution-2026-09-27/diagnostics/mlx-initial-failure.txt).
Final native checks run the corrected source. Cargo still emits the existing
unrelated `wgsl_export` manifest naming warning.

## CUDA compiler evidence and precision boundary

The existing container qualifier compiled all **54 entry ABIs** with NVRTC
12.8.93 for **compute_70, compute_80, compute_90 and compute_120**. Two new
entries are `conv_f32` and `conv_low`. The
[compiler report](../../crates/compute-cuda/qualification/nvrtc-12.8.93-linux-aarch64-convolution/report.json)
retains source, compiler-library, image, option and PTX hashes. Compilation
does not prove numerical execution, performance or actual Tensor Core use.
Reports and logs are committed. Generated PTX is retained locally under the
qualifier's existing ignore rule and can be reproduced; the audit identifies
these files separately. Externally supplied CUDA modules must be regenerated
to expose the two new required entries.

The [source manifest](tensor-convolution-2026-09-27/source-fingerprints.json)
and [artifact audit](tensor-convolution-2026-09-27/artifact-audit.json) identify
the checked code, compiler outputs, commands and test entry counts. Native
tests require GPU availability and do not count absent runtimes as successes.

The [precision requirements](../design/tensor-backends-2026-09-27.md#precision-coverage-including-f64)
now explicitly include native CUDA f64, reported backend precision capabilities
and separately identified software precision where native f64 is unavailable.
f32 is useful for portable GPU execution; it cannot silently replace existing
f64 geometry. Full f64 execution, backward/transposed convolution, prepared
CUDA/MLX convolution, optimized algorithms, NVIDIA hardware qualification and
browser/hardware CI remain open.
