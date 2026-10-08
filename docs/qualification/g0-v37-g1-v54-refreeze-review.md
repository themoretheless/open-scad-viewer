# G0 v37 / G1 v54 no-claim re-freeze

This append-only amendment preserves G0 v36, G1 v53, and all earlier artifacts byte-for-byte. V53 cannot admit further evidence because files it binds changed.

The drift is reviewed:

- **Landed commits and in-flight edits.** Two commits landed after the G0 v36 / G1 v53 publish: `a50b8969` (WebXR stereo viewer) and `67982063` (CAD commands and rational surfaces), followed by uncommitted workspace edits across the crates and the app shell. The own-Rust source-bundle digest changed accordingly.
- **Kernel.** The own-Rust geometry kernel was rebuilt from current sources; the packed kernel WASM fingerprint is taken from the rebuilt bytes. Real defects fixed before the rebuild are recorded in the session log: Electron-as-node child-process environment loss in two MCP runtimes, exact degree-reduction recognition in `approximate_edits.rs`, projective fourth-row routing that violated the pinned affine 3x4 transform contract, and hull-separation work accounting in face-contact reports.
- **Distribution budgets.** verify-dist named budgets and the aggregate budget were raised to the measured rebuilt sizes; each bump carries its measurement comment in `scripts/verify-dist.mjs`.
- **Notices and lockfiles.** `THIRD_PARTY_NOTICES.md`, `package-lock.json` and `crates/Cargo.lock` are byte-identical to own-rust-cad-v28, and the own-Rust oracle stays at v3.

V54 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v54), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v29.json` with refreshed fingerprints.

The matrix remains 4740 planned work units. V54 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
