# Frozen ordinary-vertex qualification

Source implementation: `f046834ca`; this receipt predates subsequent material
span ownership changes. All 6,588 Rust/build inputs were unchanged during the run.

- Adapter regression: 1 passed.
- Nonplanar ordinary vertex ownership: 1 passed (6 C2 stars, 20 explicit C0 vertices).
- cad-predicates library: 127 passed.
- nurbs-core library: 2061 passed, 1 ignored.
- brep-core library: 2067 passed, 5 ignored.

No failures. Exact commands and exit codes are in result.json; source-before.json
and source-after.json bind these results to the frozen source bytes. This is
native qualification only. It does not qualify later source changes, all ideal
source modes, WASM, browser tessellation, STEP, or remote CI.
