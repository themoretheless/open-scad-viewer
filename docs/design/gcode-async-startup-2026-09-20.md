# G-code Worker Async Startup

Baseline: `d6de312d`. Artifact verification introduced a measured synchronous
SHA-256 cost. The G-code worker previously entered `executeGcodePreview` directly,
causing cold synchronous decoding, hashing and compilation. It now uses
`executeGcodePreviewAsync`: validate the request, await the existing shared
`warmGeometryKernel`, then execute the same synchronous geometry operation.

The synchronous host API remains available. Validation, request IDs, error
envelopes and operation dispatch are shared, not separately implemented for
the two modes. Scalar settings and parse text are captured before awaiting.
The worker owns the structured-cloned request; no additional large mesh clone
or transfer of scene-owned buffers was introduced. Kernel initialization is
still shared once per worker, and processing itself remains synchronous WASM.

Invalid requests fail before warmup. Warmup errors use the existing response
envelope. Cancellation, supersession, disposal and timeout remain the client's
worker-termination policy; there is no attempt to interrupt a running WASM call
with a JavaScript flag. The existing idle worker reuse/expiry policy is unchanged.
No network compiler is installed in the worker; it uses verified embedded bytes.

## Measurements

`node --import tsx benchmarks/gcode-cold-start.mts` compares the synchronous
control and async entry in fresh Node processes, nine per mode, alternating
forward/reverse order. The fixture has 100 moves. Cold timing includes request
validation, embedded decoding, verified compilation, instantiation and parsing.
It excludes module imports, process startup, worker transport, file IO and UI.
Warm timing uses ten warmups and 31 samples per process. Results are checked
outside timing for move count, distance, material advance and exact output hash.

Node 22.23.2, macOS arm64. No local builds or tests ran during either campaign.
Median milliseconds; warm entries are medians of the per-process warm medians:

| Path | Campaign A cold / warm | Campaign B cold / warm |
|---|---:|---:|
| Synchronous control | 185.339 / 0.286 | 185.501 / 0.285 |
| Async warmup | 81.834 / 0.296 | 82.166 / 0.293 |

First execution improves by about 56% on this workload. Warm Promise overhead
is about 0.008-0.010 ms, not zero. This does not establish a 56% improvement in
complete panel opening, file loading, worker startup or browser rendering.
Artifact: 7,716,482 bytes, SHA256
`bb97e78ae87b5fa690fc474e76ac413b704b737cec3e319e4b548e0d92ac47d2`.
All 36 processes produce response SHA256
`e1e4e4247ed1fbff4caedf29b85486adb0e8263e164d3964347d739f93e497d9`.
Reports: `/private/tmp/osv-gcode-cold-start-{a,b}.json`.

## Runtime Verification

Unit tests verify admission before warmup, deferred execution, stable IDs/text
across the await, warmup failure/retry, and shared slice/job results. Real Node
workers execute the shipped entry and actual WASM for parsing, slicing, job
export and round-trip parsing. They preserve volumetric/flow accounting and
recover from typed parse errors without replacing a healthy worker.

Cancellation, supersession and timeout tests first observe entry into a
deliberately pending compiler hook, then require the worker to exit and a
replacement to return valid results. This establishes cancellation of active
warmup, rather than cancellation before a worker has started.

`scripts/check-gcode-worker-browser.mjs` loads the built production worker in
Chromium, with observation hooks around WebCrypto and module construction:
one digest on the first request, still one on the second, and no synchronous
module larger than 1 MiB. Termination during an entered digest yields no late
response and a replacement succeeds. Both outputs are identical. Chromium
156.0.8063.3 passed with `gcodePreview.worker-BEZgX5VA.js`; report:
`/private/tmp/osv-gcode-async-browser.json`.

This is a production-worker behavior check, not a browser latency benchmark or
qualification. Existing qualified-manifest attribution and frozen qualification
binding issues are separate and are not marked repaired by this change.

The complete Vitest run passes 3,391 tests and fails the same nine historical
qualification bindings (354 files, 125.32 seconds). Vue, MCP and standalone
benchmark/worker-test typechecks pass. Production build and `verify-dist` pass:
92 artifacts, 5,963,800 asset bytes plus 9,744,344 raw WASM bytes, 15,708,144
bytes total. This is local verification, not a claim that remote CI is green.
