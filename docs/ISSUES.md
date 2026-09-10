# Known Issues — Top 500

A comprehensive, concrete catalog of things done badly or incorrectly, produced by a 5-domain deep audit (parser, renderer, app/state, UX/CSS/a11y, build/security/i18n/exports). Each item cites a file, approximate line, and the actual problem.

Synced with: [README.md](../README.md) · [architecture.md](../architecture.md) (limitations) · [RECOMMENDATIONS.md](./RECOMMENDATIONS.md) (fixes mapped to phases) · [TOP-50-ISSUES.md](./TOP-50-ISSUES.md) (severity-ranked shortlist).

> In a hurry? Start with [TOP-50-ISSUES.md](./TOP-50-ISSUES.md) — the 50 highest-severity items curated from this catalog.

| Domain | File(s) | Count |
|--------|---------|------|
| A. Parser & geometry | `src/services/openscadParser.ts` | 100 |
| B. Renderer & math | `src/services/webgpuRenderer.ts`, `math3d.ts` | 90 |
| C. App state & logic | `src/App.vue` (script) | 120 |
| D. UX / CSS / a11y | `src/App.vue` (template+style) | 100 |
| E. Build / security / i18n / exports | config, `public/`, exporters | 90 |
| **Total** | | **500** |

## Severity hot-list (fix first)

These are the highest-impact correctness/security items pulled from the full list:

1. **Fake CSG** — `difference`/`intersection`/`minkowski` are visual only, not boolean (A72–A75). Output is non-manifold; STL/3MF export of "subtracted" models is wrong.
2. **No `$fn`/loop/recursion bounds** — `sphere($fn=1e5)` or `helix(turns=1000)` OOM/hang the tab (A40, A49, A61, A33). Trivially DoS-able.
3. **XSS via `document.write`** — print/spec-sheet/shortcuts inject unescaped tab name & `innerHTML` into a new same-origin window (E25–E29). Stored XSS → localStorage/clipboard access.
4. **Untrusted share URL & localStorage** — `loadFromHash`/`JSON.parse` of storage with no validation; a corrupt value white-screens the app at module load (C36, E32, E35–E38).
5. **Module-global parser state** — `cIdx`, `_resolveFile`, `_profiling`, `_source` make the parser non-reentrant and order-dependent (A23–A26).
6. **OpenGL depth in WebGPU** — `perspective`/`ortho` map z to [−1,1] not [0,1]; wastes half the depth buffer, z-fighting (B56–B57).
7. **`de` locale missing** — language cycle includes German but there is no `de` dictionary; selecting it shows raw keys (E59, C44).
8. **Per-frame/per-mesh allocations** in the render loop and matrix math drive GC churn (B18–B22, B59–B67, C53–C58).
9. **No tests, no CI, no code-splitting**, committed `.js` artifacts (E1–E24).
10. **`render()` vs `renderScaled()` diverged** — screenshots don't match the live view; hidden meshes reappear (B35–B36).

---

## A. Parser & Geometry (`openscadParser.ts`) — 100

