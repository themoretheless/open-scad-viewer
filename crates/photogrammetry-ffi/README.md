# photogrammetry-ffi

Import-free host ABI for the photogrammetry kernel, compiled both to
`wasm32-unknown-unknown` (the browser viewer) and to native dynamic/static
libraries — the same sources, the same pointer-based ABI.

## Native builds

```sh
cargo build --release --manifest-path crates/Cargo.toml -p photogrammetry-ffi
# crates/target/release/libphotogrammetry_ffi.{dylib,so} and .a
```

C/C++/Swift hosts use `native/photogrammetry.h`; `native/smoke.c` is the ABI
smoke (verified: photo_alloc/add/run/clear envelopes, backend selection).
Responses are MGV1 binary envelopes; the packed u64 return truncates pointers
to 32 bits, which only wasm32 allows — native callers read
`photo_response_ptr()` / `photo_response_len()` instead.

GPU stages (descriptor matching, dense NCC sweep) are opt-in per session:
`photo_set_acceleration(1)` after building with `--features gpu` (wgpu → Metal
on macOS, Vulkan on Linux/Windows). Without the feature or an adapter the CPU
reference runs unchanged. Note: `photo_set_acceleration(1)` on blank photos
exercises empty descriptor sets; these bind 4-byte placeholder buffers.

macOS caveat: with some toolchains the cdylib gets a mis-aligned LINKEDIT
string pool that ld64 rejects at client link time — build with
`RUSTFLAGS="-C link-args=-Wl,-no_compact_unwind"` or link the staticlib
(with frameworks CoreFoundation, CoreGraphics, QuartzCore, Metal, Foundation,
IOKit, IOSurface when the gpu feature is on).
