# Benchmark: 100 relevant repositories

Snapshot: **2026-07-14 00:56 +04:00**. The final set contains exactly
100 public, non-fork, non-archived repositories. At snapshot time it represented
463,267 stars in total with a median of 1,444 stars. Star counts are a discovery
signal, not a quality score.

## Selection snapshot

The final TSV is independently verifiable, but this is not a fully reproducible
benchmark artifact: the exact query strings and the raw candidate pool were not
committed with the snapshot. The recorded selection procedure was:

1. Run 24 GitHub Search API queries with `fork:false archived:false`,
   `sort=stars`, `order=desc`, and `per_page=100`.
2. Cover OpenSCAD, STL/CAD/3D viewers, WebGL/WebGPU/Three.js viewers, and the
   matching GitHub topics and JavaScript/TypeScript variants.
3. Deduplicate 1,348 candidates.
4. Add canonical geometry/rendering dependencies referenced by the viewers
   (Manifold, OCCT, Draco, meshoptimizer, glTF tooling).
5. Exclude awesome lists, model/data repositories, tiny tutorials without
   reusable code, irrelevant name matches, forks, and archived projects.
6. Rank the remaining projects by direct relevance, unique transferable
   capability, maintenance, and stars; verify every final repository through the
   REST API.

The [TSV snapshot](top-100-repositories.tsv) used to build this document has SHA-256
`729a88cb675c830e6c529613508d1cce4e52ac218e76715cbe939b9327ca51e2`.

## What the mature projects do better

Repeated patterns in the sample were:

- a real geometry kernel and explicit diagnostics;
- compilation in a Worker, debounce, render caches, and stale-job rejection;
- fit/reset, orthographic and standard views, grids and axis helpers;
- editor/file workflows, share links, persistence, and clear unsupported-state
  handling;
- event-driven redraw and geometry budgets before adding BVH/LOD complexity;
- tests, CI, security updates, documentation, and accessible controls.

