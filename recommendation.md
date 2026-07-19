# 500 Suggestions, Improvements, Problems & Defects — Catalog v2 (July 2026)

The living backlog: everything known to be done poorly, incorrectly, or not at all — plus concrete improvement ideas — ranked into 10 domains of 50. Compiled after the SOLID/DRY decomposition pass (see [ARCHITECTURE.md](./ARCHITECTURE.md) for the current module structure), superseding both the original list in this file and complementing [ISSUES.md](./ISSUES.md) (the older 500-item snapshot) and [TOP-50-ISSUES.md](./TOP-50-ISSUES.md) (the severity shortlist).

Legend: items are present-tense problems or actionable suggestions. `[fixed]` marks items resolved on branch `claude/top-issues-architecture-sync-00p2q9` during the July 2026 remediation passes — kept for the record; everything unmarked is open.

Synced with: [README.md](./README.md) · [ARCHITECTURE.md](./ARCHITECTURE.md) · [RECOMMENDATIONS.md](./RECOMMENDATIONS.md) · [TOP-50-ISSUES.md](./TOP-50-ISSUES.md)

---

## I. Architecture & Decomposition (1–50)

1. `App.vue` is still ~13.3k lines after extraction passes; target < 1,000 (thin shell composing components).
2. [fixed] i18n dictionaries (~1.6k lines) inlined in App.vue → extracted to `src/i18n/index.ts`.
3. [fixed] EDITOR_THEMES/BUILT_IN_PRESETS/SHORTCUT_PRESETS inlined → extracted to `src/config/index.ts`.
4. [fixed] 41 mesh generators trapped in the parser monolith → extracted to `src/parser/geometry.ts` (pure, Worker-ready).
5. [fixed] WGSL shaders inlined in the renderer with a triplicated Scene struct → `src/renderer/shaders.ts`, struct defined once.
6. Extract `useTabs()` composable: tab CRUD, rename, pin, color, drag-reorder (~600 App.vue lines).
7. Extract `usePreferences()` composable wrapping every `scad-pref-*` key through SafeStorage.
8. Extract `useToast()` composable (toasts array + addToast/dismissToast + the storage-failure hook).
9. Extract `useHistory()` composable (undo/redo stacks, history snapshots, restore).
10. Extract `useExportImport()` composable (STL/OBJ/3MF/PNG/ZIP + STL import), lazy-imported.
11. Extract `EditorPanel.vue`, `Toolbar.vue`, `TabBar.vue`, `ConsolePanel.vue`, `Minimap.vue` components.
12. Extract `ViewportCanvas.vue`, `OrientationGizmo.vue`, `AxisLabels.vue`, `CompassOverlay.vue` components.
13. Extract each of the 12 modals into `src/components/modals/` with a shared `AppModal.vue` (focus trap once, not 12×).
14. Introduce a store layer (Pinia or module-singleton composables) instead of 237 refs sharing one setup scope.
15. Parser: split evaluator out of `openscadParser.ts` into `src/parser/evaluator.ts` (evalNode/evalNodes + context).
16. Parser: split tokenizer + Parser class into `src/parser/tokenizer.ts` and `src/parser/parser.ts`.
17. Parser: replace 6 module-globals (`cIdx`, `_resolveFile`, `_profiling`, `_profileEntries`, `_profileDepth`, `_source`) with an `EvalContext` object threaded through evaluation — blocks Worker migration and makes parses non-reentrant.
18. Parser: replace the untyped `args: Record<string, any>` bag with a discriminated-union AST.
19. Renderer: split the ~300-line `render()` into RenderPass classes (mesh/line/outline/sky/reflection/shadow).
20. Renderer: `renderScaled()` is a diverged copy of `render()` — screenshots miss reflection/shadow passes; unify into one parameterized path.
21. Renderer: replace `_pad0`/`_pad1` flag smuggling with a typed FeatureFlags uniform field.
22. Move parsing+mesh generation into a Web Worker (geometry module is now pure and ready); main thread only uploads buffers.
23. `doRender()` still runs parse→mesh→upload synchronously; large models freeze typing.
24. Three separate `watch(code, …)` watchers with no defined ordering — consolidate into one orchestrator with named effects.
25. `MeshData` type lives in `openscadParser.ts` while `geometry.ts` type-imports it back (circular type edge) — move to `src/parser/types.ts`.
26. `cssColor()` still sits in the parser file; move to geometry or a color module.
27. The `sig()` session-restore signature builds full-code strings per compare; hash instead (djb2/fnv) to cut GC on every backup tick.
28. `matchesBinding` moved to config but keybinding *capture* logic stays in App.vue — extract `useKeyboard()`.
29. Five different popover implementations (viewport menu, context menus, dropdowns, tooltips, quick switcher) — one `usePopover()` with shared Esc/outside-click dismissal.
30. Favicon generation, document-title updates, and `<link>` insertion happen inside component code — extract a head-management util.
31. `reinitializeWebGPU()` duplicates chunks of `onMounted` init; extract shared `initRenderer()`.
32. Use/include file-resolution callback duplicated between doRender and doRenderAllTabs — extract one resolver.
33. Print-window harness duplicated between printCode/printShortcuts/spec-sheet — extract `openPrintWindow(html)`.
34. The `showCopied`-flag+timeout pattern repeats ~6× — extract `useTransientFlag()`.
35. ~25 one-line persistence watchers — replace with a `persistRef(key, ref)` helper over SafeStorage.
36. Console panel logic (entries, filters, drag-resize) is interleaved with render logic — extract `useConsole()`.
37. Breadcrumbs/AST-outline update walks the full AST on every caret move — move into the parse result and cache.
38. Statistics/weight/cost estimator recompute from meshes in the component — move to a `useModelStats()` composable with memoization.
39. Example gallery data embedded in App.vue — move to `src/examples/` with per-example files.
40. SCAD reference documentation strings embedded in App.vue — move to data files under `src/reference/`.
41. Command palette command list (~40 inline entries) — declare commands as data with ids, titles, handlers in `src/commands.ts`.
42. Animation ($t) subsystem mixed into App.vue — extract `useAnimation()`.
43. Ghost/comparison mode logic — extract `useGhostCompare()`.
44. Measurement tool state machine — extract `useMeasure()`.
45. Annotation overlay logic — extract `useAnnotations()`.
46. `zipExport.ts` is a generic ZIP writer living beside domain exporters — move to `src/lib/zip.ts`.
47. Introduce `src/lib/` for framework-free utilities (escapeHtml, safeParse, debounce) currently re-declared in App.vue.
48. Debounce is hand-rolled 5× with different handle names — one `debounce()` util with cancel.
49. Renderer camera state (`yaw/pitch/dist/tx/ty/tz`) is public mutable fields poked by App.vue — encapsulate behind methods/events.
50. Define module dependency rules (lint-enforced): `lib → services → parser/renderer → i18n/config → App` with no upward imports.

