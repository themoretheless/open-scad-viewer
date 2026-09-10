# geometry-native

Native shell of the geometry linear-memory ABI: a thin `cdylib`/`staticlib`
that re-exports the `geometry-bridge` core (`abi`) for non-browser hosts.

```sh
cargo build --release --manifest-path crates/Cargo.toml -p geometry-native
# crates/target/release/libgeometry_native.{dylib,so} and .a
```

Native callers fetch responses through `abi_response_ptr()` /
`abi_response_len()`; the packed u64 return truncates pointers to 32 bits,
which only wasm32 linear memory can promise. Rust consumers can skip this ABI
entirely and use the typed `geometry-bridge` API directly.

The wgpu lattice stage is opt-in: build with `--features gpu` (Metal on
macOS, Vulkan on Linux/Windows).
