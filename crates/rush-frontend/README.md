# Rush frontend

`rush_frontend::compile(&str)` is a native Rust source-to-graph compiler, also exported
as `compile_rush_frontend` from `geometry-bridge` for synchronous WASM use in browsers,
workers and Node. It owns lexing, layout parsing, lexical functions and records, generic
inference, numeric type annotations, collection lowering and graph construction.

The result contains `nodes`, `parameters`, `root`, `customizer`, `constraints`, `checks`
and optional `segments`. It is the authoring graph, not a triangle mesh. Production uses
the fused `execute_rush_frontend` bridge: this graph moves directly into
`rush-runtime` for validation, expression evaluation and backend preparation,
without an intermediate Rust → JavaScript → Rust graph round trip.
Runtime-dependent numeric type checks are emitted into that graph.
No JavaScript frontend fallback, host eval, network or native geometry calls are used here.

Compatibility fixtures in `tests/fixtures/rush-frontend-compatibility.json` were captured
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
cargo test --locked --manifest-path crates/Cargo.toml -p rush-frontend
npm run build:geometry
npx vitest run tests/rushGraphRustFrontend.test.ts
node --import tsx scripts/bench-rush-frontend.mts
```

An optional `--reference=/absolute/path/to/old-frontend.ts` compares complete compilation
against an archived implementation, checking graph/control equality first. Use
`--output=path.json` to save samples, environment, source hashes and WASM hash.
Measurements exclude mesh generation and include the Rust/JS transport overhead.

## Declarative Rush identity

`compile_document` routes `// @rush` and `// @rush/1` documents through the
same declarative CAD frontend with stable IDs scoped to each named part.
Independent named declarations therefore do not renumber existing parts.

```rust
// @rush/1
param size = 10mm range 1mm..20mm
body @id("stable-body") = box([size,size,size])
show body
```

An explicit ID survives renaming the binding. IDs must be unique and contain
1..64 ASCII letters, digits, underscores or hyphens. Without an explicit ID,
identity follows the binding name. Internal operations within a part use local
sequence numbers; inserting or reordering operations *inside that part* can
change those internal IDs. This is authored-part identity, not persistent
identity of every generated face or edge.

The regression suite compares authoring graphs, controls and source offsets for
131 existing models across versioned and unversioned Rush headers. Separate regressions
verify independent insertions, explicit identity through rename, duplicate-ID
rejection and numeric source edits preserving comments and units.

Historical CAD qualification records were cleaned of retired language references on 2026-10-06. Their content digests were updated; recorded test outcomes were not rerun as part of that metadata cleanup.