## II. Parser & Language (51–100)

51. String tokenizer never decodes escapes — `"\n"` stays two characters; unterminated strings run past EOF.
52. Number lexer accepts `1e` (no exponent digits) and silently parseFloats to 1.
53. Unterminated `/*` block comment scan overshoots EOF.
54. `ch <= ' '` treats NUL/DEL control chars as whitespace.
55. `||`/`&&` constant-fold to booleans; OpenSCAD semantics return the operand value.
56. `/0` folded to 0 at parse time; OpenSCAD yields inf/nan.
57. `%0` returns 0 instead of nan.
58. Ternary folded at parse time discards the unevaluated branch's side semantics for arrays/strings.
59. Only `PI` is modeled; `$fa`/`$fs`/`$t`/`$preview` are ignored by the evaluator core.
60. `min`/`max` with a single vector argument (`max([1,2,3])`) yields NaN.
61. Unknown primary token: parser advances and returns 0, silently desyncing instead of erroring.
62. List comprehensions support only `[for(x=range) body]` — no `if`, no nesting, no `let`, no multi-generator.
63. Range parse only triggers on `:` after the first element; `[a, b:c]` mis-parses.
64. `*`/`#`/`%`/`!` statement modifiers are discarded; `*` (disable) still renders its subtree.
65. Unresolved variables evaluate to their own name as a string — surprising in arithmetic contexts.
66. `expandRange` silently truncates at 10,000 entries; float-accumulated step causes off-by-one counts.
67. Empty vector is truthy (`!![]`); OpenSCAD treats it as false.
68. `undef` is not modeled at all.
69. Recursive user modules have no depth guard — `module a(){a();}` overflows the JS stack.
70. `children(i)` index argument unsupported; `$children` missing.
71. `assert()` lacks the message argument and never reports source position.
72. `echo` output loses the expression labels OpenSCAD prints (`ECHO: a = 5`).
73. No error positions: line/column exist in tokens (`p`) but never surface in messages.
74. Parser stops at the first syntax error instead of recovering and reporting all errors.
75. `let()` inside expressions (not statements) unsupported.
76. String functions incomplete: `str()`, `chr()`, `ord()`, `len()` on strings have gaps.
77. `search()` builtin missing entirely.
78. `lookup()` interpolation table function missing.
79. Vector math builtins missing: `norm()`, `cross()` in expressions.
80. `rands()` returns wrong shape (scalar vs vector list).
81. `log()` is documented base-10 in OpenSCAD [fixed] but `exp`/`ln` coverage still untested.
82. `import()`/`surface()` file loading is a no-op without an error to the user.
83. `include`/`use` resolve only across open tabs; no URL fetch option (with CORS note) for shared libraries.
84. No support for `intersection_for`.
85. `offset()` on 2D children uses radial-from-centroid — wrong for concave shapes; ignores `delta`/`chamfer` params.
86. `projection(cut=true)` keep-logic inverted/garbled for solids straddling z=0.
87. `linear_extrude` reconstructs the 2D profile from a 3D mesh (`extractFlatProfile`) — fails on shapes with holes.
88. `extractFlatProfile`'s 0.5-thickness threshold misclassifies thin solids as flat profiles.
89. Boundary walk in profile extraction picks the first neighbor without orientation — self-crossing polygons result.
90. `rotate_extrude` sorts profile points by atan2 — wrong for non-star-shaped profiles.
91. `rotate_extrude` silently mirrors negative-X profiles; OpenSCAD errors instead.
92. 360° `rotate_extrude` leaves an unwelded seam (not watertight).
93. Named-argument detection can misclassify `f(a=b==c)`.
94. Module-definition single-child path mishandles `module m();`.
95. Add a formatter (`scad-fmt`) — comment-preserving pretty printer over the AST.
96. Add an AST visitor API so tooling (outline, lint, rename) stops re-walking ad hoc.
97. Property-based fuzz tests for the tokenizer/parser (fast random near-valid SCAD).
98. Golden-file tests: every example in the gallery parses to a stable mesh hash.
99. Publish the parser as a standalone npm package once de-globalized.
100. Long-term: adopt the OpenSCAD grammar via tree-sitter for editor tooling parity.

## III. Geometry & CSG (101–150)

