# G0 v35 / G1 v52 no-claim re-freeze

This append-only amendment preserves G0 v34, G1 v51, and all earlier artifacts byte-for-byte. V51 cannot admit further evidence because a file it binds changed.

The drift is reviewed:

- **Kernel.** Two landed commits (`b59c591c`, `5f40d345`, CAD modeling and diagnostics features) rebuilt the own-Rust geometry kernel WASM, so the packed kernel bytes differ from own-rust-cad-v26.
- **Source bundle.** Bound crate sources changed with those commits; `crates/brep-core/src/operations.rs` grew to 110314 bytes.
- **`scripts/verify-dist.mjs`.** Budgets already cover the rebuilt dist; `node scripts/verify-dist.mjs` verifies 114 dist artifacts (7062161 asset bytes + 11231717 raw WASM bytes).
- **Notices and lockfiles.** `THIRD_PARTY_NOTICES.md`, `package-lock.json` and `crates/Cargo.lock` are byte-identical to own-rust-cad-v26, and the own-Rust oracle stays at v3.

V52 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v52), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v27.json` with the rebuilt kernel fingerprint and refreshed source-bundle fingerprints.

The matrix remains 4740 planned work units. V52 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
