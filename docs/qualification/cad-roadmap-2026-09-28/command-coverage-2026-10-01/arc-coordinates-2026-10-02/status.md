# Exact numeric arc authoring — 2026-10-02

Arc center X/Y, radius, start angle and signed sweep use the existing length/angle quantity parser. Preview is derived from the analytic curve without changing the document. Create commits one open analytic arc in the active sketch/workplane. Escape returns to Select. Mouse gestures retain their existing behavior.

Radius accepts 0.01–1,000,000 mm. Absolute sweep accepts 0.1° up to less than 360° (matching the existing Rust sampler's minimum). Full circles use the Circle command. Incorrect fields block create and hide preview. Errors are localized, and the invalid angle has aria-invalid/aria-errormessage.

## Final native browser evidence

Chrome Canary 157.0.8081.0 with WebGPU active; no page or console errors. Every report pins the source, built DirectModeler and WASM SHA256. All nine reports refer to the same final build. Browser actions use mouse or actual Tab/Enter/type events; file fixtures are imported through the native JSON input.

| Scenario | Input | Plane | Sweep, degrees | Tabs |
| --- | --- | --- | --- | --- |
| arc-face-keyboard | keyboard | body face Y=-10 | -120 | 1544 |
| arc-face-mouse | mouse | body face Y=-10 | 120 | 0 |
| circle-face-keyboard | keyboard | body face Y=-10 | 360 | 1457 |
| circle-face-mouse | mouse | body face Y=-10 | 360 | 0 |
| circle-regression | keyboard | XY | 360 | 1155 |
| keyboard | keyboard | rotated sketch | -120 | 1377 |
| keyboard-counterclockwise | keyboard | XY | 120 | 1241 |
| mouse | mouse | XY | 120 | 0 |
| mouse-clockwise | mouse | rotated sketch | -120 | 0 |

Every scenario proves an uncommitted preview, exact analytic parameters, blocked invalid radius and center, create, Undo/Redo, Escape without a document edit or surviving preview, and exact JSON after reload. Arc runs additionally reject bad/zero/0.01/-0.05/full-turn sweeps and invalid start; independent sin/cos calculations compare both sampled endpoints to a 1e-9 mm threshold. This is endpoint verification, not a general continuous tolerance certificate.

Body-face runs select the third planar face with the face control and prepare its workplane through Sketch on face. Created sketches retain supportBodyId=base. Basis vectors are orthonormal in Y=-10 and every sampled point maps onto that face's supporting plane. The source body remains unchanged through full-document comparisons.

Actual keyboard invalid-start, rotated draft and body-face 3D draft screenshots were inspected. The sticky command title, invalid field feedback, active plane and dashed preview are visible.

## Local checks

- Focused UI tests: 5 passed (polyline, circle, arc +/-120).
- Full DirectModeler suite: 298 passed, 99.38 seconds on the final source.
- TypeScript, production build and distribution verifier pass.
- 140 artifacts: 7,210,516 asset bytes plus 11,669,725 raw WASM bytes.
- DirectModeler: 392,515 bytes; geometry WASM unchanged, SHA256 07cd41b96faa8f5bb05507c461845711c4355613c0fb856eb4f5fb20d736b5fe.

## Scope still open

This qualifies numeric circle/arc creation and their stated fixtures. It does not close the full command matrix, all errors, arbitrary fillets/B-rep distance, general profiles/surfaces, large-scene performance or P3 integrations. Real gesture drawing and general sketch-on-face transport failures remain distinct qualification cases.