101. `difference()` is a translucent overlay, not a boolean — exported STL still contains subtrahends (unprintable). The #1 correctness gap.
102. `intersection()` renders children semi-transparent; no intersection volume computed.
103. `union()` concatenates meshes without merging — internal faces remain (non-manifold export).
104. `minkowski()` is a pass-through.
105. Integrate Manifold (manifold-3d WASM) as the boolean kernel behind a feature flag — the single highest-value geometry change; keeps the current path as fallback.
106. [fixed] `torus`/`donut` with `r1=0` divided by zero → infinite loop; now guarded and capped.
107. [fixed] `gear` teeth unbounded → 100k-point profile into O(n³) earClip; now capped at 500.
108. [fixed] `linear_extrude` twist could derive ~200k slices; now capped at 512.
109. `earClip` is O(n³) worst case with no vertex cap — SVG-traced polygons freeze the tab.
110. Replace earClip with a robust triangulator (earcut port — battle-tested, handles holes).
111. `convexHull3D` horizon detection: `isHorizon` is initialized true and never set false — hulls of 5+ points can self-intersect.
112. `convexHull3D` is O(n³); incremental hull with adjacency would be O(n log n).
113. Near-coplanar initial tetrahedron has no epsilon check — sliver tetrahedra produce warped hulls.
114. Sphere poles: duplicated pole vertices with independently computed normals → visible seam under smoothing.
115. Sphere lat/long tessellation over-denses poles; icosphere option would give uniform triangles.
116. Cylinder `h=0` degenerates with forced (0,0,1) side normals.
117. Cone (r2=0) emits a degenerate apex ring of coincident vertices.
118. `pipe` inner-wall winding doesn't match the stated inward normals; `r2<r1` self-intersects without warning.
119. Torus inner-ring normals likely inverted.
120. Helix tube radius hard-coded as `pitch*0.15`, ignoring any wire-radius argument.
121. Helix with huge `pitch` puts vertices millions of units out — Float32 precision jitter; clamp or warn.
122. Bezier 5-point control list drops point 4; segment boundaries duplicate points with wrong seam tangents.
123. Sweep with a single-point path indexes `tangents[-1]` — crash.
124. Sweep generates no end caps — open tube.
125. Gear profile is a tip/valley star, not an involute; `valleyR` can go ≤0 for small teeth inverting the profile.
126. Thread profile is half-rectified `max(0,sin)` — not a usable V-thread; derivative math has canceling constants.
127. Knurl top-cap radius mismatches side wall for non-integer vertical counts — visible crack.
128. chamferCube falls back to convex hull (approximate + expensive) and carries abandoned half-written faceDefs code.
129. Loft resamples profiles by index, ignoring edge lengths; recomputes centroid per vertex (O(n²)).
130. Surface heightmap is an open sheet (not a solid) with downward-vs-computed normal mismatch.
131. Polyhedron fan triangulation assumes convex planar faces and never bounds-checks indices.
132. Hemisphere is built on the Y axis; OpenSCAD convention is +Z.
133. `axisAngle` multiply order is inconsistent with the Euler rotate branch.
134. Mirror with a non-axis vector just scales — normals not properly reflected.
135. `multmatrix` never validates the matrix is affine.
136. No vertex welding/dedup pass — cube emits 24 verts, sphere duplicates ring starts; memory + upload waste.
137. No mesh validation (manifold check, degenerate-triangle count) surfaced to the user.
138. Compute and display volume/surface area/center of mass (accurate once real CSG lands).
139. No `resize()` primitive support.
140. `center=true` unsupported on several primitives that accept it in OpenSCAD.
141. `d=` diameter aliases inconsistently handled across sphere/cylinder/circle.
142. Negative sizes/radii neither clamped nor diagnosed.
143. `$fa`/`$fs` should participate in facet-count resolution, not just `$fn`.
144. Per-primitive tessellation quality switch (preview vs export $fn) — halving exists for preview only.
145. 2D subsystem is fake: circle/square are extruded 3D shims, so 2D booleans can't ever be right; model true 2D paths with holes.
146. `polygon(paths=…)` with holes unsupported (single outline only).
147. Text: real font loading (opentype.js) instead of the 5×7 bitmap font; `halign`/`valign`/`font` params.
148. DXF/SVG import for 2D profiles.
149. Geometry unit tests per generator (closed-mesh invariant: every edge shared by exactly 2 triangles).
150. Benchmark harness tracking triangles/sec per generator to catch perf regressions.

## IV. Renderer & WebGPU (151–200)

