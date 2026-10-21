# Finite sharp-profile threads in Rust

The typed APIs `brep_core::thread::finite_external_with_certificates` and
`finite_internal_with_certificates` return immutable `CertifiedFiniteThread`
objects. External bodies have two disk caps. Internal bodies have four cylindrical
outer walls and two annular caps with independently checked material roles.

## Independent admission

* `nurbs_core::thread::patches_certified` bounds the complete saved wall against
  nominal diameter, clearance, lead, sharp 60-degree profile and signed rotation.
  Interval parameter derivation includes sqrt(3), Machin pi and floating rounding.
  Missing patches, altered role ordering and insufficient precision/work refuse.
* `certify_trim_candidates` independently enumerates the exact affine clipping
  polygon of every original patch, checks complete retained/excluded coverage,
  and proves the saved convex UV polygon has the same perimeter order and
  orientation. The axial band is outward bounded by the original saved Z-field
  deviation plus the affine gradient times the UV error. Mutable preview polygons
  supply no proof authority.
* Polynomial diagonal traces are split at original surface knots and represented
  as degree-six NURBS (at default precision, at most 13 poles per diagonal),
  replacing the former many-segment fallback. These traces remain candidates
  until the independent continuous original curve/surface checker admits them.
* Caps reuse the existing boundary edge IDs. Their plane uses identity XY
  parameterization, preserving exact projected source curves. Exact original
  profile certificates check simplicity, winding, hole nesting and material roles.
  Internal outer-wall separation uses independent XY radial bounds, with Z
  projected to zero before calling the three-dimensional norm bound owner.
* Every actual retained wall is matched to exactly one original admitted source
  surface. A fresh continuous G0 check covers the final saved model. Closed-body
  topology and positive volume are required, under shared bounded proof budgets.

The older simple cap API automatically routes higher-degree thread boundaries
through the exact profile path and rechecks both source and cap curve uses.
Existing line/circle cap behavior is preserved. The standalone planar cap
constructor also routes higher-degree boundaries through exact profile admission
and identity XY planes; one outer followed by holes is required. Original
curves are retained, and reversed/outside material roles explicitly refuse.

## Declared limits

The source constructor accepts 1–8 starts and 1–16 turns. Finite construction
requires an interior slab with complete connected end contours and bounded
retained patch/profile complexity; not every source specification fits the cap
owner's 128-span budget. Outer cylinders must be strictly separated from the
certified internal crest. Invalid, degenerate or exhausted requests return errors.
This is a sharp mathematical thread profile; ISO fit classes, rounded roots,
runout and mating clearances are not qualified here.

`solid_geometry_certified()` is **false**. Global 3D embedding and complete owned
solid admission remain separate. `ideal_finite_body_error_upper()` is **None**:
wall error and axial clipping bands are separate certificates, not a combined
ideal finite-body Hausdorff guarantee. WASM compilation does not qualify browser,
Rush, editor or STEP runtime behavior.

## Verification

[Qualification evidence](../qualification/nurbs-thread-source-2026-10-15/native-tests.json)
records 20 core tests and 28 B-rep tests, including both thread hands, external and
internal bodies, standard precision, independent volume observations, invalid
source/witness/work cases and legacy cap regressions. Independent volume samples
are regression oracles, not substitutes for global solid geometry certification.
