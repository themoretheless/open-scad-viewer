# Ещё 100 высокосигнальных репозиториев

Срез: **2026-07-14** (`2026-07-13T21:58:48Z` по GitHub API). Это вторая,
полностью непересекающаяся сотня относительно
[первого исследования](top-100-repositories.md). В финальном наборе ровно
100 публичных, неархивных репозиториев без forks.

Суммарно набор имел **1 039 011 stars**, медиана — **2 741,5**, диапазон —
**196–187 507**. Stars использованы как сигнал распространённости, а не как
доказательство качества или пригодности к прямому копированию.

## Срез отбора

Финальный TSV можно независимо проверить, но это не полностью воспроизводимый
benchmark-артефакт: точные строки запросов и сырой пул кандидатов не были
сохранены в репозитории. Зафиксированная процедура отбора:

1. Выполнено 20 из 24 тематических GitHub Search-запросов по CAD/CSG/B-rep,
   mesh processing, SDF, BVH/picking, WebGPU, LOD, editors и WASM; четыре
   запроса остановил secondary rate limit.
2. Поиск дал 201 уникальный репозиторий; после удаления первой сотни, forks
   и archived осталось 176.
3. Отдельно через REST проверены 56 высокозвёздных инфраструктурных
   репозиториев; один archived исключён.
4. Объединённый допустимый пул составил 231 репозиторий.
5. Финальная редакционная оценка учитывала прямую переносимость идеи,
   активность, уникальность относительно первой сотни и stars.
6. Каждый из финальных 100 повторно прочитан через GitHub REST. Assertions:
   100 уникальных имён, `fork=false`, `archived=false`, пересечений с первой
   сотней — 0.

Полный проверяемый snapshot находится в
[TSV](top-100-repositories-second.tsv). SHA-256:
`bf601d899277fa56fa95f7bb94be7aa717625404dd2fca631319a91668a6d11d`.

## Покрытие

| Направление | Репозиториев |
|---|---:|
| WebGPU и rendering | 19 |
| CAD, CSG и B-rep | 19 |
| Mesh и computational geometry | 18 |
| Technical visualization и LOD | 15 |
| Editor и language tooling | 12 |
| BVH, spatial queries и picking | 9 |
| Workers и WASM | 8 |

## 15 наиболее переносимых находок