151. `perspective`/`ortho` map z to OpenGL [−1,1] instead of WebGPU [0,1] — half the depth buffer wasted, z-fighting on coplanar detail.
152. Adopt reversed-z (near=1, far=0, GREATER compare) — the modern standard for depth precision.
153. `lookAt` [fixed] guards degenerate eye==center and parallel-up; remaining: normalize `up` input.
154. `invert()` silently returns identity on singular matrices — callers can't tell; add an epsilon-fail signal.
155. Per-mesh normal matrix recomputed with full 4×4 invert+transpose every frame — cache until transform changes; affine 3×3 suffices.
156. Reflection pass allocates mirror matrix + multiply/invert/transpose per mesh per frame — hoist the constant, reuse scratch arrays.
157. Ground-shadow pass allocates flattenY per mesh per frame.
158. `new Float32Array(52)` scene scratch allocated every frame — reuse one.
159. Per-mesh 144-byte uniform buffers + bind groups — replace with one dynamic-offset uniform buffer (webgpu-utils pattern).
160. `cullMode:'none'` globally doubles fragment work; meshes are (mostly) closed — enable back-face culling once winding is trusted.
161. Specular term not gated on `dot(N,L)>0` — highlights leak onto unlit faces.
162. Gooch branch overwrites the fog/SSAO-composited color — enabling fog+Gooch loses fog entirely.
163. Lighting computed in sRGB space without gamma correction — linearize, then encode.
164. Flat-shading via `cross(dpdx,dpdy)` flips at grazing angles/MSAA edges; a real per-face normal path would be stable.
165. Fog divides by zero when fogNear==fogFar.
166. Clip/section `discard` executes even when disabled — specialize pipelines or use uniform branch hints.
167. "Section box" only clips lower bounds — it's 3 half-spaces, not a box; implement the upper bounds.
168. Outline inflation is fixed world-space 0.3 — not screen-constant; scale by distance.
169. Sky drawn after grid with `depthCompare: always` — ordering wasteful; draw first or depth-test.
170. MSAA `count:4` hard-coded with no adapter limit check or off switch.
171. `createView()` per frame for swapchain+MSAA targets — cache the MSAA view.
172. Depth texture recreated on every resize event even when size is unchanged.
173. Device-lost: loop stops [fixed], but recovery never restarts axis/gizmo/annotation RAF loops (App side).
174. Wheel zoom [fixed] normalizes deltaMode and clamps per-event factor.
175. Add zoom-to-cursor (dolly toward pointer ray) — standard in every serious viewer.
176. Add double-click to focus/frame the picked point.
177. Pan basis ignores pitch — wrong plane at steep angles; vertical pan uses world-Y instead of camera-up.
178. Auto-rotate and inertia are framerate-dependent (no delta-time normalization).
179. Fly mode never translates the camera — the toggle is decorative.
180. Ortho zoom is coupled to perspective FOV; decouple with a proper ortho scale.
181. far=dist*10/near=0.1 gives a huge depth ratio; fit near/far to scene bounds.
182. Direct `renderer.fov=` assignment bypasses clamping setters.
183. Multiple MSAA resolves per frame overwrite each other in the reflection path.
184. Shadow alpha-blend darkens overlapping triangles into bands — use stencil or a max-blend.
185. Hidden-line mode writes/restores per-mesh color UBs every frame — double uniform traffic; use a pipeline constant instead.
186. Wireframe buffer duplicates every edge (no dedup) and is CPU-baked per mesh change.
187. Edge-overlay buffer builds a string-keyed Set per edge — slow; use sorted-pair numeric keys.
188. `smoothNormals` quantizes with `(x*1000|0)` — overflows beyond ±2.1M and truncates instead of rounding.
189. Screenshot path relies on preserved drawing buffer semantics — capture straight after an explicit render into an offscreen target.
190. Timestamp queries for GPU profiling (feature-gated) instead of the meaningless FPS-under-on-demand metric.
191. Frustum culling for meshes and the grid.
192. Instanced rendering for repeated sub-assemblies (radial_array/linear_array/grid outputs).
193. Order-independent transparency (weighted-blended) instead of unsorted two-pass alpha.
194. Optional render scale (resolution slider) for heavy scenes on hi-DPI screens.
195. Pipeline creation is 5 copy-pasted blocks — a small builder would collapse them.
196. GMesh fields `vb/ib/ic/ub/bg/transp` are cryptic — rename for the next reader.
197. `setRenderMode` casts an arbitrary string — validate against the union.
198. Scene uniform byte offsets are raw literals in fillSceneData — derive from a schema (webgpu-utils) to kill the stale-index class of bugs.
199. Color override keyed by mesh array position desyncs on reorder — key by stable mesh id.
200. Canvas resize: debounced observer races the per-frame resize() call — single source of truth.

## V. Editor & UX (201–250)

