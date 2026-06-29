# Recommendations

This document catalogs the most serious problems in the current implementation of the OpenSCAD WebGPU Viewer.

The goal is to be brutally honest about what is done poorly or incorrectly so that future work can be prioritized effectively.

## Top 50 Things Done Poorly or Incorrectly

1. **Monolithic App.vue** — Over 400 lines containing state management, rendering logic, examples, event handlers, translations, and styles in a single file. Zero separation of concerns.

2. **Hardcoded examples inside the component** — All four example models live as massive JS template literals embedded directly in App.vue. They are not external assets or test fixtures.

3. **Excessive use of `any`** — Parser is full of `Record<string, any>`, `parseValue(): any`, and casts like `(localStorage... as any)`. Type safety is mostly abandoned in the core logic.

4. **Useless error handling** — `catch (e: any) { error.value = e.message || String(e) }`. Errors lose stack, context, and source location. No recovery.

5. **Silent character skipping in tokenizer** — At the end of tokenize: `i++` with no error or warning when an unexpected character is seen. Invalid input is silently corrupted.

6. **No expression support whatsoever** — The parser cannot evaluate arithmetic, variables, function calls in arguments, or even simple expressions. `1+2` or `r*2` will not work.

7. **Crude assignment hack** — When seeing `foo = ...` the parser emits a fake `__assign` node that is completely ignored later. Variables are not supported at all.

8. **Zero source location tracking** — Tokens have a crude `p` (position), but AST nodes carry no line/column/range info. Error messages are useless.

9. **Global mutable color counter** — `let cIdx = 0` at module scope + `nextC()` mutates state on every parse. Repeated parses give different colors. Pure side effect.

10. **Fake CSG implementation** — `difference()` and `intersection()` only change color of the subtracted parts to red (or lower alpha). The actual mesh geometry is never modified. The result is visually misleading and export-broken.

11. **Hand-written matrix math** — 110+ lines of manually transcribed `invert()` with high risk of transcription error. No tests. Every other 3D project uses a library for this reason.

12. **invert() fails silently** — On near-zero determinant it just returns identity(). Callers have no idea their transform is now garbage.

13. **Massive vertex duplication** — Cube, sphere, cylinder all emit 4 vertices per face (or worse) with no index sharing or deduplication. Memory and upload waste.

14. **No geometry processing** — Zero vertex welding, normal averaging, index optimization, or manifold repair.

15. **Hardcoded magic constants everywhere** — Grid size 200, step 10, specular power 40, auto-fit multiplier 1.8, rotation speed 0.005, debounce 400ms, near/far planes, etc. No constants file.

16. **Renderer exposes internal camera state** — `yaw`, `pitch`, `dist`, `tx` etc. are public mutable fields on WebGPURenderer. External code can (and will) break invariants.

17. **Expensive autoFit every time** — Full O(n) scan over every vertex on every `setMeshes`. Happens even for tiny edits.

18. **No device-lost handling** — WebGPU context or device loss is never handled. App will die silently after GPU reset or tab sleep.

19. **Resize logic is racy** — `resize()` is only called from the render loop. Rapid window changes or initial layout can leave wrong canvas size.

20. **Primitive manual debounce** — `let debounce; if (debounce) clearTimeout...` repeated in watch. No use of lodash, no AbortController, no Vue `debounce`.

21. **Everything on main thread** — Parser, mesh generation, buffer uploads, and rendering all block the UI. Large models or slow $fn will freeze the tab.

22. **Naive triangle counting** — `indices.length / 3` with no check that indices are valid or complete. Can produce fractional or wrong counts.

23. **Poor tessellation quality** — Sphere uses simple lat/long (bad poles). Cylinder caps have naive fan. No adaptive subdivision.

24. **Broken transparency ordering** — Two-pass (opaque then transparent) without sorting back-to-front per triangle or even per mesh. Classic alpha artifacts.

25. **Incorrect normal matrix for non-uniform scales** — The code always does `transpose(invert(transform))`. This is only correct for uniform or no scale. Skewed models get wrong lighting.

26. **Shaders as giant inline strings** — Both WGSL shaders live as template literals in the middle of the renderer file. No syntax highlighting, no separate files, hard to edit.

27. **Grid and gizmo are inflexible** — Generated once at startup with magic numbers. Cannot be toggled, resized, or styled by user.

28. **Zero editor features** — Plain textarea. No syntax highlighting, no error underlines, no autocomplete, no formatting, no folding, no minimap.

29. **No undo/redo** — Code changes are not tracked. User can lose work instantly with no way back except browser history.

30. **Translation strings are ad-hoc** — Big `L` object with duplicated keys. No extraction, no pluralization, no external files, easy to get out of sync between languages.

