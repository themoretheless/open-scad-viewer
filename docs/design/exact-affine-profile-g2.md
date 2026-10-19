# Exact affine profile motion and complete boundary G2

This is a Rust construction and native proof checkpoint. It is not a completed
general Solid, cap or original-body G2 qualification. TypeScript owns no geometry.

## One original profile, one motion

`brep_core::affine_station_walls` accepts literal original Bezier contours and
their compatible retained station contours. A floating affine fit only proposes
one matrix and translation per station for every pole, span and contour together.
The original controls and positive rational weights are never snapped or edited.
Each span's output domain is normalized affinely to [0,1] with identical controls.
Retained station knot vectors must agree literally throughout each span.

The candidate uses an exact binary source origin and lattice. Every matrix/pole
product and addition must be exactly representable, checked by error-free
addition and an FMA residual with conservative normal-range restrictions.
Unrepresentable products, unsupported source lattices or exhausted work refuse
the candidate. This is a finite exactly representable family, not all binary64
profiles or all possible affine poses.

Shared quintic matrix/translation controls have identical first differences and
zero second differences at ordinary stations, including the repeated closed
seam. Declared sharp stations have independent one-sided jets. Interval Bernstein
evaluation proves that the matrix determinant has a common strict sign over
every whole time interval; nonsingular station samples alone are insufficient.
This preserves the original profile's geometric joins and its sharp corners.
`affine_regular` describes only this 3D matrix, not the swept surface or Solid.

An outward coefficient bound compares every resulting quintic pole against the
degree-elevated original ruled wall. Positive weights, constant along time and
shared with the original ruled basis, give a continuous displacement upper bound.
This displacement is additional to the source owner's original-law and retained
decomposition errors; it cannot replace either. Poses act on original coordinates
minus `source_origin`. Failed candidates expose no walls, poses or sections.

## Independent complete boundary gate

The native periodic material topology builder can retain the supplied quintics
without a planar cap premise. It makes an orientation proposal only. The
`sweep_smoothness::complete` owner independently extracts every owned edge,
checks exact world-edge correspondence and joins, connected vertex links, every
face's regularity, and exact projective jets. There is no caller seam list that
could hide a sharp join. No smoothness claim follows from fitted station samples.

The spatial rational quadratic example has 128 faces and 256 owned edges;
its continuous displacement upper bound is 0.018779675412347093. Exact G2 and
whole-face regularity pass, including the closed time seam and profile closure.
A genuinely nonplanar polynomial cubic original contour also passes all 256
owned edges. Spatial authored and guided frames with simultaneous scale, twist
and affine laws also pass all owned seams. Its exact work exceeds the former 2M collection ceiling, so the
explicit native aggregate allowance is now at most 4M, with the same 1M ceiling
per seam. One-short and oversized allowances are independently tested. This does
not increase geometric tolerances or alter the source to fit a proof budget.

## Whole-face normal polynomial

For bounded literal Bezier patches of degree at most five per axis, native
surface regularity encloses the full rational normal numerator in its original
tensor Bernstein basis. With homogeneous numerator H and positive weight W,

    Su x Sv = [W(Hu x Hv) - Wv(Hu x H) - Wu(H x Hv)] / W^3.

All products use outward interval Bernstein convolution. Translation is removed
before homogeneous products. A strict sign of one numerator component on the
coefficient hull proves regularity over the entire rectangle. Otherwise exact
half subdivision continues under one created-cell ledger. Parameter labels stay
normalized dyadic values; reported natural-domain intervals are outward mapped.
Depth 100, exhausted work or uncertain coefficients refuse. Other/periodic bases
retain the previous generic derivative verifier. No sampled normal is a proof.

The thin rational moving-patch regression previously exhausted 100000 cells;
the complete numerator proves it in one cell. Collapsed, folded and rational
folded patches remain unproved. A curved normal cover tests one-short budgets
and non-dyadic natural domains.

## Original and retained global validity remain distinct

The closed source owner now has an integration regression with two cubic filled
material charts sharing every original boundary coefficient. The first chart
receives an original-volume certificate and produces a valid retained Solid.
The second folds in its interior; exact boundary binding still succeeds, but
the original-volume atlas refuses at its finite allowance. The existing boundary
and retained BRep cannot confer original embedding or topology credit on it.
The refusal is unproved/work-limited, not a claimed numerical collision witness.

Fresh global contact, nesting and orientation evidence is required for any new
quintic model. Complete boundary G2 does not issue that evidence. A future source
owner must additionally bind its actual filled carrier to the quintic motion,
certify the entire candidate volume atlas, and add its displacement to the owned
original-boundary error before Solid admission. General filled cap joins,
unsupported frame/pole/periodic atlases, WASM, new CI, external STEP and browser
qualification remain separate open requirements.

The independent 100000-cell global-volume probe terminated with
`volumeProven:false`, `boundaryProven:false` and pending pair [94,96].
The unresolved prefix includes neighboring curved profile charts [0,1], [0,3],
[1,2] and [2,3]. The optimistic positive-volume assertion is preserved in
qualification `probes/` and the failed diagnostic log; it is not in the passing
feature matrix and supplies no Solid credit. The candidate API has no Solid
success flag. Complete-boundary G2 remains tested independently.
