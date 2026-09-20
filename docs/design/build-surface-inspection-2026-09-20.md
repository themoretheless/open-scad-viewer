# Geometric Build Surface Inspection

Native foundation for the remaining lattice printability report:
`geometry_bridge::print_geometry::inspect_build_surfaces` consumes the actual
triangle mesh, a nonzero build direction, a downward-normal cone angle, an
explicit plate offset and a plate tolerance. There is no pattern-name ranking.

The normalized build direction points away from the plate. Plane coordinates
are dot(position, direction) in mm. Cone angle is measured from the negative
build direction: 0 selects directly downward normals, 90 any downward normal.
The report contains total surface area, cone-selected downward area/count
excluding plane-contact triangles, contact area and the count of triangles
with any vertex below the plane minus tolerance. Contact requires all three
vertices within tolerance. These are whole-triangle classifications, not
clipped intersection areas or slicing results.

Admission: nonempty mesh, at most 100,000 triangles and 2,000,000 coordinate
scalars, finite coordinates within +/-1,000,000 mm, cone 0..90 degrees,
finite bounded plane/tolerance. Existing polygon inspection checks closure,
non-manifold edges, winding conflicts, degeneracy and positive signed volume.
Normals still derive from supplied winding; independent containment of nested
components and self-intersections are not certified. Positive total volume
alone is not proof that every component is oriented as a valid solid boundary.

This is a geometric screen only. It does not predict supports, bridge spans,
adhesion, heat, material behavior, strength or print success, and does not
automatically choose an orientation or emit a slicer profile.

Four native tests cover analytic box areas, seated/floating/below-plane cases,
direction scaling/reversal, translation, oblique cone selection, tolerance and
invalid/open/reversed inputs. All 272 geometry-bridge library tests pass.
The previous complete integration-test run is recorded separately; it was not
repeated here. No JSON/WASM operation or UI is added yet, and the checked-in
browser artifact is unchanged. Worker admission, artifact rebuild, UI report
and workload measurements remain before this feature is user-accessible.

## JSON And WASM Adapter

`mesh_build_surfaces` now requires exactly op, mesh, buildDirection,
coneDegrees, planeOffsetMm and planeToleranceMm. Mesh fields are restricted
to positions/indices/optional uv; position/index array budgets are checked
before typed mesh decoding. Unknown fields and missing settings fail rather
than implying supported physics or inventing defaults. The result identifies
`signed-triangle-build-surfaces-v1` and returns five geometric quantities.
The TypeScript wrapper sends position/index geometry only.

Two native dispatcher tests and 25 selected WASM/G-code/truss tests passed.
The real-browser harness also asserts the 10 mm cube report (600 mm2 total,
100 mm2 plane contact) before its parent-tools/native-worker/WebGPU workflow;
desktop and mobile viewport runs pass. Vue and standalone adapter typechecks,
Vite build and verify-dist pass: 95 artifacts, 15,754,019 total bytes.

Rebuilt geometry WASM: 7,729,128 bytes, SHA256
67f48c69ea63a99cf1e26d24ffe0e8310befc52ea5c841b9b738a741d64bd899.
Raw size grew by 5,616 bytes. This rebuild also incorporates the previously
native-tested writer refactors. Historical qualification archives remain
unchanged; this is not qualification of the new binary. The full JS suite was
not repeated in this step; its known nine identity/qualification failures are
not resolved. Worker admission, user-facing report and timing remain pending.
