# vr-core

Independent VR scene kernel, usable natively or as WebAssembly. No browser,
rendering backend or CAD engine dependency.

- `prepare_scene`: row-major instance transforms, CAD Z-up → XR Y-up,
  indexed bounds, centered fit with longest dimension 0.6 metres.
- `anchor`: column-major XR viewer pose → scene pose 1.2 metres forward.
- `prepare_bytes`: checked little-endian binary transport for the WASM host.

Visibility, isolation and application material selection belong to the host.
The host also owns WebXR sessions, permissions and GPU resources.

## Build and test

From the repository root:

```
cargo test --locked --manifest-path crates/Cargo.toml -p vr-core
npm run build:vr
```

The generated module is bundled separately as `vr-core-bytes`. It has no imports
and is instantiated on first use. `src/services/vrKernel.ts` is its browser adapter;
VR scene and session tests execute this actual WASM module.

## WASM ownership

`vr_alloc(length)` returns an owned byte buffer. `vr_prepare(pointer, length)`
reads it without consuming it and returns a new output buffer, or zero for invalid
input. The first little-endian u32 of a successful output is its total byte length.
`vr_free(pointer, length)` releases either allocation; the exact original length
is required. `vr_anchor(pointer)` updates one allocated 64-byte pose in place.
Pointers and lengths are trusted host ABI arguments; mesh data is validated.
The host reacquires memory views after calls that may grow memory and copies output
before freeing it. This module has no retained scene state.
