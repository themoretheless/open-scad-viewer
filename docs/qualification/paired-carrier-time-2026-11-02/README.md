# Fair temporal refinement of paired original carriers

Base: `9112cb49a`; diagnostic reasoning and reproducible temporary trace are in
`diagnosis.md`. The trace instrumentation was removed before this qualification.

When Q=(1-t)C0+t*C1 has an inconclusive coarse UV normal, the paired source
context now considers all three original parameter widths, including t. The
unpaired context preserves its UV-only hint. This changes only subdivision
proposals; every original jet, complete fixed-metric or pair proof, work limit
and failure flag remains mandatory. No budget, tolerance or predicate changed.

The independent two-span, quarter-turn carrier interpolation certifies at
**3928 cells / 12 exact work**. Both measured budgets replay; each one-less
budget refuses. A map collapsing both cap axes at t=1/2 refuses all global flags.

Native evidence (nightly-2026-09-16, locked Cargo, test opt-level=1, test/dev
debug info disabled, two build/test threads):

- Cargo `-p nurbs-core --lib rotating_carrier_tangents -- --nocapture`: **1 passed**.
- Same freshly built NURBS unit executable, no filter, `--test-threads=2`:
  **2225 passed / 0 failed / 1 ignored**, 77.71 seconds.
- Cargo `-p brep-core --lib rational_hollow_cap_quarter_turn_frame -- --nocapture`:
  **0 passed / 1 failed**, 113.23 seconds. Complete actual boundaries pass at
  241560 work; the actual filled-volume atlas still exhausts 99800 source cells
  inside its unchanged 100000 aggregate allowance. No Solid is produced.

The hard rational hollow-cap case and full original loft/sweep/G1/G2 objective
remain OPEN. This packet qualifies temporal source-chart refinement only.
WASM, browser, STEP and CI were not run. Geometry remains entirely in Rust.
The next investigation is earlier rejection of expensive unsuitable fixed-metric
proposals, with point queries supplying rejection only, never admission credit.
