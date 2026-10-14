# G0 v38 / G1 v55 no-claim re-freeze

This append-only amendment preserves G0 v37, G1 v54, and all earlier artifacts byte-for-byte. V54 cannot admit further evidence because files it binds changed.

The drift is reviewed:

- **In-flight edits.** After the G0 v37 / G1 v54 publish the concurrent process edited bound sources again, notably nurbs-core curve and periodic-seam modules with related dependents. The own-Rust source-bundle digest changed accordingly.
- **Kernel.** The own-Rust geometry kernel was rebuilt from current sources; the packed kernel WASM fingerprint is taken from the rebuilt bytes.
- **Distribution budgets.** verify-dist budgets already cover the rebuilt dist; no new allowance was needed.
- **Notices and lockfiles.** `THIRD_PARTY_NOTICES.md`, `package-lock.json` and `crates/Cargo.lock` are byte-identical to own-rust-cad-v29, and the own-Rust oracle stays at v3.

V55 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v55), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v30.json` with refreshed fingerprints.

The matrix remains 4740 planned work units. V55 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
