# Original implicit sphere certificate

This is a sufficient Rust-owned theorem for one explicitly authored sphere,
not a certificate for arbitrary loft/sweep frames, caps, or closed seams.
The source is the literal binary64 center and positive radius. Squared source
constants are retained as exact products, including a radius whose square is
not representable in binary64. Audit never reconstructs or repairs its model.

## Native proof chain

1. `cad-predicates::binary_bezier_quadric_with_constant_products_identity`
   proves the exact polynomial identity on every original rational tensor
   Bezier patch. Its homogeneous controls are formed from the literal
   Euclidean coordinates and positive weights. It does not normalize quantities
   with different polynomial units. Arithmetic failure, cancellation and short
   budgets cannot produce equality. Only literal binary64 leaves are accepted.
2. `nurbs-core::implicit_quadric` separately proves a nonzero implicit gradient
   over each entire patch using outward interval bounds and bounded subdivision.
   Exact membership and regular ambient support do **not** establish an
   injective patch, represented surface dimension, a valid pole, or a Solid.
3. `brep-core::source_implicit_body` requires a single fully owned closed shell,
   every actual face once, complete physical vertex links after contraction of
   only declared, whole, exactly constant pole boundaries, and native regularity
   of ordinary charts or their complete pole quotient charts. Both parameter
   axes are supported by exact tensor transposition; no fitted carrier is used.
4. The same audit independently executes full native `volume_validity`:
   exact world-edge/face agreement, valid trim regions, whole-face injectivity,
   all face-pair contacts, shell roles and outward orientation. Common implicit
   support alone never authorizes overlaps, folded charts, incomplete fans or
   an unsupported contact.
5. The resulting boundary is a nonempty compact embedded closed 2-manifold
   contained in the connected regular sphere level set. Invariance of domain
   makes its inclusion open; compactness makes it closed. Thus its image is
   the entire source sphere. The oriented bounded component is the source ball.
   Source boundary error is zero, topology is preserved and the intrinsic
   tangent plane and second fundamental form agree everywhere, including poles
   and face joins. This supplies source G1/G2 without claiming nonsingular UV
   derivatives at a collapsed parameter boundary.

All success flags are set only after step 4. The opaque immutable `SphereBody`
contains that model and its native report. Refusal returns no admissible model.
Input validation precedes proof budget refusal, including invalid tail faces.
The report distinguishes parameter chart regularity from regularity modulo
poles. Other owners retain their existing strict regularity requirements.

## Construction and transport

`construct_sphere` uses the native exact rational sphere charts. Placement is
accepted only when error-free coordinate sums establish an exact translation;
unrepresentable placement refuses rather than rounding the source geometry.

Rust JSON operations are `brep_implicit_sphere_source_construct` and
`brep_implicit_sphere_source_audit`. Caller success flags are ignored. Geometry,
proof assembly, refusal and admission stay in Rust; runtime scripts transport
requests and assert returned results.

STEP /9 now preserves authored NURBS charts instead of automatically guessing
periodic replacements. The production /9 bridge uses `cad-step`; its existing
`brep-core` compatibility API has the same source-preserving behavior. Earlier
STEP versions retain their previous semantics. This is not a claim that all
legacy STEP implementations have been migrated.

Regular quarter-disk charts may be reflected or transposed by STEP face-sense
canonicalization. The exact disk-side proof transfers through a signed unit
square bijection with error-free UV subtraction. Corner-derived projective
proposals only select a bounded candidate: original whole-domain interval
Jacobian contraction still owns injectivity and shares the original work ledger.

## Qualification limits

Native fixtures include regular and pole charts, literal nonbinary radius,
exact translation, signed tensor parameter changes, one-ULP mutations,
wrong source equation, invalid orientation, incomplete pole fans, malformed
input and exhausted budgets. STEP is re-imported and independently re-audited
against the original source. External OCCT supplies numerical interchange
checks, not the exact Rust theorem: validity, physical topology after pole
contraction, converged positive volume, center, sampled radius and curvature.
OCCT may split native point edges; all such edges must belong to the original
pole locations.

Public packaged WASM, Rush/viewport/Solid UI, full semantic G1 matrices, a fresh
main merge and CI require separate evidence. This source class does not close
the broader goal for arbitrary moving frames, arbitrary filled caps, mixed
sharp C0/G1/G2 joins, or general closed source seams.
