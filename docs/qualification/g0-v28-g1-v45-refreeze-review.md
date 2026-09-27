# G0 v28 / G1 v45 no-claim re-freeze

This append-only amendment preserves G0 v27, G1 v44, and all earlier artifacts byte-for-byte. V44 cannot admit further evidence because a file it binds changed.

The drift is reviewed:

- **`tests/manifoldPlanBackend.test.ts`.** The Manifold-isolation case scans an exact `core/` + `components/` file list. `4f54afc` added `src/components/ModelingFloorGrid.vue` and `72a9a6e` added `src/components/SketchDimensionPanel.vue` without updating that list, so the case failed on `main`. Both components are now listed. Neither imports `manifold-3d`, so the isolation contract the case asserts still holds.
- **Kernel.** Nothing else changed. The geometry kernel WASM, own-Rust source bundle, `THIRD_PARTY_NOTICES.md`, `package-lock.json` and `crates/Cargo.lock` are byte-identical to own-rust-cad-v19, and the own-Rust oracle stays at v3.

V45 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v45), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v20.json` with byte-identical kernel, source bundle, notices and lockfile fingerprints.

The matrix remains 4740 planned work units. V45 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
