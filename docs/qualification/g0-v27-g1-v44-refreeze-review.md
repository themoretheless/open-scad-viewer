# G0 v27 / G1 v44 no-claim re-freeze

This append-only amendment preserves G0 v26, G1 v43, and all earlier artifacts byte-for-byte. V43 cannot admit further evidence because `main` changed bytes it binds.

The drift is reviewed:

- **Geometry kernel.** `72a9a6e` added sketch dimensions and associative G2 bridge curves to `geometry-bridge`: new `cad_dimensions` and `cad_bridge_curve` modules and a one-line `cad_sketch_trim` change. The bundled `crates/geometry-bridge/src/lib.rs` declares the two modules and registers the two new operations in the dispatcher; existing operations are untouched. The kernel WASM is rebuilt with the path-independent build (`scripts/reproducible-cargo.mjs`) on the merged sources. The committed `public/wasm/geometry-kernel.wasm` from `main` was built without path remapping, so it is replaced by the reproducible bytes rather than taken from either side of the merge.
- **Semantics.** `THIRD_PARTY_NOTICES.md`, `package-lock.json` and `crates/Cargo.lock` are byte-identical to own-rust-cad-v18. The own-Rust oracle v3 (44 cases, `tests/legacyDirectEvaluatorOracle.test.ts`) passes unchanged on the rebuilt kernel, so the oracle stays at v3 and no migration is required.

V44 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v44), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v19.json`, binding the rebuilt kernel bytes and the updated source bundle.

The matrix remains 4740 planned work units. V44 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
