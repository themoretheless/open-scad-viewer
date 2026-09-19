# Base85 full-word decoding

## Change

The Rust bootstrap decoder previously checked `output.len() < size` for every
decoded byte. The package length has already been validated, so full four-byte
words can be appended directly. Only the final partial word needs a padding
check. Checked base85 arithmetic, input/output limits and complete Brotli stream
validation remain unchanged. No unsafe code or new dependency was introduced.

New native coverage roundtrips every payload length from 5 through 512 bytes,
including every tail length, and rejects nonzero padding for partial words.
Existing overflow, invalid character, truncation and resource-limit tests remain.

## Measurement

`benchmarks/wasm-bootstrap.mjs` now also reports async geometry compile,
instantiate and total ready phases. Existing `totalMs` still ends after the
decoder output copy, so historical decoder results remain comparable.

Two runs, nine fresh-worker samples per variant in each run, alternating variant
order, same packed geometry and byte-for-byte comparison to `kernel_bg.wasm`:

| Median (ms) | Before | After | Before repeat | After repeat |
| --- | ---: | ---: | ---: | ---: |
| Rust base85+Brotli decode | 42.374 | 40.508 | 42.672 | 40.648 |
| Decoder bootstrap through copy | 52.765 | 50.763 | 53.063 | 51.077 |
| Geometry instance ready | 59.105 | 57.265 | 59.533 | 57.558 |

About 3.1-3.3% reduction to ready on this fixture/environment. This is not
whole-job latency or proof that CI readiness timeouts are fixed. Fresh workers
share a Node process: V8/OS caches may warm, and the measured geometry compile
phase must not be presented as cold-process compilation time. Fixture reads,
module imports, verification and worker lifecycle are outside the timer.

Decoder WASM shrank from 210236 to 210133 bytes; its packed base64 literal grew
from 122876 to 122892 bytes. Total dist increases by 16 bytes to 5777221 bytes
excluding raw streaming WASM. No size budget was increased for this change.

Reports (including environment, input SHA256 and all samples):
`tmp/performance/bootstrap-base85-tail.json` and
`tmp/performance/bootstrap-base85-tail-repeat.json`. Saved baseline decoder:
`/private/tmp/osv-brotli-before-tail.wasm`.

## Verification

- Native `cargo test -p wasm-brotli`: 4 tests passed.
- WASM packing, Brotli packing and streaming fallback: 16 Vitest tests passed.
- Production Vite build and `verify-dist`: passed, 88 artifacts.
- Production SVG/G-code Chrome workers: passed cold initialization, operations,
  refusal and recovery (`tmp/performance/base85-tail-workers/report.json`).

The generated decoder was rebuilt with the normal `buildWasmBrotli` helper.