1. [correctness] :38 — string tokenizer never decodes escapes (`\n` copied literally); unterminated string runs past EOF.
2. [correctness] :42 — number lexer accepts `1e` (no exponent digits) → `parseFloat` returns 1 silently.
3. [correctness] :34 — block-comment scan overshoots EOF on unterminated `/*`.
4. [correctness] :32 — `ch <= ' '` swallows all control chars (NUL/DEL) as whitespace.
5. [type-safety] :77-82 — `ExprBinary/Unary/Call` members all typed `any`; expression tree untyped.
6. [correctness] :307 — `||`/`&&` constant-folded to booleans; OpenSCAD returns the operand, not bool.
7. [correctness] :368 — `/0` folded to `0` at parse time (should be inf/nan).
8. [correctness] :369 — `%0` returns 0 (should be nan).
9. [correctness] :294 — ternary folded at parse time discarding the unevaluated branch; wrong for array/string truthiness.
10. [correctness] :401 — `parseFloat` on malformed token unvalidated; NaN propagates.
11. [correctness] :413 — only `PI` modeled; `$fa/$fs/$t/$preview` ignored.
12. [correctness] :426 — `min`/`max` with a vector arg (`max([...])`) unhandled → NaN.
13. [correctness] :451 — unknown primary token: `adv(); return 0` swallows syntax errors, desyncs parser.
14. [correctness] :492 — list comprehension supports only `[for(x=range) body]`; no `if`/nested/`let`/multi-generator.
15. [correctness] :472 — range parse only triggers on `:` after first element; `[a, b:c]` mis-parses.
16. [correctness] :266 — named-arg detection `==` offset check can misclassify `f(a=b==c)`.
17. [correctness] :169 — `*`/`#`/`%`/`!` modifiers discarded; `*` (disable) still renders subtree.
18. [correctness] :204 — module-def single-child path mismatches `module m() ;`.
19. [dead-code] :115-125 — `len/norm/cross/lookup/str/chr/concat` stubs return 0, shadowed by real impls → wrong folding.
20. [correctness] :115 — placeholder `len` reachable via generic fold path, returns 0.
21. [correctness] :107 — `log: Math.log` is natural log; OpenSCAD `log()` is base-10.
22. [correctness] :110 — `rands` map stub returns a scalar; contradicts vector reimpl.
23. [global-state] :2109 — `cIdx` module-global; non-deterministic colors across calls.
24. [global-state] :2110 — `_resolveFile` module-global function pointer; re-entrancy hazard.
25. [global-state] :2111-2114 — `_profiling/_profileEntries/_profileDepth/_source` module globals corrupt under reentry.
26. [global-state] :4748 — `parseOpenSCAD` resets only `_resolveFile`; leftover `_profiling` makes the light path profile+mutate.
27. [correctness] :2115 — `nextC` color assignment depends on prior call history.
28. [type-safety] :2117 — `arg` `??` conflates omitted vs explicit `undef`.
29. [type-safety] :2117 — `pos=-1` sentinel computes key `_-1`; confusing magic.
30. [correctness] :2122 — `resolveArg` never treats barewords as var refs; unknown vars become their name string.
31. [correctness] :1917 — unresolved variable returns its identifier string; surprising in array/color contexts.
32. [correctness] :2184 — `expandRange` silently truncates at 10000; float-accumulated step causes off-by-one counts.
33. [perf/safety] :2184 — degenerate `[0:1e-12:1]` allocates up to 10000 before bailing — DoS.
34. [correctness] :2094 — runtime `/0` returns 0 (inconsistent with IEEE/OpenSCAD inf).
35. [correctness] :1894 — comparisons coerce non-numbers to 0; `"a"=="b"` wrongly true; string equality broken.
36. [correctness] :2199 — `!!val` makes empty array truthy; OpenSCAD treats empty vector as false.
37. [correctness] :690 — sphere poles produce degenerate zero-area triangles.
38. [perf] :690 — sphere recomputes sin/cos per ring; O(seg²) trig.
39. [correctness] :690 — single `seg` for lat & long; tessellation differs from OpenSCAD, over-dense at poles.
40. [validation] :3553 — `$fn` floored at 8 but no upper bound; `sphere($fn=1e5)` OOM.
41. [correctness] :719 — cone (r2=0) makes a degenerate apex ring of coincident verts.
42. [correctness] :716 — apex verts share position with distinct normals → shading seam; zero-radius top fan.
43. [correctness] :4175 — `cone` via `makeCylinder(...,0,...)` emits `fn` degenerate top-cap tris.
44. [correctness] :761 — pipe inner-wall winding doesn't match stated inward normal (cull risk).
45. [correctness] :739 — no check `r2<r1`; `pipe(r1=5,r2=8)` self-intersects.
46. [correctness] :854 — torus `r1=0` → divide by zero → allocation blow-up.
47. [correctness] :868 — torus normals likely inverted on inner ring.
48. [correctness] :883 — helix tube radius hard-coded `pitch*0.15`, ignores wire radius.
49. [correctness] :885 — helix `ringSegs=fn*turns` unbounded → multi-million verts.
50. [perf] :887 — helix recomputes full Frenet frame per ring (analytic frame is cheap).
51. [correctness] :948 — bezier 5-point list drops point 4.
52. [correctness] :963 — bezier segment-boundary duplicates points; seam tangent wrong.
53. [correctness] :1064 — sweep `path.length===1` indexes `tangents[-1]` → crash.
54. [correctness] :1051 — sweep generates no end caps; tube open/non-manifold.
55. [correctness] :1161 — star `_fn` parameter ignored (dead param).
56. [correctness] :1178 — star caps via earClip on non-convex polygon may leave holes.
57. [correctness] :1322 — gear is a crude tip/valley star, not an involute; misleadingly named.
58. [correctness] :1323 — gear `valleyR=tipR-mod` can go ≤0 for small teeth → inverted profile.
59. [correctness] :1405 — thread `drdTheta` normal math is canceling-constants nonsense.
60. [correctness] :1397 — thread `max(0,sin)` half-rectified; not a usable V-thread.
61. [correctness] :1388 — thread `zSlices` unbounded.
62. [correctness] :1676 — knurl top cap radius ≠ side wall radius for non-integer nVertical → crack.
63. [dead-code] :1747-1756 — chamferCube has abandoned half-written `faceDefs` + `void faceDefs`.
64. [correctness] :1758 — chamferCube falls back to convex hull; approximate, expensive.
65. [perf] :1759 — chamferCube needless `points.map` copy before hull.
66. [correctness] :1776 — loft resamples by index ignoring edge lengths; wraps open profiles.
67. [perf] :1800 — loft recomputes centroid inside per-vertex loop → O(verts²).
68. [correctness] :1817 — loft side-quad winding unverified; outward normal only valid for star-convex.
69. [correctness] :1866 — surface edge gradient divisor logic mismatched.
70. [correctness] :1884 — surface winding gives downward normals vs computed +Z normals.
71. [correctness] :1848 — surface is an open sheet, not a solid; unusable in CSG/extrude.
72. [csg-fake] :3751 — `difference` is translucent overlay, not boolean; subtrahend remains in mesh.
73. [csg-fake] :3758 — `intersection` just renders children semi-transparent; no intersection volume.
74. [csg-fake] :3749 — `union` concatenates without merging; internal geometry remains (non-manifold).
75. [csg-fake] :4641 — `minkowski` is pure pass-through.
76. [approximation] :3881 — `offset` radial-from-centroid; breaks concave shapes, ignores chamfer/delta.
77. [approximation] :3964 — `projection` flattens to z=0.01 keeping triangulation; not a 2D silhouette.
78. [correctness] :3984 — projection `cut` mode keep-logic inverted/garbled.
79. [approximation] :3780 — linear_extrude reverse-engineers profile from 3D mesh; fails on holes.
80. [correctness] :2224 — `extractFlatProfile` magic 0.5 threshold misclassifies thin solids.
81. [correctness] :2284 — boundary walk picks first neighbor with no orientation; self-crossing polygons.
82. [correctness] :2237 — edgeVerts keyed by rounded coords but boundary by index — inconsistent.
83. [correctness] :3818 — rotate_extrude sorts by atan2; wrong for general profiles.
84. [correctness] :3814 — rotate_extrude silently mirrors negative-X (OpenSCAD errors).
85. [correctness] :2411 — rotateExtrude endpoint normals scaled inconsistently.
86. [correctness] :2435 — 360° rotate_extrude leaves an unwelded seam (not watertight).
87. [correctness] :1512 — earClip silently returns partial triangulation on failure (gaps).
88. [correctness] :1500 — earClip uses only XY; wrong plane for XZ profiles.
89. [perf] :1530 — earClip O(n³) point-in-triangle test.
90. [correctness] :1485 — pointInTriangle counts edge points as inside; blocks valid ears.
91. [correctness] :2650 — convexHull horizon detection logic contradictory → malformed hull.
92. [perf] :2624 — convexHull effectively O(n³).
93. [perf] :2687 — convexHull recomputes centroid inside nested loop.
94. [correctness] :2638 — convexHull single-pass; points exterior after later inserts are lost.
95. [correctness] :2500 — convexHull <4 points returns a cube at origin, discarding computed centroid.
96. [correctness] :2753 — polyhedron fan triangulation assumes convex/planar faces; no index bounds check.
97. [correctness] :2844 — pyramid cap fan valid only for convex base.
98. [correctness] :3263 — hemisphere built on Y axis vs OpenSCAD +Z convention; axis mismatch.
99. [correctness] :4683 — axisAngle multiply order inconsistent with Euler rotate branch.
100. [correctness] :3634/:3402 — `for` runs only first variable; `__assign` drops non-number (vector/string/bool) variables.

