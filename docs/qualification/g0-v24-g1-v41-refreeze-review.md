# G0 v24 / G1 v41 no-claim re-freeze

This append-only amendment preserves G0 v23, G1 v40, and all earlier artifacts byte-for-byte. V40 cannot admit further evidence because its frozen bindings no longer match current source bytes.

The drift is reviewed: the committed `d0f08040` (raster-core shadow-map depth pass, matcap textures, PBR environment maps, compute-core chain example, and the WebGPU renderer/material-control integration) changed raster-core and compute-core sources, WGSL shaders, and renderer services. None of these files are part of the own-Rust geometry kernel source bundle; the geometry kernel WASM, `THIRD_PARTY_NOTICES.md`, `package-lock.json`, and `crates/Cargo.lock` are byte-identical to the own-rust-cad-v15 evidence. Kernel semantics are unchanged: the own-Rust oracle suite (44 tests, five fixed direct-evaluator cases) still passes byte-identically against the v3 fixture on the unchanged kernel, so the oracle stays at v3 and no migration was required.

Tree state at freeze time: clean. The material arc landed as `d0f08040` before this freeze; no uncommitted work-in-progress remained in the frozen G0/G1 bundles.

V41 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v41), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v16.json` with byte-identical kernel, notices and lockfile fingerprints.

The matrix remains 4740 planned work units. V41 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
