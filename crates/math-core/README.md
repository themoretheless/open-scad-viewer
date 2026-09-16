# osv-math

Published name for the workspace `math-core` leaf: small dense `f64` vector
helpers (`V2` / `V3`) and a shared typed `Error` / `Result`.

Rust crate name remains `math_core` (`use math_core::…`). Builds on **stable** Rust.

```toml
[dependencies]
math-core = { package = "osv-math", version = "0.1" }
```

## GPU / CUDA batch transform

`transform_points_accelerated(points, m, t, acceleration)` batches `q = M*p + t`
over a point set. `Acceleration::Cpu` (default) is the exact f64 reference
(`transform_points`); `Acceleration::Gpu` (feature `gpu`) runs a portable wgpu
compute shader in f32; `Acceleration::Cuda` (feature `cuda`, needs the `gpu`
feature too) runs a CUDA driver-API PTX port, falling back to the wgpu shader
and then the CPU reference when a device or kernel is unavailable. Both
optional features require nightly Rust (through `gpu-compute`); regenerate the
checked-in PTX with `npm run build:cuda-kernels`.

## License

MIT — see repository root `LICENSE`.
