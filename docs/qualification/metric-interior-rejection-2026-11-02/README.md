# Source-paid rejection of unsuitable interior metrics

Base: `8bb47a98a`.

Preserve all eight original corner rejection probes first. If they pass, also
query original face/edge midpoints and the center (27 points total). These
native point jets ONLY reject metric proposals; all accepted geometry still
requires the same complete outward interval SPD field cover or exhaustive pair
proof. Every failed query and probe uses the same aggregate work ledger.
No visit limits, tolerance, or geometry acceptance predicates changed.

The regression uses the globally injective triangular map
F=(u,v,t+16u(1-u)v(1-v)). Its center/corner Jacobians are identity, but that
Euclidean metric fails at face midpoints. The early rejection leaves budget
for the independent complete triangular interval-matrix proof: **23 cells**.
Measured-budget replay passes and one-less refuses. A point never admits F.

Native evidence: nightly-2026-09-16, locked Cargo sources, test opt-level=1,
test/dev debug info disabled, two build/test threads.

- Cargo `-p nurbs-core --lib unsuitable_corner_positive -- --nocapture`: **1 passed**.
- Same freshly built NURBS unit executable without filter, `--test-threads=2`:
  **2226 passed / 0 failed / 1 ignored**, 63.48 seconds.
- Cargo `-p brep-core --lib rational_hollow_cap_quarter_turn_frame -- --nocapture`:
  **0 passed / 1 failed**, 106.96 seconds. Actual boundary equality passes at
  241560 work, but the filled-volume atlas exhausts the unchanged 99800 source
  cells (100000 aggregate limit), returning actual-affine-filled-volume-unproved.
  No Solid is admitted; the hard actual volume case remains OPEN.

This source-paid rejection capability is qualified independently. Full original
loft/sweep/G1/G2, full BRep and WASM/browser/STEP/CI completion are not claimed.
Geometry remains entirely in Rust. Next: reuse only complete positive proofs for
contained parameter hulls inside the SAME immutable native atlas owner.