31. **Raw localStorage everywhere** — Direct `getItem`/`setItem` calls with magic strings in multiple places. No namespace, no schema version, no migration.

32. **Poor HTML metadata** — index.html has Russian `lang`, no description, no viewport best practices beyond basic, no Open Graph, no theme-color.

33. **Missing development scripts** — package.json has only dev/build/preview. No `test`, `lint`, `format`, `typecheck`, `check` commands.

34. **Complete absence of tests** — No Vitest, no parser golden tests, no math property tests, no visual regression. The parser can regress silently.

35. **No CI/CD** — No GitHub Actions, no automated build on PR, no lint gate, no test gate.

36. **Incomplete .gitignore** — Still picks up .DS_Store. Missing coverage/, .env*, IDE folders, etc.

37. **Parser error recovery is terrible** — One syntax problem often causes large parts of the model to disappear because `stmt()` just advances and returns null.

38. **parseValue is full of silent fallbacks** — Unknown identifiers return the identifier name as string or 0. Bad numbers become zero. No diagnostics.

39. **Unsupported modules silently degrade** — `hull`, `minkowski`, `linear_extrude`, `text` etc. just recurse into children. User gets wrong geometry with no warning.

40. **Color state is lost across CSG** — When difference emits red children, subsequent siblings can inherit the wrong color because `col` parameter is passed inconsistently.

41. **No resource limits** — User can write `$fn=1000` and generate millions of triangles with no warning, no clamping beyond a weak `Math.max(8, ...)`.

42. **Input handling is spaghetti** — All pointer state (`drag`, `pan`, `mx`, `my`) lives as flat private fields. No controller class, hard to test or extend.

43. **No architecture layers** — Parser, evaluator, math, and renderer are tightly coupled inside the single Vue app. Impossible to reuse or test in isolation today.

44. **UI elements are afterthoughts** — Stats, error box, and hint are crammed into the giant component with almost no componentization.

45. **Zero accessibility** — Canvas has no ARIA description, toolbar buttons lack labels in places, no keyboard focus management beyond Tab in textarea, no reduced-motion respect.

46. **Mobile experience is an afterthought** — Touch gestures are partially inherited from pointer events but lack proper pinch, double-tap, long-press, or orientation handling.

47. **No progress indication** — Parsing or uploading a 500k triangle model shows nothing. User thinks the app is frozen.

48. **Auto-render is dangerous** — Every keystroke triggers a full reparse + re-upload after debounce. Complex models cause constant jank or battery drain.

49. **No asset I/O** — You cannot export STL, OBJ, glTF, or even a screenshot easily. The only persistence is raw source in localStorage.

50. **Core logic is not packaged** — Parser and renderer cannot be consumed by other projects (VSCode extension, web component, headless renderer, npm package). Everything is glued to this one Vue app.

## How to Use This List

- Items 1–15 are foundational and should block any claim of "production readiness".
- Items 16–30 are severe UX and correctness problems that affect daily users.
- Items 31–50 are maintainability, process and polish issues.

Any new feature work should be evaluated against whether it makes one of these 50 items worse.

## Related Documents

- [architecture.md](architecture.md) — current design + 200 forward ideas
- [README.md](README.md) — user facing overview and limitations

## Additional 200 Problems (51–250)

Here are 200 more distinct issues discovered during deeper audit. These complement the initial Top 50. All are based on inspection of the actual source.

51. Parser silently drops statement-starting modifiers like # % * ! without error or effect (OpenSCAD uses them for debug/render).

52. skipExpr() is extremely crude and can consume far too much or too little when expressions appear.

53. parseVec does not support nested vectors or ranges like [0:5].

54. Negative numbers in vectors are parsed inconsistently due to limited minus handling.

55. The tokenizer treats $fn as a normal Ident, special handling is only in evaluator via string key.

56. No distinction between module calls and variable references — everything is call or ident.

57. Parser accepts but completely ignores "function" and "module" declarations.

58. Children of unsupported nodes (hull etc) are still evaluated, producing wrong but non-empty output.

59. cIdx reset only happens at top level parseOpenSCAD; nested or partial parses pollute colors.

60. arg() helper uses ?? but many defaults pass -1 as position making call sites confusing.

61. Sphere generation has degenerate triangles and zero-area faces near poles.

62. Cylinder with h=0 or r1=r2=0 produces invalid or zero-area geometry without warning.

63. makeCube always emits 24 vertices (6 faces × 4) even for tiny cubes.

64. No face normal averaging or smoothing groups support.

65. Vertices store normals but no UVs or tangents — future texturing impossible without rewrite.

