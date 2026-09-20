# Foreign G-code Extrusion Accounting

Baseline: `67c472b3`. This selectively adapts the volumetric/flow functionality
reviewed in `gcode-core-fdm`, without replacing current preview, job or 3MF APIs.
The private Rust `foreign_extrusion` module owns logical E, tool parameters and
material totals. The interpreter retains commands, geometry, layers and time.
No new dependency, unsafe code or release-profile change is involved.

## Contract

- Linear E is filament length; volumetric E is volume. `G20` scales E by 25.4
  or 25.4 cubed respectively. Filament diameter always uses linear units.
- `M200 D` sets the target tool diameter and enables global volumetric mode;
  `S0` / `S1` explicitly toggle it. `D0` disables even alongside `S1`, retaining
  the previous diameter. A parameter-free query does not change the state.
- `M221 S` sets a nonnegative flow percentage for the active or `T`-selected
  tool. A query preserves flow. Unsupported `M221 D` selectors are explicitly
  refused instead of applying them to the wrong tool.
- Tool changes preserve logical E. Flow changes do not rescale logical E.
  `G92 E` resets that coordinate without erasing accumulated material.
- Tool identifiers are unsigned 32-bit integers stored sparsely, with at most
  256 distinct states including initial tool zero. High IDs do not cause huge
  arrays. Unknown named macros such as `T1000_RESET` remain skipped.
- Totals accumulate each positive physical filament advance and its volume
  using the parameters at that motion. Retractions update E but do not subtract
  from totals. Recovery and stationary priming count as positive advance; zero
  flow does not. These are not net deposition or net spool-consumption totals.
- Overflow and unrepresentable positive scale/product underflow return typed
  errors. Finite, nonnegative floating flow percentages are supported; firmware
  integer storage and truncation are not emulated.

The reader does not execute tool-change macros, firmware retraction, temperature
waits, pressure advance, offsets or volumetric speed limits. Estimated time
remains the existing nominal feedrate calculation. Generator/flavor detection
does not imply full emulation of that firmware. Native strict dialects are
unchanged.

## Source Basis

Semantics were checked against [Marlin M200](https://marlinfw.org/docs/gcode/M200.html),
[M221](https://marlinfw.org/docs/gcode/M221.html), and their implementation:
[M200 state changes](https://github.com/MarlinFirmware/Marlin/blob/2.1.x/Marlin/src/gcode/config/M200-M205.cpp),
[unit conversion](https://github.com/MarlinFirmware/Marlin/blob/2.1.x/Marlin/src/gcode/parser.h),
and [planner factors](https://github.com/MarlinFirmware/Marlin/blob/2.1.x/Marlin/src/module/planner.h).
No firmware source was copied. Bare flag forms are not newly supported.

[Bambu's own unload program](https://github.com/bambulab/BambuStudio/blob/master/resources/printers/ams_unload.gcode)
uses `T255`, motivating sparse storage rather than a small numeric-ID ceiling.
Accepting that identifier does not mean emulating its manufacturer-specific
meaning. Tool configuration is intentionally a bounded preview model.

## Native Control Measurements

Same checked-in `bench_foreign_parse` fixture, existing release profile,
macOS aarch64, 10 warmups and 31 samples. Two retained binaries ran A-B-B-A
without concurrent local builds or tests. Fixtures use ordinary linear extrusion
without M200/M221, measuring overhead on the existing workload, not speedup.
Median milliseconds; each pair is two independent process runs:

| Moves | Layout | Baseline A1 / A2 | Candidate B1 / B2 |
|---:|---|---:|---:|
| 10,000 | spaced | 1.694 / 1.663 | 2.030 / 1.631 |
| 10,000 | compact | 1.721 / 1.545 | 1.646 / 1.556 |
| 10,000 | comments | 1.913 / 1.973 | 2.095 / 2.229 |
| 80,000 | spaced | 12.994 / 13.072 | 13.462 / 13.851 |
| 80,000 | compact | 11.947 / 12.435 | 11.912 / 12.382 |
| 80,000 | comments | 16.393 / 15.604 | 16.061 / 17.077 |

Spaced 80k input has roughly 3-7% extra native cost. Compact ranges overlap;
comments and smaller inputs are noisier. No performance improvement is claimed
for this correctness extension. Coefficients are cached on configuration/tool
changes; the motion loop performs no per-move tool lookup or area calculation.
Raw samples: `/private/tmp/osv-gcode-extrusion-final-{a1,b1,b2,a2}.json`.

## WASM Boundary Controls

`benchmarks/gcode-parse-boundary.mts`, Node 22.23.2 on macOS arm64, 50 warmups
and 31 samples per workload, separate A-B-B-A processes. All local builds and
tests finished before timing. The compiler hook verifies the selected artifact;
result assertions run outside timing. Median milliseconds:

| Moves | Baseline A1 / A2 | Candidate B1 / B2 |
|---:|---:|---:|
| 100 | 0.274 / 0.269 | 0.269 / 0.266 |
| 10,000 | 23.542 / 23.856 | 23.551 / 23.350 |
| 80,000 | 187.606 / 188.756 | 188.506 / 189.362 |

At 80k, p95 is 194.036 / 196.954 ms before and 193.707 / 198.344 ms after.
These runs show no clearly separated complete-call regression or improvement.
The scope includes warm WASM parsing and value transport, not file IO, worker
structured clone, rendering or cold startup. No UI frame-rate claim follows.
Raw reports: `/private/tmp/osv-gcode-extrusion-wasm-{a1,b1,b2,a2}.json`.

Reproduce with `node --import tsx benchmarks/gcode-parse-boundary.mts`; select
retained baseline bytes with `GCODE_WASM_PATH`. Native controls use the existing
`bench_foreign_parse` example with both binaries built before measurement.

## Verification

The gcode-core, gcode-optimize and slicer-core suites pass 91 tests, including
11 new extrusion tests. These cover 96 deterministic combinations of diameter,
flow, units, volumetric mode and absolute/relative E, plus mode changes without
G92, arcs, priming, retractions, query commands, sparse IDs, numeric extremes
and the 256-state bound. Initial regression fixtures failed on the baseline.

Scoped Clippy passes with the same three preexisting lint categories allowed
on the command line; no repository lint configuration was relaxed. Historical
qualification archives are not rewritten by this change.

All 285 geometry-bridge tests pass, including its geometric integration suites.
The rebuilt WASM passes 38 focused host/worker/panel tests. Vue and MCP
typechecks pass. The complete Vitest run passes 3,367 tests and fails the same
nine historical qualification-binding checks across four files (351 files,
110.97 seconds). This does not constitute a green qualification or remote CI.

Production Vite build and `verify-dist` pass: 92 artifacts, 5,953,312 asset bytes
plus 9,744,344 raw WASM bytes, 15,697,656 bytes total. Geometry WASM grows from
7,711,510 to 7,716,482 bytes (+4,972). Baseline SHA256:
`1d906a431d11c13a5667b6695637bcd604396ac84305c81e8a7954fa7440845e`;
candidate SHA256:
`bb97e78ae87b5fa690fc474e76ac413b704b737cec3e319e4b548e0d92ac47d2`.
