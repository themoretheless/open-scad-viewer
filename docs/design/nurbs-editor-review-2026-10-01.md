# Editor qualification: first analytic primitives

## Observed in the actual browser

Local Vite application, Chromium in-app browser, 2026-10-01.
The Rush ellipsoid example compiled through the source editor and reported one
object, 1,984 triangles, and Ready. No compilation diagnostic was displayed.

Render updates the source result. The Solid and Mesh workspaces are separate;
the source result needs the explicit “Bring scene into Mesh” action.
After that action the Mesh scene listed Object 1 and 1,984 triangles, but the
viewport remained a floor grid. Reset view did not reveal the ellipsoid.
DOM inspection found zero polygons in `svg.mesh-view`. Captured browser warning
and error logs were empty. This is an unresolved integration failure; it is
**not** visual qualification of the ellipsoid or the other primitives.

The original source text was restored after the check. No Solid scene rebuild
or export was performed during this browser check.

## Independent checks

`node_modules/.bin/vitest run tests/meshModelerProjection.test.ts` passes all
three projection tests. That establishes the tested Rust/WASM projection cases,
not that the live Vue scene publishes its computed faces correctly.

Next investigation: verify mesh visibility, reactive scene publication, kernel
readiness and the runtime artifact used by the actual Mesh workspace. Recheck
the eight analytic examples visually before promoting any catalog entry to
qualified. JSON and OBJ/PLY round trips are covered separately by
`tests/nurbsPrimitiveGraph.test.ts`.

## Follow-up: resolved viewport failure

The Mesh component now owns its asynchronous geometry-kernel warm-up and
invalidates the scene projection after readiness. The delayed-readiness
component regression test checks that a saved triangle appears without a user
edit and that no projection is requested while the kernel is cold.

The live browser subsequently published all 1,984 ellipsoid polygons, but their
bounding boxes were below the screen: SVG height was 1,280 px, starting at
187 px. The dock was outside `mesh-stage`, consuming vertical space, and the
SVG wrapper had an unconstrained intrinsic height. The dock now belongs to the
stage row; the SVG wrapper fills the stage with absolute inset positioning.

After the fix, the actual browser shows the ellipsoid. Measured SVG bounds are
936 by 505 px, top 187 and bottom 692. The first face lies at y=462..468 inside
the viewport. The scene contains 1,984 polygons. Screenshot:
`docs/design/nurbs-editor-review/ellipsoid-mesh.jpg`.

This closes the observed ellipsoid viewport failure. It does not qualify all
15 examples visually or prove STEP round trips. The component test also checks
the dock's stage parent to prevent the misplaced closing tag from returning.
