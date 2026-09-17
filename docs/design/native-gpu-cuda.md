# Native GPU and CUDA acceleration

Heavy native kernels can run on the GPU through two opt-in placements of
`math_core::Acceleration`. `Cpu` stays the deterministic, bit-identical
reference; every accelerated path falls back to it on any failure.

| Placement | Backend | Hardware | Feature |
|-----------|---------|----------|---------|
| `Gpu` | wgpu compute shaders (WGSL) | Vulkan (Linux/Windows, NVIDIA/AMD/Intel), DX12 (Windows), Metal (macOS); the same WGSL runs in the browser through WebGPU | `gpu` |
| `Cuda` | CUDA driver API, PTX kernels | NVIDIA | `cuda` (implies `gpu`) |

## Dispatch order

For `Acceleration::Cuda` a kernel tries, in order:

1. its CUDA PTX port (when the crate ships one and a CUDA device exists);
2. its wgpu shader (Vulkan/DX12 reach the same NVIDIA device);
3. the CPU reference.

Kernels without a CUDA port treat `Cuda` as `Gpu` through
`Acceleration::is_gpu()`, so requesting CUDA is never slower than requesting
the portable GPU path on the same machine.

## Kernel ports

| Crate | Kernel | wgpu (`gpu`) | CUDA (`cuda`) |
|-------|--------|--------------|---------------|
| `math-core` | Exact nearest-neighbor and top-2 nearest-neighbor batches | `nearest_neighbor.wgsl`, `nearest_two.wgsl` | `nearest_neighbor.cu` / `nearest_two.cu` → PTX |
| `math-core` | One-to-one squared-distance vector/sum batches | `distance_pairs.wgsl`, `distance_pair_sum.wgsl` | `distance_pairs.cu` / `distance_pair_sum.cu` → PTX |
| `sdf-core` | Grid sampling of primitive/CSG/mesh-distance fields (`polygonize_accelerated`) | `SDF_WGSL`, cached grow-only buffers | `sdf_grid.cu` → `sdf_grid.ptx`, cached grow-only buffers |
| `geometry-bridge` | Lattice implicit field (`lattice_accelerated`) | `LATTICE_WGSL`, cached grow-only buffers | runs the wgpu shader |
| `photogrammetry-core` | Descriptor matching | cached WGSL pipelines/buffers | `matching.cu` → `matching.ptx` |
| `photogrammetry-core` | Frontoparallel NCC depth sweep | cached WGSL pipelines/buffers | runs the wgpu shader |

The CUDA and WGSL kernels are line-by-line ports of the same text and are
tested against each other (`cuda_and_wgpu_samplers_agree`, tolerance 1e-3 in
f32) and against the CPU reference (`cuda_sampling_matches_cpu_field_within_tolerance`).

## Building and running

The `cuda` feature does **not** require the CUDA toolkit: `gpu-compute` loads
`nvcuda.dll`/`libcuda.so` at run time (cudarc, `dynamic-loading`) and the
kernels are committed as PTX that the driver JIT-compiles for the installed
GPU. Without the driver or a device, `CudaDevice::new()` returns `None` and
the dispatch order above continues.

```sh
cargo test --manifest-path crates/Cargo.toml -p sdf-core --features cuda
cargo run --release --manifest-path crates/Cargo.toml -p sdf-core --features cuda --example bench_gpu
cargo run --manifest-path crates/Cargo.toml -p sdf-core --features cuda --example backend_report
cargo run --manifest-path crates/Cargo.toml -p geometry-bridge --features gpu --example backend_report
PHOTO_ACCELERATION=cuda cargo run --release --manifest-path crates/Cargo.toml \
  -p photogrammetry-core --features gpu --example reconstruct -- out.ply FOCAL a.ppm b.ppm
```

`OSV_CUDA_DEVICE=<ordinal>` selects a device other than 0. The photogrammetry
FFI host accepts acceleration code `2` for CUDA next to `0` (CPU) and `1` (GPU).

Regenerating PTX after editing a `.cu` file needs `nvcc` (and MSVC's `cl.exe`
on Windows; the script locates it through vswhere):

```sh
npm run build:cuda-kernels          # rewrite crates/**/*.ptx
npm run build:cuda-kernels -- --check   # CI: fail when a committed PTX is stale
```

The PTX targets `compute_75` (Turing and newer) and its ISA version follows
the nvcc release used to generate it (currently CUDA 13.x → an R580+ driver).

## Measured (RTX 5090, Windows, release, `bench_gpu`)

| Field | CPU | wgpu (Vulkan) | CUDA |
|-------|-----|---------------|------|
| Smooth-union primitives, 64³ grid | 65.5 ms | 58.6 ms | 58.1 ms |
| Mesh distance, 1088 triangles, 16³ grid | 324.7 ms | 1.5 ms | 1.5 ms |

The primitive field is dominated by CPU marching-tetrahedra extraction, which
neither placement moves off the CPU; the brute-force mesh-distance sampling
is where the device placements pay off (~270× here).

## Boundaries

- Mesh booleans (`polygon-core`), slicing and tessellation remain CPU-only.
- Extrude, Revolve and Deform SDF fields have no flat encoding and stay on the
  CPU under every placement.
- Browser builds keep both native features off; the viewer uses WebGPU
  directly with the same WGSL text.
