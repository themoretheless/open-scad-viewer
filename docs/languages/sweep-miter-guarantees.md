# Sweep/miter admission and correction

`brep_progressive_miter_sweep` combines authored frames, orientation guides,
anisotropic scale and centre laws on independently normalized knot domains.
The orientation guide may be a positive rational curved rail. It controls the
projected transverse normal; the authored axis controls the longitudinal axis.

A completed `continuousBound` covers the whole retained boundary: walls,
decomposition, correction and filled endpoint caps. Its value must fit
`max_deviation`. Solid admission independently recomputes exact boundary
agreement, trims, face injectivity, wall/cap contacts, shell nesting and material
orientation on the geometry being transferred. Insufficient work or unresolved
geometry produces a refusal.

Explicit `circle_correction_tolerance`, `circle_correction_quantum` and
`circle_correction_max_work` repair supported nine-pole circular sections with
shared binary generators. The quantum defaults to `2^-40` mm. The shared work
budget is at most one million. Circle correction runs before optional endpoint
cap projection. Both displacement upper bounds are added outward, and all
geometry audits run after the final reconstruction. Exact plane evidence from
cap projection therefore belongs to the final sections. Closed paths have no
endpoint caps and refuse cap projection; circle correction is a separate option.

A Rush `.transform(matrix: ...)` of an explicitly circle-corrected progressive
miter body requests exact binary-lattice placement. The native audit requires
integer linear matrix coefficients of absolute value at most 16, grid-aligned
translations and grid-aligned vertices and curve/surface poles with coordinate
integers of absolute value below `2^46`. It proves an exact nonzero determinant
and zero placement arithmetic error. Unsupported arithmetic or exhausted work
refuses the placement. The complete wall/cap error is multiplied outward by a
certified upper bound for the matrix operator norm and must still fit the
source's `max_deviation`. G1/G2, wall regularity and Solid are revalidated on the
placed body, including reflected material orientation. Repeated placements
repeat these checks. Caller-created copies or mutation cannot transfer a
constructor's boundary certificate.

Examples:

- `examples/rush/progressive-miter-circle-corrected-hollow.r`: scale and twist.
- `examples/rush/progressive-miter-spatial-circle-corrected-hollow.r`: combined
  circle repair and spatial cap projection.
- `examples/rush/progressive-miter-affine-circle-corrected-hollow.r`: authored
  affine laws.
- `examples/rush/progressive-miter-oblique-circle-corrected-hollow.r`: exact
  shear of a reconstructed B-rep.
- `examples/rush/miter-rational-curved-guide-frame-affine-hollow.r`: rational
  curved guide, authored frame and affine laws with independent source domains.

G1/G2 reports distinguish wall-profile seams and wall-station seams. Station
extraction includes closed-path closure and explicitly excludes cap joins. The
profile and station audits share a total exact-work budget; G1 fallback uses
only the remainder after G2. A numerical scale proposal is checked by exact
strip identities and independent regularity. The viewport displays station G2
or G1 only after validating the complete seam set and aggregate work evidence.
Sharp miter and cap joins retain C0. These scoped audits do not assert whole
boundary smoothness. See `examples/rush/progressive-miter-station-g2-hollow.r`
for a two-span affine hollow body with certified station G2.

Moving-frame and curved-guide linear retained spans can have real tangent-plane
jumps at intermediate stations. Exact endpoint witnesses distinguish these
represented C0 creases from an insufficient proof budget. Smooth reconstruction
with a composed displacement bound is still required for those cases.
