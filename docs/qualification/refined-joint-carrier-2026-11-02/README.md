# Refined original joint-carrier proof

Base: `dc39eef65`.

A coarse joint box returning Unavailable(Some(UV axis)) can now attempt the
existing fixed-metric proof on an adaptive cover of ORIGINAL source charts.
Previously this path required an already available coarse interval Jacobian.
The metric is still fixed over the entire hull; every subcell must prove SPD.
The exact same source query/cache, pair coverage, work ledger, 1024 metric visit
ceiling, and final all-or-nothing flags remain in force. Unsupported source
conditions retain their refusal. No rounded geometric replacement is admitted.

The regression supplies a conservative primitive needing smaller UV charts.
Its complete identity volume certifies at 33 cells, measured-budget replay
passes, one-less budget refuses, and a folded source refuses all global flags.
No source point supplies admission credit.

Native evidence: nightly-2026-09-16, locked Cargo sources, test opt-level=1,
two build/test threads, test/dev debug info disabled.

- Cargo `-p nurbs-core --lib unavailable_joint_carrier -- --nocapture`: 1 passed.
- The SAME freshly built NURBS test executable without filter,
  `--test-threads=2`: **2224 passed / 0 failed / 1 ignored**, 80.53 seconds.
- Cargo `-p brep-core --lib rational_hollow_cap_quarter_turn_frame -- --nocapture`:
  **0 passed / 1 failed**, 152.71 seconds. Exact actual boundary agreement passes
  at 241560 work, but actual filled-volume proof still exhausts 99800 source
  cells within the unchanged 100000 aggregate allowance and returns
  `actual-affine-filled-volume-unproved`. No Solid is produced.

The hard quarter-turn rational hollow-cap case remains OPEN. This change is a
qualified source-chart search capability, not completion of general original
loft/sweep guarantees, G1/G2, full BRep qualification or delivery. The next step
is stage-level evidence of where the actual volume atlas consumes its budget.
WASM/browser/STEP/CI were not run. No TypeScript geometry or budgets changed.