1. [microsoft/monaco-editor](https://github.com/microsoft/monaco-editor) — SCAD language service, inline diagnostics и diff.
2. [tree-sitter/tree-sitter](https://github.com/tree-sitter/tree-sitter) — инкрементальное дерево синтаксиса.
3. [GoogleChromeLabs/comlink](https://github.com/GoogleChromeLabs/comlink) — типизированная граница Worker RPC.
4. [gfx-rs/wgpu](https://github.com/gfx-rs/wgpu) — явный lifecycle GPU-ресурсов и pipeline cache.
5. [visgl/deck.gl](https://github.com/visgl/deck.gl) — dirty layers, culling и GPU picking.
6. [xiangechen/chili3d](https://github.com/xiangechen/chili3d) — CAD-команды и изоляция OpenCascade-WASM.
7. [donalffons/opencascade.js](https://github.com/donalffons/opencascade.js) — выборочная упаковка B-rep ядра в WASM.
8. [gkjohnson/three-bvh-csg](https://github.com/gkjohnson/three-bvh-csg) — BVH-ускоренный mesh CSG.
9. [ricosjp/truck](https://github.com/ricosjp/truck) — разделение topology, geometry и tessellation.
10. [libfive/libfive](https://github.com/libfive/libfive) — адаптивный implicit preview.
11. [CGAL/cgal](https://github.com/CGAL/cgal) — exact predicates для вырожденных случаев.
12. [nmwsharp/geometry-central](https://github.com/nmwsharp/geometry-central) — halfedge topology и cached quantities.
13. [mapbox/martini](https://github.com/mapbox/martini) — error-driven adaptive LOD.
14. [excalidraw/excalidraw](https://github.com/excalidraw/excalidraw) — восстанавливаемая история и durable sessions.
15. [tldraw/tldraw](https://github.com/tldraw/tldraw) — явные state machines для инструментов и жестов.

`NOASSERTION` в TSV означает, что GitHub не вернул SPDX identifier. Перед
переносом кода лицензия каждого источника должна проверяться отдельно.

## Полная вторая сотня

| # | Repository | Stars | Area | Transferable idea |
|---:|---|---:|---|---|
| 1 | [`microsoft/vscode`](https://github.com/microsoft/vscode) | 187,507 | Editor architecture | Adopt command IDs, context-aware keybindings, undoable actions, and extension boundaries instead of wiring UI controls directly. |
| 2 | [`excalidraw/excalidraw`](https://github.com/excalidraw/excalidraw) | 127,382 | Editor state | Implement durable history, selection handles, command availability, autosave, and recoverable local sessions. |
| 3 | [`godotengine/godot`](https://github.com/godotengine/godot) | 114,036 | CAD editor UX | Use transaction-based undo, persistent tool state, gizmo snapping, and consistent viewport navigation conventions. |
| 4 | [`tldraw/tldraw`](https://github.com/tldraw/tldraw) | 48,736 | Tool state machines | Model select, orbit, measure, section, and annotation tools as explicit gesture-safe state machines. |
| 5 | [`bevyengine/bevy`](https://github.com/bevyengine/bevy) | 47,139 | Render scheduling | Separate extract, prepare, queue, and render phases so scene changes do not force full-frame rebuilding. |
| 6 | [`microsoft/monaco-editor`](https://github.com/microsoft/monaco-editor) | 46,332 | Code editor | Add SCAD tokenization, markers, hover, completion, minimap controls, diff support, and large-document virtualization. |
| 7 | [`emscripten-core/emscripten`](https://github.com/emscripten-core/emscripten) | 27,495 | WASM toolchain | Package native CAD kernels as modular async WASM with a deliberate virtual filesystem and memory-growth policy. |
| 8 | [`ajaxorg/ace`](https://github.com/ajaxorg/ace) | 27,142 | Code editor | Reuse worker-fed diagnostics, syntax modes, and viewport virtualization patterns for large SCAD sources. |
| 9 | [`tree-sitter/tree-sitter`](https://github.com/tree-sitter/tree-sitter) | 26,230 | Incremental parsing | Keep a reusable incremental syntax tree so edits only reparse changed ranges and diagnostics stay responsive. |
| 10 | [`plotly/plotly.js`](https://github.com/plotly/plotly.js) | 18,257 | Interaction and charts | Add linked hover, selection, and data-driven metric panels without coupling charts to the renderer. |
| 11 | [`AssemblyScript/assemblyscript`](https://github.com/AssemblyScript/assemblyscript) | 17,947 | WASM implementation | Evaluate a TypeScript-like WASM path for small deterministic geometry kernels that do not need a C++ runtime. |
| 12 | [`gfx-rs/wgpu`](https://github.com/gfx-rs/wgpu) | 17,572 | WebGPU architecture | Track resource lifetimes explicitly and cache bind-group and pipeline layouts across render passes. |
| 13 | [`bkaradzic/bgfx`](https://github.com/bkaradzic/bgfx) | 17,267 | Rendering architecture | Use transient frame buffers and backend-neutral draw submission concepts to reduce per-frame allocations. |
| 14 | [`bulletphysics/bullet3`](https://github.com/bulletphysics/bullet3) | 14,607 | BVH and queries | Study quantized BVHs and collision-shape separation for low-memory picking over many objects. |
| 15 | [`visgl/deck.gl`](https://github.com/visgl/deck.gl) | 14,312 | Technical visualization | Structure the viewer as dirty, independently updateable layers with GPU picking and partial buffer updates. |
| 16 | [`isl-org/Open3D`](https://github.com/isl-org/Open3D) | 13,782 | Geometry processing | Combine spatial indexes, repair, sampling, registration, and visualization behind consistent geometry types. |
| 17 | [`GoogleChromeLabs/comlink`](https://github.com/GoogleChromeLabs/comlink) | 12,744 | Workers and WASM | Expose geometry workers through typed RPC proxies while transferring large buffers rather than cloning them. |
| 18 | [`mapbox/mapbox-gl-js`](https://github.com/mapbox/mapbox-gl-js) | 12,328 | Viewport architecture | Borrow stable camera gestures, render-on-demand invalidation, frustum culling, and GPU feature picking. |
| 19 | [`PointCloudLibrary/pcl`](https://github.com/PointCloudLibrary/pcl) | 11,051 | Geometry processing | Build reusable filter, sampling, normal-estimation, registration, and segmentation stages. |
| 20 | [`jrouwe/JoltPhysics`](https://github.com/jrouwe/JoltPhysics) | 10,864 | Spatial queries | Adapt broad-phase layers and precise ray/shape casts for scalable picking and collision-aware manipulation. |
| 21 | [`floooh/sokol`](https://github.com/floooh/sokol) | 10,082 | Renderer robustness | Adopt small explicit resource handles, generation checks, and predictable device/context lifecycle management. |
| 22 | [`cocos/cocos-engine`](https://github.com/cocos/cocos-engine) | 9,693 | WebGPU engine | Borrow render-graph scheduling, reusable GPU resource pools, and feature-based WebGPU fallbacks. |
| 23 | [`playcanvas/supersplat`](https://github.com/playcanvas/supersplat) | 9,571 | 3D editor UX | Study high-density selection, transform gizmos, history, property panels, and progressive scene loading. |
| 24 | [`wasm-bindgen/wasm-bindgen`](https://github.com/wasm-bindgen/wasm-bindgen) | 9,090 | WASM bindings | Use typed JS/Rust bindings and borrowed typed-array views to minimize unsafe glue and copies. |
| 25 | [`WebAssembly/binaryen`](https://github.com/WebAssembly/binaryen) | 8,551 | WASM optimization | Validate and optimize release WASM artifacts in CI, including dead-code elimination and size regression checks. |
| 26 | [`earthtojake/text-to-cad`](https://github.com/earthtojake/text-to-cad) | 8,086 | CAD workflows | Extract structured CAD intent schemas and validation loops rather than sending unconstrained text to geometry code. |
| 27 | [`WebAssembly/wabt`](https://github.com/WebAssembly/wabt) | 8,062 | WASM diagnostics | Add wasm-validate and wasm2wat diagnostics to debug malformed or unexpectedly large geometry modules. |
| 28 | [`wasm-bindgen/wasm-pack`](https://github.com/wasm-bindgen/wasm-pack) | 7,237 | WASM packaging | Standardize browser-target packaging, TypeScript declaration generation, and browser tests for Rust geometry modules. |
| 29 | [`NVIDIA/warp`](https://github.com/NVIDIA/warp) | 6,862 | GPU compute | Express geometry algorithms as parallel kernels with CPU/GPU validation paths and deterministic test fixtures. |
| 30 | [`CGAL/cgal`](https://github.com/CGAL/cgal) | 5,976 | Robust geometry | Use exact-predicate/inexact-construction principles for intersections and topology decisions around degenerate input. |
| 31 | [`toji/gl-matrix`](https://github.com/toji/gl-matrix) | 5,676 | 3D math | Use allocation-free typed-array matrix and quaternion operations in camera, picking, and gizmo hot paths. |
| 32 | [`dimforge/rapier`](https://github.com/dimforge/rapier) | 5,520 | WASM spatial queries | Use its WASM-friendly query pipeline as a model for fast ray casts, shape casts, and bounding-volume updates. |
| 33 | [`gpuweb/gpuweb`](https://github.com/gpuweb/gpuweb) | 5,429 | WebGPU correctness | Turn WebGPU alignment, validation, error-scope, and canvas reconfiguration rules into renderer invariants and tests. |
| 34 | [`jagenjo/webglstudio.js`](https://github.com/jagenjo/webglstudio.js) | 5,324 | Browser 3D editor | Reuse dockable panels, scene tree, inspector, virtual files, and live code-to-viewport feedback. |
| 35 | [`patriciogonzalezvivo/glslViewer`](https://github.com/patriciogonzalezvivo/glslViewer) | 5,303 | Shader tooling | Compile shaders incrementally, preserve the previous valid program, and surface mapped diagnostics inline. |
| 36 | [`Orillusion/orillusion`](https://github.com/Orillusion/orillusion) | 5,191 | WebGPU engine | Use a WebGPU-first render graph, compute passes, clustered lighting, and explicit resource reuse patterns. |
| 37 | [`mosra/magnum`](https://github.com/mosra/magnum) | 5,179 | Mesh pipeline | Define stable importer, mesh-tools, and renderer interfaces so file decoding is independent from GPU upload. |
| 38 | [`libigl/libigl`](https://github.com/libigl/libigl) | 5,052 | Mesh processing | Add proven normals, connected-components, winding, remeshing, and mesh-quality routines behind a worker API. |
| 39 | [`xiangechen/chili3d`](https://github.com/xiangechen/chili3d) | 4,672 | Browser CAD | Study OpenCascade-WASM worker isolation, stable topology selection, snapping, commands, and feature history. |
| 40 | [`CloudCompare/CloudCompare`](https://github.com/CloudCompare/CloudCompare) | 4,621 | Point clouds | Stream chunks asynchronously and expose scalar fields, clipping boxes, registration, and downsampling as tools. |
| 41 | [`DiligentGraphics/DiligentEngine`](https://github.com/DiligentGraphics/DiligentEngine) | 4,361 | GPU resource model | Centralize pipeline resource layouts and state transitions instead of scattering WebGPU setup across tools. |
| 42 | [`pyvista/pyvista`](https://github.com/pyvista/pyvista) | 3,739 | Analysis UX | Expose concise high-level filters and metrics while preserving access to the underlying mesh arrays. |
| 43 | [`vispy/vispy`](https://github.com/vispy/vispy) | 3,578 | GPU visualization | Use a transform system and scene graph that keep scientific overlays independent from model geometry. |
| 44 | [`patriciogonzalezvivo/lygia`](https://github.com/patriciogonzalezvivo/lygia) | 3,381 | Shader library | Break WGSL lighting, grids, tone mapping, and utility code into tested portable modules. |
| 45 | [`AcademySoftwareFoundation/openvdb`](https://github.com/AcademySoftwareFoundation/openvdb) | 3,334 | Sparse SDF | Use hierarchical sparse level sets for large implicit models rather than allocating dense voxel volumes. |
| 46 | [`Kitware/VTK`](https://github.com/Kitware/VTK) | 3,175 | Technical visualization | Adopt data-object plus algorithm-pipeline separation, provenance, and reusable filters for analysis overlays. |
| 47 | [`PixarAnimationStudios/OpenSubdiv`](https://github.com/PixarAnimationStudios/OpenSubdiv) | 3,062 | Subdivision and LOD | Refine topology separately from evaluation and move patch evaluation to the GPU where useful. |
| 48 | [`antimatter15/splat`](https://github.com/antimatter15/splat) | 3,045 | Progressive rendering | Use progressive loading and depth sorting for responsive previews of very large point-based scenes. |
| 49 | [`greggman/twgl.js`](https://github.com/greggman/twgl.js) | 2,989 | WebGL architecture | Borrow attribute reflection and buffer construction patterns for a compact fallback renderer and test harness. |
| 50 | [`mourner/rbush`](https://github.com/mourner/rbush) | 2,758 | Spatial indexing | Maintain a dynamic bounding-box index for objects and annotations before expensive triangle tests. |
| 51 | [`RenderKit/embree`](https://github.com/RenderKit/embree) | 2,725 | BVH and picking | Use quality-aware BVH builders and coherent ray batches as the reference for fast precise selection. |
| 52 | [`napari/napari`](https://github.com/napari/napari) | 2,695 | Layer architecture | Treat mesh, axes, annotations, slices, and measurements as independently toggleable async layers. |
| 53 | [`google/s2geometry`](https://github.com/google/s2geometry) | 2,687 | Spatial robustness | Adopt robust predicates and hierarchical cell IDs as design references for stable spatial indexing. |
| 54 | [`software-mansion/TypeGPU`](https://github.com/software-mansion/TypeGPU) | 2,631 | Typed WebGPU | Generate type-safe WGSL bindings and validate buffer layouts at compile time instead of hand-matching structs. |
| 55 | [`mapbox/earcut`](https://github.com/mapbox/earcut) | 2,564 | Triangulation | Triangulate polygons with holes through a robust flattened-coordinate API and verify area preservation. |
| 56 | [`josdejong/workerpool`](https://github.com/josdejong/workerpool) | 2,305 | Worker scheduling | Add a bounded job queue, cancellation, timeouts, worker recycling, and explicit concurrency limits. |
| 57 | [`PyMesh/PyMesh`](https://github.com/PyMesh/PyMesh) | 2,042 | Mesh processing | Add validation, repair, boolean, remeshing, and attribute-transfer operations as worker-side stages. |
| 58 | [`dune3d/dune3d`](https://github.com/dune3d/dune3d) | 2,020 | Parametric CAD | Study constraint solving, sketch-to-feature workflow, stable references, and undoable CAD commands. |
| 59 | [`tpaviot/pythonocc-core`](https://github.com/tpaviot/pythonocc-core) | 1,933 | B-rep integration | Mirror its high-level topology traversal and tessellation API around a future OpenCascade-WASM backend. |
| 60 | [`gradientspace/geometry3Sharp`](https://github.com/gradientspace/geometry3Sharp) | 1,885 | Mesh algorithms | Port dynamic-mesh, AABB tree, remeshing, SDF, and hole-filling concepts into isolated geometry services. |
| 61 | [`evanw/csg.js`](https://github.com/evanw/csg.js) | 1,861 | CSG reference | Keep its compact BSP polygon-splitting model as a readable oracle for boolean regression fixtures. |
| 62 | [`gkjohnson/three-gpu-pathtracer`](https://github.com/gkjohnson/three-gpu-pathtracer) | 1,777 | Progressive rendering | Reset accumulation only when camera, material, or geometry versions change; reuse texture-atlas packing ideas. |
| 63 | [`microsoft/vscode-languageserver-node`](https://github.com/microsoft/vscode-languageserver-node) | 1,772 | Language protocol | Model parse and completion work as versioned, cancellable document requests with structured diagnostics. |
| 64 | [`xibyte/jsketcher`](https://github.com/xibyte/jsketcher) | 1,720 | Parametric CAD | Borrow feature graphs, constraint-driven sketches, workplanes, and stable selection IDs for future modeling tools. |
| 65 | [`LiangliangNan/Easy3D`](https://github.com/LiangliangNan/Easy3D) | 1,642 | Mesh processing | Use its separation of drawable buffers, camera tools, spatial queries, and geometry algorithms. |
| 66 | [`libfive/libfive`](https://github.com/libfive/libfive) | 1,638 | Implicit CAD | Evaluate interval arithmetic and adaptive contouring for exact-looking previews without uniform high-resolution meshing. |
| 67 | [`mourner/flatbush`](https://github.com/mourner/flatbush) | 1,591 | Spatial indexing | Store immutable spatial indexes in compact typed arrays for worker-to-main transfer and cache persistence. |
| 68 | [`fwilliams/point-cloud-utils`](https://github.com/fwilliams/point-cloud-utils) | 1,548 | Point and mesh utilities | Add voxel downsampling, sampling, distances, nearest neighbors, and metrics for imported geometry. |
| 69 | [`ricosjp/truck`](https://github.com/ricosjp/truck) | 1,509 | CAD kernel | Study Rust separation of topology, geometry, constraints, and tessellation for a browser-native B-rep path. |
| 70 | [`pmp-library/pmp-library`](https://github.com/pmp-library/pmp-library) | 1,493 | Mesh processing | Offer optional decimation, smoothing, subdivision, and curvature analysis as deterministic processing stages. |
| 71 | [`PDAL/PDAL`](https://github.com/PDAL/PDAL) | 1,390 | Geometry IO | Represent import and conversion as streaming, composable stages with explicit metadata and coordinate handling. |
| 72 | [`TypeFox/monaco-languageclient`](https://github.com/TypeFox/monaco-languageclient) | 1,358 | Editor and LSP | Bridge Monaco to a language worker with LSP diagnostics, completion, hover, and cancellable requests. |
| 73 | [`nmwsharp/geometry-central`](https://github.com/nmwsharp/geometry-central) | 1,323 | Mesh topology | Represent editable surfaces with halfedge connectivity and lazily cached geometric quantities. |
| 74 | [`google/dawn`](https://github.com/google/dawn) | 1,067 | WebGPU portability | Negotiate adapter capabilities up front and centralize device-loss and uncaptured-error recovery. |
| 75 | [`leap71/PicoGK`](https://github.com/leap71/PicoGK) | 1,012 | Implicit kernel | Evaluate compact voxel-field operations and robust lattice generation for complex engineering previews. |
| 76 | [`gkjohnson/three-bvh-csg`](https://github.com/gkjohnson/three-bvh-csg) | 924 | CSG acceleration | Reuse BVH-pruned triangle pairing, operation grouping, and attribute preservation ideas for mesh booleans. |
| 77 | [`donalffons/opencascade.js`](https://github.com/donalffons/opencascade.js) | 901 | OpenCascade WASM | Reuse modular builds, generated bindings, worker loading, and selected-symbol packaging for exact CAD import. |
| 78 | [`MeshInspector/MeshLib`](https://github.com/MeshInspector/MeshLib) | 793 | Mesh processing | Adopt fast repair, boolean, decimation, offset, and distance-tool workflows with progress and cancellation. |
| 79 | [`playcanvas/model-viewer`](https://github.com/playcanvas/model-viewer) | 695 | 3D viewer UX | Reuse asset progress, environment controls, camera framing, animation, and format-inspection patterns. |
| 80 | [`mapbox/martini`](https://github.com/mapbox/martini) | 654 | Adaptive LOD | Generate view-dependent triangulation from a precomputed error hierarchy instead of fixed-detail meshes. |
| 81 | [`kool-engine/kool`](https://github.com/kool-engine/kool) | 562 | Multiplatform WebGPU | Study its shader DSL and WebGPU resource model for strongly typed rendering modules. |
| 82 | [`verma/plasio`](https://github.com/verma/plasio) | 538 | Point-cloud streaming | Decode LAS/LAZ off-thread and render progressively from drag-and-drop without blocking the editor. |
| 83 | [`playcanvas/supersplat-viewer`](https://github.com/playcanvas/supersplat-viewer) | 510 | High-density viewer | Borrow compressed streaming, visibility culling, camera framing, and URL-configurable viewer state. |
| 84 | [`greggman/wgpu-matrix`](https://github.com/greggman/wgpu-matrix) | 470 | WebGPU math | Use typed-array destination parameters and WebGPU coordinate conventions consistently across camera and picking code. |
| 85 | [`OpenGeoscience/geojs`](https://github.com/OpenGeoscience/geojs) | 468 | Technical visualization | Implement extensible overlay layers, interaction events, and partial redraw for measurement and analysis tools. |
| 86 | [`JuliaGeometry/Meshes.jl`](https://github.com/JuliaGeometry/Meshes.jl) | 461 | Geometry abstractions | Separate domain, topology, coordinates, and algorithms through small composable geometry traits. |
| 87 | [`OpenGeometry-io/OpenGeometry`](https://github.com/OpenGeometry-io/OpenGeometry) | 451 | Web CAD kernel | Study stable web-facing topology IDs and browser-oriented CAD-kernel boundaries. |
| 88 | [`prs-eth/point2cad`](https://github.com/prs-eth/point2cad) | 446 | Reverse engineering | Fit primitives and segment imported scans so users can recover editable CAD-like structure. |
| 89 | [`BrutPitt/imGuIZMO.quat`](https://github.com/BrutPitt/imGuIZMO.quat) | 430 | Viewport gizmos | Use quaternion-safe axis, view-cube, rotation, translation, and scale interactions with consistent snapping. |
| 90 | [`ecto/vcad`](https://github.com/ecto/vcad) | 385 | Rust and WASM B-rep | Evaluate a small Rust/WASM B-rep backend with explicit topology ownership and browser bindings. |
| 91 | [`pmndrs/react-three-csg`](https://github.com/pmndrs/react-three-csg) | 366 | Declarative CSG | Cache subtrees by operation and transform versions so only affected boolean branches recompute. |
| 92 | [`twpride/three.cad`](https://github.com/twpride/three.cad) | 354 | Browser CAD | Study its sketch, constraint, CSG, React, Three.js, and WASM boundary as an end-to-end architecture. |
| 93 | [`mourner/robust-predicates`](https://github.com/mourner/robust-predicates) | 339 | Robust geometry | Use adaptive exact orientation and in-circle predicates for triangulation and degeneracy handling in JavaScript. |
| 94 | [`owensgroup/RXMesh`](https://github.com/owensgroup/RXMesh) | 320 | GPU mesh processing | Represent adjacency in GPU-friendly form and run parallel mesh kernels without repeated CPU round trips. |
| 95 | [`KeKsBoTer/web-splat`](https://github.com/KeKsBoTer/web-splat) | 288 | WebGPU compute | Borrow compute-based sorting, culling, indirect dispatch, and Rust-to-WebGPU integration patterns. |
| 96 | [`mkeeter/futureproof`](https://github.com/mkeeter/futureproof) | 248 | Live shader editor | Keep the previous good WGSL pipeline active while compiling edits and reporting errors without interrupting rendering. |
| 97 | [`timschmidt/csgrs`](https://github.com/timschmidt/csgrs) | 242 | Rust CSG | Compare robust Rust boolean and multi-representation design as a possible permissive WASM geometry service. |
| 98 | [`jupytercad/JupyterCAD`](https://github.com/jupytercad/JupyterCAD) | 229 | Collaborative CAD | Use a serializable shared model graph, command log, awareness state, and decoupled renderer. |
| 99 | [`EliCDavis/polyform`](https://github.com/EliCDavis/polyform) | 224 | Immutable mesh processing | Compose deterministic immutable geometry operations that are easy to cache, replay, and test. |
| 100 | [`bitbybit-dev/bitbybit`](https://github.com/bitbybit-dev/bitbybit) | 196 | Web geometry nodes | Expose geometry algorithms as backend-neutral typed nodes runnable against Three.js, Babylon.js, or workers. |
