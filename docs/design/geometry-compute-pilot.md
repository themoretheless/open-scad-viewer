# Geometry compute: verified surface-area pilot

The Inspect dock now exposes **Compute surface area on GPU** for a selected
part in a current full-quality scene. This is a real WGSL compute dispatch,
separate from the viewport's vertex/fragment rendering. It analyzes the
published mesh and does not change language routing, Manifold CSG, exports,
the kernel's metrics, or any immutable qualification manifest.

## Operation and contract

`GeometryComputeBackend` currently has CPU and WebGPU Compute implementations
for world-space triangle surface area. The orchestration function validates
and computes a CPU reference first, then invokes the GPU backend. GPU work
uses 128 invocations per workgroup and a barrier-synchronized reduction. Only
one float per workgroup is read back; the host sums the partial results.

Both implementations transform local edge vectors by the affine linear
transform. Translation cancels before arithmetic, avoiding subtraction of
large translated world coordinates. Nonuniform scales, shear and reflections
are supported. Projective matrices are rejected. Area counts all triangles,
including duplicates and internal surfaces; it is not the boundary area of a
new geometric union, a printability guarantee, or a visibility/section metric.

CPU uses JavaScript double arithmetic with compensated summation. GPU uses
f32 arithmetic. A GPU result is accepted only if its absolute difference from
the CPU reference is at most `1e-8 + 5e-4 * abs(reference)`, in squared model
units. It remains approximate. Missing WebGPU, timeout, device failure or a
failed comparison returns the CPU reference with an explicit fallback status.
Malformed input and cancellation do not produce successful results.

Limits are 1,000,000 triangles and 64 MiB of interleaved vertex data, further
restricted by device storage-buffer/workgroup limits. The pilot owns a private
device per request, bounds the GPU attempt to five seconds, and destroys its
buffers/device on completion, failure and cancellation. A late device created
after cancellation is destroyed without dispatch. This is application-level
resource cleanup, not an OS-enforced GPU termination guarantee. The viewport
device is never destroyed by compute cleanup. See the
[WebGPU specification](https://www.w3.org/TR/webgpu/) for mapping and device
resource lifecycle semantics.

The UI permits one active analysis. Changing source, selection or scene
aborts it and clears its result. Explicit Cancel and unmount do likewise.
Results are scoped to that selection/scene and are not persisted.

## Performance boundary

This is an opt-in correctness pilot, **not an automatic acceleration policy**.
Every request also runs CPU verification. CPU timing includes validation and
cooperative yields; GPU timing includes adapter/device acquisition, pipeline
creation, upload, compute, readback and validation scopes. Neither number is
an isolated shader execution timer. A live 8,192-triangle sphere run measured
approximately 3.8 ms CPU versus 45.4 ms GPU end-to-end. That run demonstrates
correct GPU execution, not a speedup.

Next candidates are a retained private device/pipeline, buffer reuse, threshold
benchmarks and moving the compute host into a Worker. Removing per-request CPU
verification requires a separate numerical qualification decision. This
browser pilot has no native compute provider; the native Rust kernels have
their own opt-in placements (`Acceleration::Gpu` via wgpu, `Acceleration::Cuda`
via the CUDA driver API) — see [native GPU and CUDA acceleration](native-gpu-cuda.md).

## Verification

- Unit checks: transformed area, invalid data, GPU disagreement/failure,
  unsupported WebGPU, cancellation, late device disposal and hung acquisition.
- With the development server running, open
  `/tests/fixtures/geometry-compute-smoke.html` and press **Run compute checks**.
  It uses actual WebGPU and fails unless every case runs on the GPU and agrees
  with its expected area. It never reads/writes workspace state.
- Live browser checks passed for a partial workgroup, 129 triangles crossing a
  workgroup boundary, reflection/nonuniform scale with huge translation, and
  a degenerate triangle. The product UI also returned the expected 10,584 area
  for a side-42 cube and matched the sphere mesh's area.
