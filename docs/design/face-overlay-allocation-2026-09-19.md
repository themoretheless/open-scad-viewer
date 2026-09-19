# Face overlay allocation

## Change

`buildFaceOverlayGeometry` now fills its bounded Float32Array directly instead
of growing a JavaScript number array and copying it into a typed array afterward.
The selected triangle count is already known, including the existing oversized
face fallback. Invalid triangles are still skipped; only that exceptional path
copies the written prefix so callers do not retain a large unused buffer tail.
CSR lookup, triangle order, transforms, boundary edges and selection limits are
unchanged. This follows the existing source-overlay allocation pattern.

Unlike the rejected sparse-index prototype, this does not introduce a separate
first-hover index build or a retained Map.

## Measurement

Command: `node --import tsx benchmarks/face-overlay-allocation.mts BASELINE_PATH`.
BASELINE_PATH points to `src/services/meshSelectionOverlay.ts` from baseline
commit `b787b4f2`, with its relative imports resolvable. The report records both
source hashes, CPU/Node identity, all samples and output hashes. Measurements
alternate baseline/candidate order, use five warmups and 15 samples, and verify
exact triangle/boundary bytes outside timing. Both variants share a prebuilt CSR
index. Allocation and GC are included; index construction and rendering are not.

Two isolated local runs, median milliseconds per overlay:

| Face triangles | Baseline | Direct fill | Repeat baseline | Repeat direct fill |
| --- | ---: | ---: | ---: | ---: |
| 2 | 0.001259 | 0.001149 | 0.001206 | 0.001195 |
| 2,048 | 0.631542 | 0.496709 | 0.656333 | 0.510083 |
| 20,000 | 8.884416 | 6.829500 | 10.123000 | 7.503208 |

Retained result: approximately 21-22% less CPU time for the 2,048-triangle face,
23-26% for the 20,000-triangle face. Tiny-face improvement is too small/variable
to claim. These are coplanar synthetic fixtures, not an application FPS claim.

## Verification

- 18 overlay/grouping tests passed, including partial and entirely invalid
  triangles, exact compact buffer ownership, capped faces and indexed parity.
- UI type checking and production Vite build passed.
- Distribution verification passed: 89 artifacts, 5,777,192 asset bytes plus
  9,697,456 raw WASM bytes (15,474,648 total).
- No qualification artifacts or thresholds were updated. The known evidence
  binding failures are separate and remain unresolved.
