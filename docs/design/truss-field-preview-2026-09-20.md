# Signed Axial Force Preview

The nominal truss results panel can publish midpoint markers to the existing
CAD preview channel. Blue represents negative axial force (compression), red
positive force (tension), gray exactly zero. Color is sign only, not magnitude,
utilization, buckling risk or safe/unsafe status. Marker size is explicit in mm.

Geometry is batched into at most three meshes for 400 members (3200 triangles).
Existing STL-to-display conversion supplies normals, BVH and scene geometry
identity. Invalid field values or marker sizes are refused; no model mutation.

Changing load/material/case/geometry clears the field. Starting another CAD
calculation clears the field via the preview epoch. Publishing the field
cancels pending CAD work so an older result cannot overwrite it. Unmount and
source/selection changes retain normal preview cleanup.

Validation: seven focused geometry/UI tests, Vue typecheck, production build
and verify-dist passed (95 artifacts, 15,746,733 total bytes). The actual panel
and native-worker browser workflow passed in Chrome on 1280x900 and 390x844:
it checks marker publication, triangle count, finite vertices, invalid-size
refusal/recovery and clearing on changed case/mesh. Panel screenshots were
inspected for layout. The harness observes emitted preview meshes; it does not
render the application's CAD viewport. Actual viewport rendering/occlusion
and complete-operation ownership checks remain to verify before calling the
visual integration complete. No solver or WASM changes in this step.

## Renderer Verification

The harness now optionally initializes the real WebGPURenderer with
`TRUSS_VIEWPORT=1`. It uses the same setMeshes replacement/restoration pattern
as App.previewMainGeometry: this is an isolated marker preview, not an overlay
on the original body's surfaces. Clearing restores the source mesh.

Chrome runs passed at desktop and mobile sizes. Screenshot pixels contain
both red and blue markers; a real pointer drag changes the rendered field;
the restored scene differs and shows the original cube. Screenshots were
visually inspected. Pixel checks decode browser PNG screenshots rather than
copying the cleared WebGPU canvas backing store. Reports/screenshots are in
`/private/tmp/osv-truss-renderer-browser.json` and
`/private/tmp/osv-truss-viewport-{desktop,mobile}-{field,orbited,restored}.png`.

This closes the real-renderer nonblank/framing/orbit check. It uses an isolated
host for the actual CAD panel and renderer, not a complete App session; broader
cross-tool preview ownership remains a separate integration check.
