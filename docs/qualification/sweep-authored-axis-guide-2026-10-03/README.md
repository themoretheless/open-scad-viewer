# Authored axis + orientation guide qualification, 2026-10-03

The combined miter contract uses the normalized authored axis and the projected world-space guide offset for the transverse normal. `frameNormal` remains a validated authored input; the guide controls transverse orientation when both are supplied. Twist rotates this normal about the authored axis. Affine axis scale and center laws compose afterward. Original domains and source curves are preserved.

Native interval certificates cover axis, guide, path and twist jets with one shared cell budget. Singular/parallel guide offsets and exhausted budgets remain unresolved; partial fields are discarded. Both setter orders are tested. The moving-axis public test compares retained station coordinates to an independent analytic formula on separate source domains.

Validation: 61 native progressive-miter tests passed; 33 public miter/stream-ownership tests passed; both plain and corrected combined Rush/Solid examples passed additional focused tests. Vue type checking and scoped diff checks passed. The rebuilt WASM is 10,652,955 bytes, SHA-256 `3efb5ccd52a325ba6aa600203b5403728744ba81a4cd9183c968e47476790eab`.

Independent OpenCascade imported and checked all 24 STEP fixtures, including plain and corrected combined authored-axis/guide/affine hollow bodies. Geometry coefficients/basis, edge ownership and pcurves, topology, shell orientation, holes and analytic volume are checked for these fixtures. This is finite evidence, not proof for every surface or global containment family.

Both combined examples passed 18 real-browser lifecycle checks each at 1440 and 600 pixels. The full 19-mode matrix passed 38/38 scenarios and 338/338 assertions; the corrected combined mode is a supplementary run against the same frozen dist and WASM. Dispatch cancellation tests hold dispatch, not interruption inside an executing native kernel. Chromium CPU fallback does not qualify hardware GPU behavior.

This is local work in a broad concurrently dirty checkout. `observed-source-artifacts.json` records observed files and artifacts, not a clean-tree all-source reproducibility claim. The full goal remains active: arbitrary-mode global embedding, complete continuous-bound admission and general G1/G2 moving-frame/closed-seam guarantees remain open. Sharp miter corners remain C0. These sweep changes are not published; the earlier loft PR is separately merged.
