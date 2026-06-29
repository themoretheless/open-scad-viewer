# Top 50 Worst Issues

A severity-ranked shortlist curated from the full [ISSUES.md](./ISSUES.md) (500 items). These are the ones to fix first — highest blast radius for correctness, security, data loss, performance, and architecture.

Synced with: [README.md](./README.md) · [ARCHITECTURE.md](./ARCHITECTURE.md) · [RECOMMENDATIONS.md](./RECOMMENDATIONS.md) · [ISSUES.md](./ISSUES.md)

Each row: rank · `file:line` · problem · **why it matters** · → Recommendations phase. IDs in brackets reference the full catalog (e.g. `[A72]` = domain A item 72).

---

## 🔴 Critical (correctness / security / data-loss) — 1–20

| # | Where | Problem | Why it matters | Phase |
|---|-------|---------|----------------|-------|
| 1 | `openscadParser.ts` :3751 `[A72]` | `difference()` is a translucent overlay, not a boolean op | Every subtractive model is geometrically wrong; exported STL/3MF still contains the subtrahend → unprintable | P2 / backlog |
| 2 | `openscadParser.ts` :3758/:4641 `[A73,A75]` | `intersection()` / `minkowski()` are fake (semi-transparent / pass-through) | Core CSG silently produces wrong solids | P2 / backlog |
| 3 | `App.vue` :3041 `[E25]` | `printCode()` `document.write` with **unescaped tab name** | Stored XSS in a same-origin window → reads localStorage/clipboard | P4.7 |
| 4 | `App.vue` :5568/:5978 `[E27,E28]` | shortcuts/spec-sheet write `innerHTML` / `modelName` unescaped | More XSS sinks via same path | P4.7 |
| 5 | `App.vue` :3077 `[E32]` | `loadFromHash()` assigns share payload to `code` with no validation/size cap | Malicious link → DoS + XSS delivery vector on page load | P4.7 |
| 6 | `App.vue` :6083 `[C36]` | `customThemes` `JSON.parse` at module scope, no try/catch | One corrupt localStorage value → white screen on every load | P1/P4.6 |
| 7 | `openscadParser.ts` :3553 `[A40]` | `$fn` has no upper bound (`sphere($fn=1e5)`) | OOM / hung tab; trivially DoS-able | P2 |
| 8 | `openscadParser.ts` :885/:1388 `[A49,A61]` | `helix`/`thread` vertex counts unbounded | Same OOM/DoS class | P2 |
| 9 | `stlImport.ts` :13 `[E90]` | Trusts STL `uint32` triangle count, no ASCII detection | Crafted 84-byte header → multi-GB allocation (DoS) | P3 |
| 10 | `openscadParser.ts` :3402 `[A100]` | `__assign` drops non-number vars; `for` runs only first variable | Any model assigning a vector to a variable silently breaks | P2 |
| 11 | `App.vue` :1683 `[E59]` | Lang cycle includes `de` but there is **no `de` dictionary** | Selecting German shows raw keys — fully broken UI | P1 |
| 12 | `math3d.ts` :53/:62 `[B56,B57]` | `perspective`/`ortho` map z to OpenGL [−1,1], not WebGPU [0,1] | Half the depth buffer wasted → z-fighting everywhere | P4.1 |
| 13 | `App.vue` :8124 `[C108]` | Session restore compares only tab IDs, not content | Recovery never offered when it's needed → silent data loss | P1 |
| 14 | `App.vue` :2445 `[C83]` | `saveHistories` quota-catch "trim" branch is empty (comment lies) | History silently stops saving; lost versions | P1 |
| 15 | `stlExport.ts` :78 `[E77]` | Negative-determinant transform (mirror) not winding-reversed | Mirrored exports have inverted facets → slicers reject | P3 |
| 16 | `App.vue` :12 / :1745 `[C34,E35]` | `scad-tabs`/`scad-lang` parsed with no schema/validation | Corrupt/tampered storage flows straight into editor & exporters | P4.6 |
| 17 | `openscadParser.ts` :1894 `[A35]` | Comparisons coerce non-numbers to 0 → `"a"=="b"` is true | String equality broken; conditionals misbehave | P2 |
| 18 | `webgpuRenderer.ts` :1588 `[B36]` | `renderScaled` ignores mesh visibility | Hidden objects reappear in high-res screenshots | P4.3 |
| 19 | `App.vue` :4898 `[C86]` | `perfMeshGenTime` assigned parse time (copy-paste) | Mesh-gen metric is simply wrong | P1 |
| 20 | `App.vue` :4867 `[C88]` | `$t` global-replace corrupts `$t` inside strings/comments | Animation feature mangles source | P1 |

## 🟠 High (architecture / perf / reliability) — 21–38

