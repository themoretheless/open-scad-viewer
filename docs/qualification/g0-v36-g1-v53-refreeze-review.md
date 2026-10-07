# G0 v36 / G1 v53 no-claim re-freeze

This append-only amendment preserves G0 v35, G1 v52, and all earlier artifacts byte-for-byte. V52 cannot admit further evidence because a file it binds changed.

The drift is reviewed:

- **Source bundle and manifest.** The concurrent process edited bound files again after the G0 v35 / G1 v52 publish: `crates/Cargo.toml` (dependency pin) and `crates/manifold-core/src/lib.rs`. The own-Rust source-bundle digest and the crates workspace manifest fingerprint changed accordingly.
- **Kernel.** The packed own-Rust geometry kernel WASM fingerprint is taken from current bytes; it is unchanged unless the concurrent process rebuilt it, in which case the rebuilt bytes are the frozen ones.
- **Notices and lockfiles.** `THIRD_PARTY_NOTICES.md`, `package-lock.json` and `crates/Cargo.lock` are byte-identical to own-rust-cad-v27, and the own-Rust oracle stays at v3.

V53 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v53), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v28.json` with refreshed fingerprints.

The matrix remains 4740 planned work units. V53 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
