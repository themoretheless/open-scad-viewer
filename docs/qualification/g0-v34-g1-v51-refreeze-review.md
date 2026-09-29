# G0 v34 / G1 v51 no-claim re-freeze

This append-only amendment preserves G0 v33, G1 v50, and all earlier artifacts byte-for-byte. V50 cannot admit further evidence because a file it binds changed.

The drift is reviewed:

- **Kernel.** The concurrent feature branch completed its face-contact work and rebuilt the own-Rust geometry kernel WASM with the shared-edge plane criterion; `tests/solidFaceContacts.test.ts` passes 7/7 against the rebuilt kernel. The packed kernel bytes therefore differ from own-rust-cad-v25.
- **Source bundle.** Bound crate and service sources changed with the completed face-contact work across `brep-core`, `nurbs-core`, `geometry-bridge` and the solid-face-contact service layer, plus `package.json`.
- **`scripts/verify-dist.mjs`.** The geometry chunk budget was raised to 3231000 per the file's documented convention for the 3230813-byte packed kernel; `node scripts/verify-dist.mjs` verifies 114 dist artifacts against it.
- **Notices and lockfiles.** `THIRD_PARTY_NOTICES.md`, `package-lock.json` and `crates/Cargo.lock` are byte-identical to own-rust-cad-v25, and the own-Rust oracle stays at v3.

V51 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v51), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v26.json` with the rebuilt kernel fingerprint and refreshed source-bundle fingerprints.

The matrix remains 4740 planned work units. V51 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
