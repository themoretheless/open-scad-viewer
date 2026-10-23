# NURBS offset qualification

Bounded offsets support planar XY curves, including constant Z. Smooth curves retain source parameter domains; piecewise bevel wires retain separate station and source domains. Source curves remain unchanged.

Represented-chain diagnostics identify crossings, contacts, degeneracy and unresolved pairs. Bevel results are untrimmed; neither original offset topology nor region validity is certified. Inner crossings are highlighted in the viewport.

Verified in the working candidate: native worker integration, independent Decimal position samples, cancellation and delayed result rejection, apply/undo/redo/reload, and 20 consecutive bevel applies followed by 20 undo and 20 redo. The long sequence inspects the JSON Blob created by the real export button; short scenarios use actual downloads. Latest layout screenshot confirms both numerical inputs and the join selector are visible.

Publication uses an isolated checkout to exclude unrelated crate extraction and primitive work. Checks for that exact checkout are recorded below. Full P0–P3 completion remains open, including trimmed offset regions and general joins.

## Isolated publication checks

- Native nurbs-core: 176 unit tests and 1 doctest passed.
- Type checking passed.
- CAD roadmap: 731 tests passed initially; the one missing photogrammetry artifact check passed in the focused 9-test packing recheck (732 CAD tests covered). Photogrammetry is unchanged; its packed and emitted bytes were compared to the tracked published raw artifact. The Brotli decoder was rebuilt.
- Real worker and new offset validation: 19 tests passed.
- Dist: 130 artifacts, 7,087,338 asset bytes; unchanged size budgets passed.
- Trim controls now emit only the changed field, preserving rapid consecutive coordinate updates. UI tests preload the separate panel and wait on real event-loop turns.

- Browser on the isolated emitted build: 20 applies, 20 undo, 20 redo, reload, cancellation and delayed responses after cancellation/document replacement passed. Browser contract reports no page errors.