## B. Renderer & Math (`webgpuRenderer.ts`, `math3d.ts`) — 90

1. [shader] :12 — Scene struct duplicated verbatim in 3 shaders; guaranteed drift.
2. [shader] :12 — feature flags smuggled into `_pad0/_pad1` fields; dishonest/undocumented.
3. [shader] :480 — sceneUB 208 bytes with zero slack; any new field overflows.
4. [shader] :12 — relies on coincidental 16-byte alignment, unchecked.
5. [shader] :46 — flat-shading `cross(dpdx,dpdy)` sign undefined → inverted lighting on half faces.
6. [shader] :59 — magic specular exponent 40 / color 0.25; no material params.
7. [shader] :60 — ad-hoc "back light" magic constants, no physical basis.
8. [shader] :61 — lighting in sRGB space, no gamma correction.
9. [shader] :64 — fog divide-by-zero when fogNear==fogFar → NaN.
10. [shader] :77 — gooch overwrites `c`, silently disabling fog/SSAO computed above.
11. [shader] :28 — clip/section `discard` defeats early-Z even when off.
12. [shader] :29 — clip axis from `i32(float)`; fragile, unvalidated.
13. [shader] :37 — "section box" only clips lower bounds — really 3 half-spaces.
14. [shader] :109 — outline inflation fixed world-space `*0.3`; not screen-constant.
15. [shader] :125 — sky fullscreen-triangle gradient extrapolates beyond [0,1].
16. [shader] :127 — sky drawn after grid with depthCompare always; ordering wrong.
17. [resource] :752 — `createView()` per frame for MSAA + swapchain.
18. [resource] :606 — per-mesh bind group; bind-group churn.
19. [resource] :591 — per-mesh 144-byte UB instead of one dynamic-offset buffer.
20. [resource] :940 — reflection allocates mirrorY + 3 matrices per mesh per frame.
21. [resource] :985 — ground-shadow allocates flattenY per mesh per frame.
22. [resource] :817 — exploded view recomputes transpose/invert per mesh per frame.
23. [resource] :851 — hidden-line write/restore of color UB doubles uniform traffic.
24. [render-loop] :1008 — pointless per-frame color restore after submit.
25. [render-loop] :727 — `resize()` called every frame inside render().
26. [render-loop] :1062 — `requestRender` dirty-flag/raf race.
27. [render-loop] :1113 — FPS meaningless under on-demand rendering.
28. [render-loop] :1092 — auto-rotate framerate-dependent (no delta time).
29. [render-loop] :1098 — inertia decay not delta-normalized.
30. [render-loop] :1051 — animation lerps yaw the long way around ±π.
31. [render-loop] :347 — init() mixes direct loop() call with RAF scheduling.
32. [pipeline] :404 — 5 pipelines built inline with copy-pasted blocks.
33. [pipeline] :408 — `cullMode:'none'` globally; doubles fragment work.
34. [pipeline] :727 — render() is a ~300-line monolith.
35. [pipeline] :1491 — `renderScaled` is a diverged copy of render().
36. [pipeline] :1588 — renderScaled ignores mesh visibility; hidden meshes reappear in screenshots.
37. [camera] :730 — pitch clamp 1.5 magic number duplicated 6 sites.
38. [camera] :1147 — "fly mode" never translates camera; nonfunctional.
39. [camera] :2270 — `setFlyMode` doesn't requestRender and changes nothing; dead.
40. [camera] :736 — ortho zoom coupled to perspective FOV.
41. [camera] :740 — far=dist*10, near=0.1 → huge depth ratio, z-fighting.
42. [camera] :738 — ortho near/far clip scene with no fit to bounds.
43. [camera] :269 — direct `this.fov=` bypasses clamp.
44. [camera] :1191 — wheel zoom ignores `deltaMode`; inconsistent across devices.
45. [camera] :1152 — pan basis ignores pitch; wrong plane at steep angles.
46. [camera] :1154 — vertical pan always world-Y, not camera up.
47. [msaa] :670 — DPR change without clientWidth change doesn't recreate textures.
48. [msaa] :1070 — debounced resize races with per-frame resize().
49. [msaa] :752 — MSAA color+depth+resolve doubles VRAM; can't disable MSAA.
50. [msaa] :410 — `count:4` hard-coded; no adapter limit check/fallback.
51. [device-lost] :366 — lost handler doesn't stop the loop; methods called on dead device.
52. [device-lost] :360 — `requestDevice` no descriptor/try-catch; rejection escapes.
53. [device-lost] :355 — adapter.info vs requestAdapterInfo swallowed into null.
54. [resize-race] :1438 — screenshotScaled swaps textures; render() between swap and async callback corrupts.
55. [resize-race] :1448 — sets canvas size without reconfiguring ctx.
56. [math3d] :53 — perspective maps z to OpenGL [−1,1] not WebGPU [0,1].
57. [math3d] :62 — ortho maps z to OpenGL [−1,1].
58. [math3d] :3 — "row-major" claim + renderer transposing everything is confusing.
59. [math3d] :12 — multiply allocates, no output reuse.
60. [math3d] :106 — full 4x4 invert per mesh per frame for normal matrix.
61. [math3d] :125 — invert returns identity on near-singular silently.
62. [math3d] :985 — flattenY (det 0) → invert returns identity → wasted shadow normal math.
63. [math3d] :77 — lookAt doesn't normalize up; pole → NaN matrix.
64. [math3d] :79 — lookAt no guard for eye==center → Infinity.
65. [math3d] :23 — translate/rotate/scale build identity + full multiply for trivial ops.
66. [math3d] — no vec3 helpers; cross/normalize reimplemented inline repeatedly.
67. [math3d] :99 — transpose allocates; chained transpose(invert(...)) = 3 throwaways per mesh/frame.
68. [correctness] :592 — triple-transpose normal-matrix logic opaque; bug-prone.
69. [correctness] :597 — meshIdx read before push; brittle.
70. [correctness] :598 — color override keyed by array position; desyncs if meshes reorder.
71. [correctness] :915 — reflection depthLoadOp clear; reflections don't depth-test vs solids.
72. [correctness] :920 — multiple MSAA resolves per frame overwrite each other.
73. [perf] :914 — reflection+shadow = 2x geometry; with outline 3-4x.
74. [perf] :826 — toon mode silently doubles geometry via outline pass.
75. [correctness] :961 — shadow alpha-blends overlapping tris → dark bands.
76. [dead-code] :222 — clipY getter/setter + setClipY + setClipValue (3 ways).
77. [dead-code] :1731 — commented-out `cz` in getScreenPosition.
78. [type-safety] :599 — color typed `number[]`; wrong length silently produces garbage uniform.
79. [type-safety] :1951 — setRenderMode casts string with no validation.
80. [type-safety] :203 — renderMode union has 'hidden-line' but doc comment omits it.
81. [magic-number] :480 — uniform sizes 208/144/32 raw literals.
82. [magic-number] :698 — fillSceneData writes raw float offsets with no doc; stale `_pad` indices.
83. [perf] :747 — `new Float32Array(52)` every frame (render + renderScaled).
84. [naming] :145 — GMesh fields vb/ib/ic/ub/bg/transp cryptic.
85. [correctness] :1900 — smoothNormals `(x*1000|0)` overflows >2.1M, truncates not rounds.
86. [correctness] :1925 — smoothNormals copies indices/verts even when no welding.
87. [perf] :2168 — buildWireframeBuffer no edge dedup; 2x size, CPU-baked.
88. [perf] :2200 — buildEdgeBuffer string-key Set per edge; slow/memory-heavy.
89. [correctness] :1419 — screenshot() relies on preserved drawing buffer; may capture blank.
90. [correctness] :1746 — setClearColor stores un-premultiplied; fog/occlusion bg mismatch with premultiplied alpha.

