# G0 v29 / G1 v46 no-claim re-freeze

This append-only amendment preserves G0 v28, G1 v45, and all earlier artifacts byte-for-byte. V45 cannot admit further evidence because a file it binds changed.

The drift is reviewed:

- **Kernel.** The own-Rust geometry kernel WASM was rebuilt from current crate sources; the feature branch added mesh intersection enumeration plus NURBS distance and trim work, so the rebuilt kernel bytes differ from own-rust-cad-v20.
- **Source bundle.** `crates/brep-core/src/operations.rs`, `crates/brep-core/src/lib.rs`, `crates/geometry-bridge/src/lib.rs` and `src/services/geometry/brep.ts` changed with that feature work.
- **`package.json`.** Gained the `test:cad-roadmap` script.
- **`tests/manifoldPlanBackend.test.ts`.** The Manifold-isolation case scans an exact `core/` + `components/` file list. `src/components/CadQuantityInput.vue` and `src/components/SceneObjectControls.vue` were added without updating that list, so the case failed on `main`. Both components are now listed. Neither imports `manifold-3d`, so the isolation contract the case asserts still holds.
- **`scripts/verify-dist.mjs`.** Two size budgets were bumped per the file's documented convention to cover audited bundle growth (`DirectModeler` and `geometryChunkBudget`).
- **Notices and lockfiles.** `THIRD_PARTY_NOTICES.md`, `package-lock.json` and `crates/Cargo.lock` are byte-identical to own-rust-cad-v20, and the own-Rust oracle stays at v3.

V46 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v46), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v21.json` with the rebuilt kernel fingerprint and current source-bundle fingerprints.

The matrix remains 4740 planned work units. V46 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
