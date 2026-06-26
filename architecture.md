# Architecture

This document describes the internal architecture of the OpenSCAD WebGPU Viewer and lists future directions.

## Current High-Level Architecture

The application is a client-only single-page Vue 3 + TypeScript + Vite project. There is no backend.

Key layers:

1. **Entry**
   - index.html + main.ts bootstraps Vue app.
   - App.vue holds the entire UI: bilingual toolbar, editor, canvas, stats and error display.

2. **Editor**
   - Plain `<textarea>` with basic Tab and Ctrl+Enter handling.
   - Code is stored in localStorage and triggers debounced render when auto mode is on.
   - Examples are embedded as template literals.

3. **Parser (openscadParser.ts)**
   - Hand-written lexer (`tokenize`) that produces a flat token stream.
   - Recursive-descent parser (`Parser` class) that recognizes a limited statement/call grammar.
   - Produces a tiny AST consisting only of `{type:'call', name, args, children}`.
   - No real expression parser — only literals, simple vectors and idents.
   - Evaluator walks the AST and emits flat `MeshData[]`.
   - Mesh generators are pure JS (cube, sphere, cylinder with basic normals).
   - CSG (difference, intersection) is visual only: subtracted bodies are emitted with translucent red color. No actual boolean mesh operations are performed.
   - Transforms are accumulated as 4x4 matrices (row-major Float32Array).
   - Color is inherited or overridden via the `color()` module.

4. **Math (math3d.ts)**
   - Minimal row-major matrix and vector helpers.
   - Hand-written `multiply`, `invert`, `transpose`, `lookAt`, `perspective`, rotate/translate/scale.
   - No external math dependency.

5. **Renderer (webgpuRenderer.ts)**
   - WebGPU only. Requires Chrome 113+ / Edge / Firefox Nightly.
   - Two pipelines:
     - Opaque mesh (Phong + simple specular + back lighting).
     - Transparent mesh (alpha blend, depth write off).
     - Line pipeline for grid + axis.
   - Per-object uniform buffer containing model + normal matrix + color.
   - Scene uniform: VP matrix, eye position, light dir, ambient.
   - Orbit camera implemented in JS (yaw/pitch/dist + target tx/ty/tz).
   - Pointer events for rotate/pan/zoom.
   - Grid is generated once on CPU as colored lines.
   - Auto-fit on setMeshes computes bbox and repositions camera.
   - Double-buffered render loop via requestAnimationFrame.
   - Destruction cleans up all GPU resources.

6. **Data Flow**
   ```
   source text
      -> tokenize
      -> parseAll -> ASTNode[]
      -> evalNodes (with accumulating transform + color)
      -> MeshData[] (vertices interleaved pos+norm, indices, color, transform)
      -> WebGPURenderer.setMeshes
      -> write GPU buffers + per-object bindgroups
      -> render loop uses current camera state
   ```

7. **State**
   - All in Vue refs + a few module-scoped renderer/camera variables.
   - Theme and language persisted in localStorage.
   - No formal state management library.

## Known Limitations (Current Design)

- Parser is extremely limited — real OpenSCAD programs with variables, loops, modules, expressions will mostly fail or be ignored.
- No source locations on AST nodes → poor error messages.
- CSG is not real; the resulting mesh is not correct for export or measurements.
- All geometry lives on CPU then uploaded. No incremental updates.
- Editor is a textarea — no highlighting, no intellisense, bad UX for code.
- Renderer is simple forward Phong. No shadows, no advanced materials, limited transparency.
- Single monolithic App.vue.
- No tests, no CI, no type-level guarantees on AST or MeshData.
- Everything is synchronous; long models will jank the UI.
- No persistence beyond localStorage, no import/export of models or projects.
- No accessibility story, limited mobile support.

## Critical Flaws and Technical Debt

The project has many serious implementation problems. The full lists (initial Top 50 + another 200 additional problems = 250 total documented issues) live in [recommendation.md](recommendation.md).

Highlights from the 250 documented issues:
- Monolithic architecture with no separation.
- Extremely weak parser with almost no expressions or language features + silent corruptions (now partially fixed).
- Fake CSG that lies about geometry.
- Hand-rolled untested matrix math.
- Zero tests, zero CI, raw `any` everywhere.
- Main-thread blocking on every keystroke.
- No device recovery, no proper error reporting (partially improved).
- Editor and UX are at 1995 textarea level.
- Hundreds of granular correctness, perf, and maintainability smells (full list in recommendation.md).

