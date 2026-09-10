# Top 50 Worst Issues

A severity-ranked shortlist curated from the full [ISSUES.md](./ISSUES.md) (500 items) plus a second, independent audit pass. These are the ones to fix first — highest blast radius for correctness, security, data loss, performance, and architecture.

Synced with: [README.md](../README.md) · [architecture.md](../architecture.md) · [RECOMMENDATIONS.md](./RECOMMENDATIONS.md) · [ISSUES.md](./ISSUES.md)

Each row: rank · `file:line` · problem · **why it matters** · → Recommendations phase.

---

## Audit history

- **Pass 1** (branch `claude/extract-openscad-viewer-q3BT0` → merged): produced the original 500-item [ISSUES.md](./ISSUES.md) catalog and fixed **~34 of the original top-50 items** (XSS sinks, `$fn`/loop bounds, STL-import validation, `de` locale, CSG-honesty badge, a11y quick wins, device-lost handling, i18n plurals, service-worker rework, CI added). See the "✅ Resolved in Pass 1" section below for the full record.
- **Pass 2** (this update, branch `claude/top-issues-architecture-sync-00p2q9`): a fresh 4-domain audit (parser/geometry, renderer/math, App.vue state/logic, App.vue UX/a11y/CSS + build/security/exports) re-verified every item Pass 1 had left open — all 16 were confirmed still present, several with sharper root causes than originally described — and independently hunted for new defects. Every item below was grep/read-verified against the current source (line numbers current as of this pass); a few candidate findings from the audit were checked and **dropped** because they turned out to already be fixed or overstated (e.g. a claimed `customThemes` white-screen bug at App.vue:6083 no longer exists at that line — `customThemes` has used `safeParse` since Pass 1; a claimed XSS gap in `printShortcuts()`'s `document.write` turned out to already escape the only dynamic values it writes).

**This ranked list of 50 supersedes the old numbered 1–50 list.** The Pass-1 resolved record is kept below for history.

---

## ✅ Resolved in Pass 1

| Items | What was done |
|-------|---------------|
| CSG honesty | Persistent "CSG: visual preview" badge + tooltip when difference/intersection/minkowski used (interim honest fix; true boolean kernel still pending) |
| XSS | `escapeHtml`/`escapeAttr` on all `document.write` sinks (printCode, spec-sheet) |
| Share links | `loadFromHash` validates + 1 MB size-caps share payload in try/catch |
| localStorage | `safeParse` wraps every localStorage `JSON.parse`; `lang` validated |
| DoS bounds | `$fn` capped at 256; helix/thread/spring/spiral/sweep/fibonacci vertex counts bounded |
| STL import | Validates triangle count + exact byte size before allocating |
| Variables | Assignment stores vectors/strings/bools; `for` iterates all vars (Cartesian, now also bounded — see Pass 2) |
| i18n fallback | Falls back to English before raw key; `de` dictionary now exists |
| Session restore | Compares tab content+names, not just IDs (see Pass 2 for a regression found in the fix itself) |
| History quota | Trims oldest tab's history on quota error and retries |
| Equality | String/array equality compares like-typed values (`"a"=="b"` now false) |
| Perf metrics | Measure their real phases (parse/mesh/upload) |
| `$t` substitution | Skips strings/comments |
| Bundle | Vite `manualChunks` splits parser/renderer/exporters/vue (main 490→293 KB) |
| Tests/CI | Vitest + parser/STL tests + GitHub Actions (typecheck + test + build) |
| Device-lost | Stops loop/cancels RAF; `requestDevice` try/catch; shortest-path yaw lerp |
| Exports | STL/OBJ reverse winding for mirror transforms; OBJ normals via inverse-transpose; STL skips degenerate/out-of-range tris |
| Screenshots | `renderScaled` applies per-mesh visibility (+ hidden-line mode) |
| Perf polling | `saveTabs` debounced with flush; stats/camera intervals idle when panels closed |
| Polygon caps | `earClip` strict containment + degenerate-skip + fan fallback |
| i18n formatting | `formatNumber` uses correct BCP-47 locale; `Intl.PluralRules` for relative-time/undo counts |
| Touch targets | Coarse-pointer media query enlarges small buttons/swatches to 44px |
| Service worker | Network-first navigations + cache-first assets, resilient precache, fetch timeout, offline fallback |
| Regressions (self-review) | `boundsSize` refreshed every render (not just panel-gated interval); multi-variable `for` Cartesian product bounded at 100,000 iterations; `computeBounds`/`autoFit` skip non-finite projected vertices |

---

## 🔴 Critical (data loss / hangs on common input / breaks common workflows) — 1–18

| # | Where | Problem | Why it matters | Phase |
|---|-------|---------|-----------------|-------|
| 1 | `App.vue:1819` | `flushSaveTabs()` catches quota-exceeded errors with only a comment — no retry, no toast, unlike the analogous `saveHistories` | A user typing under a full storage quota believes tabs/code are saved; after a crash/reload, all unsaved edits are silently gone | P4.6 |
| 2 | `App.vue:5004` | The 3-second debounced history-snapshot callback reads `activeTabId.value` live when the timer *fires*, not when it was scheduled | Typing in tab A then switching to tab B within 3s writes tab A's code into tab B's history, corrupting tab B's history | P1 |
| 3 | `App.vue:4775` | `SHORTCUT_PRESETS[shortcutPreset.value]` is indexed with no existence guard against a value that only a type cast (not a runtime check) validated | A corrupted `scad-shortcut-preset` value makes `bindings` undefined; `bindings.commandPalette` throws on every keydown, breaking all keyboard shortcuts app-wide until reload | P4.6 |
| 4 | `src/services/openscadParser.ts:860` | `makeTorus`'s `tubeSegs = Math.max(8, Math.floor(fn*r2/r1))` has no guard against `r1=0` | `torus(r1=0, r2=3)` → `tubeSegs = Infinity` → the `for (j=0; j<=tubeSegs; j++)` loop never terminates, hanging the tab | P2 |
| 5 | `src/services/openscadParser.ts:2934` | `makeDonut` has the identical unguarded `r1` division as `makeTorus` | Same infinite-loop hang from a partial-angle torus/donut call with `r1=0` | P2 |
| 6 | `App.vue:2406` | `saveUserPresetsToStorage()` calls `localStorage.setItem` with no try/catch, unlike sibling save functions | Under quota pressure, clicking "Save Preset" throws uncaught and the preset is silently lost | P4.6 |
| 7 | `App.vue:3221` | `saveRecentFiles()` writes to localStorage with no try/catch, called on nearly every file-open/save | On quota-exceeded storage, opening or saving a file throws uncaught mid-action | P4.6 |
| 8 | `App.vue:7797` | `saveBookmarksToStorage()` has no try/catch even though bookmarks embed base64 PNG thumbnails — the payload most likely to hit quota | Saving a camera bookmark with a thumbnail throws uncaught near the storage quota, silently losing the bookmark | P4.6 |
| 9 | `App.vue:8316` | `saveSessionBackup()` — the crash-recovery safety net itself — swallows quota-exceeded errors with a bare comment and no retry | Under near-quota storage, the auto-save-for-recovery feature silently stops protecting the user's work exactly when it's needed most | P4.6 |
| 10 | `App.vue:8218` | `saveSnapshots()`'s quota retry only fires when `snapshots.value.length > 1`; with 0–1 snapshots it silently no-ops while `takeSnapshot()` still shows a success toast | A user with one large stored snapshot takes a new one, sees "Snapshot saved," but the write silently failed | P4.6 |
| 11 | `src/services/openscadParser.ts:4268` | `gear`'s `teeth` arg has only a floor of 3 and no ceiling | `gear(teeth=50000)` builds a 100k+-point profile and feeds it into the O(n³) `earClip`, freezing the tab for a long time with no progress indicator | P2 |
| 12 | `src/services/openscadParser.ts:3852` | `linear_extrude`'s `twist`/`slices` args have no upper clamp before driving `actualSlices` inside `extrudeMesh` | `linear_extrude(twist=2000000, height=10) square(5);` allocates ~200,000 slices worth of geometry and stalls the main thread | P2 |
| 13 | `App.vue:8402` | The batch "Render All Tabs" loop's `catch (e: any)` discards the real error and records a fake `{tris:0, timeMs:0}` result indistinguishable from a genuinely empty model | Batch-rendering a tab with a real syntax error silently shows "0 triangles" with no indication that tab actually failed | P1 |
| 14 | `App.vue:2168` | `reinitializeWebGPU()` recreates the renderer after device-loss but never restarts `updateAxisLabels()`/`updateGizmo()`/the annotation RAF loop, which self-terminated when `renderer` was nulled | After a GPU device-lost event and clicking "Reinitialize," the 3D view renders again but axis labels, the gizmo, and annotations freeze permanently for the rest of the session | P4.3 |
| 15 | `src/services/math3d.ts:77` | `lookAt()` has no guard for `eye === center` (zero-length view vector) or `up` parallel to the view direction (zero-length cross product) | A scripted "fit to view" or camera reset that momentarily sets `eye≈target` produces `NaN`/`Infinity` in the view matrix and blanks the entire viewport | P4.1 |
| 16 | `src/services/openscadParser.ts:2696` | In `convexHull3D`'s horizon-edge detection, `isHorizon` starts `true` and the matching branch only ever sets it `true` again — it is never set `false` on a hidden-face match | Any `hull()` call on 5+ points can misclassify edges, producing a visibly non-convex or self-intersecting hull mesh for ordinary multi-point `hull()` usage | P2 |
| 17 | `App.vue:8333` | Two literal raw NUL (0x00) bytes and one raw SOH (0x01) byte are committed directly in the source, used as ad-hoc field delimiters inside the session-restore signature function's template literal, instead of the escaped `\0`/`\x01` character-literal syntax a developer would normally type | Plain `grep` (no `-a`) already treats the whole 15,000-line `App.vue` as a "binary file" because of these two bytes, silently breaking any grep-based tool/script/pre-commit hook run against it; the raw bytes are also one bad find-and-replace away from being corrupted since most editors don't render them | P4.6 |
| 18 | `App.vue:6234` | `applyUIDensity()` is only invoked from inside the `uiDensity` watcher (no `{ immediate: true }`) and is never called once on startup | A returning user whose density preference was persisted as compact/comfortable sees default "normal" spacing on every page load until they manually re-toggle the setting | P1 |

## 🟠 High (architecture / reliability / perf) — 19–38

| # | Where | Problem | Why it matters | Phase |
|---|-------|---------|-----------------|-------|
| 19 | `src/services/webgpuRenderer.ts:988` | The ground-shadow and reflection passes that exist in `render()` (line ~942) are entirely absent from `renderScaled()` | Enabling "ground shadow" or "reflection" and then using "Screenshot at 2x/4x" produces an exported image silently missing the shadow/reflection visible on screen | P4.3 |
| 20 | `src/services/math3d.ts:57` | `perspective()`/`ortho()` map z to OpenGL's [−1,1] instead of WebGPU's native [0,1] depth range | Rendering nearly-coplanar geometry (section box, CAD detail lines) wastes half the depth-buffer precision, causing visible z-fighting/flicker | P4.1 |
| 21 | `App.vue` (script, lines 1–~8920 of 14,993 total) | The entire app remains one component with 237 refs/computeds and 333 functions | Any change requires reasoning about the whole file's shared state; reviewers and tests cannot isolate a single feature, which is exactly how bugs #1–#18 above went unnoticed | P1 |
| 22 | `src/services/openscadParser.ts:2158` | `cIdx`, `_resolveFile`, `_profiling`, `_profileEntries`, `_profileDepth`, `_source` remain module-global mutable state | Re-entrant/concurrent parses (e.g. a second file opened while a profiling run is in flight) corrupt each other's colors and profiler output; also blocks moving parsing to a Web Worker | P2 |
| 23 | `App.vue:5060` (`doRender`) | Parse, mesh generation, and GPU upload still run synchronously in one function on the main thread | A large model or heavy `$fn` freezes the UI on every keystroke/render, with no way to cancel or show progress | P2/P3 |
| 24 | `App.vue:4983` / `:6876` / `:8371` | Three separate `watch(code, ...)` watchers independently fan out unrelated side effects (save/fold/undo/render/minimap; parameter extraction; code stats) with no defined ordering | Every keystroke triggers three independently-scheduled watcher callbacks doing overlapping work, making change-driven bugs hard to reason about | P1 |
| 25 | `App.vue:126`–`1723` | The ~1.6k-line inline i18n dictionary `L` is typed only as `Record<string, Record<string, string>>`, no union over actual keys | A typo'd or newly-added key in one locale silently falls back to English/raw-key at runtime instead of failing at compile time — locale drift ships unnoticed | P1 |
| 26 | `App.vue` (~98 call sites) | Direct `localStorage.getItem`/`setItem` calls remain scattered with no shared abstraction/registry | Storage bugs (see criticals #1, #6–#10 above) must be fixed piecemeal at each call site instead of once in a shared `SafeStorage` helper, so the same class of bug keeps recurring | P4.6 |
| 27 | `src/services/webgpuRenderer.ts:12` | The `Scene` WGSL struct (feature flags smuggled into `_pad0`/`_pad1`) is copy-pasted verbatim across 3 shader strings (`MESH_WGSL`, plus 2 more) | Adding/reordering a flag in one shader without mirroring the edit in the others causes a binding-layout mismatch WebGPU silently misinterprets as garbage floats rather than an error | P4.1/4.2 |
| 28 | `App.vue:2596` | The shortcut preset restored from localStorage uses a type-only cast (`as ShortcutPreset`) with no runtime validation against the actual preset keys | Feeds directly into the app-wide keydown crash at critical #3 | P4.6 |
| 29 | `App.vue:6226` | `uiDensity` is seeded via `(localStorage.getItem(...) as UIDensity) \|\| 'normal'` with no validation against the 3 known values | An invalid stored value resolves `UI_DENSITY_SCALES[uiDensity.value]` to `undefined`, producing `NaN` CSS custom properties across the whole UI's spacing | P4.6 |
| 30 | `App.vue:4218` | `openFile()`'s `FileReader` sets only `onload`; no `onerror`, no try/catch around the load body | A `.scad` file that fails to read (disk/permission error) fails completely silently — no toast, no console entry | P1 |
| 31 | `src/services/webgpuRenderer.ts:967` | The reflection pass allocates a new mirror matrix plus fresh `multiply`/`invert`/`transpose` results once per mesh, per frame | With reflections enabled on a many-mesh scene, every frame does dozens of extra heap allocations for a constant matrix, increasing GC pressure and causing visible frame-time jitter | P4.3 |
| 32 | `src/services/webgpuRenderer.ts:612` (+847, 974, 1019, 1047, 1612, 1715) | The per-mesh normal matrix is recomputed via a full general 4×4 `invert()`+`transpose()` every single frame, even for meshes whose transform hasn't changed | Wasteful CPU work that scales with mesh count × frame rate; a scene with hundreds of static meshes visibly loses FPS for no reason | P4.1 |
| 33 | `src/services/openscadParser.ts:1510` | `earClip` remains worst-case O(n³) with no vertex cap or bailout | Importing/generating a high-resolution 2D profile (e.g. an SVG/DXF trace with thousands of points) freezes triangulation for minutes with zero progress feedback | P2 |
| 34 | `App.vue:7203` / `:6968` | `sectionBoxRangeX/Y/Z` and `clipRange` computeds read only the non-reactive module-level `renderer` variable, so Vue never re-tracks and the value caches forever after first access | After loading a new model or reinitializing WebGPU post device-loss, the section-box and clip-plane sliders stay frozen at the first model's bounds and can't reach the current geometry | P1 |
| 35 | `App.vue:2318` / `:2636` / `:3668` | `prefFontSize`/`prefTabSize`/`prefAutoRenderDelay`/`consolePanelHeight`/`editorWidth` all do unguarded `parseInt(localStorage.getItem(...) \|\| default)` with no `NaN` check, and their own resize handlers re-persist the corrupted value | A corrupted stored value yields `NaN` applied directly as CSS (font-size / panel width), and the drag-resize handler writes the `NaN` right back to storage, making the corruption permanent across reloads | P4.6 |
| 36 | `src/services/math3d.ts:125` | `invert()` silently returns `identity()` on a near-zero determinant, with no warning or caller-visible signal | A degenerate model matrix (e.g. `scale([1,0,1])`) makes the normal matrix silently become identity instead of failing loudly — the mesh renders with wrong lighting and no diagnostic clue | P4.1 |
| 37 | template, ~9410/~10314 | The main toolbar packs ~21 buttons into one non-wrapping row (clips on ≤1024px screens) and the Render menu packs ~21 flat toggles + 1 dropdown with no grouping/fieldset | Severe feature overload with no way to reach clipped toolbar buttons except resizing the window, and screen readers hear 21 unlabeled "checkbox" announcements in a row with no category context | P1/UX |
| 38 | `App.vue:2883` / `:2891` / `:2899` | Three separate export entry points (`exportSTL`, `exportOBJ`, `export3MF`) each hard-code their own filename/option defaults with no shared options dialog | A user who sets a custom scale/unit for STL export finds OBJ/3MF export silently ignores that choice because each path re-derives its own defaults | P1/UX |

## 🟡 Medium (a11y / CSS / polish / edge-case correctness) — 39–50

| # | Where | Problem | Why it matters | Phase |
|---|-------|---------|-----------------|-------|
| 39 | style block | **134** hardcoded hex colors + **213** rgba/rgb literals bypass the CSS theme variables (up from the 103/208 last measured) | Custom/dark theme switching leaves scattered UI elements rendering in stale hardcoded colors | P1 |
| 40 | style block | **109** physical `left`/`right`/margin/padding/border/text-align properties (up from 43 last measured); zero RTL support despite ru/de/zh/en locales | Non-RTL-ready; an Arabic/Hebrew locale would be visually broken throughout | P4 |
| 41 | template, root layout | No landmark roles (`role="main"`) around the primary editor/viewport region | A screen-reader user must tab through the entire ~21-button toolbar to reach the main content, every time | P4.4 |
| 42 | template, ~15 sites (theme swatches, recent-file rows, tree rows/toggles, timeline dots, gizmo dots/axes, breadcrumbs, context menus) | Interactive elements remain plain `<div>`/`<span>` with `@click` only — no `role`, `tabindex`, or keyboard handler | A keyboard-only user cannot operate any of these controls; they are permanently unreachable without a mouse | P4.4 |
| 43 | template, ~6 range sliders (comparison, FOV, section-box X/Y/Z, screenshot compare) | No `aria-label`/`aria-valuetext` on any of them | A screen-reader user dragging any of these sliders hears only a generic "slider" with no indication of what it controls or its current value | P4.4 |
| 44 | template (ghost-diff stats, `.profile-slow`, compass N marker) | Status is still encoded by color alone with no icon/text backup | A colorblind user cannot tell which stat increased/decreased or which operation was flagged slow | P4.4 |
| 45 | `src/services/webgpuRenderer.ts:1224` | `onWheel` scales `dist` by `(1 + deltaY*0.001)` with no cap and no `deltaMode` handling | Some trackpads report large line-mode `deltaY` per tick, making the camera "teleport" instead of zooming smoothly | P4.3 |
| 46 | `src/services/openscadParser.ts:706` | `makeSphere` duplicates pole vertices with independently-computed (bit-non-identical) normals, and has no `seg` clamp inside the function itself (only at call sites) | Visible seam artifact at the poles under flat/normal-smoothing, and unbounded allocation if any future call site forgets the outer clamp | P2 |
| 47 | `src/services/webgpuRenderer.ts:77` | The Gooch-shading branch unconditionally overwrites `c`, discarding the fog blend and SSAO darkening already computed earlier in the same fragment shader | Enabling fog + Gooch shading together shows zero fog on Gooch-shaded objects | P4.1 |
| 48 | `src/services/threemfExport.ts` / `stlExport.ts` / `objExport.ts` | 3MF still merges all meshes into one object (drops per-mesh color, no welding); STL/OBJ still build/join the entire output in one pass rather than streaming | Multi-color models lose per-part color on 3MF export; very large models risk an out-of-memory error mid-export instead of a bounded/streamed write | P3 |
| 49 | `.github/workflows/ci.yml`, `tsconfig.json` | CI has no lint step (no ESLint config exists to run one); `tsconfig.json` has neither `noUnusedLocals` nor `noUnusedParameters` enabled | Dead code and unused parameters accumulate silently in the 15k-line `App.vue` with no automated signal | P3 |
| 50 | `App.vue:2592` | `historyDebounce`'s timer has no matching `clearTimeout` in `onUnmounted`, unlike sibling debounces | Combined with critical #2's stale closure, a pending timer can fire after a tab is closed and write an orphaned history entry for a tab id that no longer exists | P1 |

---

## Fix sequencing (recommended)

1. **Stop the data loss first** — items 1, 6, 7, 8, 9, 10 (wrap every un-guarded `localStorage.setItem` in the same try/retry pattern `saveHistories` already uses — this is one `SafeStorage.setItem()` helper away from being fixed everywhere at once, which is exactly Recommendations Phase 4.6).
2. **Stop the hangs** — items 4, 5, 11, 12, 33 (bound `r1`/`teeth`/`twist`/earClip the same way `$fn` was already bounded in Pass 1 — small, mechanical, high value).
3. **Fix the two silent-corruption bugs** — items 2 (history writes to the wrong tab) and 17 (raw control bytes) — both are small, surgical fixes once seen.
4. **Validate storage on read, not just write** — items 3, 18, 28, 29, 35 all stem from the same root cause (#26: no shared storage abstraction) and would mostly disappear with one `SafeStorage.getItem<T>(key, validator, fallback)` helper.
5. **Architecture** — items 19–24, 27, 32, 34, 37, 38 (decompose `App.vue`, de-globalize the parser, split `render()`/`renderScaled()`, fix WebGPU depth convention — the multi-session refactor tracked in [RECOMMENDATIONS.md](./RECOMMENDATIONS.md) Phases 1, 2 and 4.1–4.3).
6. **A11y/CSS/polish** — items 39–50.

> Note: true boolean CSG (`difference`/`intersection`/`minkowski`) remains a large, separately-tracked backlog item — see [architecture.md](./architecture.md#known-limitations). It did not place in this pass's top 50 only because the *interim honest-labeling* fix from Pass 1 (the "CSG: visual preview" badge) already prevents it from silently misleading users; the underlying geometry is still wrong and export of a "subtracted" model is still unprintable.
