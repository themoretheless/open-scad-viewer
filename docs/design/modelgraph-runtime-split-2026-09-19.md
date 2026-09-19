# ModelGraph runtime/schema boundary

The refreshed source-map audit found 81 repeated source identities, including
18 Zod modules repeated across the worker/main graphs. Text compilation imported
hash functions and error classes from modules that also eagerly construct MCP
schema descriptions. NURBS runtime building and the mechanical/SVG UI paths
similarly reached the combined schema/compiler modules.

The actual compilation functions already delegate validation and evaluation to
Rust through `prepareGraphRust`. This change does not remove validation or
replace Zod with handwritten input checks. It separates two responsibilities:

- `modelGraphCompiler.ts` and `modelGraphNurbsCompiler.ts`: Rust calls, revision
  hashing, and runtime errors, with only erased type imports from schema modules.
- Existing `modelGraph.ts` and `modelGraphNurbs.ts`: schema descriptions, public
  types, guides/examples and compatibility re-exports of the runtime functions.

Text, NURBS building, SVG conversion and the mechanical generator UI import the
lightweight runtime. MCP callers keep their existing API. Re-exported functions
and error constructors retain identity, including `instanceof` behavior.

The two historical canonical hashing algorithms are intentionally not merged:
their key ordering/encoding contracts differ. All recorded document hashes and
structured errors must remain unchanged. Mechanical input still uses its real
Zod validation; that dependency was not discarded merely to reduce size.

## Size evidence

Same generated kernel payloads and Vite settings; no budget increase or asset
exclusion:

| Delivery | Before | After | Reduction |
| --- | ---: | ---: | ---: |
| Normal `dist`, bytes | 5,934,741 | 5,782,657 | 152,084 |
| Diagnostic assets excluding maps, bytes | 5,937,657 | 5,785,581 | 152,076 |
| Repeated source identities | 81 | 62 | 19 |
| Repeated Zod source identities | 18 | 0 | 18 |

Both diagnostic builds have 56 JavaScript source maps. Their asset totals
include source-map reference comments; the normal build was restored before
browser testing and `verify-dist`. Unminified source attribution is not used as
an estimate of removable bytes.

`verify-dist` still fails the aggregate 5,600,000-byte budget by 182,657 bytes.
This is progress toward the gate, not a gate pass. Runtime latency improvements
are not claimed.

Reports: `tmp/performance/bundle-refresh-before.json`,
`modelgraph-compiler-bundle-after.json`, `modelgraph-compiler-dist-check.log`.
Reproduce attribution with `vite build --sourcemap` and
`node scripts/audit-bundle.mjs`; restore with `vite build` afterward.

## Compatibility and browser checks

- Six pre/post fixtures have byte-identical serialized results/error envelopes:
  box text, planetary-spinner text, NURBS curve, NURBS surface, malformed NURBS
  document and unknown text operation. Raw results are in
  `tmp/performance/modelgraph-compiler-before.json` and `...-after.json`.
- New compiler tests pin four pre-change revision hashes, error constructor
  identity, facade aliases and structured Rust diagnostics.
- 707 tests passed across 43 ModelGraph/SVG/worker suites, including MCP schema
  discovery and execution. Log: `tmp/performance/modelgraph-compiler-tests.log`.
- UI and MCP typechecks, production build and `git diff --check` passed.
- Two production Chrome runs preserved the three existing geometry hashes
  (one box, two boxes, 20-body spinner), unsupported-operation refusal, abort,
  recovery, source-to-Solid build and group replacement. No page errors.
- The extended second run also opened the lazy mechanical generator, generated
  gear/planetary/thread documents, refused two teeth, recovered at 24 teeth,
  and left the existing scene/source unchanged.

Browser evidence: `tmp/performance/modelgraph-compiler-browser/report.json` and
`modelgraph-compiler-browser-repeat/report.json`. The latter records mechanical
UI coverage. The browser script times geometry before the UI assertions, so
the added form checks do not extend its measured intervals. No tests/builds ran
alongside those timed browser workloads.

The full suite was not rerun in this step; the 707 targeted tests and browser
checks are the verification scope for this extraction. Earlier historical
qualification failures are not repaired or re-certified by these results.