66. MeshData transform is always applied at eval time; no way to keep symbolic transforms.

67. evalNode for difference hardcodes alpha=0.35 and red color — magic and not overridable.

68. intersection() forces alpha 0.55 on everything — changes user colors silently.

69. Color names via cssColor only support a tiny hardcoded list, no #hex or rgb().

70. In rotate, axis-angle path assumes default [0,0,1] but code has bug in default vec.

71. mirror() with non-axis vector produces incorrect (non-orthogonal) result because it just scales.

72. multmatrix applies matrix but never validates that it is affine or has proper last row.

73. No handling for "center" in sphere or other primitives beyond cylinder/cube.

74. $fn lower bound was 8 now 4, but can still produce < 3 sides for cylinder which collapses.

75. No convex hull even for simple 2-point cases.

76. All mesh generation allocates new arrays every time; no pooling.

77. parseOpenSCAD always resets color index — no way to continue coloring across multiple parses.

78. Tokenizer does not recognize scientific E notation fully in all edge cases (sign after E).

79. Comments inside numbers or strings? No, but nested /* */ not properly depth counted.

80. Parser has no recovery for mismatched braces — can leave AST in bad state.

81. In renderer, normal matrix is always computed even for identity transforms.

82. Scene uniform buffer size is hardcoded 112 bytes — fragile if Scene struct changes.

83. WGSL structs are duplicated between MESH_WGSL and LINE_WGSL.

84. Depth texture is recreated every resize even if size didn't change enough.

