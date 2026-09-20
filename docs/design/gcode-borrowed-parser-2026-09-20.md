# Bounded Borrowed G-code Parsing

## Scope

Baseline: `efc4268d`. This selectively integrates the lexer/interpreter review
of `gcode-core-fdm` (`a96efc17`) into the existing foreign preview reader.
It does not replace native preview v2, the job dialect, layer identity, firmware
flavors or 3MF packaging. It is not a printer protocol or a print-safety check.

The branch was executed in an isolated archive with unchanged `lex.rs`,
`interpret.rs` and `command.rs`. A small example called `lex_line` and
`parse_fdm` after `G90`, `M83` and a known initial position:

| Last line | Branch X | Branch Y | Branch extrusion mm |
|---|---:|---:|---:|
| `G1 X10 Y1 E2 F600` | 10 | 1 | 2 |
| `G1X10Y1E2F600` | 10 | 100 | 0 |
| `G1X10E2F600` | 1000 | 0 | 0 |

Its lexer consumes `E` as a numeric exponent. The current implementation keeps
the existing compact extruder-word semantics instead. The branch's lexer SHA256
was `ebdff4291067fe6967488883e1620db8b2030a6ddeb8136cd9eeae1c19cded58`.
The executable audit log is `/private/tmp/osv-gcode-legacy-compact.log`.

## Changes

- `foreign_words` owns comment/suffix removal, borrowed word boundaries and
  command normalization. Ordinary lines and semicolon/checksum suffixes do not
  allocate cleaned strings; parenthesized comments use one bounded string.
- The interpreter retains numeric admission, modal state, layers and totals.
  It no longer allocates strings for each word, a normalized command string,
  a separate vector of word references, or a point vector for a linear move.
- Checksum suffixes attached directly to a numeric word are now discarded
  consistently. They remain **unverified**, as required by the tolerant reader's
  existing contract; native strict dialects have not been relaxed.
- A semicolon inside a parenthesized comment no longer discards the rest of
  that motion line. Nested comments retain the earlier depth behavior.