201. The editor is a `<textarea>` with an overlay highlighter — replace with CodeMirror 6 (undo model, decorations, folding, find/replace, IME, a11y come free; ~150 kB, lazy-loadable).
202. Hand-rolled syntax highlighting re-tokenizes the whole document per keystroke.
203. Undo: custom stack + native undo fight each other; suppress flag is racy across nextTick.
204. Find matches go stale when code changes while the find panel is open.
205. No multi-cursor/column selection.
206. Autocomplete is prefix-only over a static list; no argument hints from the reference data.
207. Hover docs exist but aren't keyboard-invokable.
208. Error squiggle underlines only the first error line; parser recovery (item 74) would enable multi-error display.
209. Go-to-definition for user modules within/between tabs.
210. Rename-symbol for variables/modules (needs AST positions — item 73).
211. Tab size setting exists but tab-vs-spaces choice doesn't.
212. Bracket auto-close ignores selection wrapping (select text + `(` should wrap).
213. Comment toggle doesn't preserve cursor column.
214. Share links use raw base64 — lz-string compression would triple capacity within the 1 MB cap.
215. Share links lose tab name and camera pose — encode a small envelope {name, code, camera}.
216. Drag & drop a `.scad` file onto the editor to open it.
217. `Ctrl+S` should download/save instead of the browser dialog.
218. Recent-files list stores full code copies — store digests + lazy content.
219. Session restore prompt shows no diff preview of what would be restored.
220. Welcome modal crowds onboarding + changelog + CTAs — split.
221. The Render menu is ~21 flat toggles + a select — group into labeled sections (Display/Effects/Camera/Debug) with headers.
222. Toolbar is ~21 buttons in one overflow-prone row — group with separators + overflow menu ("More…").
223. Three export entry points with inconsistent options — one export dialog with format/scale/units, shared by all.
224. Icon-only stats bar segments need visible labels on hover focus, not title-only.
225. Simple mode is a v-show scatter — curate an actual reduced layout.
226. Advanced mode is the implied default for first-timers — flip the default, offer "show everything".
227. Multiple popovers can be open simultaneously and overlap.
228. Popups anchor to fixed pixels with no viewport-edge collision handling.
229. Context menu isn't keyboard-openable (Shift+F10/menu key) and has no roles.
230. Command palette lacks fuzzy matching and recent-command ranking.
231. Keyboard shortcut editor exists, but conflicts aren't detected.
232. `?` button is dual-purpose (what's-new vs shortcuts) — separate.
233. Tip banner has 3 same-weight controls — unclear hierarchy.
234. Canvas hint text shows permanently — auto-hide after first interaction.
235. Animation timeline appears even when the code has no `$t`.
236. Ghost/animation overlays compete for the same bottom strip.
237. No visual "dirty" indicator when auto-render is off and code changed.
238. Tri-count warning toasts on every render past threshold — once per crossing [check] and offer "don't warn again".
239. Batch render results modal [fixed: shows per-tab errors] — add re-run failed only.
240. No progress indicator during long parses — the Worker migration (item 22) enables a real progress/cancel UI.
241. Examples gallery lacks thumbnails and search.
242. Parametric customizer: auto-generate sliders from top-level variables (OpenSCAD Customizer parity) — the single most-requested CAD-playground feature.
243. View cube with clickable faces/edges/corners instead of gizmo dots only.
244. Named camera bookmarks exist; add view transitions between them for turntable-style demos.
245. Measurement: snap to vertices/edges/face centers; show angle and diameter, not just distance.
246. Print-bed presets (Prusa/Bambu/Ender sizes) for the build-volume overlay.
247. Overhang visualization (>45°) for print planning.
248. Auto-orient suggestion for printability.
249. Model comparison mode: side-by-side or blink two tabs.
250. Zen mode hides the exit affordance — keep a subtle persistent escape hint.

## VI. Accessibility (251–300)

251. 12 modals: no focus trap — Tab escapes behind the backdrop.
252. Focus is never moved into a dialog on open nor restored to the trigger on close.
253. Toasts have no `aria-live` region — screen readers miss all feedback. (Error box has role=alert [fixed earlier].)
254. Batch/loading states lack `role="status"`/`aria-busy`.
255. [fixed] Main canvas has role=img+label; minimap canvas now labeled too — but minimap is still keyboard-unreachable (no tabindex/keyboard nav).
256. [fixed] Range sliders (FOV/explode/clip/section XYZ) now have aria-labels; remaining sliders in preferences/watermark need the same.
257. [fixed] `role="main"` landmark added; header/aside landmarks still missing.
258. Theme option swatches are clickable divs — no role/tabindex/keyboard.
259. Recent-file rows are clickable divs — keyboard-unreachable.
260. Object-tree rows: no `role="treeitem"`, toggles lack `aria-expanded`, eye/color controls lack accessible names.
261. Timeline dots are div@click — unreachable by keyboard.
262. Gizmo axes/dots are SVG click targets with no keyboard equivalent — provide the Numpad views as the documented alternative and label the gizmo group.
263. Breadcrumb segments are clickable spans without role/tabindex.
264. Context menus lack `role="menu"`/`menuitem` and arrow-key navigation.
265. Viewport dropdown uses menuitems but no roving tabindex — mouse-only.
266. Language toggle doesn't announce the newly active language (`aria-live` or aria-pressed pattern).
267. Preference labels not associated via for/id — clicking text doesn't focus controls.
268. Comparison slider has no label or value readout for AT.
269. Modal close `×` buttons: several lack aria-label.
270. Tip/help "?" buttons expose only `title` — no accessible name.
271. Disabled buttons are dimmed but still focusable/clickable (`.btn-disabled` isn't `disabled`).
272. `v-html` highlighter output is read as markup soup by AT — `aria-hidden` the overlay and expose the textarea value.
273. Tab order jumps: zen-exit FAB and late-DOM overlays appear in unpredictable sequence.
274. Tour spotlight is visual-only — target element not conveyed to AT.
275. Print windows drop theme/contrast entirely.
276. `--text-dim: #888` on dark is ~3.5:1 — fails AA for body text; bump to #9a9aa2.
277. Light-theme `--text-dim: #777` is borderline 4.48:1.
278. Ghost-diff +/− encoded by color only — add glyphs.
279. `.profile-slow` rows flagged by color only.
280. Compass N marker is hardcoded red only — add the letter form/shape distinction it already has, verify contrast.
281. `prefers-contrast: more` / forced-colors mode unhandled.
282. [fixed] `prefers-reduced-motion` now honored globally.
283. Focus ring [fixed]; but ~12 `outline: none` rules remain — audit each has a visible replacement.
284. Tab-name inline edit input has outline:none with no replacement focus style.
285. Keyboard-only camera controls: arrow keys orbit is present; document it and add zoom/pan keys.
286. Skip-link to jump from header to editor/canvas.
287. `<html lang>` is static "ru" — sync with the active locale on toggle.
288. RTL: 100+ physical left/right CSS props; adopt logical properties (margin-inline-start …) for future ar/he.
289. Modal Escape handling inconsistent — some close, some don't.
290. Roving focus for the example gallery grid.
291. Touch targets: most fixed [44px media query], but tree eye/color swatches remain ~16px on touch.
292. Error messages should link the error line (click → caret jump) — exists visually? make it a real focusable link.
293. Announce render completion ("Rendered: N triangles") politely for AT.
294. High-contrast editor theme preset.
295. Screen-reader-friendly stats summary behind a visually-hidden live region (throttled).
296. `aria-keyshortcuts` on buttons that have bindings (Render: Ctrl+Enter).
297. Dialog titles: `aria-labelledby` pointing at the visible heading (several use aria-label duplicating text).
298. Minimap click-to-navigate needs a keyboard path (Ctrl+G exists — cross-reference it in the tooltip).
299. Automated axe-core smoke test in CI on the built index.html.
300. An ACCESSIBILITY.md documenting the support level and known gaps.

## VII. Performance (301–350)

301. Parsing on main thread (see 22/23) — the umbrella perf item.
302. Full reparse on every debounced keystroke — incremental/again-if-changed hash gate first (cheap win: skip if code hash unchanged).
303. Mesh regeneration ignores caching — same subtree → same mesh; hash-cons geometry by (node, args, $fn).
304. `saveTabs` stringifies all tabs on every debounced flush — serialize only the dirty tab into a keyed store.
305. Session backup every 30s re-serializes everything even when nothing changed — skip via dirty flag/hash.
306. History snapshot writes read-modify-write the full history store per burst.
307. `currentTabHistory` re-parses localStorage on unrelated reactivity — cache by tab id + invalidation.
308. Two always-on setInterval polls (stats, camera info) tick while panels closed [partially fixed] — verify both gate.
309. Compass/axis/gizmo RAF loops run even when overlays hidden.
310. Minimap renders on 3 different cadences (50ms drag, 300ms debounce, immediate) — unify.
311. `computeFolds` runs un-debounced on every keystroke.
312. `updateBreadcrumbs` walks the AST on every caret move — throttle + reuse the outline index (37).
313. `updateSelectionInfo` substrings/splits the whole document per selection change.
314. Console: every entry schedules a nextTick scroll; batch.
315. Console array re-slices wholesale per log — ring buffer.
316. codeStats watcher recomputes lines/words per keystroke — debounce with the render watcher.
317. `$t` regex scans the source twice per render even without `$t` — single indexOf gate.
318. Fast-preview `$fn` halving regex only matches integer literals — misses `$fn=fn` var forms; also double regex pass.
319. Vertex data uploads recreate every GPU buffer on every parse — diff by mesh identity, update transforms only when geometry unchanged.
320. Grid rebuilt into a fresh buffer on theme change — keep static, tint via uniform.
321. Sphere generator recomputes sin/cos per ring — precompute tables (O(seg) vs O(seg²) trig).
322. Loft centroid recomputation per vertex (O(n²)) — hoist.
323. convexHull centroid recomputed inside nested loop.
324. Text generator allocates per-glyph arrays — pool.
325. `multiply()` allocates a new Float32Array per call in hot paths — out-param variants for the render loop.
326. translate/rotate/scale helpers build identity + full multiply for trivial ops — fused constructors.
327. transpose(invert(m)) chains allocate 3 temporaries per mesh per frame (see 155).
328. Bounds computation runs over all vertices per render — cache per-mesh local bounds, transform 8 corners.
329. Wireframe/edge buffers rebuilt on any mesh change even if topology identical.
330. Screenshot at 4× renders everything twice (see renderScaled unification 20).
331. Turntable ZIP export blocks the UI between frames — yield via rAF loop with progress.
332. STL export allocates the entire buffer contiguously — stream in chunks to the Blob.
333. OBJ export builds a giant string array — write into a growing buffer.
334. 3MF ZIP CRC table recomputed per file — memoize.
335. STL import creates a mesh per 5,000 triangles — batch into fewer, larger meshes.
336. localStorage JSON.parse of tabs at module scope delays first paint — defer to idle.
337. i18n module loads all 4 locales eagerly (~1.6k lines) — split per-locale JSON with dynamic import.
338. Reference/help data loads with the main chunk — lazy-load on first open.
339. Export modules could be `import()`-ed on first use (manualChunks already isolates them, but they're statically imported).
340. Minimap canvas re-renders whole document — render visible window ± overscan.
341. Occurrence highlighting rescans the document per cursor move — index word positions once per parse.
342. Long-line documents (minified SCAD) freeze the highlighter — line-length guard, fall back to plain text.
343. Auto-render debounce is fixed 400ms — adapt: longer for heavier last-parse times.
344. Adaptive quality: drop $fn under interaction (orbit), restore on idle.
345. `requestRender` dirty flag races RAF — single scheduling gate.
346. Inertia decay is per-frame multiplicative without dt — normalize.
347. Idle callback to pre-parse the next-likely tab (hover-intent on tab bar).
348. Track and cap total GPU buffer bytes; warn before OOM instead of crashing.
349. Performance budget test in CI: parse+mesh time for the largest example must stay under threshold.
350. Report Web Vitals-style startup metrics (time-to-first-render) in the perf panel.

## VIII. Reliability, Storage & Security (351–400)

351. [fixed] SafeStorage module: validated reads + quota-safe writes with user-visible failure toasts.
352. [fixed] flushSaveTabs/saveUserPresets/saveRecentFiles/saveBookmarks/saveSessionBackup/saveSnapshots wired through SafeStorage with retry/trim.
353. [fixed] History snapshot stale-closure (wrote into the wrong tab after a quick switch).
354. [fixed] shortcutPreset/uiDensity enum-validated on read; keydown guarded against missing preset.
355. [fixed] NaN-guarded numeric prefs (fontSize/tabSize/delay/console-height/editor-width/cost-per-kg).
356. ~80 direct localStorage call sites remain — migrate all to SafeStorage (mechanical, high value).
357. No schema version key — add `scad-schema: N` + migration runner before any format change.
358. Keys are inconsistently named — central registry const with typed keys.
359. Tabs parse has no shape validation — validate {id,name,code}[] before trusting (feeds editor + exporters).
360. Presets parse unvalidated — crash in applyPreset on tampered storage.
361. customThemes validated as array [fixed earlier] but element shape unchecked.
362. SessionBackup `as` cast; `.map` throws if tabs is null despite the Array check upstream — tighten.
363. `scad-last-version` parseInt unvalidated.
364. Onboarding flag set in two places can diverge.
365. document.write print paths [escaped earlier] — replace with Blob URL + print stylesheet, removing the sink class entirely.
366. Screenshot dataURL is interpolated into spec-sheet HTML — it's self-generated, but move to DOM assignment anyway.
367. Share-URL payload: 1 MB cap [fixed earlier]; add integrity (length+hash) so truncated links fail loudly.
368. Add CSP meta (default-src 'self'; no inline script beyond the SW registration — move it to a file).
369. SW registration is an inline script in index.html — CSP-hostile; externalize.
370. Service worker: cache name version bump is manual — inject build hash at build time.
371. SW: no cache size cap for runtime caches — prune LRU.
372. SW: navigations network-first [fixed earlier]; add offline fallback page test.
373. FileReader paths: openFile [fixed]; STL import reader still lacks onerror.
374. copyCanvasToClipboard: unhandled promise rejection when clipboard permission denied — catch + fallback to download.
375. reinitializeWebGPU is fire-and-forget — surface failure to the user with retry.
376. 22 empty catch blocks remain — each needs at least a console.warn with context.
377. `catch (e: any) { e.message }` crashes on non-Error throws — normalize via `toErrorMessage(e)` util.
378. Renderer init failure collapses to boolean — keep the reason for the error screen.
379. GPU errors during upload surface as "parse error" — separate the pipeline stages' error channels.
380. No error boundary around the app — one thrown watcher kills everything; add app-level errorCaptured + recovery UI.
381. Crash reporting opt-in (console ring buffer + last code hash, copy-to-clipboard).
382. `window.onerror`/unhandledrejection hooks feeding the console panel.
383. Tab IDs use Date.now().toString(36) — collisions possible across rapid creates; use crypto.randomUUID().
384. Session-restore signature uses \x00/\x01 joins [fixed from raw bytes] — replace with a hash for robustness (see 27).
385. Autosave conflict: two browser tabs of the app fight over scad-tabs — detect via storage events, warn, or adopt BroadcastChannel coordination.
386. Import size guard: STL byte-size validated [fixed earlier], but no user-facing size ceiling on text file open (100 MB .scad freezes).
387. Paste guard: pasting multi-MB text into the editor has no confirmation.
388. History retention policy is count-based only — add age+size budget.
389. Snapshots store JPEG dataURLs at fixed 0.6 quality — make quality/size explicit, show storage usage meter in prefs.
390. Storage usage panel: navigator.storage.estimate() surfaced to the user.
391. localStorage → IndexedDB for bulky payloads (histories, snapshots, bookmarks): higher quotas, async, structured.
392. Persist preference for "ask before restore" vs auto-restore.
393. Undo/redo stacks are unbounded in memory — cap with size accounting.
394. Console retains 50 entries but the ID counter grows monotonically — fine, but timestamps use locale-naive formatting (see i18n 424).
395. Numeric inputs (cost, density, bed size) accept negative/absurd values — clamp at the input layer too, not just read.
396. Renderer `destroy()` order: device.destroy while in-flight passes exist — await queue idle first.
397. Multiple App mounts (HMR) can leak the module-level renderer listener set — idempotent init guard.
398. beforeunload flush [exists] — also flush on visibilitychange: hidden (mobile tab kill).
399. Fuzz the share-URL decoder with truncated/corrupt payloads in tests.
400. Threat-model doc: what's trusted (own storage) vs untrusted (URL hash, imported files) — encode in code comments at each boundary.

## IX. Build, Tests, CI & Hygiene (401–450)

401. No ESLint — add flat config (typescript-eslint + eslint-plugin-vue), `npm run lint`, CI step.
402. No Prettier/format script — or adopt ESLint stylistic; either, but one.
403. tsconfig lacks noUnusedLocals/noUnusedParameters — dead code accumulates silently.
404. tsconfig lacks noImplicitReturns/noFallthroughCasesInSwitch/noUncheckedIndexedAccess.
405. skipLibCheck hides dependency type breakage — document why or drop.
406. allowJs true serves no purpose now — drop to stop stale-JS pickup class of bugs for good.
407. isolatedModules not set (esbuild semantics divergence risk).
408. [fixed] engines field; packageManager field still absent (npm@x pin for reproducibility).
409. [fixed] LICENSE file (README claimed MIT with no license text).
410. No CONTRIBUTING.md (how to add a primitive end-to-end: geometry.ts + evaluator case + docs + example + test).
411. No CHANGELOG.md — adopt keep-a-changelog, cut 0.2.0 after this branch merges.
412. Version stuck at 0.1.0 through ~250 features.
413. No issue/PR templates (.github/ISSUE_TEMPLATE with bug/feature forms).
414. CI runs typecheck+test+build [fixed earlier] — add lint (401) and axe smoke (299).
415. CI: no Node version matrix (18/20/22).
416. CI: no npm cache… (actually setup-node cache: npm present) — add artifact upload of dist for preview.
417. Deploy preview per PR (GitHub Pages/Netlify) — reviewers see the app, not just code.
418. Coverage reporting with a parser-coverage floor (start 50%).
419. Visual regression: Playwright + WebGPU screenshot of 3 examples, pixel-diff gate (Chromium supports WebGPU headless).
420. Test the exporters against golden binary fixtures (STL/OBJ/3MF byte-exact).
421. Test SafeStorage quota paths with a mocked throwing localStorage.
422. Unit tests for math3d (invert·M≈I property test, lookAt orthonormality) — zero exist.
423. i18n completeness test: every key present in en; ru/de/zh missing-key report artifact.
424. Locale-aware date formatting test (currently toLocaleString with no locale arg in history panel).
425. `.vscode/` launch configs reference chrome debugging port 5173 — parameterize; tasks.json has empty problemMatcher.
426. Source maps for production builds (opt-in) for field debugging.
427. Bundle-size budget check in CI (fail on +10% main chunk).
428. dead-code: rightScrollTop and other suspected-unused refs — knip or ts-prune sweep.
429. Rename `recommendation.md` (this file) vs RECOMMENDATIONS.md confusion — merge or clearly scope both (this = catalog, that = phased plan).
430. Commit hooks (husky + lint-staged) once lint exists.
431. Conventional commits or at least a subject-line convention documented.
432. Release process: tags + GitHub Releases with dist zip.
433. Renovate/Dependabot for dependency updates.
434. npm audit in CI (currently 2 low-severity findings unreviewed).
435. Document the dual test locations (src/services/*.test.ts vs tests/) — consolidate into tests/.
436. Add vitest --coverage script wired to CI.
437. Type the i18n keys: generate a union from the en dictionary, `t(key: I18nKey)`.
438. Extract magic numbers scattered in App.vue (debounce durations, limits, sizes) into src/config/limits.ts.
439. `simpleMode`, `zenMode`, `embedMode` interactions are undocumented — state matrix in docs.
440. ARCHITECTURE.md module diagram [updated this pass] — keep a docs-sync checklist in PR template.
441. JSDoc the public API of parser/renderer/exporters (typedoc-able).
442. `npm run check` = typecheck+lint+test aggregate.
443. Storybook (or Histoire) once components exist (Phase 1) for isolated UI dev.
444. Node 18 EOL is 2025-04 — bump engines floor to 20 and CI accordingly.
445. Lockfile-only renovation policy documented (no floating carets on runtime deps — currently only Vue).
446. Pre-commit secret scan (gitleaks) — hygiene for a public repo.
447. `dist/` size tracking over time (bundlewatch badge).
448. README badges: CI status, license, PRs-welcome.
449. Repo topics/description for discoverability once public.
450. Mirror the examples as .scad files under examples/ so they're testable and diffable instead of embedded strings.

## X. Product & Ecosystem (451–500)

451. Real boolean CSG (see 105) — unlocks: correct exports, volume, mass, printability; the #1 product gap vs openscad-wasm-based playgrounds.
452. OpenSCAD-WASM compatibility mode: run the real engine in a Worker as a "high fidelity" toggle, keep the fast custom engine for live preview — best of both.
453. Customizer panel (see 242) — parity with OpenSCAD's killer feature.
454. Library ecosystem: bundle BOSL2-style helpers (rounded_cube, screws) as importable stdlib tabs.
455. Import STL [exists] → add OBJ/3MF/AMF import.
456. Export glTF/GLB (with per-mesh colors) — the web-native interchange format.
457. Export STEP via occt-import-js (long-term, heavy).
458. PNG export with transparent background option.
459. Turntable GIF/WebM export (MediaRecorder) alongside frame ZIP.
460. Shareable embeds: read-only iframe mode with `?embed` [exists] — document + add height auto-messaging.
461. PWA: install prompt UX, offline example pack, file handling API (`.scad` file association).
462. File System Access API: open/save real files with permission persistence (Chromium).
463. VS Code extension embedding the viewer for .scad previews (webview reusing the same modules — enabled by the decomposition).
464. Headless render CLI (node + dawn/webgpu or software raster fallback) for CI thumbnails of models.
465. GitHub Action: render .scad → STL/PNG artifacts on push (community distribution vector).
466. Gallery of community models (opt-in, static JSON to start, no backend).
467. "Open in OpenSCAD" file download with proper mime + instructions.
468. Deep links to docs per keyword (hover → F1 → cheatsheet.openscad.org anchor).
469. Interactive tutorial series (tour exists; add guided modeling lessons with checkpoints).
470. AI assist: prompt → SCAD starter (behind an API-key setting, no backend of our own).
471. Diff view between history snapshots (side-by-side with mesh diff highlight).
472. A/B render compare slider between two tabs (comparison slider exists for screenshots — extend to live scenes).
473. Multi-file projects: folders, use/include across a virtual FS, import/export as ZIP.
474. Git-friendly project export (each tab → file + manifest).
475. Print-time/filament estimator refinement: infill %, wall count parameters (currently naive volume-based).
476. Slicer hand-off: "Download STL + open Cura/PrusaSlicer" protocol links where registered.
477. Units support (mm/inch display toggle; SCAD is unitless-mm by convention).
478. Annotation export into the spec sheet PDF.
479. BOM extraction from named modules/colors.
480. Exploded-view animation recording for assembly instructions.
481. Section-view screenshots with hatching for engineering drawings.
482. 2-view/4-view orthographic layout export (front/top/side + iso).
483. Keyboard-only modeling mode documentation page.
484. Mobile: bottom-sheet UI for panels, larger touch targets [partially done], pinch gestures [exist] — do a real phone usability pass.
485. Tablet + pencil: two-finger orbit, pencil for measure/annotate.
486. Theme marketplace: shareable theme JSON export/import [custom themes exist — add import/export].
487. Locale contributions: extract per-locale JSON (item 337) + community translation guide; es/fr/ja as candidates.
488. Public roadmap page generated from this catalog (buckets by domain, checkboxes).
489. Telemetry opt-in (privacy-respecting counts: feature usage, parse times) to prioritize the next 500.
490. Feature flags module for experimental subsystems (real CSG, worker, OIT) with a hidden dev panel.
491. Session replay for bug reports: record code edits + camera (local only, user-exported).
492. Benchmark page: run standard models, report parse/mesh/render times, compare across releases.
493. Model health report: manifoldness, degenerate tris, open edges, bbox — one click after render.
494. Educational mode: display the AST/CSG tree live as a teaching aid (object tree exists — add CSG-op annotations).
495. Competitive analysis doc vs openscad-playground/JSCAD/CascadeStudio kept in-repo and refreshed quarterly.
496. Community: Discussions enabled, "good first issue" labels seeded from this catalog's S-effort items.
497. Sponsorware/donations link if maintenance continues (FUNDING.yml).
498. Security policy (SECURITY.md) with reporting contact.
499. Versioned docs site (VitePress) building from these markdown files.
500. The north star: full-fidelity OpenSCAD in the browser with modern UX — every item above serves it; re-rank this catalog quarterly and retire `[fixed]` rows into the changelog.

---

## How to use this catalog

- **Right now**: the top of [TOP-50-ISSUES.md](./TOP-50-ISSUES.md) is the severity-ranked subset; [RECOMMENDATIONS.md](./RECOMMENDATIONS.md) maps work into phases with exit criteria.
- **Quick wins** (S-effort, high value): 27, 105-flag-off prototype, 110, 214, 221, 222, 276, 287, 302, 317, 356, 373, 374, 383, 401, 403, 408-413, 423, 437, 450.
- **The three big rocks**: real CSG (105/451-452), Web Worker parsing (22), CodeMirror editor (201). Everything else compounds around them.
