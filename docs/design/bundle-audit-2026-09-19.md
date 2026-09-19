# Bundle attribution: 2026-09-19

Latest follow-up: [ModelGraph runtime/schema separation](modelgraph-runtime-split-2026-09-19.md)
removes another 152,084 delivery bytes, preserving compiler hashes, MCP schemas
and browser workflows. The current normal build is 5,782,657 bytes; the
5,600,000-byte aggregate budget remains unmet.

Follow-up: the [exact-solid worker change](exact-solid-worker-2026-09-19.md)
removed the main-thread evaluation route and reduced delivery size by 400,955 bytes.
The measurements below remain the pre-change attribution baseline.

## Reproduce

```sh
npm run test:bundle-audit
node_modules/.bin/vite build --sourcemap
node scripts/audit-bundle.mjs > tmp/performance/bundle-audit.json
# Restore the actual delivery build before checking its budget.
node_modules/.bin/vite build
node scripts/verify-dist.mjs
```

`npm run audit:bundle` runs the first build and prints the report. It does not
rebuild WASM. Use the existing geometry build first if Rust changed. Source maps
contain source code and are diagnostic artifacts, not the normal delivery build.

The report records emitted-file byte lengths and SHA-256 identities separately
from original source attribution. A source must have the same normalized path
and content hash in distinct chunks to count as repeated. Duplicate source-map
entries within one chunk are counted once; different transforms are not merged.
Original source bytes are **not** minified bytes or predicted removable bytes.

## Measured baseline

- Normal delivery build: 6,333,440 bytes; budget: 5,600,000 bytes.
- Diagnostic build excluding maps: 6,336,316 bytes. The extra 2,876 bytes are
  source-map references, not a production regression.
- 55 JavaScript source maps; 113 repeated source identities across chunks.
- Local report: `tmp/performance/bundle-audit.json`.

Largest repeated application sources include `openscadParser.ts`,
`openscadSemanticLowerer.ts`, `openScadImport.ts` and
`semanticProgramValidator.ts`. They occur in the geometry worker and in the
main application's parser/build-engine chunks. Zod occurs in both
`modelGraphNurbs` and `modelgraph-text` chunks. HarfBuzz JS glue remains repeated;
its much larger packed WASM payload is already shared.

## Architecture finding

`App.vue::buildSolidGroup` deliberately imports `geometryBuildEngine` and calls
`evaluateExactSolidsOnMainThread`. Its comment describes the limitation: the
bounded display worker protocol cannot transport the exact graph. This is a
real user workflow, not a dead import. Removing its parser chunk would break
group source editing. It can also block the interface on large inputs.

The next candidate is a bounded exact-graph worker response, with explicit
schema validation, admission limits, cancellation and resource ownership. Only
then can the main-thread build route be removed, and its actual bundle and
responsiveness effects measured. Do not transfer realm-local kernel handles.
An alternative is shared emitted ESM code between build graphs; that saves
download bytes but does not move computation off the main thread or share WASM
instances. Neither approach has been implemented or qualified by this audit.

Before accepting delivery changes, check worker startup and failure handling,
exact-body equivalence, group replacement, lazy kernel loading, offline asset
availability and browser responsiveness. Preserve the existing payload identity
checks and total budget. Chunk renaming or a higher budget is not an optimization.

## Verification

Two Node tests cover byte accounting, content identity, distinct-chunk counting,
hash emission and refusal when source maps are absent. Vite source-map build
and report generation passed. No application runtime or bundler configuration
was changed by this diagnostic step.
