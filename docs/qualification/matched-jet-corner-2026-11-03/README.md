# Matched-jet corner proposal — 2026-11-03

Status: native profile qualified for the cases below; full central surface and closed body remain unqualified. No new Solid, WASM, UI or STEP admission.

## Geometry and numerical work

`bounded_corner::Proposal::composite_matched_g2` authors degree-five rational end runouts and a symmetric rational middle arc. Separate short parameter intervals match their homogeneous endpoint jets. A small normal-acceleration relief changes the actual runout shape; it does not increase the curvature cap or requested G2 tolerances. The initial unrelieved proposal had a real endpoint curvature of approximately 10.0000001075/mm and was rejected. Symmetric clocks and fresh original-coefficient checks are mandatory after rounding.

The absolute curvature predicate now attempts a bounded outward Bernstein cover before its exact fallback. Failed filter cells consume the same cell allowance. Exact original endpoint/midpoint signs can establish rejection before constructing the entire high-degree inequality; positive point signs cannot establish acceptance. Tests check both insufficient cell/work allowances and an interior violation behind safe endpoints.

For literal unit axial products, exact original control structure permits the rank-one shape-operator formula. A subnormal transverse control change or one-ULP station weight change disables this path. For clamped curve endpoints whose first three actual poles lie on a coordinate line, exact endpoint two-jet structure proves zero normal acceleration; the actual first derivative still needs a proved nonzero sign. Other endpoints retain the general differential comparison.

The explicit matched-profile B-rep builder recomputes contact evidence. Central layer timing retains matched whole-piece speeds when representable. A whole-piece polynomial displacement field is multiplied by the rational denominator and degree elevated before composing central rows; this replaces per-pole displacement sampling on micro-clock proposals. The resulting stored geometry is still a new proposal. A rational multi-span regression checks its values and proves the actual complete internal G2 join, and checks unchanged source ownership and zero-work refusal.

## Positive evidence

For radii 0.1/0.2/0.35 mm and factory tolerances 1e-7 and 2.5e-8 mm:

- All 36 original/new curve-cell checks prove regularity and curvature <=10/mm over the closed interval.
- All 12 original extruded profile joins prove normal error <=1e-5 and tensor error <=1e-4/mm with zero pending cells. Maximum observed tensor upper bound: 1.3708495188609654e-5/mm.
- The profile integration test additionally checks both raw and conditioned clocks, original endpoint ownership, zero work and all 18 new curve cells.
- 96 scoped native tests pass: curvature predicate 5, surface-G2 39, exact axial endpoint 1, new profile 2, curve joins 7, legacy bounded profile 4, region replacement 38. The source and log hashes are in `review.json`.

The probe flag `--c2` selects the matched-jet proposal. It does not assert exact parametric C2 or qualify a body.

## Current complete-body result

The 10-mm origin-zero cube, radii 0.1/0.2/0.35 mm, handle 1/256 mm and explicit matched-profile builder reach central inspections. After the whole-piece weighted field rewrite, the three plane exterior seams pass; the three curved exterior seams remain Unresolved. Of ten internal central joins, eight have original-geometry counterexamples and two remain Unresolved. The sewing gate refuses with an exterior G2 diagnostic. Process exit zero means the diagnostic completed, not body acceptance.

Thus the profile certificate does not transfer to the rounded loft. Actual stored central two-jets must be reconciled across the micro-clock cells; whole-field arithmetic and placement precision require further work. The present central candidate must not be admitted as a closed G2 body. The absolute whole-body cap, thickness, global embedding/self-intersection, material volume, general NURBS trimming, complex corners, WASM/worker/UI and the three-part STEP/history acceptance remain open.