The closest modern OpenSCAD reference was
[zacharyfmarion/openscad-studio](https://github.com/zacharyfmarion/openscad-studio):
it uses OpenSCAD WASM in a Worker, a CSG backend, compiler diagnostics,
multi-file dependencies, render caching, stale-result protection, and rich CAD
viewport controls. The official runtime is GPL-2.0+, so this project uses its
own-Rust CAD kernel and labels itself a strict subset.

## Changes implemented from the benchmark

- Replaced simulated CSG with Manifold boolean geometry.
- Added variables, expressions, ranges, loops, modules, 2D shapes and extrusion.
- Added line/column errors and browser-safe complexity limits.
- Moved compilation to a dedicated Worker with transferable typed arrays.
- Fixed Z-up coordinates and WebGPU depth projection.
- Added event-driven drawing, orthographic/standard views, fit/reset and grid.
- Added file open/save/drop/share, stale/build status, a resizable workspace,
  keyboard/focus/ARIA improvements, tests, docs, and dependency security updates.
- Added a Plasticity-inspired command palette, view cube, exact object picking,
  focus/isolate actions, and shaded/mesh-edge/x-ray inspection modes.

Deferred items include full OpenSCAD WASM compatibility, multi-file
`include/use`, CodeMirror/Monaco, face/edge-level CAD selection, measurements,
section planes, feature-edge extraction, and format export.

## Top 100

| # | Repository | Stars | URL |
|---:|---|---:|---|
| 1 | `mrdoob/three.js` | 113711 | https://github.com/mrdoob/three.js |
| 2 | `FreeCAD/FreeCAD` | 32096 | https://github.com/FreeCAD/FreeCAD |
| 3 | `pmndrs/react-three-fiber` | 31417 | https://github.com/pmndrs/react-three-fiber |
| 4 | `BabylonJS/Babylon.js` | 25786 | https://github.com/BabylonJS/Babylon.js |
| 5 | `google/filament` | 20250 | https://github.com/google/filament |
| 6 | `playcanvas/engine` | 16234 | https://github.com/playcanvas/engine |
| 7 | `CesiumGS/cesium` | 15460 | https://github.com/CesiumGS/cesium |
| 8 | `assimp/assimp` | 13054 | https://github.com/assimp/assimp |
| 9 | `openscad/openscad` | 9826 | https://github.com/openscad/openscad |
| 10 | `pmndrs/drei` | 9737 | https://github.com/pmndrs/drei |
| 11 | `prusa3d/PrusaSlicer` | 9175 | https://github.com/prusa3d/PrusaSlicer |
| 12 | `zeux/meshoptimizer` | 8191 | https://github.com/zeux/meshoptimizer |
| 13 | `google/model-viewer` | 8160 | https://github.com/google/model-viewer |
| 14 | `KhronosGroup/glTF` | 7785 | https://github.com/KhronosGroup/glTF |
| 15 | `google/draco` | 7400 | https://github.com/google/draco |
| 16 | `Ultimaker/Cura` | 6996 | https://github.com/Ultimaker/Cura |
| 17 | `pmndrs/gltfjsx` | 5818 | https://github.com/pmndrs/gltfjsx |
| 18 | `cnr-isti-vclab/meshlab` | 5752 | https://github.com/cnr-isti-vclab/meshlab |
| 19 | `regl-project/regl` | 5557 | https://github.com/regl-project/regl |
| 20 | `potree/potree` | 5530 | https://github.com/potree/potree |
| 21 | `CadQuery/cadquery` | 5448 | https://github.com/CadQuery/cadquery |
| 22 | `Adam-CAD/CADAM` | 4797 | https://github.com/Adam-CAD/CADAM |
| 23 | `f3d-app/f3d` | 4538 | https://github.com/f3d-app/f3d |
| 24 | `slic3r/Slic3r` | 3633 | https://github.com/slic3r/Slic3r |
| 25 | `kovacsv/Online3DViewer` | 3586 | https://github.com/kovacsv/Online3DViewer |
| 26 | `gkjohnson/three-mesh-bvh` | 3419 | https://github.com/gkjohnson/three-mesh-bvh |
| 27 | `jscad/OpenJSCAD.org` | 3206 | https://github.com/jscad/OpenJSCAD.org |
| 28 | `pmndrs/postprocessing` | 2810 | https://github.com/pmndrs/postprocessing |
| 29 | `IfcOpenShell/IfcOpenShell` | 2646 | https://github.com/IfcOpenShell/IfcOpenShell |
| 30 | `Open-Cascade-SAS/OCCT` | 2645 | https://github.com/Open-Cascade-SAS/OCCT |
| 31 | `gumyr/build123d` | 2629 | https://github.com/gumyr/build123d |
| 32 | `hujiulong/vue-3d-model` | 2519 | https://github.com/hujiulong/vue-3d-model |
| 33 | `visgl/luma.gl` | 2458 | https://github.com/visgl/luma.gl |
| 34 | `donmccurdy/three-gltf-viewer` | 2446 | https://github.com/donmccurdy/three-gltf-viewer |
| 35 | `yomotsu/camera-controls` | 2415 | https://github.com/yomotsu/camera-controls |
| 36 | `NASA-AMMOS/3DTilesRendererJS` | 2385 | https://github.com/NASA-AMMOS/3DTilesRendererJS |
| 37 | `facebookincubator/FBX2glTF` | 2325 | https://github.com/facebookincubator/FBX2glTF |
| 38 | `BelfrySCAD/BOSL2` | 2266 | https://github.com/BelfrySCAD/BOSL2 |
| 39 | `kennetek/gridfinity-rebuilt-openscad` | 2200 | https://github.com/kennetek/gridfinity-rebuilt-openscad |
| 40 | `mkeeter/antimony` | 2186 | https://github.com/mkeeter/antimony |
| 41 | `webgpu/webgpu-samples` | 2146 | https://github.com/webgpu/webgpu-samples |
| 43 | `CesiumGS/gltf-pipeline` | 2121 | https://github.com/CesiumGS/gltf-pipeline |
| 44 | `fougue/mayo` | 2094 | https://github.com/fougue/mayo |
| 45 | `microsoft/maker.js` | 2012 | https://github.com/microsoft/maker.js |
| 46 | `fogleman/sdf` | 1992 | https://github.com/fogleman/sdf |
| 47 | `donmccurdy/glTF-Transform` | 1913 | https://github.com/donmccurdy/glTF-Transform |
| 48 | `nophead/NopSCADlib` | 1604 | https://github.com/nophead/NopSCADlib |
| 49 | `Kitware/vtk-js` | 1515 | https://github.com/Kitware/vtk-js |
| 50 | `KhronosGroup/glTF-Sample-Viewer` | 1464 | https://github.com/KhronosGroup/glTF-Sample-Viewer |
| 51 | `zalo/CascadeStudio` | 1424 | https://github.com/zalo/CascadeStudio |
| 52 | `flyfish-dev/file-viewer` | 1406 | https://github.com/flyfish-dev/file-viewer |
| 53 | `SolidCode/SolidPython` | 1259 | https://github.com/SolidCode/SolidPython |
| 54 | `playcanvas/editor` | 1223 | https://github.com/playcanvas/editor |
| 55 | `CadQuery/CQ-editor` | 1197 | https://github.com/CadQuery/CQ-editor |
| 56 | `ThatOpen/web-ifc-viewer` | 1026 | https://github.com/ThatOpen/web-ifc-viewer |
| 57 | `ThatOpen/engine_web-ifc` | 990 | https://github.com/ThatOpen/engine_web-ifc |
| 58 | `JustinSDK/dotSCAD` | 929 | https://github.com/JustinSDK/dotSCAD |
| 59 | `xeokit/xeokit-sdk` | 912 | https://github.com/xeokit/xeokit-sdk |
| 60 | `repalash/threepipe` | 895 | https://github.com/repalash/threepipe |
| 61 | `GridSpace/grid-apps` | 881 | https://github.com/GridSpace/grid-apps |
| 62 | `ostat/gridfinity_extended_openscad` | 877 | https://github.com/ostat/gridfinity_extended_openscad |
| 63 | `pmndrs/three-stdlib` | 853 | https://github.com/pmndrs/three-stdlib |
| 64 | `specklesystems/speckle-server` | 824 | https://github.com/specklesystems/speckle-server |
| 65 | `mlightcad/cad-viewer` | 817 | https://github.com/mlightcad/cad-viewer |
| 66 | `revarbat/BOSL` | 688 | https://github.com/revarbat/BOSL |
| 67 | `ThatOpen/engine_components` | 682 | https://github.com/ThatOpen/engine_components |
| 68 | `Irev-Dev/Round-Anything` | 670 | https://github.com/Irev-Dev/Round-Anything |
| 69 | `sgenoud/replicad` | 657 | https://github.com/sgenoud/replicad |
| 70 | `gdsestimating/three-dxf` | 639 | https://github.com/gdsestimating/three-dxf |
| 71 | `deadsy/sdfx` | 625 | https://github.com/deadsy/sdfx |
| 72 | `fstl-app/fstl` | 587 | https://github.com/fstl-app/fstl |
| 73 | `xeokit/xeokit-bim-viewer` | 551 | https://github.com/xeokit/xeokit-bim-viewer |
| 74 | `DSchroer/dslcad` | 534 | https://github.com/DSchroer/dslcad |
| 75 | `KhronosGroup/glTF-Validator` | 452 | https://github.com/KhronosGroup/glTF-Validator |
| 76 | `openscad/openscad-playground` | 434 | https://github.com/openscad/openscad-playground |
| 77 | `opensourceBIM/BIMsurfer` | 425 | https://github.com/opensourceBIM/BIMsurfer |
| 78 | `openscad/openscad-wasm` | 405 | https://github.com/openscad/openscad-wasm |
| 79 | `Irev-Dev/cadhub` | 379 | https://github.com/Irev-Dev/cadhub |
| 80 | `bernhard-42/three-cad-viewer` | 373 | https://github.com/bernhard-42/three-cad-viewer |
| 81 | `thingraph/bim-viewer` | 322 | https://github.com/thingraph/bim-viewer |
| 82 | `kovacsv/occt-import-js` | 273 | https://github.com/kovacsv/occt-import-js |
| 83 | `wx-chevalier/ts-3d-model-viewer` | 222 | https://github.com/wx-chevalier/ts-3d-model-viewer |
| 84 | `bldrs-ai/Share` | 178 | https://github.com/bldrs-ai/Share |
| 85 | `zacharyfmarion/openscad-studio` | 171 | https://github.com/zacharyfmarion/openscad-studio |
| 86 | `ieskudero/three-dxf-viewer` | 157 | https://github.com/ieskudero/three-dxf-viewer |
| 87 | `yeicor-3d/yet-another-cad-viewer` | 132 | https://github.com/yeicor-3d/yet-another-cad-viewer |
| 88 | `mkeeter/erizo` | 126 | https://github.com/mkeeter/erizo |
| 89 | `seasick/openscad-web-gui` | 126 | https://github.com/seasick/openscad-web-gui |
| 90 | `castle-engine/castle-model-viewer` | 116 | https://github.com/castle-engine/castle-model-viewer |
| 91 | `Rufus31415/react-webgl-3d-viewer-demo` | 104 | https://github.com/Rufus31415/react-webgl-3d-viewer-demo |
| 92 | `gabotechs/react-stl-viewer` | 101 | https://github.com/gabotechs/react-stl-viewer |
| 93 | `earthtojake/cad-viewer` | 64 | https://github.com/earthtojake/cad-viewer |
| 94 | `arbaev/dxf-kit` | 24 | https://github.com/arbaev/dxf-kit |
| 95 | `looeee/multiformat-model-viewer` | 19 | https://github.com/looeee/multiformat-model-viewer |
| 96 | `Kompakkt/Viewer` | 17 | https://github.com/Kompakkt/Viewer |
| 97 | `flyfish-dev/dwf-viewer` | 9 | https://github.com/flyfish-dev/dwf-viewer |
| 98 | `flyfish-dev/cad-viewer` | 4 | https://github.com/flyfish-dev/cad-viewer |
| 99 | `tegos/cad-3d-viewer` | 3 | https://github.com/tegos/cad-3d-viewer |
| 100 | `CameronBrooks11/openscad-web` | 1 | https://github.com/CameronBrooks11/openscad-web |
