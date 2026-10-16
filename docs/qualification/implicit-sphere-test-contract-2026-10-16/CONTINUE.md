# Qualified original implicit sphere successor

Native code: `5bb7320b2` (geometry is byte-for-byte the production code at
`6476dc11c`; the intervening source correction changes only one JSON unit test).
The frozen source guard contains 3782 inputs:
`c05e18ee41fc72e32b87e5789348a27d618c12d98d559211a1df781f47c18d2a`.
`summary.json` records **3337 passed, 4 ignored, 0 failed** across 14 gates.

Six successful, dependency-disjoint owner gates are preserved from the earlier
same-production run. Their complete logs and hashes are linked in the summary.
The exact reviewed test-only diff and all other source bytes are verified by
`validate.py`; no older feature checkpoint supplies new geometry credit.
The complete bridge, native JSON/STEP matrices, architecture checks, exact
predicate suite and no-codec builds were freshly checked for this successor.

The original failed attempts remain visible in the predecessor directory.
A stale no-codec predicate metadata entry in the shared target omitted the new
quadric module. Regenerating that package's dev cache fixed the compile failure
without source changes. `finish.py` preserves the original failure, independently
reruns the failed gate and exact-kernel suite, verifies regenerated dependency
metadata, verifies source/log guards and completes the receipts. Do not describe
the failed predecessor as a passed run.

Independent OCCT: four original spheres (including poles, literal nonbinary
radius and exact translation) and six original rational-frame/filled-annular-cap
cases. Sphere volume integration converges at two requested accuracies; maximum
analytic volume discrepancy is 4.2633e-14 mm³. OCCT may split native point edges;
each imported degenerate edge is checked against the original pole ownership.
Sampled radius/curvature checks supplement the exact native theorem.

Private release WASM ABI: four sphere cases and six rational-cap cases, including
construction, supplied-model audit, STEP re-audit and short-budget refusal;
wrong sphere source refuses and a real sharp cap join retains explicit C0.
Fresh native fixture inputs match the runtime-tested inputs byte-for-byte.
Final-source build is byte-identical to the runtime-tested binary: 18,073,723
bytes, `bf079682200cccd37f3cd9b0ea77147e6d25e1b07f3da8eef5bf78139e822a6a`.
The binary is separately preserved in
`/private/tmp/osv-implicit-sphere-artifact-20261016/geometry_wasm.wasm`.
`wasm-build.json` and `wasm-runtime-inputs.json` bind source, binary, Node runtime,
codec/harness inputs and native fixture correspondence. This is Node 26 private
ABI qualification, not a public package, Node 20 floor, browser or UI result.

## Remaining goal

- General ideal source embedding/regularity/topology beyond the qualified
  sufficient original-map classes; general filled end-cap atlases and
  non-spherical complete pole fans.
- Applicable G1/G2 for general frames/caps/seams, source-to-retained differential
  correspondence and mixed C0/G1/G2 ownership. Keep sharp joins explicit C0.
- Corrected-Frenet planar arc source jets currently stop outside the forward
  line in `ideal_volume/arc_frames.rs`. Source smoothness alone cannot replace
  the missing frame jet correspondence.
- Closed spatial Bishop arc seams with only geometric clock continuity retain
  a conservative Cartesian source-jet gate. Relaxing it requires exact original
  arc translation/frame/law jets and complete knot/seam coverage together.
- See the predecessor CONTINUE for the separate ordered-fiber theorem needed
  when smooth flat end caps make a volume parameterization singular at its ends.
  An inner-wall cusp is a mandatory negative case; planar tangency alone is G0.
- Public package/fingerprints, browser/Rush/viewport/Solid UI, full semantic G1,
  fresh main integration and CI remain separate unfinished gates.

`fullGoalComplete` remains false. Publication needs fresh human authorization
for the exact GitHub destination and accumulated source/evidence payload after
the earlier automatic approval rejection. Local code and reports are preserved
in their Git branch; do not alter the user's dirty primary checkout.
