# gpu-compute

GPU platform services shared by `compute-core`, `raster-core` and optional math
adapters. The historical package name is retained for compatibility.

- `GpuContext`: device/queue creation and shared handles. Cloning a context
  shares the existing device; create one at the application boundary.
- Backend/capability reports and workgroup-size heuristics.
- Byte packing, legacy blocking readback and buffer binding helpers.
- Optional `cuda` driver adapter, dynamically loaded without a build-time toolkit.

Native wgpu backend features are selected here. Workspace dependencies pin the
wgpu/naga versions used by compute and raster. Domain algorithms, render state
and automatic CPU/GPU placement remain in the higher-level crates.

[Architecture and migration](../../docs/design/gpu-library-architecture.md)
describes module ownership, compatibility boundaries and remaining work.

`GpuBuffer` and `GpuBufferView` validate context ownership, range, usage and storage
binding alignment. Context identity survives cloning and distinguishes devices
created by separate wgpu instances. Context handles are immutable; field reads
remain available, while struct literals and replacing handles are unsupported.

`ByteReadback` centralizes mapping, cancellation, consumption and timeout errors.
Typed decoding and texture row layouts belong to higher-level crates.
`GpuContext::features` is adapter support; `enabled_features()` is what shaders
may actually use on the device.
