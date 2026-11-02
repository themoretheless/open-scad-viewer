# Original spline point-lift diagnosis

The full seven-stage goal remains active. This step moves the exact rational counterexample from the standalone diagnostic into the native predicate and B-rep report. It does not repair the generated contact geometry or admit a closed solid.

## Implementation

The additive CAD predicate consumes literal original XYZ+W and UV+W controls, all original C/P/U/V knots, degrees, and an authored normalized parameter. It derives parameter mappings and tensor evaluation exactly, including rational weights and C-only reversed traversal. It accepts no sampled XYZ/UV or caller-selected span. Original values are restored exactly from the predicate input normalization. Degree/control bounds and the existing PredicateContext work, expansion, cancellation and deadline checks remain active. Ambiguous/discontinuous knot values, nonpositive denominators and out-of-chart points remain unproven.

A distinct `SplinePointLiftDecision.point_equality` avoids treating equality in a point as a curve certificate. The NURBS adapter binds immutable original definitions and traversal. When full-curve identity has no supported representation, the B-rep exact audit may inspect the normalized midpoint under the SAME remaining work budget. It records the point-only result and normalized location separately in `ExactUse.point_witness`. `ExactReport.has_exact_counterexample()` recognizes exact differences; absence of a counterexample does not certify anything. Partial authored coedge trims are not silently checked as full intervals.

## Evidence

All 117 CAD predicate regressions and all 31 curve/surface agreement regressions pass. New cases cover rational multispans, unrelated curve/pcurve domains, nonunit surface-weight gauge, reversal, sub-float residuals, point equality with a later mismatch, exhausted budgets, out-of-chart images and early rejection of disconnected knot input. One B-rep integration test passes: a degree-nine interior pcurve retains a located exact counterexample, refuses whole-lift admission, preserves the input and charges all point work to the total budget.

On the expanded control proposal, `--native-point-lift` returns Different for BOTH original contacts at normalized t=0.5 (physical t=1), each with 625024 work under a 1000000 limit. This independently confirms the preceding BigRational counterexample using the production shared-budget arithmetic. These are proposal contact checks, not a new saved-model global audit. The previous saved-body global refusal remains authoritative; that long audit was not rerun in this step.

Reproduce using `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo`, then `--offline --manifest-path crates/Cargo.toml`; use `test -p cad-predicates --lib`, `test -p nurbs-core --lib curve_surface_agreement::`, or with `--config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0`, `test -p brep-core --test exact_lift_point_witness` and `run -p brep-core --example expanded_contact_runout -- --native-point-lift`.

## Remaining construction

Preserve exact original-support contact authority through transition-boundary construction/storage, without snapping source surfaces or weakening the global exact gate. Then finish the unresolved global face pairs and full-body checks. No geometry was changed, the 10/mm curvature threshold is unchanged, and there is no WASM/worker/UI, STEP/history, arbitrary NURBS corner or full-goal completion claim. Scoped hashes describe files in the shared dirty worktree, not an isolated delivery revision. Previous receipts are unchanged.
