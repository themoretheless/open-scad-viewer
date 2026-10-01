# Sphere exact boundary qualification

The ignored curved-volume gate was run explicitly with `--ignored --nocapture`. It fails for the authored sphere of radius 2; the cylinder passes before it.

The exact predicate returns `Different` for coedge 1 of all eight sphere faces, on equator edges 1, 3, 5 and 7. The meridian uses are not reported as failures. Trim closure and trim validity pass; face injectivity, pair classification, nesting and orientation remain unproved. This is an exact source-representation mismatch, not a work-budget exhaustion result.

The test now prints each failing face, wire, coedge, edge and predicate decision. Resolving the equator representation must preserve shared topology and prove equality of the original edge and lifted pcurve; a tolerance check does not establish that equality. This evidence does not qualify general sphere volume validity.
