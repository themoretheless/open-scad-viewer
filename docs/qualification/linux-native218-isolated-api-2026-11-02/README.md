# Isolated Linux-compiler WASM API checkpoint

The 218-capability immutable source snapshot compiled with exit 0 using the pinned Linux compiler; Binaryen optimization ran on macOS. This is not a hosted CI build. The candidate SHA-256 is `13d57db09409356dd9a7fa410aeb78937f73010915e77ada400823b3d0857fda`, 15,908,186 bytes. It excludes later NACA5 and CST native source changes.

`tools/browser-qualification/isolated-candidate-direct-api.mjs OUTPUT CANDIDATE` packs into a private output directory and overrides only that Vite server's module loading and WASM response. It reuses the finite direct API fixtures. Chromium passed 28 main-thread and 28 qualification-worker checks with no page errors. Production public WASM, generated identity and packed bytes hashes remained unchanged. No candidate hash was added to provider admission or delivery budgets.

These observations do not establish production packaging, G0/G1, whole-UI, full workspace, STEP round-trip or main delivery. The original source manifest and compiler/build details accompany the candidate receipt. The candidate binary remains in the local diagnostic directory, not production.
