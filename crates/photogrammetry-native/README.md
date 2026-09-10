# photogrammetry-native

Native shell of the photogrammetry host ABI: a thin `cdylib`/`staticlib` that
re-exports the `photogrammetry-ffi` core for non-browser hosts.

```sh
cargo build --release --manifest-path crates/Cargo.toml -p photogrammetry-native
# crates/target/release/libphotogrammetry_native.{dylib,so} and .a
```

Native callers fetch responses through `photo_response_ptr()` /
`photo_response_len()`; the packed u64 return truncates pointers to 32 bits,
which only wasm32 linear memory can promise.

GPU stages are opt-in: build with `--features gpu` (wgpu → Metal on macOS,
Vulkan on Linux/Windows) and call `photo_set_acceleration(1)` per session.

macOS caveat: with some toolchains the cdylib gets a mis-aligned LINKEDIT
string pool that ld64 rejects at client link time — build with
`RUSTFLAGS="-C link-args=-Wl,-no_compact_unwind"` or link the staticlib
(with frameworks CoreFoundation, CoreGraphics, QuartzCore, Metal, Foundation,
IOKit, IOSurface when the gpu feature is on).
