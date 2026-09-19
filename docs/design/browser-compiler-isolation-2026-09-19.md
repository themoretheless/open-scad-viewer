# Browser compilation belongs to the host

## Evidence and boundary

CI run 35473144674, Node22 job 105977674450 on e2283155, reports
`mcpProductionIsolationContract` failing because the transitive direct-worker
dependency graph contains `fetch` in `wasmStreaming.ts`. A runtime `window`
guard is insufficient for the source-level no-I/O contract.

Geometry and language kernels now depend on a small pure optional-compiler
interface. Every realm defaults to `null`, selecting embedded bytes. Only
`src/main.ts`, the browser composition root, installs the bounded streaming
compiler. No network module is reachable through this interface from the MCP
worker. Photogrammetry's browser-specific module keeps its existing loader.
Worker isolation, provider readiness deadlines and qualification routing are
unchanged. The existing transitive no-I/O assertion passes without exemptions.

The injected callback is trusted host code, not a plugin interface. It is
installed before application startup, not switched to replace live instances.
The kernel's existing single warmup and initialized-instance checks still own
instance lifetime. Test reset removes the callback from the test realm.

## Correct browser evidence

The exact-solid benchmark HTTP server previously served raw WASM as
`application/octet-stream`, so streaming compilation failed and browser checks
exercised fallback only. The server now serves `application/wasm`. A counter
around the real native compileStreaming call asserts successful streaming in
the normal application run. `EXACT_SOLID_WASM_FALLBACK=1` returns 404 for raw
WASM and asserts zero successful streaming compiles in a separate run.

Both production runs passed source builds, refusal, cancellation, recovery,
application wiring and mechanical generator checks with no page errors.
All three geometry SHA256 values match between streaming and fallback runs.
Local reports:

- `tmp/performance/compiler-isolation-streaming/report.json`: 1 successful compile.
- `tmp/performance/compiler-isolation-fallback/report.json`: 0 successful compiles.

## Qualification asset isolation

The full local test run reported 3260 passed and 11 failed. Nine were existing
manifest/frozen-evidence binding checks; two browser-runner failures hit its
8 MiB aggregate artifact cap. Product `public/wasm` modules were being copied
into the isolated qualification build even though it uses embedded kernels.

`vite.qualification.config.ts` now sets `publicDir: false`. The test additionally
asserts that the bundle contains no `wasm/` public artifacts. Its aggregate cap
and evidence requirements are unchanged. This source/configuration change does
not refresh frozen qualification fingerprints or claim new qualification.

## Local checks

- Browser-runner, unchanged MCP isolation, compiler adapter and streaming tests:
  30 passed after the qualification configuration fix.
- Earlier focused adapter/streaming/isolation/real supervisor run: 30 passed.
- Typecheck and production build passed; dist verification: 89 artifacts,
  5776925 bytes excluding raw streaming WASM.
- Both production browser paths passed as described above.

The entire suite was not rerun after the final qualification config change.
Its focused failures were rerun successfully. Manifest/evidence drift and
CI-only cold-readiness failures remain open; no green CI claim is made.

## Streaming artifact identity

The distribution gate additionally compares each emitted raw streaming module
with the same source binary used to verify its packed alternative. It now
checks the packed language kernel too; previously only geometry, photogrammetry,
HarfBuzz and the Brotli decoder had packed-byte identity checks.

Current geometry/language/photogrammetry raw modules match their generated
counterparts. Negative integration checks ran the real `verify-dist` script
on a temporary copy of dist: changing one byte (without changing length) in
each of the three raw modules was rejected with the corresponding identity
error; deleting the raw language module was also rejected. Original artifacts
were untouched. Five package-audit unit tests pass, including identity/view
offset/truncation checks and duplicate packed-payload checks.

The verifier reports both totals: 5776925 non-raw asset bytes plus 9697456 raw
WASM bytes, 15474381 bytes in all. Raw files remain excluded from the historical
non-raw budget, but are no longer omitted from the reported distribution size.
This validates local publication consistency, not freshness of files served
by a remote deployment/CDN or equivalence to an unrebuilt Rust source tree.
