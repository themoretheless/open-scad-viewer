# G0 v30 / G1 v47 no-claim re-freeze

This append-only amendment preserves G0 v29, G1 v46, and all earlier artifacts byte-for-byte. V46 cannot admit further evidence because a file it binds changed.

The drift is reviewed:

- **Kernel.** The own-Rust geometry kernel WASM was rebuilt again from current crate sources; the feature branch continued with exact cylinder end-cap push/pull support and further work across `brep-core`, `geometry-bridge`, `nurbs-core` and `polygon-core`, so the rebuilt kernel bytes differ from own-rust-cad-v21.
- **Source bundle.** Files across `crates/brep-core/src`, `crates/geometry-bridge/src`, `crates/nurbs-core/src` and `crates/polygon-core/src` changed with that feature work.
- **`scripts/verify-dist.mjs`.** Three size budgets were bumped per the file's documented convention to cover audited bundle growth (`DirectModeler` to 365000, `mainSolid.worker` to 116000, `totalBudget` to 7009000); `node scripts/verify-dist.mjs` verifies 114 dist artifacts against them.
- **Notices and lockfiles.** `THIRD_PARTY_NOTICES.md`, `package-lock.json` and `crates/Cargo.lock` are byte-identical to own-rust-cad-v21, and the own-Rust oracle stays at v3.

V47 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v47), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v22.json` with the rebuilt kernel fingerprint and current source-bundle fingerprints.

The matrix remains 4740 planned work units. V47 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
