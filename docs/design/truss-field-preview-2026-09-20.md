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

## Parent Tool Reset

The MainModelingTools cancel path now advances a preview epoch through
CadWorkbenchPanel to the field controls. This fixes a retained CAD panel whose
field checkbox stayed on after another parent command restored the scene.

`TRUSS_TOOLS=1 TRUSS_VIEWPORT=1` runs the browser workflow through the real
MainModelingTools parent, native worker and WebGPU renderer. It enables the
field, invokes box-select (which leaves the CAD panel mounted), verifies null
preview and an unchecked field control, disables box-select and re-enables the
field. Desktop/mobile runs passed, including the existing solve/stale reply/
restoration/orbit checks. Report: `/private/tmp/osv-truss-tools-browser.json`.
Seven focused tests, Vue typecheck, production build and dist verification pass.
This covers the concrete parent-command ownership defect, not all App workflows.

## Geometry Preparation Baseline

`node --import tsx benchmarks/truss-field-meshes.mts` compares the current
batched conversion against one identical marker mesh per member. It warms
the verified geometry WASM, checks exact sorted position/normal/color tuples,
then uses 20 warmup pairs and 31 alternating measured pairs. Builds/tests were
not run alongside measurements. Two process runs:

| Members | Batched p50 ms A/B | Batched p95 ms A/B | Separate p50 ms A/B |
| ---: | ---: | ---: | ---: |
| 36 | 0.56775 / 0.57762 | 0.60496 / 0.63404 | 4.90158 / 4.91592 |
| 400 | 2.74850 / 2.77033 | 3.19842 / 3.17046 | 54.33846 / 54.14633 |

The control is not a checkout of PR7: it uses the current identical marker
conversion repeatedly, isolating per-mesh setup and batching. The maximum
fixture has 125 nodes, 400 unique members and 3200 marker triangles. Fields
are synthetic signed values; this measures display preparation, not solving.
Reports: `/private/tmp/osv-truss-field-bench-{a,b}.json`. The result supports
batching but does not prove browser main-thread/frame latency; a browser
timing check remains necessary before deciding on a worker migration.

## Browser Preparation Timing

Enable `TRUSS_FIELD_BENCH=1` alongside `TRUSS_TOOLS=1 TRUSS_VIEWPORT=1` in
the browser harness. The 125-node/400-member fixture is created outside timing;
20 warmups precede 31 measurements separated by animation frames. Each result
is checked for three meshes and 3200 triangles outside its timed interval.
Only mesh preparation is timed, not renderer upload, GPU work or presentation.

Chrome 156.0.8063.3, two process runs:

| Viewport | p50 ms A/B | p95 ms A/B |
| --- | ---: | ---: |
| 1280x900 | 5.5 / 5.5 | 6.6 / 6.4 |
| 390x844 | 6.0 / 5.9 | 6.6 / 6.4 |

Both viewports use the same development machine, not mobile hardware or CPU
throttling. Reports: `/private/tmp/osv-truss-field-browser-bench-{a,b}.json`.
The browser cost is materially higher than Node. Retain the bounded,
user-triggered synchronous preparation for now; no continuous per-frame
rebuild is performed. A larger graph budget, animated fields or measured
slow-device stalls would require revisiting worker preparation. These results
are not a universal frame-time guarantee or a browser batching speedup claim:
the per-member control was measured only in Node.

## Field Integrity At Display Precision

Two regressions were reproduced before the guard fix: a sparse force array
colored a missing member value as zero, and a 0.001 mm marker near X=1,000,000
could collapse in Float32 while other markers in the same group survived.
The STL display converter intentionally drops degenerate triangles, which is
appropriate for imports but must not silently remove members from a result field.

The field adapter now rejects sparse/nonfinite values, collapsed Float32 marker
extents and any conversion that loses triangles. It publishes no partial field.
A larger representable marker is accepted; the source model remains unchanged.
Five field tests pass, along with the four focused CAD UI tests, Vue typecheck,
production build/dist verification and the parent-tools/native-worker/WebGPU
browser workflow on both viewports. Full-suite results above predate this fix;
the qualification failures are unchanged and were not modified.
