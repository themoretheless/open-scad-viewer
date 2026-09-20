# Packed G-code Preview Transport

## Measured Bottleneck

Baseline `4ae19e31`, geometry SHA-256
`a780636d5770f240c415f568b9022268e362b1e89eccc1a7e8a452cd3cc4cebb`.
Two independent warm stage measurements on 80,000 moves gave:

| Stage | Run A/B p50, ms |
| --- | ---: |
| WASM request, including native parsing and serialization | 82.05 / 82.57 |
| JS decoding and response release | 99.97 / 99.71 |
| Complete measured call | 183.42 / 183.51 |

The response occupied 10,240,435 bytes. The separate JS codec prototype
replaced object-per-move payloads with flat f64 rows, reducing its value payload
from 10,240,412 to 5,040,350 bytes. Alternating decoder measurements including
rebuilding ordinary JS move objects gave 8.54 / 8.44 ms versus 101.59 / 101.29
ms for object decoding. Those prototype measurements exclude Rust packing;
they motivated the implementation, not an end-to-end performance claim.
Baseline reports: `/private/tmp/osv-gcode-stages-{a,b}.json`.

## Compatible Opt-In

The four existing Rust operations (`gcode_preview`, `gcode_parse`, `mesh_gcode`,
`mesh_gcode_job`) accept optional boolean `packedMoves`. Missing/false retains
the old object response. True replaces preview `moves` with `moveRows`, whose
seven f64 values per move are X, Y, Z, cumulative E, feedrate, layer index and
extrusion flag (0 or 1). Invalid option types fail before parsing/slicing.
No opcode or MGV1 codec change is required.

The public JS wrappers opt in and restore their existing object-shaped return
types through one private transport adapter. Worker/UI consumers still receive
ordinary moves; there is no typed-array lifetime or borrowed WASM-memory API.
The adapter admits at most 100,000 complete rows, checks finite values, layer
indices and flags, and preserves f64 precision/signed zero. Source rows are
not retained by reconstructed move objects. Existing artifact integrity checks
bind the wrappers to their packaged kernel; no silent legacy fallback is used
for a malformed packed reply.

The Rust adapter still allocates numeric `Value` entries and the JS side still
allocates public move objects. This is not zero-copy or allocation-free. Worker
structured cloning, rendering, file IO and cold startup are outside these
warm-call measurements.

## Reproduction

`node --import tsx benchmarks/gcode-transport-stages.mts` explicitly verifies
and loads the public WASM artifact. It alternates legacy and packed calls on
the same module, checks complete decoded values against the public API, and
times encoding, copying, Rust/serialization and decoding/object restoration.
There are 20 warmups and 31 samples per workload (100, 10,000, 80,000 moves).
Request-buffer release occurs after the timed interval. Each stage's quantiles
are independent and need not sum to the total quantile.

The report also retains the separate JS codec experiment, clearly distinct
from actual native packed response measurements. Statistics retain only times
and byte counts, not every parsed scene. The initial pilot that retained all
results was discarded before the baseline runs above.

## Implemented Results

Two fresh-process campaigns alternate old and new responses on the same
rebuilt WASM. Complete public values compare equal, and input/result hashes
match between campaigns. No local tests/builds ran during these measurements.

| Moves | Legacy p50 A/B, ms | Packed + object rebuild p50 A/B, ms |
| --- | ---: | ---: |
| 100 | 0.2779 / 0.2763 | 0.0762 / 0.0765 |
| 10,000 | 23.2635 / 23.3129 | 3.9832 / 3.9808 |
| 80,000 | 186.6863 / 190.2166 | 33.9210 / 34.8462 |

The large-case warm boundary is approximately 5.5x faster. Its response shrinks
from 10,240,435 to 5,040,373 bytes. In campaign A the packed native/serialization
stage is 22.82 ms and decode/object rebuild is 9.87 ms. The separate unchanged
public `gcode-parse-boundary.mts` benchmark reports p50 0.0777 / 3.9193 / 32.5937
ms for the three workloads, confirming the wrapper path uses the optimization.
These are kernel/API measurements, not full UI latency or cold-start claims.

Reports: `/private/tmp/osv-gcode-packed-{a,b,public}.json`.
New geometry WASM is 7,723,512 bytes (+724), SHA-256
`909b94a4b895db447a184bcc6cfb544a226b51589b94b124424da6670eb0b8ca`.

## Verification

- 268 native geometry-bridge library tests pass.
- 20 focused host/worker/adapter tests pass. All four public wrappers match
  the corresponding legacy Rust operation responses, including job/3MF bytes.
- Adapter tests cover signed zero, double precision, no retained input rows,
  empty rows, malformed flags/layers/numbers, truncation and oversized replies.
- Vue/MCP typechecks and strict standalone benchmark typecheck pass.
- Vite and `verify-dist` pass: 92 artifacts, 15,718,333 total bytes.
- Chrome production G-code/SVG worker startup, reuse and cancellation recovery
  pass. The delayed cold-start STEP production-browser roundtrip also passes.
- Full Vitest: 3411 passed, nine existing qualification binding failures across
  four suites. This is not a green full-suite or qualification claim.

No archived qualification evidence or runtime-manifest binding was changed.
