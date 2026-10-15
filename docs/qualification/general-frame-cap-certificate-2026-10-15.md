# Native general boundary certificate (candidate)

This candidate adds two explicit Rust contracts. It does not supersede the
published OwnRust 72 / G0 86 / G1 103 qualification records.

## Original sweep and authored filled caps

`brep_core::sweep_boundary_owned::construct` owns the source `MultiSweep`, the
complete loop partition, two original filled rational cap carriers, and two
retained cap carriers. It accepts no precomputed success flags or supplied body.

1. The original sweep owner proves the complete retained wall union against the
   original frame/path/scale/twist/affine/guide laws. Sampled acceptance is insufficient.
2. Exact original curve/surface identities bind each cap boundary and every hole
   to the original profile loops. Original and retained caps share the same UV trims.
3. At each endpoint, Rust encloses the original affine pose and applies it to
   every original rational cap control. Positive rational weights make the
   maximum control displacement a bound over the complete filled carrier.
4. A stored proposal encloses the ideal cap within a charged rounding bound.
   Whole-domain homogeneous Bernstein comparison bounds that proposal against
   the actual retained cap, including different knots, degrees and weights.
5. The maximum wall/cap bound must fit the requested deviation. The constructor
   builds the model from that exact source level and retained cap definitions.
   Whole-boundary regularity, exact edge agreement, trim validity, injectivity,
   every face pair, shell nesting and material orientation must all pass before
   a private `CertifiedBody` can be produced.

All walls and both caps share the source enclosure/product allowances. Exact
identities have a separate explicit shared allowance. Regularity and volume
have separate explicit stage limits. Unknown or unfinished work returns refusal.
Sharp seams remain C0; source approximation does not prove ideal-body embedding
or G1/G2. Closed source paths have no endpoint caps and use the existing closed
body contract; this open capped constructor explicitly rejects them.

## Authored rational reference and retained boundary

`brep_core::boundary_certificate::inspect` compares two actual rational BRep
boundaries with explicit common face indices, UV trims, incidence and shell roles.
It proves both boundaries independently regular and geometrically valid solids.
No moving frame or ideal motion law is inferred from an authored reference.

For corresponding carriers A=Xa/Wa and B=Xb/Wb, the numerator
`Xa*Wb-Xb*Wa` is formed in outward interval Bernstein coefficients on every
cell of the union of the original knot partitions. The positive Bernstein coefficients of `Wa*Wb` turn corresponding numerator/denominator coefficient ratios into a convex enclosure of every point, including closed cell boundaries.
The maximum over identical trimmed domains bounds both Hausdorff directions.
Identical positive rational bases use the direct convex control-difference bound.
Original source arrays are never replaced by rounded decompositions.

Both complete boundaries share the correspondence and regularity allowances;
reference and retained volume audits have explicitly separate stage budgets.
Missing faces, changed holes/roles/UV correspondence, singular carriers and
unclassified contacts cannot be rescued by a positional error bound.

## Shared contacts across moving charts

A new native contact alternative handles transposed charts, unequal transverse
span partitions and unequal transverse degrees without elevating or rounding
world-space poles. Original knots and weights remain unchanged, including
non-dyadic station knots. Enclosed source parameter rates express the two maps
in one common coordinate system: `[-La,0] × [0,1]` and `[0,Lb] × [0,1]`.
The virtual lengths need not be representable binary64 numbers. Their common
rational boundary is checked coefficient by coefficient, including proportional
positive weights and the exact common normalized V basis. No stored virtual
surface or rounded shifted knot becomes a premise.

One fixed integer projection and one fixed metric must have a strictly positive
symmetric Jacobian on **both** rectangles, using original coefficient interval
bounds. Their union is convex and the map is continuous across its C0 seam, so
integration along any segment proves joint injectivity. Independent P-matrix
results or different per-chart metrics cannot authorize this conclusion; the
new strict metric APIs explicitly disable those alternatives. Derivative
samples strictly inside original spans only propose the common map and metric.
No derivative is assumed to exist at a C0 seam. Both proofs consume the same
pair allowance, and existing independent contact fallbacks retain work.

## Spatial path error and exact edge parameters

For parameter-spaced spatial Bishop transport, Rust cancels the original path
translation before bounding relative frame motion. On a single smooth original
span, the original second derivative bounds path/chord deviation by `M*h²/8`
in normalized traversal units. A continuous piecewise smooth path can instead
use its derivative minus the stored chord slope, bounded by `M*h/2` from the
nearer endpoint. Both include endpoint enclosure/rounding error and share the
existing original-source allowance. The original direct Cartesian bound remains
an independent fallback; no sampled derivative authorizes either inequality.

Provided-cap lofts preserve increasing natural edge parameters and put reversed
traversal in the coedge sense. They never rely on mirroring a binary64 knot
twice: `1-(1-.2)` need not equal `.2`. Exact natural multi-span polynomial edge
identity also permits distinct constant positive homogeneous weight scales;
the original common basis and every Euclidean pole must still agree exactly.
Other loft construction paths retain their previous edge convention.

## Transport and limits

`brep_boundary_certificate` and `brep_sweep_boundary_construct` serialize these
Rust computations through the geometry bridge. The latter emits a model only
on full native success. There is no TypeScript geometry implementation.

These are sufficient bounded certificates, not a promise that every valid NURBS
representation can be proved within finite work. Source cap boundary composition
and whole-shell contact audits retain their explicit representation/refusal
limits. The common UV correspondence is authored, not inferred by sampling.

Validation results and rebuilt WASM hashes must be added after the candidate
checks finish. Existing frozen qualifications must remain immutable.
