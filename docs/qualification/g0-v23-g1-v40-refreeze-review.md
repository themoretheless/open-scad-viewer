# G0 v23 / G1 v40 no-claim re-freeze

This append-only amendment preserves G0 v22, G1 v39, and all earlier artifacts byte-for-byte. V39 cannot admit further evidence because its frozen bindings no longer match current source bytes.

The drift is reviewed: the committed `b81fe7f9` (brep-core analytic surface intersections, gcode-core 3mf packaging/parser, `openScadStableExpressionEval` and related MCP/manifold qualification work) changed own-Rust geometry sources and `crates/Cargo.lock`; the geometry kernel WASM was rebuilt from these sources. Kernel semantics are unchanged: the own-Rust oracle suite (44 tests, five fixed direct-evaluator cases) passes byte-identically against the v3 fixture on the rebuilt kernel, despite the new brep-core intersection modules, so the oracle stays at v3 and no migration was required.

Tree state at freeze time: NOT clean. `git status` showed one uncommitted work-in-progress file owned by another line of work: `src/services/shaders/index.ts` (meshMatcap renderer-wide capture-texture binding). The file is not inside the frozen G0/G1 bundles, but the freeze still covers the exact bytes present at freeze time; it must be re-issued once that work is committed or reverted.

V40 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v40), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v15.json`.

The matrix remains 4740 planned work units. V40 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
