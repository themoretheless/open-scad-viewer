# Loft publication qualification

Branch: codex/loft-library, based on origin/main 8e7cb190.

- nurbs-core: 273 library tests passed.
- brep-core: 11 rational/guided/closed loft and STEP tests passed.
- modelgraph-runtime: 54 library tests passed.
- modelgraph-text: 6 library, 6 frontend and 2 loft tests passed.
- TypeScript: 42 tests across guided loft, advanced loft, graph schema, NURBS graph and text frontend passed with the rebuilt release WASM.
- OpenCascade: valid hollow solid, 10 faces (8 cubic), volume relative error 3.2441e-13 against independent analytic integration. See ocp-solid.json and natural-hollow-solid.step.

Spatial budget is in mm. U-station parameter_tolerance is dimensionless, defaults to 1e-8, and must lie in (0,1].

The earlier qualification directories retain evidence from the original shared checkout. This directory records the isolated publication checkout. CI and GPU qualification are not claimed.

Final optimized geometry WASM: 9,676,259 bytes. The 38 focused loft/text/Brotli tests and vue-tsc passed after packaging. Three fresh Chromium CPU mesh previews were rendered and visually inspected; capped loft reports 1116 vertices / 2228 triangles and closed. The harness pans geometry into the viewport for screenshots.

CI previously failed on fresh runners because build-vr.mjs forced Cargo --offline while the pinned rbench Git dependency was not cached. The build retains --locked and now allows normal dependency acquisition; local VR build passed. CI is rerun after publication.
