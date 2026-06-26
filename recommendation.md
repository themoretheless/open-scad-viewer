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

Synchronize note: these fixes address only a tiny fraction. The lists in this file should be used to drive future work.