85. No mipmaps or sampler usage at all (not needed yet but architecture doesn't plan for it).

86. Camera projection near/far planes are magic 0.1 and dist*10 — can clip large models badly.

87. lookAt and perspective in math3d use row-major but some conventions expect column.

88. Pitch clamp is hardcoded -1.5/1.5 radians (~86 deg) — not configurable and asymmetric with yaw.

89. Grid lines include axis highlights but axis colors are magic arrays inside buildGrid.

90. No way to disable grid or axes at runtime.

91. Render loop always redraws everything even if nothing changed (no dirty flag).

92. setMeshes destroys old buffers synchronously but new ones are queued — potential race on rapid edits.

93. Transp flag is only based on color[3] < 0.99 — any alpha <1 treated same, no sorting key.

94. drawIndexed called even when ic===0 — wastes a draw call.

95. GPU buffers for uniforms are 144 bytes (model + nmat + color) but no versioning.

96. Pointer capture is set but never checked if capture succeeded.

97. Wheel listener uses {passive:false} but no equivalent for touch.

98. No velocity or momentum on camera after drag release.

99. AutoFit can set dist=5 minimum even for tiny objects causing bad initial view.

100. Bbox computation in autoFit does not account for transformed normals correctly? Wait it does vertices only.

101. Multiple rapid calls to setMeshes leak old bindgroups until GC (buffers are destroyed but).

102. Canvas clear color is hardcoded dark gray in render().

103. Light direction is hardcoded [0.55,0.75,0.45].

104. Ambient is hardcoded [0.22,0.22,0.24].

105. Specular contribution is fixed 0.25 white — no material control.

106. Backlight term "bd" is arbitrary 0.25 factor.

107. Phong exponent 40 is magic and too high for many materials.

108. No support for point lights or multiple lights (only one directional).

109. No fog, no environment, nothing beyond basic directional + ambient.

110. Line pipeline for grid uses same scene bindgroup but different layout expectation.

111. In math3d, multiply is O(n^3) naive loop — fine for 4x4 but no SIMD or fast path.

112. All rotate functions allocate new identity every call.

113. translate, scale etc create temp matrices unnecessarily.

114. invert code is 50+ duplicated arithmetic expressions — copy-paste hell, easy to desync.

115. No quaternion or euler conversion helpers despite rotate supporting them indirectly.

116. perspective matrix formula uses 1/tan but no handling for fov==0 or aspect==0.

117. lookAt does not handle collinear eye/center (division by zero risk before normalize).

118. identity() always allocates new Float32Array.

119. No matrix stack or push/pop for hierarchical transforms (relies on recursion in evaluator only).

120. transpose is used heavily on every upload — unnecessary work if we stored column major.

121. Vec3 is just a TS tuple alias, no class or methods.

122. Mat4 type is just Float32Array — no nominal typing, easy to pass wrong sized array.

123. No determinant function exposed (only inside invert).

124. App.vue contains the only usage of EXAMPLES — dead code if we ever remove the buttons.

125. L translation object has duplicated keys between ru and en for some entries.

126. diff_note string is bilingual in one key and not used consistently.

127. Theme is applied via data-attribute but no CSS variables for all colors in one place.

128. Canvas hint text color uses hard rgba in light/dark.

129. Media query only for 800px — no other breakpoints or container queries.

130. Textarea has no max length or protection against huge pastes.

131. Code change watcher always writes to localStorage even if value didn't change meaningfully.

132. Auto render debounce 400ms is arbitrary and not user configurable.

133. When autoRender off, there is no visual "dirty" state.

134. Stats line mixes count with a long italic note that may overflow on mobile.

135. Error box uses inline styles-ish via classes but white-space pre-wrap can explode on bad errors.

136. Brand logo is inline SVG with no title or aria.

137. Toggle buttons have no aria-pressed or labels beyond text.

138. All buttons lack type="button" — could submit forms if ever wrapped.

139. No keyboard shortcut for theme or language toggle.

140. Ctrl+Enter works but no visual affordance on the Render button for the shortcut.

141. EXAMPLES are only loaded at module evaluation time — cannot be extended at runtime.

142. loadExample does silent fail if name unknown.

143. The second <script lang="ts"> for EXAMPLES is an anti-pattern in Vue SFC (mixes two scripts).

144. No use of defineProps or better composition in the giant setup.

145. Reactive refs for counts are updated after parse but UI may flicker.

146. No use of computed for triCount or derived stats.

147. onMounted does async work without loading indicator.

148. gpuOk ref is set only once at start; never retried.

149. No handling for when user pastes OpenSCAD with Windows \r\n line endings specially.

150. Code is saved on every keystroke to localStorage — wear on mobile + privacy.

151. No "clear code" or reset button.

152. Examples buttons overwrite current code without confirmation.

153. No way to download the current .scad source easily.

154. No shareable link generation (encode code to URL).

155. The project name in package.json has hyphen but title has spaces inconsistently.

156. Version is stuck at 0.1.0 with no plan.

157. No "engines" field or browserslist in package.

158. vite.config has zero config — no alias, no define, no optimizeDeps.

159. tsconfig allowsJs true and no "noImplicitAny" override (but strict is on).

160. No "declaration": true or types export for possible future library.

161. vue-tsc is used in build but no incremental or project references.

162. No source maps control for prod.

163. The worktree path in user info suggests this is developed in unusual git setup.

164. No CONTRIBUTING, LICENSE, or CHANGELOG files visible.

165. .vscode configs reference chrome debugger which may not be installed.

166. Tasks.json has very weak problemMatcher (empty regexp).

167. Launch configs hardcode port 5173.

168. No .editorconfig.

169. No prettier / biome / eslint config.

170. package-lock is committed (good) but no "lockfileVersion" discussion.

171. Math functions like rotate use degrees->radians conversion only in evaluator — inconsistent API.

172. axisAngle function can return non-normalized result if input matrix had scale.

173. CSS_COLORS uses approximate values (0.5 instead of 0.50196 for gray etc).

174. No support for color alpha in cssColor path.

175. In App.vue the canvas-hint is always English-ish in style, not translated.

176. L object keys are not typed — t(k) can return any string.

177. No use of Vue's useTemplateRef or better refs in 3.5+.

178. The entire UI is flex column without any landmark roles (header, main, aside).

179. Stats div has no live region for screen readers when counts update.

180. When WebGPU fails the message is static and not actionable beyond "use newer browser".

181. Parser never reports which line or which module failed.

182. Large models with thousands of meshes will create thousands of GPU buffers and bind groups — will OOM.

183. No instancing or multi-draw indirect planned.

184. setMeshes always uploads full vertex data even if only transform changed.

185. There is no "preview quality" vs "final" $fn switch.

186. Camera target (tx ty tz) can drift to NaN if bad bbox from degenerate input.

187. dist can become Infinity in some edge autoFit cases.

188. No protection against NaN or Infinity in camera math propagating to matrices.

189. perspective with very large far plane loses depth precision.

190. Grid is drawn every frame with line-list — thousands of lines with no culling.

191. No frustum culling for grid or meshes.

192. The 3D math uses degrees in some places internally but radians exposed in rotate helpers inconsistently.

193. No matrix equality or approx equal helper for tests.

194. invert can produce denormal floats.

195. No SIMD or @std/math usage even if available.

196. The bilingual strings contain the note about difference() which mixes languages in same string in places.

197. When language changes the whole app doesn't force re-render of some static parts.

198. EXAMPLES contain Russian comments? No, English only.

199. There is no test that the parser roundtrips its own examples.

200. Adding a new primitive requires changes in 4+ places (tokenizer not needed, but parser + evaluator + examples + docs).

201. Tokenizer position 'p' is byte index but never used for user messages.

202. Parser class has private tok and pos but no encapsulation of token stream abstraction.

203. Many switch cases in evalNode fallthrough to same code without comment.

204. nextC cycles 8 colors — after 8 objects colors repeat without user control.

205. difference always renders subtracts in red regardless of user color() on them.

206. No way to visualize the "positive" part of difference only.

207. AutoFit uses vertices[i] directly assuming stride 6 without constant.

208. In setMeshes the uniform write for color is new Float32Array every time.

209. Buffers are created with exact byteLength but no alignment consideration.

210. The line grid vertices interleave pos+color as vec3+vec4 = 28 bytes — not 16-byte aligned nicely.

211. No use of WebGPU timestamp queries for profiling.

212. Render pass always clears — no preserve for overlays.

213. No scissor or viewport control.

214. destroy() destroys device but other code may hold references.

215. App.vue never calls renderer.destroy on error paths fully.

216. Multiple canvas elements or hot reload can leave orphan listeners.

217. The hint text at bottom of canvas is not hidden when error is shown.

218. No "copy error" button.

219. Error messages from parser contain no suggestion of what was expected.

220. In parser, LParen after ident for call but function call args not parsed at all.

221. Vectors in args like translate(v=[1,2,3]) work only because of parseVec.

222. But expressions like translate(v=[1,2,3]+[0,1,0]) do not.

223. No support for "true" "false" as module args beyond bools in some places.

224. center=true is compared with === true after arg — fragile.

225. In makeCylinder the slope normal calculation can divide by zero when h=0 or r1==r2? Handled poorly.

226. Cap indices for cylinder can overlap or be wrong for fn=3.

227. Sphere segments use inclusive ri<=seg which produces one extra ring.

228. No "convexity" parameter respected anywhere.

229. import() and surface() are no-ops that just return children.

230. The whole system assumes all geometry is closed manifold — no open surfaces or lines.

231. No wireframe only mode.

232. Stats "Objects" actually counts meshes (one per color group or primitive).

233. Triangles count can be misleading because of duplicated verts.

234. When using difference the tri count includes the "invisible" red parts.

235. No volume or area computation exposed.

236. Camera can be zoomed inside the model without clipping warning.

237. No "focus selected" or "frame all" separate from auto on load.

238. Drag with right button only works because of shift check, not consistent cross platform.

239. Contextmenu prevention is global on canvas.

240. No double-click handler (common for fit in 3D apps).

241. Touch on canvas may trigger unwanted browser behaviors.

242. No pinch to zoom implementation beyond pointer.

243. The 420px editor width is magic in CSS.

244. Code font stack may not exist on user's machine (no system fallback listed fully).

245. Tab insert always uses 4 spaces — no setting.

246. No way to change font size in editor or canvas.

247. Dark/light theme switch does not persist the choice across all elements perfectly on first load.

248. localStorage keys are not prefixed ("scad-") consistently for all (only some).

249. No migration strategy if localStorage format ever changes.

250. The entire project has no automated way to verify that a change didn't break an example visually.

These 200 additional problems bring the documented issues to 250 concrete items. Many are small but real; some are architectural.

Fixes performed in this pass (examples):
- .gitignore now covers .DS_Store and common junk.
- Added "typecheck" and "check" scripts.
- Improved HTML metadata.
- Removed one `as any`, switched catch to unknown.
- Added real window resize listener + ResizeObserver in renderer.
- Extracted several magic numbers to class constants and used them.
- Made tokenizer throw on unexpected characters (no more silent corruption).
- Clamped and bounded $fn to [4,256].
- Improved triangle count robustness.
- Added basic device.lost listener stub.
- Parser now supports basic expressions (+-*/ and parens) in sizes/args.
- Added exportToSTL() and UI button for STL download.
- Basic Vitest setup + parser tests.
- Examples extracted to src/examples.ts.
- Ongoing: implementing from Top 200 list.

Synchronize note: these fixes address only a tiny fraction. The lists in this file should be used to drive future work.

## Top 200 Ideas, Suggestions and Problems (Prioritized - June 2026)

This is a fresh, curated top 200 list mixing high-impact ideas, concrete suggestions, and remaining problems. Prioritized roughly by user impact + implementation feasibility. Many build on the partial fixes already landed (tokenizer errors, $fn clamp, resize handling, etc.).

1. **Upgrade editor to Monaco/CodeMirror** — Replace textarea with real code editor supporting OpenSCAD syntax highlighting, error squiggles, autocomplete for primitives.
2. **Full expression evaluator** — Add support for arithmetic (+-*/%), comparisons, parentheses, and function calls inside arguments.
3. **Implement real CSG** — Replace visual-only difference/intersection with actual boolean operations (WASM Manifold or port csg.js) so meshes are correct for export.
4. **Variables and assignments** — Support `foo = 10;` and use of variables in expressions and module calls with proper scoping.
5. **For loops** — Parse and evaluate `for (i = [0:10]) { ... }` and list-based for.
6. **User modules with params** — Allow `module mybox(w=10) { cube([w,10,10]); }` and calls with overrides.
7. **let() scoping** — Support `let(a=5) { cube(a); }` for local variables.
8. **$fn / $fa / $fs resolution** — Make special vars affect tessellation dynamically instead of only static arg lookup.
9. **hull() implementation** — Add convex hull algorithm for 3D points generated by children.
10. **linear_extrude with twist/scale** — Support advanced params for extrude beyond simple height.
11. **polygon and paths** — Add 2D polygon support with holes for more accurate 2D-to-3D workflows.
12. **STL import** — Parse binary and ASCII STL files and turn them into renderable meshes.
13. **Proper error positions** — Store line/column in tokens and AST; show "Error at line 12 col 4: ..." 
14. **Undo/redo for code** — Maintain edit history with Ctrl+Z support (beyond browser).
15. **Customizer panel** — Auto-generate sliders/color pickers from top-level variables in the SCAD code.
16. **Export STL / glTF** — Add buttons to download current geometry as binary STL or glTF for real use.
17. **Add Vitest + parser tests** — Golden tests for all examples and edge cases; run on every change.
18. **Move EXAMPLES out of App.vue** — Load from external .json or separate .scad files for easier maintenance and testing.
19. **CI with GitHub Actions** — Add build, typecheck, test, and visual regression jobs.
20. **Real math library** — Replace hand-written math3d with gl-matrix or similar; add tests for invert/multiply.
21. **Web Worker for parse/render** — Offload parsing, evaluation and mesh gen to worker so UI stays responsive.
22. **Vertex deduplication + index optimization** — Post-process generated meshes to share vertices and reduce size.
23. **Better camera controls** — Add zoom-to-cursor, double-click focus, inertia, orthographic toggle.
24. **View cube + preset buttons** — Standard 3D navigation aids (top/front/right/iso) with hotkeys.
25. **Measurement tools** — Click points to measure distances, angles, diameters in the 3D view.
26. **Section plane / clipping** — Interactive cutting plane to inspect internal geometry.
27. **Wireframe + edges overlay** — Toggle to show model edges on top of shaded view.
28. **Shadows and better lighting** — Add simple shadow mapping + multiple lights or IBL.
29. **PBR materials** — Allow color() to set roughness/metallic for more realistic rendering.
30. **Better transparency** — Implement proper order-independent or sorted transparency instead of crude two-pass.
31. **Export high-res PNG with alpha** — Canvas capture at 2x/4x resolution for documentation.
32. **Shareable links** — Base64-encode SCAD code in URL hash for easy sharing.
33. **Multi-file support** — Virtual project with tabs or sidebar for include/use simulation.
34. **Syntax error recovery** — Parser should continue after bad statements and report all errors.
35. **Clamp and warn on extreme $fn** — Prevent OOM from $fn=10000; show warning.
36. **Device lost recovery** — Attempt to recreate device/context when WebGPU signals lost.
37. **Performance metrics overlay** — Show parse time, triangle count, FPS, buffer sizes in dev mode.
38. **i18n extraction** — Move all strings out of code into JSON files; add more languages.
39. **Accessibility audit + fixes** — ARIA labels, keyboard nav for canvas (limited), focus management, reduced motion.
40. **Mobile gestures** — Proper pinch zoom, two-finger pan, tap for selection on touch devices.
41. **Print bed visualization** — Select printer size and show build volume + origin.
42. **Volume / surface area calc** — Compute and display after successful render.
43. **Named camera bookmarks** — Save/restore specific views.
44. **Animation scrubber for $t** — Support OpenSCAD animation variable with timeline.
45. **Command palette** — Ctrl/Cmd+K for render, load example, export, toggle grid etc.
46. **Snippet library** — Common patterns (difference with hole, rounded box) insertable via UI or editor.
47. **Formatter** — Button or on-save that pretty-prints the SCAD code.
48. **Minimap in editor** — Once real editor is in.
49. **Find & replace** — Across current file (later project).
50. **Theme sync** — Make editor theme follow app light/dark automatically.

51. **Support rotate_extrude** — With angle and other params.
52. **polyhedron primitive** — Direct points + faces support.
53. **text() primitive** — Using 2D canvas or font loading for extruded text.
54. **surface() from image** — Heightmap import.
55. **projection(cut)** — Generate 2D outlines from 3D.
56. **offset() 2D** — Round, delta, chamfer for 2D shapes.
57. **minkowski() basic approx** — At least for simple cases.
58. **color alpha in all paths** — Consistent propagation.
59. **mirror with proper normal flip** — Current scale hack doesn't always invert normals correctly.
60. **multmatrix validation** — Warn on non-affine or bad matrices.
61. **Better sphere tessellation** — Icosahedron or geodesic for even triangles.
62. **Cylinder cone normal fix** — Improve for non-tangent cases.
63. **Cap generation cleanup** — Avoid degenerate triangles on flat ends.
64. **Mesh metadata** — Attach original module name / source range to each MeshData.
65. **Scene graph instead of flat list** — For future selection and hierarchy view.
66. **Incremental re-evaluation** — Only re-parse changed subtrees when possible.
67. **Mesh caching** — Cache results of identical sub-expressions.
68. **Frustum culling** — Skip drawing meshes outside view.
69. **Level of detail** — Simplify high-$fn models when far or in preview mode.
70. **Instanced rendering** — For repeated identical subassemblies.
71. **Post-processing stack** — FXAA, SSAO, simple bloom.
72. **Environment map** — For metal materials.
73. **Outline / selection highlight** — Click mesh to highlight.
74. **Exploded view mode** — Separate parts along axes.
75. **Layer simulation** — Simple sliced preview for 3D printing.
76. **Support preview** — Generate tree-like supports.
77. **Build volume config** — User-selectable printers.
78. **Cost estimator** — Rough filament/weight from volume.
79. **Send to slicer** — Download STL + open local slicer if possible.
80. **DXF/SVG export of projections** — For laser/cnc.

81. **VSCode extension** — Embed viewer in VSCode for .scad files.
82. **Web component** — <scad-viewer src="..." /> reusable.
83. **npm packages** — Publish parser and renderer separately.
84. **PWA + offline** — Installable, cache examples and last code.
85. **Tauri / Electron desktop** — Native file open/save, better perf.
86. **GitHub Action for render** — Render SCAD to STL/PNG in CI.
87. **Public gallery** — Upload/share example models (opt-in).
88. **Prompt-to-SCAD** — Basic LLM integration to generate starter code.
89. **Parametric sweep** — Generate variants for a variable range.
90. **BOM extraction** — List "parts" based on colors or named groups.
91. **Overhang detection** — Visualize angles >45 deg.
92. **Orientation optimizer** — Suggest best print orientation.
93. **2D drawing export** — Orthographic views as SVG/DXF.
94. **Engineering drawing** — Auto multi-view + dimensions.
95. **Physics preview** — Simple drop or stability test (future).
96. **AR mode (WebXR)** — Place model in real world.
97. **Collaborative editing** — CRDT for shared models (advanced).
98. **Plugin system** — Register custom primitives from JS.
99. **Node-based alternative UI** — Visual graph that generates SCAD.
100. **Headless render API** — For server or batch use.

101. **Extract constants everywhere** — Finish the job started in renderer (grid, speeds, shader params, debounce ms, etc.).
102. **Strict TypeScript cleanup** — Remove remaining `any`, add branded types for Mat4/MeshData.
103. **AST visitor pattern** — Make future transformations and analyses easy.
104. **Separate concerns in App.vue** — Split into Editor, Viewport, Toolbar, Console components.
105. **State management** — Use Pinia or simple store for editor state, camera, settings.
106. **Config object** — Central place for all defaults (speeds, colors, grid, quality).
107. **Feature flags** — Easy toggle for experimental features (real CSG, worker, etc.).
108. **Better localStorage abstraction** — Namespaced keys, versioned schema, migration.
109. **Error boundary + recovery** — Catch render errors without full crash.
110. **Progress indicator** — For long parses / large uploads.
111. **Throttle auto-render** — Only re-render after meaningful change or on Ctrl+Enter always.
112. **Debounce with Abort** — Use modern patterns instead of manual timeout.
113. **Canvas context menu** — Right click for measure, focus, isolate, hide.
114. **Keyboard shortcuts overlay** — Show available keys.
115. **Recent files / history** — List of last edited codes.
116. **Drag & drop .scad** — Load file directly.
117. **Download current .scad** — Easy save button.
118. **Reset to example** — With confirmation.
119. **Dark/light more palettes** — Solarized, high-contrast.
120. **Live stats in title or favicon** — Triangle count badge?

121. **Support more OpenSCAD keywords** — group, render, projection without crash.
122. **Better default colors** — Per-primitive or user palette.
123. **Color inheritance fixes** — Make sure difference red doesn't pollute siblings.
124. **Normal matrix correctness** — Handle non-uniform scale properly (current transpose(invert) is approximate).
125. **Grid customization** — Size, step, visibility, polar option.
126. **Axis labels and ticks** — Make gizmo more useful.
127. **Camera speed settings** — User-adjustable rotate/pan/zoom sensitivity.
128. **Min/max distance clamp** — Prevent flying to infinity or inside model badly.
129. **Bbox computation optimization** — Use typed arrays or WebAssembly later.
130. **Memory monitoring** — Warn when too many buffers created.
131. **Dispose checks** — Dev-only leak detection for GPU resources.
132. **Source maps in prod** — Optional for debugging shipped errors.
133. **Bundle size audit** — Code-split heavy editor when added.
134. **Tree shaking** — Ensure unused math functions are dropped.
135. **Strict lint + format on commit** — Add biome or eslint + husky.
136. **Coverage thresholds** — Fail CI if parser coverage drops.
137. **Property based testing** — Generate random valid-ish SCAD for parser.
138. **Visual regression** — Playwright screenshots of key examples on canvas.
139. **Benchmark harness** — Track parse/render time regressions.
140. **Axe-core + pa11y** — Automated a11y checks.

141. **Help / docs panel** — In-app reference for supported syntax.
142. **Tutorial mode** — Step-by-step guided modeling.
143. **Example browser with thumbnails** — Generate small previews.
144. **Search in examples** — By keyword or feature.
145. **Version the saved code** — So old localStorage doesn't break new parser.
146. **Telemetry opt-in** — Anonymous usage (parse time, features used) to prioritize.
147. **Crash reporting** — Opt-in send error + SCAD snippet (sanitized).
148. **Contribution guide** — Make it easy to add a primitive or fix.
149. **Changelog automation** — From commits.
150. **Release process** — Tags + GitHub releases.

151. **Support center=true for sphere**.
152. **Handle d= diameter in sphere/cylinder consistently**.
153. **Support negative sizes?** — Or warn.
154. **Vector math helpers** — len, norm, cross inside expressions.
155. **Math functions** — sin, cos, sqrt, min, max, abs usable in code.
156. **Undef handling** — More graceful.
157. **Comment preservation** — For future formatter.
158. **Include/use from URL** — Fetch remote .scad (with CORS note).
159. **Stdlib bundle** — Common modules like rounded_cube built-in.
160. **Recursion guard** — Prevent stack overflow on bad modules.

161. **Better grid generation** — Fewer vertices, major/minor lines.
162. **Axis with arrows and labels**.
163. **Camera target snapping** — Optional grid snap for pan target.
164. **Reference images** — Load background plane for modeling aid.
165. **Multi light presets** — Sunny, studio, rim.
166. **X-ray / ghost mode**.
167. **Isolate / hide selected** — Once selection exists.
168. **Hierarchy tree view** — Show CSG structure.
169. **Properties inspector** — For selected mesh (color, bbox).
170. **Raycast selection** — Click to select which primitive.

171. **Volume calculation** — Accurate after real CSG.
172. **Center of mass**.
173. **Manifold check** — Warn if result is not watertight.
174. **Auto center on plate**.
175. **Multiple views** — Side by side perspective + ortho.
176. **Recording** — Export turntable video or GIF.
177. **Comparison mode** — Load two models side by side or difference.
178. **Versioned examples** — Tag which OpenSCAD version they target.
179. **Compatibility warnings** — "This uses unsupported linear_extrude twist".
180. **Migrate to proper 3D math lib** — Finish the suggestion.

181. **Support for child() and $children**.
182. **resize() primitive**.
183. **echo / assert to console panel**.
184. **Special vars $vpr $vpt $vpd for view**.
185. **$preview flag**.
186. **Convexity param** (hint for renderer).
187. **Render() module respect**.
188. **Better default $fn** — 32 or dynamic.
189. **Support negative radii?** — Clamp or error nicely.
190. **String color names** — More than the 20 hard-coded.

191. **Drag to reorder in future multi-doc**.
192. **Diff view** — Show before/after of edit.
193. **Live link to official docs** — For each keyword.
194. **Keyboard only mode** — Full nav without mouse.
195. **High contrast mode**.
196. **Voice commands** (experimental).
197. **Integration with FreeCAD / Blender** — Import/export bridges.
198. **Slicer preview bridge** — Call external slicer via API if available.
199. **Print farm integration** — Send job (future).
200. **Become the best browser OpenSCAD tool** — Full fidelity + modern UX while staying lightweight and offline-first.

This list can be used as a living backlog. Items 1-50 are highest priority for the next development phase. Many low-number items directly address the 250 problems already logged.