Treat recommendation.md as the current source of truth for what is broken.

## 200 Ideas, Suggestions and Improvements

Below is a numbered list of 200 concrete ideas. They range from small polish to ambitious architectural changes. They are not strictly ordered by priority.

1. Replace the custom minimal math3d with gl-matrix or similar for robustness and performance.
2. Move tokenizer and parser into a dedicated reusable @openscad/parser npm package.
3. Implement a full expression parser and evaluator supporting arithmetic, comparisons, logic, and ternary operators.
4. Add support for variable assignments and lexical scoping.
5. Implement `for` loops over ranges and explicit lists.
6. Implement `if` / `else` conditionals at both statement and expression level.
7. Add user-defined modules with named parameters and default values.
8. Add user-defined functions that return values usable in expressions.
9. Fully parse and respect `$fn`, `$fa`, `$fs` special variables.
10. Support list comprehensions.
11. Implement `hull()` with a proper 3D convex hull algorithm.
12. Implement `minkowski()` (at least a usable approximation for common cases).
13. Add full `linear_extrude` with twist, scale, slices and twist.
14. Add `rotate_extrude` with angle and convexity parameters.
15. Support true 2D primitives: `square`, `circle`, `polygon` with holes.
16. Add `polyhedron` primitive accepting points and faces.
17. Implement `text()` using canvas or a font library for 3D text.
18. Add `surface()` heightmap from image data.
19. Support `import()` for STL (binary and ASCII), OBJ, and 3MF.
20. Implement `projection(cut=...)` generating 2D outlines.
21. Add 2D `offset()` (round, chamfer, delta).
22. Replace fake CSG with real boolean operations (library or custom BSP / manifold).
23. Improve difference visualization to show actual resulting geometry when possible.
24. Store source ranges on every AST node for accurate error reporting.
25. Produce high-quality error messages that include line and column.
26. Implement `echo()`, `warn()` and `assert()` writing to an in-app console panel.
27. Correctly handle animation special variables `$t`, `$vpr`, `$vpt`, `$vpd`.
28. Implement `let()` expressions and statement blocks for local bindings.
29. Add recursion depth guard for user modules.
30. Parse module and function declarations even when not immediately executed.
31. Support `include` and `use` with URL fetching and simple caching.
32. Bundle a small browser-compatible standard library of useful modules.
33. Add vertex deduplication and index optimization in mesh generation.
34. Preserve per-face or per-vertex colors when color() appears inside children.
35. Make `multmatrix` handle full arbitrary 4x4 matrices correctly.
36. Improve cone/cylinder normal calculation for sloped sides.
37. Offer alternative sphere tessellation (icosahedral) for more uniform triangles.
38. Cache generated primitive geometry keyed by parameters.
39. Introduce a clean MeshBuilder fluent API for internal generators.
40. Respect the `render()` module as a quality hint.
41. Implement `$children` and `child()` inside custom modules.
42. Add the `resize()` primitive.
43. Make `mirror()` also correctly invert normals.
44. Propagate and composite alpha values more consistently.
45. Change internal representation from flat meshes to a lightweight scene graph.
46. Attach original module path / names and source info to each MeshData.
47. Add distinct "Preview" (low $fn) vs "Render" (high quality) modes.
48. Make the parser more permissive and continue after unknown statements.
49. Implement vector utility functions: `len`, `concat`, `slice`, etc.
50. Expose common math functions (sin, cos, sqrt, min, max, round...) inside expressions.
51. Replace the textarea with Monaco Editor or CodeMirror 6.
52. Create or adapt a proper OpenSCAD syntax grammar for the editor.
53. Show live syntax and semantic errors as editor decorations.
54. Provide context-aware autocompletion for modules and parameters.
55. Show hover documentation and parameter signatures.
56. Offer a rich snippet library for common modeling patterns.
57. Implement a document formatter / pretty-printer for OpenSCAD source.
58. Add smart bracket matching, auto-close, and selection expansion.
59. Display a minimap in the editor.
60. Support multiple open documents with an in-memory virtual filesystem.
61. Add a project/files sidebar for multi-file workflows.
62. Accept drag-and-drop of .scad and geometry files.
63. Persist open documents and offer "Download .scad" and "Save as".
64. Auto-generate a Customizer panel from top-level variables and parameters.
65. Keep code and customizer widgets in two-way sync.
66. Add powerful find & replace (with regex and scoped options).
67. Maintain an in-app edit history with time travel / undo beyond browser back.
68. Record and export camera animation paths.
69. Show a dedicated output console for echo/warn/assert.
70. Display extended model statistics: volume, surface area, bounding box, center of mass.
71. Add an interactive measurement tool (point-to-point distance, angle, diameter).
72. Compute and display volume and center-of-mass estimates.
73. Add a 3D View Cube widget for quick orientation changes.
74. Provide one-click preset views (Top, Bottom, Front, Back, Left, Right, Isometric) + hotkeys.
75. Toggle between perspective and orthographic projection.
76. Add "Zoom to Fit" and "Zoom to Selection" commands.
77. Improve camera controls: middle-mouse pan, alt-orbit, wheel-zooms-to-cursor, inertia.
78. Double-click in viewport to focus camera on surface point.
79. Add optional camera damping / inertia for smoother orbiting.
80. Offer grid-snapping options for the camera target.
81. Allow loading reference images as background planes.
82. Add multiple configurable lights and simple IBL / environment maps.
83. Implement shadow mapping (cascaded or single).
84. Add SSAO (screen space ambient occlusion) as post-process.
85. Support combined shaded + wireframe render modes.
86. Render sharp model edges as overlay lines.
87. Add movable section / clipping plane with capped fill.
88. Improve transparent rendering quality (order-independent or depth peeling).
89. Add debug visualization modes (normals, UV, curvature, rainbow).
90. Add a post-processing stack (FXAA, SMAA, bloom, vignette).
91. Export high-resolution screenshots with supersampling and transparent background.
92. Export binary/ASCII STL (basic geometry, later with color via 3MF/AMF).
93. Export glTF 2.0 / GLB with materials for downstream pipelines.
94. Export OBJ + MTL and support for 3MF.
95. Allow importing external STL/OBJ into the current scene.
96. "Send to Slicer" button that writes STL and optionally launches local slicer.
97. Preview simple support structures (tree or grid) before export.
98. Simple layer-by-layer simulation / sliced view.
99. Exploded view mode with per-part separation sliders.
100. Timeline scrubber controlling `$t` for animated models.
101. Canvas capture to WebM / GIF of rotating or animated models.
102. Named camera bookmarks that can be restored instantly.
103. On-screen rulers and dimension callouts.
104. Raycast selection of faces / volumes from the 3D view.
105. Synchronized tree view of the CSG / module hierarchy.
106. Properties inspector for the currently selected object or subtree.
107. Canvas context menu with common actions (isolate, hide, focus, measure).
108. In-app keyboard shortcut reference / cheat sheet modal.
109. Global command palette (Ctrl/Cmd+K) for all actions.
110. Fully resizable panels with persisted sizes and layout presets.
111. Robust mobile touch support: single finger orbit, two finger pan+zoom.
112. Proper pinch-to-zoom + two-finger rotation gestures.
113. Responsive UI that gracefully collapses on phones and small tablets.
114. Turn the viewer into a PWA with installability and offline shell.
115. Implement a service worker that precaches the core app and examples.
116. Show a smart "Add to Home Screen" banner.
117. Provide native desktop builds using Tauri (preferred) or Electron.
118. Build a VS Code extension that uses the same parser + renderer for inline .scad previews.
119. Publish the parser and the renderer as separate, well-typed npm packages.
120. Export a framework-agnostic web component `<scad-viewer>`.
121. Improve parser error recovery so it can still render what it can.
122. Comprehensive tokenizer unit tests covering numbers, strings, comments, all operators.
123. Parser unit tests for every supported primitive, transform, and CSG operator.
124. Golden/snapshot tests against all built-in examples and many edge cases.
125. Property-based testing that generates random (but valid-ish) OpenSCAD fragments.
126. Visual regression tests via Playwright that diff canvas screenshots.
127. Performance benchmarks (parse time, mesh gen, upload, frame time) tracked over time.
128. GitHub Actions CI pipeline: lint, typecheck, test, build, deploy demo.
129. Code coverage reporting with meaningful thresholds.
130. Pre-commit hooks enforcing formatting and lint via husky + lint-staged.
131. Replace hand-written matrix math with a tested dependency and add matrix tests.
132. Introduce Pinia (or similar) for future complex UI state.
133. Refactor App.vue into focused components: Editor, Viewport, Toolbar, Console, Stats.
134. Extract all numeric constants, colors, speeds into a single constants module.
135. Add a feature flag system for experimental functionality.
136. Expand theming to multiple palettes plus user CSS variable overrides.
137. Full i18n system with extraction tooling and more languages.
138. Strong accessibility: full keyboard navigation, ARIA, focus management.
139. Live region announcements for render completion, errors, and measurements.
140. Respect `prefers-reduced-motion` throughout the UI and camera.
141. Show determinate progress for long parses and large mesh uploads.
142. Provide a graceful fallback when WebGPU is unavailable (CPU or helpful message).
143. Optional Three.js or Babylon fallback renderer for broader browser support.
144. Real-time FPS, frame time, and parse time overlay (toggleable).
145. GPU memory and JS heap usage indicators.
146. Offload parsing + mesh generation to a Web Worker with transferable buffers.
147. Incremental / differential evaluation that only re-processes changed subtrees.
148. Subtree result caching keyed by normalized arguments.
149. Introduce a visitor-based AST walker for future analyses and transforms.
150. Create a rendering abstraction layer so multiple backends can be plugged in.
151. Adopt a single canonical math library everywhere and remove math3d.ts duplication.
152. Extract camera + input handling into a reusable, testable OrbitController class.
153. Make the color palette, grid style, and lighting fully configurable at runtime.
154. Create a hierarchy of typed error classes with recovery hints.
155. Extract all user-facing strings into a proper i18n catalog.
156. Eliminate every `any` in the codebase and maximize type safety.
157. Adopt a fast formatter/linter (Biome or ESLint+Prettier) with zero-tolerance CI.
158. Code-split the heavy editor so initial load stays tiny.
159. Keep source maps in production builds behind a flag for easier debugging.
160. Surface parse duration, triangle count, draw call count, and buffer sizes in UI.
161. Add strict GPU resource leak detection helpers in development.
162. Use Vitest for fast unit and integration tests.
163. Add a large suite of parser regression fixtures.
164. Snapshot the exact vertex/index output for example files.
165. Investigate readback + CPU comparison for renderer visual tests.
166. Playwright + pixelmatch or jest-image-snapshot for visual diffing.
167. End-to-end flows: load, type, Ctrl+Enter, rotate, export STL, change theme.
168. Track performance budgets and fail CI on regression.
169. Consider mutation testing for critical parser paths.
170. Run axe-core accessibility audits on every PR.
171. Write CONTRIBUTING.md, issue templates, and pull request template.
172. Add CODE_OF_CONDUCT and security policy.
173. Automate CHANGELOG generation from conventional commits or PR titles.
174. Adopt a release process (changesets, release-please, etc.).
175. Enable Dependabot + CodeQL + dependency review.
176. Produce high-quality animated GIFs and screenshots for documentation.
177. Write a living architecture decision record (ADR) collection.
178. Host an always-up-to-date public demo with short shareable links.
179. Create a public gallery of example models with thumbnails generated on the fly.
180. Build an in-app interactive "Learn OpenSCAD" tutorial mode.
181. Prepare video tutorial scripts and caption templates.
182. Write technical blog posts about the parser and WebGPU renderer.
183. Set up a community space (Discord / Matrix / GitHub Discussions).
184. Accept and showcase user-contributed example models.
185. Add "Report an issue with this model" that pre-fills a GitHub issue.
186. Automatic GitHub Pages deployment on main.
187. Full PWA with offline examples and last-edited document.
188. Desktop app via Tauri with native file system access and printing support.
189. Lighter alternative desktop build using Wails or pure webview if desired.
190. VS Code extension using the web viewer in a Webview panel.
191. Standalone reusable web component published to npm.
192. Publish the pure parser as a zero-dependency ESM package.
193. Offer a small Docker image for self-hosting the static site.
194. GitHub Action that renders .scad files to STL/PNG as build artifacts.
195. Bridge to Blender, FreeCAD or other tools via importers/exporters.
196. Real-time collaborative editing using CRDTs (Yjs) for both code and camera.
197. AI-assisted modeling: natural language prompt generates or edits SCAD code.
198. Chat-style iterative refinement inside the app ("make the wall thicker").
199. Integrate an external or WASM slicer and display layer previews.
200. AR placement mode using WebXR with correct scale to real world objects.

## Potential Large Initiatives (beyond individual items)

- Full parity effort toward real OpenSCAD language semantics.
- Production-grade real-time CSG (Manifold or similar compiled to WASM).
- Collaborative cloud workspace with persistence and sharing.
- Plugin / extension marketplace for custom generators and importers.
- Integration with physical 3D printers and farm management tools.

This list will be revisited and prioritized as the project evolves.
