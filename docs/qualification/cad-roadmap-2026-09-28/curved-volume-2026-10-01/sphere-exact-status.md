# Sphere exact boundary qualification

The ignored curved-volume gate was run explicitly with `--ignored --nocapture`. It fails for the authored sphere of radius 2; the cylinder passes before it.

The exact predicate returns `Different` for coedge 1 of all eight sphere faces, on equator edges 1, 3, 5 and 7. The meridian uses are not reported as failures. Trim closure and trim validity pass; face injectivity, pair classification, nesting and orientation remain unproved. This is an exact source-representation mismatch, not a work-budget exhaustion result.

The test now prints each failing face, wire, coedge, edge and predicate decision. Resolving the equator representation must preserve shared topology and prove equality of the original edge and lifted pcurve; a tolerance check does not establish that equality. This evidence does not qualify general sphere volume validity.

## Rational equator correction

The new sphere constructor uses quarter-circle weights `[1, 1, 2]` for both equator edges and their pcurves. This is the rational traversal `((1-t²)/(1+t²), 2t/(1+t²))`; its binary coefficients satisfy the unit circle identity without an approximate square-root weight. Existing sphere recognition admits both this traversal and the legacy symmetric one. Boolean seam splitting inverts the actual pcurve traversal rather than assuming symmetric weights.

For radius 3, all 24 original edge/lift identities and trim joins now pass the exact predicate. For radius 2, rounded stereographic surface control coordinates such as `4/3` still prevent exact agreement; tolerance checks pass, exact qualification remains refused. The boundary regression asserts both outcomes and immutable input. This fixes the equator traversal but does not establish sphere face injectivity, pair classification, volume validity or general exact scalar representation.
