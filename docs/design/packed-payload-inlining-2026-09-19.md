# Packed payload inlining regression

Baseline: integrated main e2283155426f2afaa40a9d558b2d604e65ad3a28.

## Cause

The streaming warmup used `await compileStreamingWasm(url) ?? await
WebAssembly.compile(unpackBrotliWasmBase64(wasmBase64))`. Rolldown's default
smart imported-constant inlining applies inside logical expressions. It copied
the 2.79 MB geometry payload into the renderer, geometry worker and SVG worker,
while retaining the shared payload chunk for synchronous consumers.

Source-map attribution did not expose these copies as repeated generated-byte
modules. Inspection of emitted literals found four identical payloads. Worker
runtime isolation does not require four copies of their immutable encoded data.

Geometry and language warmup now use an explicit `if` fallback. Streaming,
synchronous worker initialization, and instance ownership remain unchanged.
Photogrammetry already used an explicit branch and needed no change.

## Artifact measurement

Normal production Vite builds, same generated WASM and dependencies:

| Artifact | Before bytes | After bytes |
| --- | ---: | ---: |
| Geometry worker | 3,258,801 | 470,353 |
| Renderer | 2,872,959 | 84,511 |
| SVG worker | 2,822,512 | 34,064 |
| Assets excluding raw streaming WASM | 14,142,085 | 5,776,741 |
| Including raw streaming WASM | 23,839,541 | 15,474,197 |

Reduction: 8,365,344 bytes, 59.2% of the non-raw-WASM assets. The raw
streaming modules remain 9,697,456 bytes. This is artifact size, not cold-load
latency, compressed network transfer, or a CPU throughput measurement.
Baseline source-map build totals include source-map comments and are not used
in this table. Geometry shared chunk remains 2,788,471 bytes.

## Regression protection

`verify-dist` now scans emitted JS string literals for duplicate base85 payloads,
including repeats within a file. This complements source-map attribution and
the existing decoded-WASM identity check. It recognizes the current unescaped
base85 literal format; it is not a general JavaScript parser or detector of
equivalent differently encoded payloads.

The inflated per-chunk allowances return to 500/100/50 KB for geometry worker,
renderer and SVG worker. The non-raw-WASM total budget is 6 MB, down from 14.4 MB.
The existing raw-WASM per-file bounds and exclusion from that total are unchanged.

## Verification

- `vite build` and `node scripts/verify-dist.mjs`: 88 artifacts passed.
- `node --test tests/auditWasmPackage.test.mjs`: 4 tests passed, including the
  new duplicate detection test (cross-file, same-file, distinct and absent data).
- `vue-tsc --noEmit`: passed.
- GeometryBuildEngine and meshSurfaceGroups: 23 tests passed.
- Production SVG/G-code browser worker qualification: passed.
- Exact-solid production browser scenario: passed, including refusal,
  cancellation, recovery and application checks; no page errors.

Local evidence: `/private/tmp/osv-bundle-baseline.log`,
`/private/tmp/osv-bundle-fixed.log`,
`tmp/performance/inline-payload-workers/report.json`, and
`tmp/performance/inline-payload-browser/report.json`. Browser checks used local
Chrome 156.0.8063.3. No new cold-start latency claim is made.