| # | Where | Problem | Why it matters | Phase |
|---|-------|---------|----------------|-------|
| 21 | `App.vue` 1–14737 `[C1]` | God component: ~14.7k lines, 255 refs, 333 fns | Untestable, unreviewable; every change risks regressions | P1 |
| 22 | `openscadParser.ts` :2109-2114 `[A23-A26]` | Module-global parser state (`cIdx`, `_resolveFile`, `_source`…) | Non-reentrant, order-dependent; blocks Web Worker | P2 |
| 23 | `App.vue` :4859 `[C53]` | Parse + mesh-gen + GPU upload run synchronously on main thread | Large models freeze the UI on every keystroke | P2/P3 |
| 24 | `vite.config.ts` `[E1]` | No `manualChunks`; whole app is one >500 KB chunk | Slow first load; parser/exporters loaded eagerly | P3.4 |
| 25 | none `[E6,E8]` | No tests, no CI | 500 issues, zero regression safety net | P3.3/3.6 |
| 26 | `webgpuRenderer.ts` :1491 `[B35]` | `renderScaled` is a diverged copy of `render()` | Two code paths drift; screenshots ≠ live view | P4.3 |
| 27 | `webgpuRenderer.ts` :12 `[B1,B2]` | Scene struct triplicated; flags smuggled into `_pad` fields | Latent uniform-layout hazard; silent corruption on edit | P4.1/4.2 |
| 28 | `App.vue` :4833+6662+7829 `[C11]` | Three separate `watch(code)`, one fat watcher with 6 effects | Redundant work, ordering ambiguity, un-debounced folds | P1 |
| 29 | `App.vue` :1777 `[C81]` | `saveTabs` stringifies ALL tabs on every switch/rename/pin/edit | O(total source) writes on trivial actions; quota risk | P1 |
| 30 | `App.vue` :4803/:7873 `[C56,C57]` | Two always-on `setInterval` polls (stats, camera) | Wake-ups + re-renders even when panels closed | P1 |
| 31 | `webgpuRenderer.ts` :940-990 `[B20,B21]` | Per-mesh per-frame matrix allocs in reflection/shadow | GC churn in the render loop | P4.3 |
| 32 | `math3d.ts` :106 `[B60]` | Full 4×4 invert per mesh per frame for normal matrix | Wasteful; affine 3×3 suffices | P4.1 |
| 33 | `App.vue` — 99 sites `[C29-C31]` | Direct localStorage everywhere, most without try/catch | Crashes in private mode / on quota; no namespacing | P4.6 |
| 34 | `webgpuRenderer.ts` :366 `[B51]` | Device-lost handler doesn't stop the loop | Methods keep running on a dead device → exceptions | P4.3 |
| 35 | `openscadParser.ts` :1512 `[A87]` | `earClip` silently returns partial triangulation on failure | Caps get holes with no error (star/text/gear) | P2 |
| 36 | `openscadParser.ts` :2624 `[A92]` | `convexHull3D` is effectively O(n³) with buggy horizon logic | Slow + can produce malformed hulls | P2 |
| 37 | `App.vue` :98 `[C2]` | ~1.6k lines of inline i18n; untyped keys, locales drift | Translators blocked; `en`/`zh` silently incomplete | P1 |
| 38 | `objExport.ts` :18 `[E82]` | Normals transformed by 3×3, not inverse-transpose | Wrong normals under non-uniform scale/shear | P3 |

## 🟡 Medium (UX / a11y / polish, high visibility) — 39–50

| # | Where | Problem | Why it matters | Phase |
|---|-------|---------|----------------|-------|
| 39 | template `[D1-D3]` | 12 modals: no focus trap, no focus move/restore | Keyboard/AT users lost behind backdrop | P4.4 |
| 40 | template `[D4,D5]` | 0 `aria-live`; toasts & errors never announced | Screen-reader users miss all feedback | P4.4 |
| 41 | CSS `[D9,D10]` | `:focus-visible` 0×; `outline:none` ~12× | Keyboard focus invisible across the app | P4.4 |
| 42 | `<canvas>`/`<textarea>` `[D7,D8]` | Core canvas + editor unlabeled | Primary content inaccessible | P4.4 |
| 43 | CSS `[D66,D67]` | 103 hex + 208 rgba bypass theme vars | Theme switching silently breaks those rules | P1 |
| 44 | CSS `[D72]` | 43 physical `left/right`; no RTL despite RU/DE/ZH | Non-RTL-ready; Arabic/Hebrew impossible | P4 |
| 45 | template `[D43,D44]` | Render menu ~25 toggles + ~20-button toolbar | Severe feature overload, no grouping | P1/UX |
| 46 | template `[D45]` | Three competing export entry points, inconsistent options | Confusing; 3MF only in two of three | P1 |
| 47 | `App.vue` :7233 `[E67]` | `formatNumber` only `ru-RU`/`en-US`; de/zh wrong | Wrong decimal/grouping for German | P1 |
| 48 | i18n `[E63,E65]` | `.replace('{n}')` first-occurrence only; broken plurals | Ungrammatical counts ("1 шагов", "1 undo steps") | P1 |
| 49 | template `[D88]` | Touch targets <44px (tab/modal close, zoom, eye) | Fails WCAG 2.5.5; unusable on touch | P4.4 |
| 50 | `sw.js` :2/:34 `[E43,E44]` | Cache-first HTML, bundles not precached | Deploys apply only on 2nd load; not actually offline | P3 |

---

## Fix sequencing (recommended)

Per [RECOMMENDATIONS.md](./RECOMMENDATIONS.md) "write tests first":

1. **Safety net first** — items 25, 9, 7, 8 (add Vitest + `$fn`/loop bounds + STL-import guard; cheap, stops DoS).
2. **Security pass** — items 3, 4, 5, 6, 16, 33 (kill `document.write` XSS, validate all storage/share input).
3. **Correctness** — items 1, 2, 10, 11, 13, 14, 17, 19, 20 (CSG honesty, `de` locale, var handling, data-loss bugs).
4. **Architecture** — items 21, 22, 23, 26, 27, 28, 37 (decompose, de-globalize parser, split render paths).
5. **A11y/UX/perf** — items 39–50 + 29–32.

> Note: items 1–2 (true boolean CSG) are large; the honest interim fix is to **label them clearly in-UI as visual-only** (already noted in [ARCHITECTURE.md](./ARCHITECTURE.md#known-limitations)) until a real CSG kernel is integrated.