## C. App State & Logic (`App.vue` script) — 120

1. [god-component] 1-14737 — entire app in one component; untestable.
2. [god-component] 98-1680 — ~1.6k-line inline i18n object.
3. [god-component] 23-90 — EDITOR_THEMES table inlined.
4. [god-component] 2284/2496 — BUILT_IN_PRESETS / SHORTCUT_PRESETS inlined.
5. [separation] 4745 — favicon SVG generation in onMounted.
6. [reactivity] 13 — `isDark` ref is derived state; should be computed.
7. [reactivity] 1769 — stale `activeTabId` makes `code` setter mutate wrong tab.
8. [reactivity] 2228/3135 — `prefShowMinimap` & `showMinimap` duplicate the same key.
9. [reactivity] 2381 — watcher mutates another ref instead of a computed.
10. [reactivity] 4833 — fat code watcher does 6 unrelated side effects.
11. [reactivity] 4833/6662/7829 — THREE separate `watch(code)`.
12. [reactivity] 4837 — computeFolds runs un-debounced every keystroke.
13. [reactivity] 2064 — perfDeviceInfo computed depends on non-reactive renderer; stale.
14. [reactivity] 2069 — perfBufferStats fakes deps via `void` hacks.
15. [reactivity] 2463 — currentTabHistory parses localStorage on every recompute.
16. [reactivity] 1859 — sortedTabs rebuilds arrays on every access.
17. [stale-closure] 2056 — `suppressUndoPush` racy across nextTick; can stick true.
18. [stale-closure] 8173 — batch snapshots tabs but resolver reads live tabs.
19. [memory-leak] 7903 — compass RAF loop runs even when hidden.
20. [memory-leak] 4794 — axis/annotation/gizmo RAF loops run forever.
21. [memory-leak] 2545 — console drag listeners leak if unmounted mid-drag.
22. [memory-leak] 5722 — selectionchange listener in fragmented lifecycle.
23. [memory-leak] 7873 — cameraInfo interval can leak.
24. [memory-leak] 6554 — animRAF not canceled in main unmount.
25. [memory-leak] 2759+ — bare setTimeouts fire after teardown.
26. [lifecycle] 1685/4730/5721/7903 — FOUR onMounted hooks; fragile order.
27. [lifecycle] 4818+ — FIVE onUnmounted hooks; cleanup scattered/incomplete.
28. [memory-leak] 4775 — onDeviceLost handler retains old renderer on reinit.
29. [localstorage] — 99 direct call sites, no abstraction.
30. [localstorage] 12+ — most reads no try/catch; crashes in private mode.
31. [localstorage] 1777+ — most writes no try/catch; QuotaExceeded breaks app.
32. [localstorage] — ~40 keys, no central registry.
33. [localstorage] 4839 — every keystroke double-serializes code (scad-code + scad-tabs).
34. [localstorage] 1747 — tabs JSON.parse with no schema validation.
35. [localstorage] 2311 — presets parsed with no validation; crashes applyPreset.
36. [localstorage] 6083 — customThemes parsed at module top-level, no try/catch → white screen.
37. [localstorage] 2225 — parseInt with no NaN guard.
38. [localstorage] 2228 — `!== 'false'` treats garbage as true.
39. [localstorage] 7162 — `|| 220` masks NaN and `parseInt('0')`.
40. [localstorage] 8137 — restoreSession trusts parsed tabs fully.
41. [i18n] 1682 — `t()` falls back to key string silently.
42. [i18n] 133+ — ad-hoc `.replace('{n}')` everywhere.
43. [i18n] 98 — untyped locale keys; locales drift.
44. [i18n] 99 — late keys missing from some locales.
45. [i18n] 12 — lang cast `as any`; bad value → total fallback.
46. [duplication] 4874 vs 8186 — use/include resolver copy-pasted.
47. [duplication] 5563 vs 5990 — print harness copy-pasted.
48. [duplication] 2570+ — showCopied flag+timeout pattern repeated.
49. [duplication] 1904 — next/prev tab nearly identical.
50. [duplication] 7289+ — ~25 one-line persistence watchers; need persistRef helper.
51. [duplication] 2236 — savePref exists but most persistence bypasses it.
52. [duplication] 4893 vs 8195 — tri-count reduce duplicated, inconsistent types.
53. [perf] 4859 — doRender parse+mesh+upload synchronous on main thread.
54. [perf] 4867 — `$t` regex rebuilds whole source even with no `$t`.
55. [perf] 4846 — computeFolds/undo/save un-debounced in render watcher.
56. [perf] 4803 — 500ms stats interval writes refs forever even when panel closed.
57. [perf] 7873 — second always-on camera-info interval.
58. [perf] 7251 — stats recompute twice/sec (watcher + interval).
59. [perf] 2463 — currentTabHistory re-parses on unrelated reactivity.
60. [perf] 8195 — `m: any` on hot batch path.
61. [error] 4767 — renderer.init failure swallowed into boolean.
62. [error] — 22 empty catch blocks swallow errors.
63. [error] 8197 — batch catch discards error.
64. [error] 4735 — loadFromHash no try/catch; bad hash aborts mount.
65. [error] 2570 — copyCanvasToClipboard unhandled rejection.
66. [error] 2078 — reinitializeWebGPU fire-and-forget; unhandled rejection.
67. [error] 4866 — parse/mesh/upload lumped; GPU error shown as parse error.
68. [type-safety] 12 — lang `as any`.
69. [type-safety] 2486/6012 — unchecked union casts from storage.
70. [type-safety] — 8 `as any` + hot-path `m:any`/`e:any`.
71. [type-safety] 2415 — event target cast without check.
72. [type-safety] 4874 — parse result type under-specified (optional chaining).
73. [type-safety] 1736 — colorTag allowed values in a comment, not a union.
74. [naming] 2005 — `debounce` handle name; 5 inconsistent debounce handles.
75. [naming] 1981 — mixed `*Mode`/`*Enabled`/bare boolean conventions.
76. [naming] 1983+ — refs grouped by meaningless "Batch N" comments.
77. [magic-number] 2014+ — scattered limits (100/50/20/100000/86400000).
78. [magic-number] 4846+ — zoo of hard-coded debounce/interval durations.
79. [magic-string] 1953/4747 — accent `#4a9eff` duplicated favicon vs themes.
80. [reactivity] 1771 — code setter deep-mutates tab array element.
81. [perf] 1777 — saveTabs stringifies ALL tabs on switch/rename/pin/edit.
82. [reactivity] 2316 — userPresets add/delete relies on reactive object; manual persist.
83. [error] 2445 — saveHistories "trim" branch empty; comment lies.
84. [perf] 2451 — addHistorySnapshot full read-modify-write per burst.
85. [reactivity] 4790 — undo stack seeded only if canvas exists.
86. [bug] 4898 — perfMeshGenTime assigned parse time (copy-paste).
87. [bug] 4870 — fast-preview only matches integer `$fn`, ignores `$fa/$fs`.
88. [bug] 4867 — `$t` replace corrupts `$t` in strings/comments.
89. [reactivity] 6668 — two watchers redundantly trigger extractParameters.
90. [perf] 6681 — computeDiff on main thread, no memoization.
91. [dead-code] 2522 — consoleIdCounter unbounded though 50 kept.
92. [reactivity] 2530 — console slice reassigns whole array per log.
93. [perf] 2534 — every console entry schedules nextTick scroll.
94. [perf] 3876 — minimap has 3 inconsistent render cadences (50/300/immediate).
95. [reactivity] 1762 — activeTabId validated in side-effectful module body.
96. [reactivity] 1759 — tab migration loop at module-eval time.
97. [separation] 1743 — loadTabsFromStorage mixes migration+default+i18n at init.
98. [perf] 4903 — tri-count warning toast every render (no once-guard).
99. [reactivity] 7829 — third code watcher (codeStats).
100. [reactivity] 8288 — screenshotMeta deep-watch serializes on nested change.
101. [perf] 2532 — renderer destroy only in main unmount; use-after-teardown risk.
102. [error] 2965 — scad-last-version parseInt unvalidated.
103. [reactivity] 3525 — editorWidth parseInt-read vs `|0`-write mismatch.
104. [reactivity] 7162 — build-plate dims read/write coercion mismatch.
105. [maintainability] 6020 — watchers used as persistence+DOM layer.
106. [separation] 4744 — data-URI + <link> insertion in component.
107. [perf] 8101 — saveSessionBackup duplicates full tab serialization every 30s.
108. [bug] 8124 — session restore compares only IDs, not content; never offered.
109. [type-safety] 8095 — SessionBackup `as` cast, `.map` throws if null.
110. [naming] 6083 — customThemes/EDITOR_THEMES/userPresets/BUILT_IN_PRESETS muddled.
111. [dead-ref] 2396 — rightScrollTop / one-directional split scroll likely unused.
112. [perf] 5604 — updateBreadcrumbs walks full AST on every caret move.
113. [perf] 5584 — updateSelectionInfo substrings+splits on every selection change.
114. [error] 5566 — printShortcuts silent when popups blocked.
115. [reactivity] 1951 — inertiaEnabled toggle before render lost (missing watcher).
116. [consistency] 7449 — png-scale persisted inline vs watcher siblings.
117. [perf] 4870 — source regex-scanned twice per render.
118. [maintainability] — 333 functions / 255 refs in one setup; un-tree-shakeable.
119. [error] 2123/2146 — scad-onboarded set in two places, can diverge.
120. [magic-string] 2435+ — some storage keys named, most inline; typo risk.

