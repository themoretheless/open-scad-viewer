# G0 v20 / G1 v37 no-claim re-freeze

This append-only amendment preserves G0 v19, G1 v36, and all earlier artifacts byte-for-byte. V36 cannot admit further evidence because its frozen bindings no longer match current source bytes.

The drift is reviewed: the committed perf series `5999646f` (allocation-free gcode fixed7 emit, SDF FxHashMap, raster write_ppm, mechanics AABB prefilter) and `c086c8d9` (subdivision validation cascade, scanline hatch, borrowing gcode-core emit_ref, brep-core/brep-topology FxHash, nurbs dedup) changed own-Rust geometry sources, and the geometry kernel WASM was rebuilt. Kernel semantics are unchanged: gcode fixed7 output is bit-identical, the scanline hatch path is oracle-fuzzed, and the own-Rust oracle cases keep byte-identical LME1/LSE1 output, so the oracle stays at v3 with no migration.

Tree state at freeze time: NOT clean. `git status` showed 20 modified files, all under `crates/`: `crates/Cargo.lock` (rustc-hash entries plus one offline line), `crates/compute-core/src/lib.rs`, and `crates/math-core` (Rust sources and WGSL kernels). This in-flight work is compiled into the bound kernel bytes and is recorded here explicitly; the freeze must be re-issued once that work is committed or reverted.

V37 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v37), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v12.json`.

The matrix remains 4740 planned work units. V37 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
