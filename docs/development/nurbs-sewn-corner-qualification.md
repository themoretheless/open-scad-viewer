# Native sewn trihedral corner

`CentralLoftCandidate::sew_corner` constructs an owned B-rep from the central
surface, three fillet strips, three planar far caps and the six retained stock
faces. The source stock is borrowed immutably. Far stops remain sharp caps.
This is a bounded rectangular-stock corner family, not a general fillet,
Boolean, arbitrary trimmed-surface or STEP/UI qualification.

## Shared geometry

The actual fixture has 13 faces, 22 vertices and 35 edges, including two
explicit collapsed pole edges. Its Euler characteristic is two. Topology
uses explicit edge and vertex identities; no distance welding is performed.
The native model validator checks opposite edge uses, connected shell and
manifold vertex links before setting closed-shell/body metadata.

A bounded control proposal makes near cutter coordinates and stock contact
heights literal. Positive unchanged rational weights enclose its complete
surface displacement by the largest outward control-displacement norm.
The proposal is rejected above the original positional tolerance / 64; the
actual fixture bound is approximately 7.474e-13 mm. All pole, internal and
exterior jet inspections are repeated on that proposal.

Integer clocks reparameterize literal C0 Bernstein cells without changing
any control point, weight or geometric image. Final central borders become
shared spatial edge definitions. Coordinate-plane charts use signed world
coordinates, so their pcurves copy the corresponding original coefficients.
Stationary strips use these same rational profiles, with an exact far cutter
plane. A separate full-image deviation proof compares them with the original
strip skins; endpoint proximity alone does not admit this replacement.

The native exact boundary audit checks every saved edge/pcurve/surface use.
New sufficient routes recognize literal C0 sub-borders, constant poles and
signed coordinate identity planes. Unsupported bases, nonexact knot mirrors,
altered coefficients and exhausted exact work cannot produce equality.

Final shared central/strip and central/stock seams are inspected on the actual
saved edges and final surfaces, with native pole certificates. Six additional
stock/strip contact G2 inspections use literal original end-span views and an
exact parameter transpose. All three prescribed contact-center radii are
recomputed from the final B-rep contacts; stored source radius reports are not
inherited. This does not claim circular cross sections of a quintic G2 skin.

## Independent embedding and material gates

The general face and contact diagnostics are retained unchanged. Additional
corner-specific results do not overwrite their unresolved decisions.

* The central quotient disk has a Jordan projected boundary proved using
  exact rational homogeneous coefficients. All original interior cells have
  the same strict projected Jacobian sign; native flat-pole certificates cover
  both collapsed endpoints. This yields a one-sheet central projection.
* Opposite coordinate hulls permit contacts only in an exact separating
  plane. Positive original Bernstein coefficients exclude interior zeros on
  cells, internal grid edges and internal grid vertices. Any retained zero
  boundary segment must have an actual shared edge identity; isolated zeros
  must have actual shared vertices. Extra internal-knot contact is refused.
* All positive-rational support hulls lie inside the original stock. All trim
  control hulls remain inside their actual natural UV rectangles, and their
  original contours have positive native simplicity/domain audits. Together
  with closed embedded topology this bounds the entire enclosed material
  inside the original convex stock, rather than merely checking sample points.

## Volume

Original oriented surface flux is integrated with outward intervals. Affine
plane contributions use their actual trim Green area and physical parameter
scale. Rational contributions retain original knot cells. Narrow oblique
trim bands retain their complete signed-mask enclosure; no area is discarded.
An iterated tensor midpoint Peano bound uses the pure second derivatives,
with independent mixed-bilinear and quartic regression tests.

A signed flux interval becomes an enclosed-volume interval only after native
closed topology, exact boundary equality, every face's embedding, all distinct
face-pair classifications, convergence and positive orientation are proved.
Otherwise the raw flux remains diagnostic and `volume_bounds_mm3` is absent.
The material-preservation result additionally requires whole-boundary stock
containment and the source-volume comparison. Removed volume is outward
interval subtraction from the qualified original stock volume.

`SewnCornerCandidate::audit` exposes independent diagnostics.
`SewnCornerCandidate::qualify` recomputes them internally and returns an
immutable `QualifiedSewnCorner` borrowing the same owned candidate only if all
gates pass. A caller-supplied or modified report cannot grant admission. The
rounded original-skin sewing path remains diagnostic and does not receive
this final fitted-strip/G2 qualification.

```rust
let sewn = center.sew_corner(sewing_limits)?;
let qualified = sewn.qualify(tolerance_uv, audit_limits)?;
let body = qualified.model();
let volume = qualified.volume_mm3();
let removed = qualified.removed_volume_mm3();
```

## Current fixture evidence

For stock `[0,10]^3` mm, radii `0.1, 0.2, 0.35` mm, runout `0.1` mm and retained
strip span `2` mm:

* All 68 edge uses and all joins are exactly equal.
* The central disk projection uses 443 geometry cells and 7245 exact queries.
* All 78 distinct face pairs are visited. Eighteen additional coordinate
  certificates close the generic unresolved pairs, using 9060 charged units.
* The oriented volume interval is `[999.9580224963673, 999.9581224941044]` mm3,
  with 7604 flux cells at absolute width target `1e-4` mm3. Source volume is
  approximately 1000 mm3. These are finite fixture results, not a universal
  guarantee for other radii, stock sizes or corner configurations.

No tolerance, geometry-work ceiling, exact-work ceiling or architecture
baseline is increased to obtain these results. General SS/Boolean, arbitrary
offsets, analytic simplification and WASM/Rush/UI/STEP delivery remain separate.

## Final validation

The four native libraries pass 4269 tests, with zero failures and six existing
ignores. The no-default-features library check passes. All 14 final G2 reports
and all three fresh contact-center radius reports pass. The complete-image
strip displacement bounds are below 3.607e-9 mm, within the unchanged 1e-7 mm
positional allowance. The removed-volume interval is
`[0.04187750589096594, 0.04197750363732667]` mm3.

The public map (239 modules, 200 catalog entries), catalog consistency and ten
infrastructure tests pass. There remain 69 pre-existing global architecture
violations outside these changes; all changed source files satisfy their size
limits. Source hashes, commands and compressed native, minimal, corner and
architecture logs are retained in the
[qualification evidence](../qualification/nurbs-sewn-corner-2026-10-30/README.md).
