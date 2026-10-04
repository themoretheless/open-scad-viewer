# Retained profile pair diagnostics — 2026-10-01

## Qualified behavior

A refused retained-profile preparation keeps its original assembled XY curve definitions and segment provenance. A read-only Rust query inspects distinct segment pairs with a shared work budget. Reports retain source loop/curve indices, numerical contact representatives, parameter intervals, overlaps, unresolved cells and unvisited pairs.

The worker prepares diagnostic display samples. Both viewports mark crossings and involved source curves; uncertain contacts use yellow dashes. Enter refuses the invalid profile. Escape preserves the document, and a late worker result cannot restore canceled diagnostics.

Exact adjacent endpoint joins are omitted from defect markers. Numerical event enclosures containing an authored shared endpoint remain uncertain: they can describe the join or another contact within tolerance. They do not become definite defects or an absence certificate. Pair coverage and these endpoint bands are shown separately.

This stage does **not** certify self-intersection absence inside one source curve, construct a profile arrangement, authorize trimming/sewing, or implement a general region offset. P0–P3 remain open.

## Evidence

- Native NURBS library: 232 passed.
- Native B-rep library: 684 passed, 3 existing ignored tests. The wider integration run failed; see below.
- Geometry bridge profile tests: 6 passed; the separate rational crossing/preparation test passed.
- Host tests: 29 passed, including rational crossings, joins/overlaps, budgets, malformed responses, source ownership and source preservation.
- CAD roadmap suite: 863 passed in 74 files.
- Final UI suite after the compact status change: 275 passed.
- `vue-tsc --noEmit`: passed. Vite build and distribution verification: passed.
- Final Chromium mouse and keyboard scenarios: two crossings of source segments 0 and 2, disabled Apply, source-preserving cancellation and JSON export. Keyboard scenario used sequential Tab/Enter selection and Escape. Final screenshots were inspected.

The browser fixture is a quadratic rational arch with control points `[0,0], [1,5], [2,0]`, weights `[1,.8,1]`, and a closing polyline through `[2,0], [2,2], [0,2], [0,0]`. Both intersections with the upper segment are located at y=2 mm. Source definitions and document identities survive refusal and cancellation.

## Integration failure remains open

The full native B-rep run failed six `tolerant_ssi` tests. Each failed while measuring mass properties with `BREP_RESOURCE_LIMIT: Mass-property integration exhausted its evaluation budget` at a 2,000,000-evaluation limit.

An isolated source snapshot of HEAD `1d76d7093a69c083f0cb78e7eff5f6b726ae0026` reproduced the same failure in `off_axis_sphere_in_a_cylinder_wall`. That representative failure predates this stage. The other five baseline cases were not rerun individually. The full native suite is not green, and this failure belongs to the remaining P1 dimension/diagnostic qualification work.

## Size and admission

The diagnostic query admits 1–64 nonempty loop arrays, 2–254 source curves and at most 8192 controls, subject to the existing curve-pair solver admission. Pair budgets are 1–10000; shared subdivision budgets are 1–1000000. Each solver call gets at most 8192 boxes. Preparation diagnostics use 128 pairs and 8192 total boxes. Coverage stops honestly at these limits.

Final optimized geometry WASM: 9,607,164 bytes; SHA-256 `64022a84450cb4d79bd051368b063f9203f5afa9c51fa33f7fe4940263442877`.

Final assets: 7,193,674 bytes, an increase of 21,171 bytes from the previous qualified build. The new diagnostic presentation loads asynchronously. Measured size allowances are recorded explicitly:

- DirectModeler: 379,305 bytes; budget 380,100.
- Main solid worker: 117,652 bytes; budget 118,500.
- Diagnostic presentation module: 4,621 bytes.
- Assets budget: 7,194,600 bytes, leaving 926 bytes.
- Packed geometry retains its existing 3,244,000-byte budget.

These budget changes account for added functionality; they are not a claim of a smaller total build or completed performance work.
