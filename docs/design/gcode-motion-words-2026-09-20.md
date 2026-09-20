# Fixed-Field G-code Motion Parsing

Baseline: main `be0f0a36`. Foreign G0-G3 parsing allocated a temporary vector
of numeric words per line, then repeatedly scanned it backward for XYZ, E, F
and arc parameters. The new private `MotionWords` parses those fields once
into fixed stack storage. Unknown fields are still numerically validated;
duplicate fields retain last-value-wins behavior. Shared `axis_word` keeps the
same validation/errors for motion and configuration commands.

G92 retains ordered processing: replacing all command vectors indiscriminately
would risk changing validation of intermediate duplicate E resets. No public
API, motion/line budget, extrusion semantics or arc approximation changed.
This removes temporary motion-word allocations, not output mesh/move allocation
or every allocation in the parser (comment stripping may still allocate).

## Native Controls

Both release executables were built before timing. Existing
`bench_foreign_parse` checks movement count, layer count, positive extrusion,
distance and nominal duration. Each workload has 10 warmups and 31 samples.
Runs were ordered baseline A, candidate A, candidate B, baseline B, without
concurrent local builds/tests. macOS/aarch64, native parse including result
allocation/drop, not WASM or UI.

| Moves / style | Baseline A/B p50, ms | Candidate A/B p50, ms |
| --- | ---: | ---: |
| 10,000 spaced | 1.988 / 1.823 | 1.770 / 1.500 |
| 10,000 compact | 1.664 / 1.624 | 1.434 / 1.384 |
| 10,000 comments | 2.208 / 2.286 | 1.927 / 1.920 |
| 80,000 spaced | 13.720 / 14.123 | 12.137 / 12.282 |
| 80,000 compact | 12.738 / 13.036 | 11.306 / 11.564 |
| 80,000 comments | 16.448 / 17.436 | 15.159 / 15.089 |

The large cases improve in both comparisons, roughly 8-13%. The 100-move
cases show material run-order variation; no small-input speedup is claimed.
Reports: `/private/tmp/osv-motion-{before,after}-{a,b}.json`.
Executable hashes:

- baseline: `c97273c0501bc7e7e35b0aaadae7feada2c8d94384d948df55921c26d4268e66`
- candidate: `67704a1166bc8550bea85019637edfb6e066f2a7b1df207a3e0a2bbd9ff769eb`

## Correctness

All 83 G-code/optimizer native tests pass. A new private differential test
compares fixed fields with the prior ordered lookup for 26 rotated sequences
of all 26 letters repeated four times with changing values/case. Malformed
known and unknown fields, including a bad value followed by a valid duplicate,
return the same errors rather than being hidden by last-value-wins behavior.
Existing tests cover compact words, comments, units, arcs, material accounting,
feedrate overrides and strict job/3MF compatibility.

## WASM Boundary

The existing `gcode-parse-boundary.mts` benchmark measures the complete warm
call, including JS transport, with 50 warmups and 31 samples. One separate
process before and after rebuilding (no overlapping tests/builds) gave:

| Moves | Before p50, ms | After p50, ms |
| --- | ---: | ---: |
| 100 | 0.2704 | 0.2662 |
| 10,000 | 22.9074 | 23.4865 |
| 80,000 | 188.1729 | 188.7454 |

There is no demonstrated full-boundary speedup: the large case is essentially
flat in this observation and the middle case is 2.5% slower. This single
ordered comparison does not distinguish a small regression from variability.
Retention is based on the repeated native benefit, not an asserted UI win.
Reports: `/private/tmp/osv-motion-wasm-{before,after}.json`.

Geometry WASM grows 1,286 bytes to 7,722,788 bytes; SHA-256:
`a780636d5770f240c415f568b9022268e362b1e89eccc1a7e8a452cd3cc4cebb`.
The baseline artifact SHA-256 is
`344139bfaba42c1fefbb2aa4a3e348ccd5e4f9b4ef7f2b2a897287695d63c368`.
Production Vite/`verify-dist` passes with 92 artifacts and 15,715,974 total
bytes. Chrome production G-code worker startup, reuse, M220 semantics and
recovery after cancellation pass. `cargo check -p slicer-core` also passes.
The delayed cold-start STEP production-browser scenario passes with the new
artifact. Full Vitest: 3408 passed and nine existing qualification binding
failures across four suites; historical evidence was not rewritten.
