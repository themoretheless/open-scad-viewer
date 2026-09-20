# SVG asynchronous startup

## Boundary

`svgPreview` and `svgProfile` previously called the synchronous Rust document
reader before any asynchronous kernel initialization. The first standalone SVG
operation therefore paid the synchronous integrity-check/compilation cost even
though its public API returned a Promise. Opening an evaluation session later
in `svgProfile` was too late to avoid this.

Both now use `readSvgDocumentAsync`. The synchronous OpenSCAD reader is retained.
The two readers share payload preparation: limits and options are validated,
and font bytes are serialized before awaiting `warmGeometryKernel`. Callers
cannot mutate the submitted options/fonts during compilation. Document parsing,
paint normalization, text outlining and profile construction remain in Rust.
No new geometry algorithm, artifact or qualification claim is introduced.

This only makes initialization asynchronous. Actual SVG execution remains
synchronous inside the kernel; the existing SVG worker remains the boundary
for cancellable interactive operations.

## Controlled measurements

Run `node --import tsx benchmarks/svg-cold-start.mts`. Each campaign alternates
9 fresh Node processes per mode. The workload is a 20 x 10 mm red rectangle
preview. Timing includes payload preparation, embedded decoding, verified
compilation, instantiation and preview; it excludes imports, process startup,
worker transport and UI. Warm results use 10 warmups and 31 measured calls per
process. No local builds or tests ran alongside either campaign.

| Campaign | Sync cold median | Async cold median | Sync warm median | Async warm median |
| --- | ---: | ---: | ---: | ---: |
| A | 189.342 ms | 86.122 ms | 0.085917 ms | 0.086667 ms |
| B | 192.548 ms | 87.168 ms | 0.085666 ms | 0.086209 ms |

Cold startup improves approximately 55% for this workload. Warm overhead is
below 0.001 ms in these runs; this is not an end-to-end UI latency claim.
All responses across 36 processes have SHA-256
`d848f79f995a40434eeea5ad4dee1cda8dfeb3f7f7171fba49534df84082c9e6`.
The unchanged geometry artifact is 7,716,482 bytes, SHA-256
`bb97e78ae87b5fa690fc474e76ac413b704b737cec3e319e4b548e0d92ac47d2`.

## Verification

- 32 focused document, geometry, worker and actual worker-boundary tests passed.
- New boundary tests cover validation before compilation, input snapshots,
  synchronous/asynchronous payload equality, compilation failure and retry.
- `scripts/check-geometry-worker-startup-browser.mjs svg` loads the production
  worker. Chrome 156.0.8063.3 observed one WebCrypto digest, zero synchronous
  modules larger than 1 MiB, no repeat digest on reuse, and successful recovery
  after terminating a worker paused inside hashing. No late response occurred.
- The shared browser probe also passes with `gcode`.
- Vue, MCP and standalone benchmark/test type checks passed; Vite build and
  distribution verification passed (92 artifacts, 15,708,287 total bytes).
- Full Vitest: 3394 passed, nine existing qualification-binding failures in
  four files; 355 files total, 124.98 seconds. This is not a green full suite.
  Log: `/private/tmp/osv-svg-async-full-tests.log`.

Existing qualified-manifest/archive drift remains a separate issue; see
[runtime manifest binding](runtime-manifest-binding-2026-09-20.md).
