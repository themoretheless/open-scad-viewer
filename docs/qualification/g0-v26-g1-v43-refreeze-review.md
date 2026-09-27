# G0 v26 / G1 v43 no-claim re-freeze

This append-only amendment preserves G0 v25, G1 v42, and all earlier artifacts byte-for-byte. V42 cannot admit further evidence because `main` changed bytes it binds.

The drift is reviewed:

- **Geometry kernel.** `5d21f7f` added cooperative mesh analysis and build recovery. It touches `geometry-bridge` (ABI, `mesh`, `mesh_analysis`), `geometry-wasm` exports and `polygon-core` (BVH, edges). `f45a7da` moved `osv-math` native geometry to software binary64. All of these crates are in the geometry kernel's dependency graph (`osv-math` is reached through `brep-topology`, `brep-core` and `geometry-bridge`), so the kernel WASM changes. The kernel is rebuilt with the path-independent build (`scripts/reproducible-cargo.mjs`) on the merged sources. The committed `public/wasm/geometry-kernel.wasm` from `main` was built without path remapping, so it is replaced by the reproducible bytes rather than taken from either side of the merge.
- **Semantics.** The own-Rust geometry source bundle, `THIRD_PARTY_NOTICES.md` and `package-lock.json` are byte-identical to own-rust-cad-v17. The own-Rust oracle v3 (44 cases, `tests/legacyDirectEvaluatorOracle.test.ts`) passes unchanged on the rebuilt kernel, so the oracle stays at v3 and no migration is required.
- **`crates/Cargo.lock`.** `52c8c1a` added optional `osv-math` dependencies on `compute-cuda`, `compute-mlx` and `tensor-core`. The kernel build does not enable them: none appears in `cargo tree -p geometry-wasm --target wasm32-unknown-unknown`.
- **`package.json`.** `5d21f7f` added the `test:cooperative-edges`, `test:cooperative-bvh` and `test:build-recovery` scripts.

V43 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v43), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v18.json`, binding the rebuilt kernel bytes and the current Rust lockfile.

The matrix remains 4740 planned work units. V43 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
