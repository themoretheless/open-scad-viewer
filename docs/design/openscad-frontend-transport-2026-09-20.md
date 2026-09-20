# OpenSCAD frontend transport measurement

## Decision

Do not switch the product's TypeScript parser to `scadCompileRust` for a
performance claim. The current complete-AST WASM endpoint is slower on the
measured workloads, even after initialization. This does not establish whether
the Rust parser itself is slower: the endpoint also encodes the request,
serializes the AST, decodes it into JavaScript objects and frees ABI buffers.

The next architecture experiment should keep AST consumers inside Rust and
return a compact execution/lowering result. The existing Rust evaluator only
returns shape descriptors, so it is not yet a replacement for CAD execution.
An alternative compact AST transport needs its own end-to-end measurements.

## Reproduction

`node --import tsx benchmarks/openscad-frontend.mts`

Node 22.23.2, macOS arm64; two separate sequential runs, no concurrent build or
test process launched by this task. Each fixture uses five warmups and 21
alternating TS/Rust samples. Full AST parity is checked outside every timed
sample. Allocation and GC are included. Inputs are repeated translated cubes
with vector arguments and arithmetic; this is not a representative corpus of
all programs or a browser/geometry benchmark.

| Call nodes | TS median, ms | Rust ABI median, ms | TS repeat, ms | Rust repeat, ms |
| ---: | ---: | ---: | ---: | ---: |
| 2 | 0.02175 | 0.08638 | 0.02163 | 0.08413 |
| 200 | 0.49858 | 4.20204 | 0.57971 | 4.20442 |
| 1000 | 2.67100 | 20.91338 | 2.74025 | 20.91604 |

Cold language initialization was 18.70 and 19.18 ms, reported separately;
module import time is excluded. The script emits every sample, source hashes,
compiler/host/generated payload hashes and environment information.
Local reports: `/private/tmp/osv-openscad-frontend.json` and
`/private/tmp/osv-openscad-frontend-repeat.json` (not durable release evidence).

## Compatibility boundary

`openscadRustFrontend.test.ts` now compares complete trees, not just top-level
statement names, for all conformance fixture sections under both profiles.
Additional nested-expression cases must compile under the stable profile;
subset rejection is compared exactly. Together with the evaluator suite,
43 tests passed before this benchmark was introduced.

The wire comparison explicitly converts TS numeric operator enums to their
names, `undef` literals to null, nonfinite numbers to `$number` objects and
omits undefined optional fields. These are representation conversions, not
proof that the wire AST can directly enter the TS evaluator. No production
adapter or routing change is made here.

The corpus is not universal acceptance parity: an assignment containing 125
literal terms joined by `+` passes full-tree comparison, but 126 terms still
parse in TypeScript while the Rust endpoint returns `LANGUAGE_TRANSPORT` with
`Response exceeds transport limit`. The MGV1 response nesting limit is 128;
response wrappers add depth to the expression tree. A regression test covers
both profiles and successful reuse of the kernel after the refusal. The host
result types now include this boundary failure instead of promising source
diagnostics for every refusal. This records, but does not remove, a blocker to
using the complete-AST endpoint as the product compiler. Forty-four frontend
and evaluator tests pass with this boundary test included.
