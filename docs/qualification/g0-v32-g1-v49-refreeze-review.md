# G0 v32 / G1 v49 no-claim re-freeze

This append-only amendment preserves G0 v31, G1 v48, and all earlier artifacts byte-for-byte. V48 cannot admit further evidence because a file it binds changed.

The drift is reviewed:

- **Source bundle.** The concurrent feature branch kept editing bound crate sources after the G0 v31 / G1 v48 publish: `crates/brep-core/src/lib.rs` and the face-contact work (`face_contact.rs`, new `face_contacts.rs`), plus `nurbs-core` surface contact search (`lib.rs`, `surface_contact_search.rs`). The own-Rust source-bundle digest changed accordingly. No contract, isolation list or budget semantics changed with it.
- **Kernel.** The packed own-Rust geometry kernel WASM is byte-identical to own-rust-cad-v22; it was not rebuilt with these source edits.
- **Notices and lockfiles.** `THIRD_PARTY_NOTICES.md`, `package-lock.json` and `crates/Cargo.lock` are byte-identical to own-rust-cad-v23, and the own-Rust oracle stays at v3.

V49 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v49), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v24.json` with a byte-identical kernel fingerprint and refreshed source-bundle fingerprints.

The matrix remains 4740 planned work units. V49 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
