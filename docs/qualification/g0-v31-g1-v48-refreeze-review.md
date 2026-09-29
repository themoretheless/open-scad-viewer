# G0 v31 / G1 v48 no-claim re-freeze

This append-only amendment preserves G0 v30, G1 v47, and all earlier artifacts byte-for-byte. V47 cannot admit further evidence because a file it binds changed.

The drift is reviewed:

- **Source bundle.** The concurrent feature branch kept editing bound crate sources (including `crates/brep-core/src/lib.rs`) after the G0 v30 / G1 v47 publish, so the own-Rust source-bundle digest changed. No contract, isolation list or budget semantics changed with it.
- **Kernel.** The packed own-Rust geometry kernel WASM is byte-identical to own-rust-cad-v22; it was not rebuilt with this source edit.
- **Notices and lockfiles.** `THIRD_PARTY_NOTICES.md`, `package-lock.json` and `crates/Cargo.lock` are byte-identical to own-rust-cad-v22, and the own-Rust oracle stays at v3.

V48 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v48), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v23.json` with a byte-identical kernel fingerprint and refreshed source-bundle fingerprints.

The matrix remains 4740 planned work units. V48 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
