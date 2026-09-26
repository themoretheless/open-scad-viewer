# G0 v21 / G1 v38 no-claim re-freeze

This append-only amendment preserves G0 v20, G1 v37, and all earlier artifacts byte-for-byte. V37 cannot admit further evidence because its frozen bindings no longer match current source bytes.

The drift is reviewed: the committed `38637e43` (brep-topology incremental revalidation via EditScope), `fe93a17f` (raster-core mesh shader variants, SDF GPU batch) and `f75d2e4c` (PolygonMesh on Float64Array/Uint32Array typed arrays) changed own-Rust geometry sources and `crates/Cargo.lock`; the geometry kernel WASM was rebuilt in `fe93a17f`. The working tree was clean at freeze time (`git status` empty), so every bound byte corresponds to a committed state. Kernel semantics are unchanged: the own-Rust oracle suite (44 tests, five fixed direct-evaluator cases) passes byte-identically against the v3 fixture, so the oracle stays at v3 and no migration was required.

V38 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v38), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v13.json`.

The matrix remains 4740 planned work units. V38 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
