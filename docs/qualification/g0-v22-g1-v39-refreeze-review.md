# G0 v22 / G1 v39 no-claim re-freeze

This append-only amendment preserves G0 v21, G1 v38, and all earlier artifacts byte-for-byte. V38 cannot admit further evidence because its frozen bindings no longer match current source bytes.

The drift is reviewed: the committed `f66c2893` (photogrammetry-core pipeline/host_matching overlap counters, geometry-bridge cad_lattice decimate scratch buffers) and `2512c7ca` (allocation-free MCP geometry validation, memoized runtime verify, `scripts/wasm-pack-stamp` build-stamp cache) changed own-Rust geometry sources and `crates/Cargo.lock`; the geometry kernel WASM was rebuilt from these sources. Kernel semantics are unchanged: the own-Rust oracle suite (44 tests, five fixed direct-evaluator cases) passes byte-identically against the v3 fixture, so the oracle stays at v3 and no migration was required.

Tree state at freeze time: NOT clean. `git status` showed uncommitted work-in-progress owned by another line of work: `crates/gcode-core` (`Cargo.toml`, `src/package_3mf.rs`), `crates/Cargo.lock`, and TypeScript sources (`src/App.vue`, `src/core/geometryExecution.ts`, `src/mcp/engineManifest.ts`, `src/mcp/manifoldPlanQualification.worker.ts`, `src/services/*`, `src/workers/*`, plus untracked `src/services/manifoldPlanQualificationWorkerRuntime.ts` and `src/services/openScadStableExpressionEval.ts`). Several of these files are inside pinned bundles (including `src/core/geometryExecution.ts`), so this freeze covers uncommitted bytes; it must be re-issued once that work is committed or reverted.

V39 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v39), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v14.json`.

The matrix remains 4740 planned work units. V39 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
