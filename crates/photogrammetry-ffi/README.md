# photogrammetry-ffi

Shared host ABI core for the photogrammetry core: session state, MGV1
binary response envelopes, transport budgets, and the pointer-based buffer
protocol. It is an `rlib` with no `extern "C"` surface of its own — the
export shell lives in `photogrammetry-wasm` (import-free `cdylib` for
`wasm32-unknown-unknown`, built by `scripts/build-photogrammetry.mjs`).

Responses are MGV1 binary envelopes returned as packed u64
(`len << 32 | ptr`), which is sound because wasm32 linear memory pointers
fit 32 bits.

GPU stages (descriptor matching, dense NCC sweep) live in
`photogrammetry-core` behind its `gpu` feature and are exercised by its
native examples; the wasm build stays on the CPU reference.
