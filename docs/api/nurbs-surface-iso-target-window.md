# Original surface iso / target window

Bridge operation: `nurbs_surface_iso_target_window`.

Required fields:

- `source`, `target`: original 3D NURBS surfaces, using the existing Surface codec.
- `axis`: 0 for a source iso varying in U, 1 for one varying in V.
- `parameter`: finite normalized transverse source parameter, strictly between 0 and 1. The affine map to the source knot domain is interpreted exactly over the input binary64 values.
- `window`: three closed finite strict intervals, in order: source natural varying parameter t, target natural U, target natural V. Each lies inside its own original active domain.
- `maxCells`: integer 1..100000, shared across initial search, subdivision and refinement.

No additional request fields are accepted. The source iso is bounded directly from the original Surface; no rounded extracted Curve becomes authoritative. Both charts must be nonperiodic, valid positive-weight 3D surfaces. The degree product is limited to 4096 and intersecting initial span products to 1024.

Response schema: `nurbs-surface-iso-target-window/1`.

Each `roots` item contains `method: original-curve-surface-krawczyk`, original span IDs, `existenceDomain`, `parameterBox`, world `positionBounds`, and `crossingSign`. Its existence domain owns the strict interval existence/uniqueness proof. Parameter/world boxes enclose that root; an interval certificate is not an authored point vertex.

`presenceProven` means at least one root was certified. `windowCoverageComplete` means no requested parameter-product region remains unresolved. `windowAbsenceProven` additionally requires no roots. The status is respectively `present`, `absent-in-window`, or `unresolved`; a present report can still be incomplete.

`unresolved` preserves remaining natural parameter boxes, including exhausted or singular regions. Tangencies and roots on unresolved subdivision/span boundaries need not be certified by this transverse solver.

`allSurfaceBranchesEnumerated`, `trimAdmission`, and `bodyAdmission` remain false. Even a complete window covers only the selected source iso, not the other fibers or all branches of the surface/surface intersection. Read-only input charts remain unchanged.
