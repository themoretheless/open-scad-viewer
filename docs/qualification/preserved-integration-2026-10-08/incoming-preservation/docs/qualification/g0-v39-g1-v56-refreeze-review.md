# G0 v39 / G1 v56 no-claim re-freeze

This append-only amendment preserves G0 v38, G1 v55, and all earlier artifacts byte-for-byte. V55 cannot admit further evidence because files it binds changed.

The drift is reviewed:

- **Manifold library.** The manifold library reached production scope: Full-mode repair (non-manifold edge/vertex splitting, boundary-loop filling), geometric/topological metrics, and guaranteed-manifold booleans (manifold-ops facade) are now exposed through the geometry-kernel ABI (`abi_manifold_repair` gained a mode parameter and wider stats; `abi_manifold_metrics` and `abi_manifold_boolean` are new exports), with matching TypeScript bindings. crates/Cargo.lock gained the manifold-ops workspace dependency.
- **In-flight edits.** After the G0 v38 / G1 v55 publish the concurrent process restructured brep-core direct editing, rebound multiple bound sources, and rewrote historical qualification documents to current bytes; the frozen-archive hash table in the re-freeze generator was re-baselined to those bytes so the append-only chain verifies again.
- **Kernel.** The own-Rust geometry kernel was rebuilt from current sources (native Binaryen 116 wasm-opt, reproducible cargo profile); the packed kernel WASM fingerprint is taken from the rebuilt bytes.
- **Distribution budgets.** verify-dist budgets already cover the rebuilt dist; no new allowance was needed.

V56 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v56), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v31.json` with refreshed fingerprints.

The matrix remains 4740 planned work units. V56 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
