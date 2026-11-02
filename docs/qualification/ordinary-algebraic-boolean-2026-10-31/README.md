# Exact ordinary Model promotion for certified interior pockets

Scope: native Rust geometry. This is a bounded extension, not a general Boolean solver.

`certify_pocket_boolean_model` promotes a checked `AlgebraicModelPocket` to the ordinary rational-NURBS `Model`. `verify_pocket_boolean_model` binds the original Model, floor carrier, native pocket certificate, complete boundary witnesses, and full result. The verifier reruns construction and compares geometry and topology. The reported mathematical boundary approximation error is exactly zero. Floating-point evaluation still has numerical error.

Boundary witnesses must cover complete selected CAD edges once each. Exact substitution proves the original polynomial identity; strict parameter monotonicity, endpoint locations and ordered-root ownership prove the correct complete branch. Automatic witnesses support straight edges and rational-clock circle arcs with rational endpoints and radius. Supplied rational witnesses can cover further exactly representable curves. Unrepresentable binary64 controls, unsupported branches, genuine denominator poles and arithmetic/degree budget failures refuse.

Floors retain their independent original carrier. New side walls are exact rational ruled surfaces over the lifted original boundary. The ordinary Model contains actual Curve/Surface geometry, pcurves, closed shells and inherited topology IDs, rather than a symbolic overlay on the input. Tests include continuous edge/pcurve agreement, Model serialization roundtrip and rejected forged arc controls.

All four regularized operations are available **between the source and its certified-contained pocket volume**: difference and xor retain the cut source, intersection returns the removed volume, union returns the original source. This subset proof does not solve Boolean operations between arbitrary independent Models.

Signed coordinate permutations permit an edited carrier and floor folded in the original XY projection when each has a proved affine projection in another coordinate frame. The frame is an exact isometry; it introduces no geometric approximation. Automatic frame selection stops after the first structurally admissible direction, including on subsequent material/conversion refusal. Arbitrary nongraph carriers requiring nonlinear inverse maps remain unsupported.

`certify_model_pocket_batch` supports 1..16 sequential difference requests on original face slots. Every intermediate ordinary Model must pass the existing source-atlas proof for the next operation. This is not a single cut crossing a shared face boundary. A sequence after a circular pocket currently refuses with `trimmed atlas UV disk Jacobian is unproved`; the pre-fix two-coedge diagnostic and subsequent Jacobian diagnostic are retained. The minimum wire check was corrected to count extracted authored spans rather than coedges, while preserving exact continuity, orientation, isotopy, Jordan and triangulation checks.

## Remaining scope

- General exact algebraic-edge representation in legacy Model, including non-rationally-parametrizable intersections.
- NURBS approximation with a proved nonzero tolerance and independent topology preservation proof.
- General nongraph edited carriers and arbitrary cross-face cuts.
- General independent-Model Boolean construction.
- Source-atlas admission of the circular-pocket output for later edits.
- WASM, UI, STEP and dist validation were not run for this extension.

## Evidence

See the adjacent logs and hash manifest. The test executable is copied before suite execution to prevent another concurrent build from replacing it. Source hashes cover the scoped files only, not a frozen transitive build environment. Passing these suites does not qualify unrelated dirty work in this checkout.

Qualified on 2026-10-31 using one copied native test executable:

- Focused ordinary Model tests: **5 passed**, 0 failed (4.35 s).
- Full `minkowski3d::` suite: **136 passed**, 0 failed (118.29 s), including those same five tests.
- `profile_certificate::branches::`: **7 passed**, 0 failed (0.11 s).
- Total distinct passing tests across the full suites: **143**.
- Scoped source SHA-256 verification and `git diff --check` passed.

The circular-pocket batch diagnostic is an unresolved admission limit, not a qualified success. The successful batch regression uses disjoint rectangular pockets on two original faces.
