# ModelGraph Text frontend

`modelgraph_text::compile(&str)` is a native Rust source-to-graph compiler, also exported
as `compile_modelgraph_text` from `geometry-bridge` for synchronous WASM use in browsers,
workers and Node. It owns lexing, layout parsing, lexical functions and records, generic
inference, numeric type annotations, collection lowering and graph construction.

The result contains `nodes`, `parameters`, `root`, `customizer`, `constraints`, `checks`
and optional `segments`. It is the authoring graph, not a triangle mesh. Production uses
the fused `execute_modelgraph_text` bridge: this graph moves directly into
`modelgraph-runtime` for validation, expression evaluation and backend preparation,
without an intermediate Rust → JavaScript → Rust graph round trip.
Runtime-dependent numeric type checks are emitted into that graph.
No JavaScript frontend fallback, host eval, network or native geometry calls are used here.

Compatibility fixtures in `tests/fixtures/modelgraph-text-compatibility.json` were captured
from the former TypeScript implementation. They cover SKADIS, generic functions, ranges
and NURBS. Source offsets use UTF-16 code units, matching editor selections in JavaScript.

Optimizations:

- JSON subtrees move into parents; construction does not serialize/copy every child.
- Function/lambda definitions and their captured environments use reference-counted
  handles, so copying a function does not recursively copy its captured functions.
- Ordered maps preserve field/check order and provide indexed lookups.
- Line lookup uses a precomputed newline index instead of rescanning the source per statement.
- Both geometry and the frontend share one initialized WASM module per JS realm.

The source, nesting, field, function-expansion and graph-node limits remain bounded.
Canonical numeric and geometry evaluation budgets remain enforced by the graph backend.

```
cargo test --locked --manifest-path crates/Cargo.toml -p modelgraph-text
npm run build:geometry
npx vitest run tests/modelGraphRustFrontend.test.ts
node --import tsx scripts/bench-modelgraph-text.mts
```

An optional `--reference=/absolute/path/to/old-frontend.ts` compares complete compilation
against an archived implementation, checking graph/control equality first. Use
`--output=path.json` to save samples, environment, source hashes and WASM hash.
Measurements exclude mesh generation and include the Rust/JS transport overhead.

Local measurement after the runtime migration (2026-09-08, Apple M4 Max,
Node 22.23.2, warm WASM): SKADIS complete source compilation 91.68 ms → 2.52 ms,
generic-functions 3.38 ms → 0.37 ms, range-pattern 0.77 ms → 0.32 ms.
Eight alternating batches of ten calls compare the archived TypeScript implementation
with the new path; documents and controls are checked for equality first.
Cold shared WASM initialization is separate (48.4 ms in this run).
Raw batches and source/WASM/reference hashes are saved in
`output/modelgraph-runtime-benchmark.json`.

An additional five alternating full-build pairs compile SKADIS and call
`parseOpenSCAD(source, { quality: 'full' })` with identical options: median total
106.46 ms → 10.66 ms on the final build (compiler alone 95.26 ms → 2.63 ms).
Generated SCAD and all mesh vertices/indices, volume and bounds
match. This measures local Node compilation plus geometry, excluding browser rendering,
worker transport and cold start. The unchanged geometry stage is not claimed to be faster.
See `output/bench-modelgraph-end-to-end.mts` and
`output/modelgraph-end-to-end-final.json` for the full measurement. Pass
`--output=output/modelgraph-end-to-end-final.json` to the benchmark script to reproduce
that report. The final production build contains 22 artifacts totaling 2,838,544 bytes;
the existing total size budget is unchanged.
