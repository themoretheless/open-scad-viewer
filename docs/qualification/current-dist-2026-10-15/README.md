# Current WASM distribution qualification — 2026-10-15

Rebuilt the root checkout's `dist` directly with Vite against the current generated WASM. The Rust kernel was not rebuilt or replaced. Parallel source changes were preserved; no commit or push was performed for this rebuild.

Geometry WASM SHA-256 in `src/generated/geometry-kernels/kernel_bg.wasm`, `public/wasm/geometry-kernel.wasm`, and `dist/wasm/geometry-kernel.wasm`:

`21ac2eb50621ea90ae1caaac38ffb62b653d8b2e8d5b72cee6ee31cb9913dbc2`

## Results

- Vue application and MCP TypeScript checks passed.
- Production build passed; existing size budgets were unchanged.
- `verify-dist.mjs` passed: 173 artifacts, 8,050,841 asset bytes plus 14,103,024 raw WASM bytes, 22,153,865 bytes total. It verifies packed WASM contents as well as raw delivery.
- 116 selected tests across 13 files passed after the drawing suite was rerun in isolation. Coverage includes profile certificates, Bézier degeneracies/booleans, drawings/specifications, Solid worker/client/transport, G2 surfaces/tubes, closed patch networks, G2 evidence, and automatic annular caps.
- Four direct tests of the current WASM passed: same-side cusp, affine cusp, nonprimitive rational cubic, and retained rational ellipse. Source-bound topology certificates verified; forged certificates were rejected. Extruded meshes were closed with zero degenerate triangles, nonmanifold edges, and orientation conflicts; volume error stayed below 0.3%.
- Native browser verification in Microsoft Edge loaded the production preview, imported the freshly generated `ellipse-solid.json`, and displayed the retained sketch and extruded body. Disabling “Гладкий показ B-rep” verified the ordinary mesh display. Screenshot: `browser-mesh.png`.
- Browser console inspection found no application-code exception during this scenario. The installed extensions emitted an unrelated `$ is not defined` error and MetaMask listener warnings; `/favicon.ico` returned 404.

## Timing qualification

The sphere hidden-line test exceeded its existing 60-second limit during three attempts under concurrent system load. No timeout was increased. Once load decreased, all seven drawing tests passed in 20.52 seconds total (15.66 seconds executing tests). A separate run of the same sphere projection assertions passed in 10.02 seconds and produced `sphere-drawing.svg`.

These checks qualify the rebuilt package and the scenarios above; they do not claim exhaustive UI coverage or formal certification of arbitrary 3D solid geometry. Profile certificates retain the `represented-profile-topology` scope.

## Evidence

- `build.log`, `verify-dist.log`
- `tests-initial.log`, `drawing-tests.log`, `drawing-functional.json`
- `wasm-smoke.json`, `certificates.json`
- `ellipse-solid.json`, `sphere-drawing.svg`, `browser-mesh.png`, `browser-console.txt`
