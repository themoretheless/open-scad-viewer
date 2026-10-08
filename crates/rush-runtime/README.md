# RushGraph runtime

Native Rust validation, evaluation and execution planning for Rush IR/1 and
RushGraph/NURBS-1. It is shared by the browser, worker and Node WASM adapters.

`compile(Value)` normalizes the strict canonical schema, evaluates parameters and
constraints, checks graph structure and produces generated SCAD, source maps,
resolved geometry assertions, sketch/assembly reports and mechanical reports.
`nurbs::compile(Value)` validates and resolves an own-NURBS document.
`nurbs::compile_text(...)` resolves the numeric snapshot emitted by Rush.
Errors carry stable `code`, `path`, `message` and optional `details`. JavaScript graph
arguments must contain finite JSON values: explicit `undefined`, `BigInt`, functions
and symbols produce structured input errors instead of being lost in serialization.

The bridge's `execute_rush_frontend` passes the frontend graph directly to this
crate. JavaScript retains host transport, MCP input schemas and canonical SHA-256 hashing. The SCAD
executor and browser renderer remain separate; compiling a graph does not construct
its triangle mesh.

Named sketch constraints are resolved to indexed `sketch-core` types. The shared
solver computes coordinates, rank and diagnostics; this crate restores IDs and
RushGraph error paths. Named sketches keep the 64-iteration policy.

## Performance choices

- Schema unions dispatch on their discriminator instead of trying every recursive
  branch. Defaults are inserted in place. This removes the measured dominant cost
  in the former TypeScript SKADIS compiler.
- The public MCP schema also dispatches expression operations directly, preserving
  its JSON contract while avoiding the same recursive union search before Rust is
  called. Independent schema fixtures cover validation and defaults.
- The evaluator borrows expression trees and shares immutable collection values
  and lexical scopes, retaining lazy conditionals and bounded range/collection work.
- Graph indexes and reusable sketch differentiation buffers avoid repeated work.
- Thread mesh deduplication uses numeric keys with the original rounding semantics.
- Source serialization uses JavaScript-compatible number spelling for stable
  geometry sources; the host computes document hashes from the Rush IR, including its language tag.
- Source, nesting, node expansion, evaluation and collection budgets remain enforced.

## Verification

```
cargo test --locked --manifest-path crates/Cargo.toml -p rush-runtime
npm run build:geometry
npx vitest run tests/rushGraph*.test.ts
node --import tsx scripts/bench-rush-frontend.mts
```

Checked-in parity fixtures cover 456 schema cases, 58 emitter/report/error cases and
56 mechanical cases captured from the original TypeScript implementation. The
mechanical corpus compares 118,439 coordinates as well as topology and reports.
Native unit tests cover expression budgets, units, lazy evaluation and own-NURBS
resolution. Existing integration tests exercise the actual generated WASM module.

See `../rush-frontend/README.md` for reproducible compiler and full geometry build
measurements, their environment and scope. Archived reference modules and raw reports
are under the repository's `output/` directory.
