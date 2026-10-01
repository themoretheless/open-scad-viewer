# Exact sphere volume and separation stage

The shared-boundary proof now handles a rational quarter-disk trim. An exact determinant verifies that the homogeneous height from a shared coordinate plane is `(pole-plane)*(1-u²-v²)` in the original surface coefficients. Positive rational weights and the exact simple quarter-disk trim imply strict opposite sides in the interiors. Exact edge/pcurve lift agreement is additionally required for both uses of the circular boundary, with a bounded 32768-work predicate limit. This is a sufficient UV-domain/plane certificate, independent of analytic sphere recognition.

Supporting-plane hull restrictions now iterate at most three times. A restriction can expose another supporting plane of the already restricted hull; original controls must satisfy all accumulated equalities. This isolates the eight shared equator vertices between adjacent hemisphere quadrants as well as the four common poles. Interior cutting planes remain excluded, and point certificates still require common authored topology.

For sphere radius 3, all 28 distinct face pairs are classified: eight meridian boundaries, four equator boundaries, twelve shared vertices and four disjoint pairs. Exact edge/lift agreement, trim joins, trim validity, global face injectivity and outward orientation pass, establishing native filled-volume validity. Radius 2 still fails exact agreement because its control coordinates include rounded thirds; an ordinary regression requires refusal.

Two radius-3 spheres with centers 8 mm apart along X have the known filled-volume gap 2 mm. Native distance converges within 1e-5 mm, excludes material overlap and returns original-face UV witnesses whose evaluated points exactly match the report. Both input models remain unchanged.

The distance test exposed a translation-dependent perspective candidate. Numerator offsets now come from the original chart corner, and denominator offsets/extents come from an original boundary control anchor. These coefficients only choose a projection: outward interval derivative bounds still prove the original surface. The checked worker expectation derives the same coefficients from the submitted source.

This stage is native. WASM packaging, protocol fixtures for the complete sphere volume stage and browser distance qualification remain outstanding. Arbitrary radii, rotated spheres and general curved solids are not qualified by these cases; the broader curved-volume roadmap gate remains open.
