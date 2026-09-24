# G0 v19 / G1 v36 no-claim re-freeze

This append-only amendment preserves G0 v18, G1 v35, and all earlier artifacts byte-for-byte. V35 cannot admit further evidence because its frozen bindings no longer match current source bytes.

The drift is legitimate and reviewed: the committed dual-format f64 import ABI (`d4b9dd07`), the Float64Array parser path (`c25daafd`), and the raster-core series (`78bee83d..e39f5747`) changed own-Rust geometry sources and the rebuilt geometry kernel WASM. The working tree was clean at freeze time (`git status` empty), so every bound byte corresponds to a committed state — unlike the v18/v35 freeze, no uncommitted work-in-progress is covered. Kernel semantics are unchanged: the own-Rust oracle cases keep byte-identical LME1/LSE1 output, so the oracle stays at v3 and no oracle migration was required.

V36 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v36), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v11.json`, which pins the rebuilt dual-format kernel and the current dependency files.

The matrix remains 4740 planned work units. V36 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
