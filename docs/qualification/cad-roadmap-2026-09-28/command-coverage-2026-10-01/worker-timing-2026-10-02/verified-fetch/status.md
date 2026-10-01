# Verified fetch for browser CAD worker, 2026-10-02

The browser CAD worker composition root now installs the bounded optional network compiler. It fetches the existing geometry-kernel.wasm, verifies exact byte count and build SHA-256 before compilation, and keeps embedded Brotli fallback for missing, corrupt or timed-out resources. Shared kernels and Node/MCP realms retain their default no-network behavior. compileStreamingWasm requires an explicit worker opt-in when window is absent. Every worker still owns an independent WASM instance and cancellation remains termination of that owner.

61 artifact/compiler/runtime/transport tests pass, including no-network default outside a window, worker opt-in verified compilation and digest rejection. Existing deadline/body-bound tests still pass. Typecheck, Vite and dist verification pass. MainSolid worker measures 119,260 bytes; all assets 7,204,548 bytes. Budget growth follows the measured network compiler addition. No Rust binary was rebuilt.

Two sequential native Chrome Canary 157 / Apple Metal measurement scenarios pass: strict console/page errors, GPU active, canceled workers, input validation, localized retry, whole-document Undo/Redo and reload. First measurement round trips now 35.1..39.8 ms, warmup 18.9..22.6 ms; the preceding two embedded-path runs measured 86.8..100.7 ms and warmup 67.4..78.3 ms. Execution remains 0.4..0.6 ms. These are small controlled paired observations, not statistically qualified percentiles or general scene speed.

A third scenario fulfills the WASM URL with a valid eight-byte empty module of the wrong identity. It is rejected, embedded fallback restores actual geometry, and the complete scenario passes. First measurements then take 87..89 ms with warmup 69.6..71.7 ms. Browser resource failure is injected as HTTP 200 with wrong bytes, so strict console errors remain enabled. Missing/stalled resource fallback is covered by compiler tests, not a separate browser run here.

P0 end-to-end latency for all tools and larger scenes remains open; actual paint and pure Rust-only phases remain unmeasured. This is one verified cold-start improvement, not completion of P0–P3 or publication.
