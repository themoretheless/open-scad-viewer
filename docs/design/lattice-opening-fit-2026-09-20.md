# Nominal Lattice Opening Fit

Selectively replaces the opening optimizer from PR #7 at
`7b3afccf379aead6151c7ceaf9b633e40eb62924`. The remaining strength UI,
scenario and heuristic recommendations are not imported by this change.

## Reproduction

`node scripts/audit-lattice-branch.mjs` evaluates the unchanged pinned legacy
implementation. With nozzle 0.6, layer 0.25, four lines, cell 6, rib 1.35 and
maximum opening 1 mm, ordinary fitting produces an opening of 3.375 mm.
Its optimizer thickens the rib and then raises the minimum cell, increasing
the opening to 6.075 mm rather than satisfying the requested limit.

## Native Replacement

The existing `cad_lattice_print_fit` operation accepts optional boolean
`limitOpening`, default false. After nozzle quantization, it preserves rib
thickness and constrains `cell - rib` without violating `cell >= 2*rib + width`.
An incompatible limit returns an error. Underflowed extrusion width and
overflowed fitted rib/cell also fail. No additional opcode is introduced.

The workbench exposes an opt-in checkbox. Failed fitting leaves options
unchanged; transient empty numeric input does not throw during rendering.
This is a nominal geometric constraint, not actual toolpath bridge length,
strength, orientation analysis, or a promise of support-free printing.
Existing geometry generation budgets still apply after fitting.

## Verification

- 268 native geometry-bridge library tests pass, including three new tests
  with 128 combinations of nozzle, line count and opening limit.
- 49 focused host/WASM and workbench tests pass. New boundary tests failed
  against the preceding WASM before rebuilding it.
- Vue and MCP typechecks pass.
- `node scripts/check-lattice-print-browser.mjs` passes on Chrome
  156.0.8063.3 at 1280x900 and 390x844: successful fit, atomic refusal,
  recovery after empty input, no page errors or panel overflow.
- Vite production build passes. This is not qualification or print validation.
- `verify-dist` passes: 92 artifacts, 15,712,890 total bytes. Production SVG
  and G-code workers pass verified asynchronous startup, reuse and recovery
  after cancellation with the rebuilt artifact.
- Full Vitest run: 3405 passed, nine failed across four existing qualification
  artifact-binding suites. Historical qualification archives were not changed;
  the full suite and current-artifact qualification are not green.

Rebuilt geometry WASM: 7,720,239 bytes (+3,757), SHA-256
`88e587c92c1b5f596f009c3187cf00a9f03ffee9c061ebecb5068a245dc744e7`.
This feature makes no performance improvement claim.