- I/J full circles without X/Y are no longer silently dropped. This form is
  explicitly supported by [Marlin's arc documentation](https://marlinfw.org/docs/gcode/G002-G003.html).
- Both source motion count and expanded output count are bounded at 100,000.
  Previously 100,000 source arcs could expand to roughly 6.4 million points.
  Refusal occurs before extending the preview and before WASM serialization;
  the temporary arc contains at most the existing 64 points.

No whole-program AST, new crate dependency, release-profile change or unsafe
code was introduced. Native strict parsing and foreign tolerant parsing remain
separate because their admission rules differ, not because they need a shared
generic parser abstraction.

## Native Measurements

`bench_foreign_parse` builds deterministic alternating unit-length moves,
checks output count, layers, extrusion, distance and time outside timing,
then performs 10 warmups and 31 samples. Timing includes output allocation
and destruction, but not fixture construction. Existing release settings:
size optimization and LTO, macOS aarch64. Both executables were built before
the A-B-B-A sequence; no local build or test ran during timing.
Compiler: `rustc 1.100.0-nightly (215a8af4b 2026-09-15)`.

Median milliseconds, with each pair showing the two separate process runs:

| Moves | Layout | Baseline | Candidate |
|---:|---|---:|---:|
| 10,000 | spaced | 4.136 / 4.064 | 1.937 / 1.703 |
| 10,000 | compact | 3.933 / 4.091 | 1.435 / 1.511 |
| 10,000 | comments | 4.217 / 4.316 | 1.972 / 2.060 |
| 80,000 | spaced | 32.316 / 33.986 | 12.927 / 12.743 |
| 80,000 | compact | 31.328 / 32.376 | 11.991 / 12.365 |
| 80,000 | comments | 33.671 / 34.509 | 16.620 / 15.775 |

At 100 moves, the first candidate process had intermittent medians around
0.055 ms versus 0.020 ms in the second. Do not claim a stable improvement for
that small workload. The large-workload native improvement is repeatable;
it does not by itself establish a browser or complete import speedup.

Reports, including every sample:
`/private/tmp/osv-gcode-parse-{a1,b1,b2,a2}.json`.

Reproduce the candidate:

```sh
cargo run --locked --release --manifest-path crates/Cargo.toml -p gcode-core --example bench_foreign_parse
node --import tsx benchmarks/gcode-parse-boundary.mts
```

For controls, build the identical benchmark example against the baseline in
an isolated checkout, retain both executables, and then run them sequentially.
The WASM benchmark accepts `GCODE_WASM_PATH` to select a retained raw artifact;
it verifies that the actual compiler hook used those bytes and records SHA256.
It measures the full `gcode_preview` value transport, not just Rust parsing.

## WASM Boundary Measurements

Node 22.23.2, macOS arm64, 50 warmups and 31 samples per workload in each
separate process, again A-B-B-A without concurrent local builds/tests. Raw
artifacts were compiled through the real optional compiler hook. Assertions
run outside the timed call; fixture construction and startup are excluded.
The spaced inputs match the native workload. Medians in milliseconds:

| Moves | Baseline A1 / A2 | Candidate B1 / B2 |
|---:|---:|---:|
| 100 | 0.292 / 0.293 | 0.269 / 0.267 |
| 10,000 | 25.581 / 25.718 | 23.077 / 23.043 |
| 80,000 | 205.903 / 204.536 | 187.136 / 185.563 |

For 80,000 moves, p95 is 212.390 / 213.294 ms before and 193.120 / 194.413 ms
after. The complete warm call improves by roughly 9%, not the native parser's
roughly 2.5x. Each returned move is still materialized as a value object in
`geometry-bridge::gcode::preview_value`, encoded and decoded across the ABI.
Those costs were not separately profiled, so do not attribute all remaining
time to one stage. A packed move buffer is a future measurement candidate,
not a benefit delivered by this change.

The application already invokes parsing in `gcodePreviewRuntime` in its
dedicated worker. These numbers do not include that worker's structured clone,
file reading, rendering or cold embedded-WASM decoding; they are not UI frame
times. Reports: `/private/tmp/osv-gcode-wasm-{a1,b1,b2,a2}.json`.

Artifact identity:

- Baseline: 7,712,405 bytes, SHA256
  `eb44e5441614ed25604b0e8cf095cf2df7d75826de9236f7ca2b4444b716b8f8`.
- Candidate: 7,711,510 bytes, SHA256
  `1d906a431d11c13a5667b6695637bcd604396ac84305c81e8a7954fa7440845e`.

The rebuilt tracked WASM is 895 bytes smaller. Build profiles and dependencies
are unchanged; historical qualification identities are not rewritten.

## Verification And Remaining Work

Three initial regressions were red before the change: glued checksum words,
dropped full circles, and output expansion beyond 100,000 moves. They now pass.
Additional fixtures compare 32 deterministic 64-move programs in spaced,
lowercase compact/numbered/checksummed, and nested-comment layouts, including
layer markers. Invalid numbers, coordinates, feedrates, arcs and input budgets
remain typed errors with source line numbers.

All nine `meshToolpathHostApi` tests pass against the rebuilt WASM, including
compact extrusion/full circles through both `gcode_preview` and `gcode_parse`
and the typed refusal for expanded output above the move limit. Vue and
standalone benchmark typechecks pass.

The complete Vitest run passes 3,365 tests and fails the same nine historical
qualification binding checks across four files (351 files, 110.09 seconds).
No new test failure appeared. The geometry-bridge native suite also passes
all 265 tests. Production Vite build and `verify-dist` pass: 92 artifacts,
5,952,292 asset bytes plus 9,739,372 raw WASM bytes, 15,691,664 bytes total.
This is local verification, not a claim that the new remote CI has passed.

The gcode-core, gcode-optimize and slicer-core suites pass (80 tests total).
Strict Clippy is blocked by six existing lints in `job.rs` and `package_3mf.rs`;
the all-targets check passes with only those three lint categories allowed on
the command line. No lint configuration or unrelated source was changed.

The old branch is still not fully integrated: volumetric/flow interpretation,
thermal state and firmware-retraction semantics require explicit adaptation
to current APIs. This change makes no new material-consumption accuracy claim
for those unsupported modes. Historical qualification archives are unchanged.