## D. UX / CSS / A11y (`App.vue` template+style) — 100

1. [a11y] modals — 12 `role="dialog"` with no focus trap.
2. [a11y] modals — focus never moved into dialog on open.
3. [a11y] modals — focus not restored to trigger on close.
4. [a11y] toasts — 0 `aria-live`; never announced.
5. [a11y] `.error` ~9939 — no role=alert/aria-live for compile errors.
6. [a11y] batch/loading — no role=status/aria-busy.
7. [a11y] `<canvas>` ~10052 — no aria-label / text alternative.
8. [a11y] `<textarea.code>` ~9755 — no label.
9. [a11y] CSS — `:focus-visible` used 0 times.
10. [a11y] CSS — `outline:none` ~12× with no replacement.
11. [a11y] — `prefers-reduced-motion` honored 0 times.
12. [a11y] — `prefers-contrast`/forced-colors unhandled.
13. [a11y] — `--text-dim:#888` on dark ~3.5:1 fails AA.
14. [a11y] — light `--text-dim:#777` ~4.48:1 borderline.
15. [a11y] diff stats — color-only encoding.
16. [a11y] ghost stats ~10649 — color-only +/−.
17. [a11y] `.profile-slow` ~10740 — color-only.
18. [a11y] compass N marker — hardcoded red color-only cue.
19. [a11y] theme options ~8740 — clickable divs, not keyboard-navigable.
20. [a11y] recent items ~9504 — clickable divs, no role/tabindex.
21. [a11y] tree rows ~10552 — clickable divs; toggle span no aria-expanded.
22. [a11y] timeline dots ~10874 — div@click, unreachable by keyboard.
23. [a11y] annotation/axis labels — no association to canvas.
24. [a11y] toggle buttons — no aria-pressed/aria-current.
25. [a11y] viewport menuitems — no roving tabindex/activedescendant; mouse-only.
26. [a11y] context menus ~10205 — no role=menu, mouse-only.
27. [a11y] modal close `&times;` — several with no aria-label.
28. [a11y] tip buttons ~10380 — glyph title only, no aria-label.
29. [a11y] help "?" buttons — "?" as accessible name.
30. [a11y] language toggle ~8732 — no aria-pressed/listbox.
31. [a11y] swatch grids — no group label/role.
32. [a11y] pref labels ~9045 — not associated via for/id.
33. [a11y] range inputs — no aria-label/valuetext/unit.
34. [a11y] comparison slider ~8833 — no label/readout.
35. [a11y] minimap canvas ~9825 — interactive, no role/label, keyboard-unreachable.
36. [a11y] gizmo dots ~10160 — SVG click targets, no keyboard.
37. [a11y] FAB/zen-exit ~10813 — late/unpredictable tab order.
38. [a11y] tour overlay ~9359 — spotlight visual-only, target not conveyed.
39. [a11y] shortcuts — print window drops theme/contrast.
40. [a11y] disabled buttons — `.btn-disabled` dims only; still focusable/clickable.
41. [a11y] canvas tabindex=0 ~10057 — focus state not visually indicated.
42. [a11y] v-html for highlight/lineNumbers — AT reads markup spans.
43. [ux] Render menu ~10314 — ~25 toggles, flat, no grouping/search.
44. [ux] toolbar ~9413 — ~20 buttons in one overflowing row.
45. [ux] 3 competing export entry points with inconsistent options.
46. [ux] simpleMode is v-show scatter, not a curated layout.
47. [ux] advanced mode is the implied default for first-timers.
48. [ux] five different popover idioms.
49. [ux] inconsistent dismissal (no unified Esc/outside-click).
50. [ux] multiple popovers can open simultaneously/overlap.
51. [ux] stats bar — 11 icon-only segments, tooltip-only.
52. [ux] `.stat-camera-toggle` looks like non-interactive stats.
53. [ux] buried features (measure/ghost/fly/zen/section) icon-only, deep.
54. [ux] "?" button dual-purpose (whatsNew vs shortcuts).
55. [ux] welcome modal crowds onboarding + changelog.
56. [ux] 3 similar "start" CTAs dilute primary action.
57. [ux] popups anchor to px with no viewport-edge collision handling.
58. [ux] tip banner — 3 similar-weight controls; unclear distinction.
59. [ux] batch modal — one-off inline markup, inconsistent look.
60. [ux] About version hardcoded vs aboutVersion key.
61. [ux] ghost/anim overlays compete for bottom area.
62. [ux] `.canvas-hint` always shown; permanent noise.
63. [ux] anim timeline shown even when code has no `$t`.
64. [css] 3.7k lines scoped CSS in one block.
65. [css] tokens exist but ~819 raw px bypass them.
66. [css] 103 hardcoded hex bypass color vars; theme breaks.
67. [css] 208 rgba/rgb literals hardcode accent.
68. [css] axis/grid/compass colors inlined, duplicated.
69. [css] 35 `!important`.
70. [css] 18 distinct z-index values up to 20000; z-index war.
71. [css] inline z-index 9999/20000 in template.
72. [css] 43 physical left/right; no RTL despite RU/DE/ZH.
73. [css] panels positioned with physical right/left; wrong in RTL.
74. [css] `transition: all` ×3.
75. [css] focus-glow rgba duplicated, no token.
76. [css] control border/radius re-inlined instead of class.
77. [css] inconsistent radius (3/4/6/12 vs tokens).
78. [css] inconsistent spacing vs `--sp-*`.
79. [css] 101 `style=` + 29 `:style` in template.
80. [css] stats material/cost rows fully inline-styled.
81. [css] export SVGs copy-pasted (opacity differs).
82. [css] `.perf-panel-*` classes reused on non-perf panels.
83. [css] `.stats-panel-*` reused by ghost stats.
84. [css] mixed rem/px defeats user font scaling.
85. [css] inconsistent hover treatment.
86. [responsive] only 4 media queries.
87. [responsive] divider mouse-only; no touch/keyboard resize.
88. [responsive] touch targets <44px (tab close, modal close, zoom, eye).
89. [responsive] swatches/gizmo dots far under touch size.
90. [responsive] long viewport dropdowns exceed mobile height.
91. [responsive] hamburger omits theme/language/mode on mobile.
92. [responsive] stats bar non-wrapping; overflows narrow panels.
93. [responsive] fixed-position overlays overlap on small viewports.
94. [template] ~25 hand-duplicated vp-dd-item toggles; need v-for/component.
95. [template] one 2.1k-line monolithic template.
96. [template] complex inline expression handlers.
97. [template] chained multi-statement inline handlers.
98. [template] nested v-if/else-if belong in computed/sub-components.
99. [template] index-based `:key` in batch/playground/profile lists.
100. [template] ref-in-v-for collision; duplicate-timestamp `:key` risk.

