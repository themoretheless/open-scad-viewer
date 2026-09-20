# Foreign G-code Feedrate Overrides

The `gcode-core-fdm` branch at `a96efc17` classifies M220 as `SpeedFactor`,
but its interpreter discards that command. Main also skipped it. This change
implements the missing behavior in the existing bounded foreign reader,
without replacing strict preview/job/3MF APIs or adding a second interpreter.

## Semantics and Scope

The private `foreign_feedrate` module separates the programmed F value from
the global percentage and saved percentage. M220 S changes the override;
bare B saves it; bare R restores it. A command with both B and R saves the
incoming value, then S, if present, supplies the final override. Both factors
start at 100 percent. A parameterless query leaves them unchanged.

References inspected on 2026-09-20:
[Marlin M220 documentation](https://marlinfw.org/docs/gcode/M220.html) and
[Marlin handler](https://github.com/MarlinFirmware/Marlin/blob/2.1.x/Marlin/src/gcode/config/M220.cpp).
The preview deliberately accepts only integral positive percentages through
32767 and refuses unsupported parameters, rather than reproducing signed
integer overflow, truncation, or zero-speed execution. Effective speed must
remain finite, positive and within the existing preview coordinate/speed cap.
This is a documented preview admission policy, not universal firmware emulation.

The override applies to subsequent linear and XY arc preview moves, including
new F values and inch-mode unit conversion. Restoring 100 percent recovers
the programmed speed rather than compounding an earlier override. Unknown
F stays unknown. Geometry and cumulative positive extrusion are unchanged.
Estimated time remains distance divided by effective feedrate: no acceleration,
jerk, heating, dwell, E-only motion duration or firmware-specific fixed G0 rate
is modeled by this change. It is not a physical print-time guarantee.

## Verification

Four new native integration tests cover modal behavior, simultaneous B/R,
unit changes, arcs, invariant material/geometry, range refusals and unknown F.
Host/WASM tests check the same override sequence through both parse APIs.
The real worker fixture combines M200/M221 accounting with M220 S50 and must
report 2 seconds for 10 mm at a programmed 10 mm/s, preserving material totals.
Both new host/worker assertions failed against the preceding artifact before
the WASM rebuild, which returned unmodified speeds and a 1-second estimate.

The 80 native G-code/optimizer tests, `cargo check -p slicer-core`, Vue and
MCP typechecks pass. Vite and `verify-dist` pass with 92 artifacts and
15,714,738 total bytes. Chrome 156.0.8063.3 production-worker probes pass
for G-code (including M220's exact 2-second result) and SVG, asynchronous
verified startup, reuse and recovery after cancellation.

Full Vitest: 3406 passed, nine failed in the same four historical qualification
artifact-binding suites. Those archives are unchanged; this is not a green
qualification result or a claim that CI has completed.

Geometry WASM grows by 1,263 bytes to 7,721,502 bytes, SHA-256
`344139bfaba42c1fefbb2aa4a3e348ccd5e4f9b4ef7f2b2a897287695d63c368`.

## Performance Observation

`node --import tsx benchmarks/gcode-parse-boundary.mts` was run before and
after rebuilding, with no concurrent tests/builds, 50 warmups and 31 samples
per workload. This measures the complete warm WASM call and JS transport,
not only native parsing. Inputs contain ordinary moves without M220.

| Moves | Before p50, ms | After p50, ms |
| --- | ---: | ---: |
| 100 | 0.270 | 0.260 |
| 10,000 | 22.592 | 23.170 |
| 80,000 | 182.753 | 185.637 |

This single ordered comparison is not a repeatable speedup/regression claim;
the largest case is 1.6% slower in this observation. The change is retained
for correct override behavior, not advertised as a performance improvement.
Reports: `/private/tmp/osv-feedrate-{before,after}.json`. The baseline is
main `31b830cc`, artifact SHA-256
`88e587c92c1b5f596f009c3187cf00a9f03ffee9c061ebecb5068a245dc744e7`.

Other unique branch features, including thermal timelines and firmware
retraction interpretation, remain pending; this is not a whole-branch merge.
