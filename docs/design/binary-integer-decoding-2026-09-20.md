# Binary integer decoding

The MGV1 decoder now combines the unsigned low word with a signed or unsigned
high word instead of creating a BigInt for each integer. It still rejects
non-safe integer results, checks all eight bytes before reading and leaves
floating-point decoding (including negative zero) unchanged. Object keys,
prototype protection, nesting and item limits are unchanged.

Tests compare decoding against DataView's BigInt reference for boundary values,
1000 deterministic 64-bit bit patterns and 2000 derived positive/negative safe
integers, under both integer tags. Existing truncation, prototype-key, numeric
triple and CAD transport tests remain in the focused gate.

## Measurements

Node 22.23.2 arm64. Run:

`node --import tsx benchmarks/binary-integers.mts /path/to/baseline.ts`

The baseline is `src/services/valueBinaryCodec.ts` at commit `3f79fe14`.
Five warmups and 21 alternating pairs per fixture; exact parity checks excluded
from timing, allocations/GC included. The report records both source hashes.

For 100,000 signed/unsigned safe integers spanning both 32-bit words:

| Run | Baseline median ms | Candidate median ms |
| --- | ---: | ---: |
| 1 | 2.287 | 1.028 |
| 2 | 2.352 | 1.112 |

The 1000-integer case varied substantially across runs; no stable small-input
speedup claim. Full OpenSCAD frontend transport on 1000 call nodes measured
20.817 ms after the change versus 20.913/20.916 ms in previous controls: no
meaningful end-to-end improvement established. This optimization does not
justify switching the product parser to the full-AST Rust endpoint.

A preliminary CPU profile of the full benchmark included warmups and untimed
assertions: approximately 314 ms sampled in WASM, 308 ms in the binary codec,
82 ms in Node encoding, plus substantial assertion/normalization cost. Those
figures locate investigation targets, not precise production time shares.

Local reports are `/private/tmp/osv-binary-integers.json`,
`/private/tmp/osv-binary-integers-repeat.json` and
`/private/tmp/osv-frontend-integer-candidate.json`.