## E. Build / Security / i18n / Exports — 90

1. [build] vite.config.ts:4 — no manualChunks; single monolithic chunk.
2. [build] — no chunkSizeWarningLimit; >500KB ignored.
3. [build] — no dynamic import() for heavy services.
4. [build] package.json — no lint script/ESLint.
5. [build] — no format script/Prettier.
6. [build] — no test script/framework.
7. [build] — no standalone typecheck script.
8. [build] — no CI workflow.
9. [build] — no engines field.
10. [build] — no packageManager field.
11. [build] tsconfig — allowJs picks up stale `.js`.
12. [build] tsconfig — no noUnusedLocals/Parameters.
13. [build] tsconfig — no noImplicitReturns/noFallthrough/noUncheckedIndexedAccess.
14. [build] tsconfig — no exactOptionalPropertyTypes/forceConsistentCasing.
15. [build] tsconfig — skipLibCheck masks dep type errors.
16. [build] tsconfig — sourceMap+allowJs produced `.js.map` in src/.
17. [build] tsconfig — include matches emitted js.
18. [build] tsconfig — no isolatedModules.
19. [build] tsconfig — noEmit not set; vue-tsc may emit.
20. [artifacts] src/App.vue.js — compiled artifact in source tree.
21. [artifacts] src/main.js(.map) — transpiled entry in src/.
22. [artifacts] src/services/*.js(.map) — 18 stray generated files.
23. [artifacts] .gitignore — ignore rules are a band-aid hiding misconfig.
24. [artifacts] .gitignore — `src/**/*.js` too broad.
25. [security] App.vue:3041 — printCode document.write with unescaped tab name → XSS.
26. [security] :3043 — written window inherits origin; XSS → localStorage/clipboard.
27. [security] :5568 — shortcuts write `el.innerHTML` unsanitized.
28. [security] :5978 — spec-sheet document.write unescaped modelName.
29. [security] :5992 — screenshot dataURL interpolated into src without escaping.
30. [security] :3052 — shareLink no size cap; multi-MB hash breaks.
31. [security] :3052 — no integrity check on share round-trip.
32. [security] :3077 — loadFromHash assigns to code with no validation; DoS.
33. [security] :3074 — hash payload is an XSS delivery vector for the write sinks.
34. [security] :12 — scad-lang `as any`, unvalidated.
35. [security] :1745 — scad-tabs parsed without schema.
36. [security] :6083 — customThemes parsed at module scope, no try/catch → white screen.
37. [security] :2225 — fontSize parseInt no range clamp.
38. [security] :7265 — cost-per-kg parseFloat unvalidated.
39. [security] index.html — no CSP meta.
40. [security] index.html:13 — inline SW-registration script (CSP-hostile).
41. [security] manifest — no CSP/permissions-policy/cache strategy.
42. [sw] sw.js:28 — SWR caches any 200 basic indefinitely; unbounded.
43. [sw] sw.js:34 — cache-first HTML; deploys take effect only on 2nd load.
44. [sw] sw.js:2 — precache only `/` + index.html; hashed bundles not cached → not offline-capable.
45. [sw] sw.js:9 — addAll rejects whole install if one URL 404s.
46. [sw] sw.js:11 — unconditional skipWaiting + claim swaps SW mid-session.
47. [sw] sw.js:1 — cache name hardcoded v1; manual bump forgotten.
48. [sw] sw.js:23 — no offline fallback page.
49. [sw] sw.js:27 — no fetch timeout; hangs on flaky network.
50. [sw] sw.js:28 — type==='basic' filter skips CORS assets.
51. [pwa] manifest:9 — single icon; no 192/512 PNG.
52. [pwa] manifest:13 — SVG data-URI icon; rejected by iOS/some Android.
53. [pwa] manifest:14 — maskable on non-padded SVG; art clipped.
54. [pwa] manifest:1 — no id field.
55. [pwa] index.html — no apple-touch-icon/iOS meta.
56. [pwa] index.html:5 — viewport lacks viewport-fit=cover.
57. [i18n] :98 — translations inline, not external files.
58. [i18n] :12 — lang union duplicated in 3 places.
59. [i18n] :1683 — toggleLang cycles to `de` but NO `de` dictionary → broken UI.
60. [i18n] :1682 — fallback to key name, not default locale.
61. [i18n] :1513 — zh ~162 keys vs ru ~636; mostly missing.
62. [i18n] :811 — en/ru key counts differ; no sync check.
63. [i18n] :2476 — `.replace('{n}')` only replaces first occurrence.
64. [i18n] :133 — multi-placeholder requires chained replaces; drift.
65. [i18n] :150 — Russian plurals grammatically wrong (no plural rules).
66. [i18n] :862 — English plurals "1 undo steps".
67. [i18n] :7233 — formatNumber only ru-RU vs en-US; de/zh wrong.
68. [i18n] :3000 — dates toLocaleString with no locale arg.
69. [i18n] :8052 — history timestamps ignore app language.
70. [i18n] :3123 — relative time reimplemented, not Intl.RelativeTimeFormat.
71. [i18n] index.html:2 — `<html lang="ru">` hardcoded, never updated.
72. [i18n] — no RTL support anywhere.
73. [i18n] :723 — mixed placeholder vocabulary ({n}/{x}/{y}/{total}).
74. [i18n] :1683 — language is a blind 4-way cycle, no menu.
75. [exporter-stl] stlExport.ts:78 — recomputes normals, discards authored ones.
76. [exporter-stl] :10 — transformPoint ignores w-row.
77. [exporter-stl] :78 — negative-determinant transform inverts winding; not reversed.
78. [exporter-stl] :37 — degenerate tris written with zero normal.
79. [exporter-stl] :45 — allocates whole STL contiguously; OOM on large models.
80. [exporter-stl] :37 — no index bounds/multiple-of-3 validation; NaN floats written.
81. [exporter-stl] :111 — revokeObjectURL right after click; Firefox download fails (inconsistent with zipExport).
82. [exporter-obj] objExport.ts:18 — normals transformed by 3x3, not inverse-transpose.
83. [exporter-obj] :38 — builds string[] then join; memory blowup.
84. [exporter-obj] :51 — toFixed(6) loses precision/bloats.
85. [exporter-obj] :30 — no empty-mesh guard/error handling.
86. [exporter-3mf] threemfExport.ts:50 — ZIP stored method writes invalid 1980 zero date.
87. [exporter-3mf] :92 — 32-bit size/CRC, no ZIP64; >4GB overflows silently.
88. [exporter-3mf] :96 — UTF-8 filename flag (bit 11) not set; non-ASCII names misdecoded.
89. [exporter-3mf] :200 — merges all meshes into one object; drops per-mesh color; no welding.
90. [import-stl] stlImport.ts:13 — trusts uint32 count, no ASCII detection; crafted header → multi-GB alloc (DoS).

---

## How to use this list

- Fixes are grouped into phases in [RECOMMENDATIONS.md](./RECOMMENDATIONS.md). Mapping:
  - **A (parser)** → Recommendations **Phase 2** + Correctness backlog
  - **B (renderer/math)** → **Phase 4.1–4.3**
  - **C (app state)** → **Phase 1**
  - **D (UX/CSS/a11y)** → **Phase 1** (components) + **Phase 4.4–4.5**
  - **E (build/security/i18n/exports)** → **Phase 3** + **Phase 4.6–4.7**
- The geometry-correctness items (fake CSG, extrude/offset approximations) are the documented limitations in [architecture.md](./architecture.md#known-limitations).
