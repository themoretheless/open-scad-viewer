# photogrammetry-ffi

Shared host ABI core for the photogrammetry kernel: session state, MGV1
binary response envelopes, transport budgets, and the pointer-based buffer
protocol. It is an `rlib` with no `extern "C"` surface of its own — the
export shells live in:

- `photogrammetry-wasm` — import-free `cdylib` for `wasm32-unknown-unknown`
  (the browser viewer; built by `scripts/build-photogrammetry.mjs`);
- `photogrammetry-native` — `cdylib`/`staticlib` for native hosts, adding
  `photo_response_ptr`/`photo_response_len` and the `gpu` feature.

Responses are MGV1 binary envelopes; the packed u64 return truncates pointers
to 32 bits, which only wasm32 allows — native callers read
`photo_response_ptr()` / `photo_response_len()` instead.

GPU stages (descriptor matching, dense NCC sweep) are opt-in per session:
`photo_set_acceleration(1)` after building the native shell with
`--features gpu` (wgpu → Metal on macOS, Vulkan on Linux/Windows). Without
the feature or an adapter the CPU reference runs unchanged. Note:
`photo_set_acceleration(1)` on blank photos exercises empty descriptor sets;
these bind 4-byte placeholder buffers.
