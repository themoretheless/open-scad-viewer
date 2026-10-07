import { readFileSync, readdirSync, statSync } from 'node:fs'
import { extname, join, relative } from 'node:path'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { verifyPackedWasmChunk, verifyUniquePackedWasm, verifyRawWasm } from './verify-packed-wasm.mjs'

const rootUrl = new URL('../dist/', import.meta.url)
const root = fileURLToPath(rootUrl)
const files = []
function walk(directory) {
  for (const name of readdirSync(directory)) {
    const path = join(directory, name)
    if (statSync(path).isDirectory()) walk(path)
    else files.push({ path: relative(root, path).replaceAll('\\', '/'), bytes: statSync(path).size, extension: extname(path) })
  }
}
walk(root)

const limits = new Map([
  ['.html', 10_000],
  ['.css', 100_000],
])
// Static SVG adds usvg/resvg, shaping, raster decoders and bundled Noto Sans.
// CSS geometry and instance-correct non-scaling strokes bring the packed kernel
// to ~2.32 MB. Isolated size-profile trials saved only 4.7 kB for usvg/resvg
// or increased decoder size by 5.1 kB, with slower text. Retain the fast profile
// and bound this feature addition; the complete distribution stays below 4.7 MB.
// The native occurrence-production replay + $expansion/module-activation
// anchoring port (brep_production.rs, brep_identity.rs) adds ~16 kB packed
// (2365958 bytes measured). Newton one-sided C0-knot jets and half-open
// boundary ownership in brep-core intersections add ~4.8 kB packed
// (2401388 bytes measured). The analytic sphere/sphere SS cell (canonical
// sphere recognizer, outward classification, exact circle + UV lifts) adds
// ~6.6 kB packed (2407948 bytes measured). The axial analytic
// sphere/cylinder SS cell (canonical cylinder recognizer, band
// classification, side/cap circles + both UV lifts) adds ~7.4 kB packed
// (2415328 bytes measured). The analytic plane/sphere + plane/cylinder SS
// cells (canonical planar patch recognizer, certified angle classification,
// exact circle/ruling/ellipse + UV lifts, null oblique cylinder-side lift)
// add ~8.6 kB packed (2425233 bytes measured). The analytic plane/cone
// (frustum) SS cell (canonical frustum recognizer, certified angle
// classification, exact circle/ruling/ellipse/parabola/hyperbola + UV lifts,
// null cone-side lift for the oblique conics) adds ~12.5 kB packed (2437723
// bytes measured). The analytic plane/torus SS cell (canonical ring-torus
// recognizer, certified axial angle classification, exact parallel/meridian
// circle pairs + iso-u/iso-v torus lifts and plane-UV ellipse lifts, honest
// Cassini/Villarceau refusals) adds ~8.6 kB packed (2439918 -> 2448493 bytes
// measured).
// Release-qualified-v2 adds context-bound evidence, correspondence/sew/audit/
// naming, partial-contact Boolean, STEP identity, audited feature, tessellation,
// and mass certificates. The source-bound packed chunk measures 2651548 bytes;
// retain a bounded 18452-byte margin. Geometry Closure V3 adds proof-bound
// healing, finite graph-patch Boolean successors, and direct STEP topology;
// the final packed chunk is 2728218 bytes. Retain a bounded 21782-byte margin.
// The bounded multi-span Boolean certificate and exact branch/UV proof payload
// produce a 2759353-byte chunk. Retain a bounded 40647-byte margin.
// V11 direct IGES/STEP parsers produce a measured 2813833-byte packed chunk;
// retain a bounded 36167-byte margin without adding a second kernel payload.
// STEP /8 whole-domain regularity and coupled-sense certificates measure
// 2862093 bytes; retain a bounded 7907-byte margin.
// NURBS Foundation /2 adds stationary/root and 2D projection isolation,
// homogeneous normal cones and rollback-certified simplification. The shared
// packed kernel measures 2898818 bytes; retain a bounded 31182-byte margin.
// NURBS Foundation /4 adds Krawczyk uniqueness, recursive singularity
// localization, exact map materialization and cloud Hausdorff fitting. The
// shared packed kernel measures 2937483 bytes; retain a bounded 32517-byte margin.
// NURBS SS /1 adds certified general surface/surface intersection with 4D
// Bernstein/Krawczyk/continuation; GPU SDF view bridging added more bridge
// surface. The shared packed kernel measures 3060308 bytes; retain a bounded
// 39692-byte margin.
// Periodic rebuild and cyclic Greville collocation: measured 3,106,403 bytes.
// Certified curve endpoint matching: measured 3,144,213 bytes.
// Profile cycle assembly and exact orientation checks: measured 3,147,988 bytes.
// Retained arc assembly and common snap endpoints: measured 3,153,648 bytes.
// Analytic retained region offset: measured 3,156,133 bytes.
// Native display refinement, picking map and normals: measured 3,160,333 bytes.
// Point-driven rational curve trim: measured 3,162,248 packed bytes.
// Interval NURBS curve distance: measured packed geometry 3,172,373 bytes.
// Surface distance and interval tensor restriction: measured 3,180,728 bytes.
// Radial bounds and shell distance kernels: measured 3,200,233 bytes.
// Full-interval boundary diagnostics: measured 3,218,328 packed bytes.
// Face-contact diagnostics: measured packed kernel 3,226,638 bytes.
// Shared-edge plane criterion: measured packed kernel 3,230,813 bytes.
// Opposite-side shared-edge criterion: measured 3,234,683 packed bytes.
// Exact rational Bezier identity: measured 3,236,243 packed bytes.
// Prismatic cap Push/Pull: measured geometry chunk 3,240,213 bytes.
// Incidence-based cap identity preservation: measured 3,243,603 bytes.
// User-approved allowance: 15,000 bytes above the previous geometry budget.
// CAD commands, rational surfaces and analytic feature kernels: measured 3,550,136 bytes.
const geometryChunkBudget = 3_244_000 + 15_000 + 292_000
const jsChunkBudgets = [
  // CAD worker with lazy RushGraph importer: surface boundary diagnostics included: measured 108,116 bytes.
  [/^assets\/mainSolid\.worker-[^/]+\.js$/, 136_000], // CAD command and contact diagnostics dispatch: measured 135,579 bytes. // Distance kernel worker additions: measured 115,579 bytes. Surface distance worker dispatch: measured 115,110 bytes. Retained NURBS snap intervals: measured 114,200 bytes. Sketch snap preparation: measured 113,885 bytes. Body snap preparation: measured 112,083 bytes. Face sketch preparation in worker: measured 109,472 bytes.
  // After removing logical-expression payload inlining: 470353 / 84511 / 34064 bytes.
  // CAD command and rational-surface worker surface: measured 554,484 bytes.
  [/^assets\/geometry\.worker-[^/]+\.js$/, 555_000],
  // Theme uniforms + default-material setters added ~0.8 kB; measured: 100,763 bytes.
  // Textured matcap (group-2 capture binding, texture loading, material alpha)
  // adds ~4 kB; measured: 107,981 bytes.
  // PBR environment maps (group-2 equirect binding, setEnvMap loading)
  // add ~6 kB; measured: 113,721 bytes.
  // Key-light contact shadows (depth-only pass, shadow-map bindings, UI
  // toggle) add ~13 kB; measured: 126,965 bytes.
  [/^assets\/renderer-[^/]+\.js$/, 132_000],
  [/^assets\/svg\.worker-[^/]+\.js$/, 50_000],
  // Split OpenSCAD language kernel bytes, measured: 436,678 bytes.
  // Rational-surface and transform-plan language kernels: measured 516,458 bytes.
  [/^assets\/language-kernel-bytes-[^/]+\.js$/, 517_000],
  // Photogrammetry kernel bytes, measured after WGSL variants: 243,472 bytes.
  [/^assets\/photogrammetry-bytes-[^/]+\.js$/, 270_000],
  // Eager app shell, measured: 211,966 bytes.
  [/^assets\/index-[^/]+\.js$/, 240_000],
  // Packed HarfBuzz runtime, measured: 179,520 bytes.
  [/^assets\/harfbuzz-bytes-[^/]+\.js$/, 200_000],
  // Direct modeling panel with command guidance, inline dimensions, and isolation.
  // Workspace recovery and mesh diagnostics: measured 195,146 bytes.
  // Exact edge authoring adds the canonical capability registry and native wrappers (measured 220614 bytes).
  // Instance editing, draft recovery, exchange, diagnostics materials, SVG exchange and localized patch diagnostics and framed sweep controls: about 279 kB.
  // Profile region commands and target selection: measured 310,394 bytes.
  [/^assets\/DirectModeler-[^/]+\.js$/, 378_000], // Curve chain inspection and offset controls: measured 377,284 bytes. // Current UI with calculation retry/localized worker failures and VR controls: 376,153 bytes; 847-byte headroom. // Face-contact panel: measured 369,457 bytes. Distance kernels panel growth: measured 364,044 bytes. Radial bound diagnostics: measured 358,648 bytes. Trimmed face distance: measured 353,799 bytes. Full NURBS surface distance panel: measured 348,231 bytes. NURBS edge distance controls and witnesses: measured 344,995 bytes. All mesh contacts, completion and navigation: measured 340,273 bytes. Diagnostic input errors, retry and focus: measured 337,331 bytes. Screen point picking and focus: measured 334,107 bytes. Point trim preview and numeric inputs: measured 332,651 bytes. Retained NURBS targets in world coordinates: measured 328,305 bytes. Async body snap readiness: measured 327,041 bytes. Async authored edges: measured 326,006 bytes. Async topology selection: measured 325,581 bytes. Localized CV errors and accessible field association: measured 324,730 bytes. Cancellable retained profile display: measured 323,401 bytes. Cancellable surface display queue: measured 320,648 bytes. Worker startup recovery: measured 317,274 bytes. Cancellable JSON import: measured 316,313 bytes. Durable draft head and async restoration; previously async extrusion preview: measured 312,541 bytes.
  // Modeling tools with validated transferable G-code moves, measured: 100,285 bytes.
  [/^assets\/MainModelingTools-[^/]+\.js$/, 102_000],
  // WASM brotli unpacking helper chunk, measured: 122,900 bytes.
  [/^assets\/wasm-brotli-bytes-[^/]+\.js$/, 140_000],
  [/^assets\/vr-core-bytes-[^/]+\.js$/, 76_000], // Independent VR core: 56,115 raw WASM bytes, base64 packed.
]
const namedJsBudgetThreshold = 100_000
// WASM is losslessly packed in JS chunks; validate its actual decoded module
// and source identity below instead of relying on an artifact's file suffix.
for (const required of ['.html', '.css', '.js']) {
  if (!files.some(file => file.extension === required)) throw new Error(`dist is missing a ${required} artifact`)
}
for (const file of files) {
  const streamingWasmLimit = /^wasm\/(geometry-kernel|language-kernel|photogrammetry)\.wasm$/.test(file.path)
    ? 16_000_000
    : undefined
  const explicitJsLimit = jsChunkBudgets.find(([pattern]) => pattern.test(file.path))?.[1]
  const limit = streamingWasmLimit
    ?? (/^assets\/geometry-kernel-bytes-[^/]+\.js$/.test(file.path) ? geometryChunkBudget : undefined)
    ?? explicitJsLimit
    ?? limits.get(file.extension)
  if (file.bytes <= 0) throw new Error(`dist artifact ${file.path} is empty`)
  if (file.extension === '.js' && file.bytes > namedJsBudgetThreshold && limit === undefined) {
    throw new Error(`dist artifact ${file.path} is ${file.bytes} bytes; add an explicit named JS budget`)
  }
  if (limit !== undefined && file.bytes > limit) {
    throw new Error(`dist artifact ${file.path} is ${file.bytes} bytes; budget is ${limit}`)
  }
}
const total = files
  .filter(file => !/^wasm\/(geometry-kernel|language-kernel|photogrammetry)\.wasm$/.test(file.path))
  .reduce((sum, file) => sum + file.bytes, 0)
verifyUniquePackedWasm((function* () {
  for (const file of files) {
    if (file.extension === '.js') yield {path: file.path, source: readFileSync(join(root, file.path), 'utf8')}
  }
})())
// Own CAD adds ~96 kB to the shared Rust payload and must not ship a separate
// foreign CSG package. The complete distribution is smaller (~2.3 MB).
if (files.some(file => /manifold-3d/i.test(file.path))) throw new Error('Foreign manifold-3d artifact in dist')
// opt-level=3 for polygon-core measured 310 -> 240 ms warm on the own-cad
// benchmark (identical triangles) at +68 kB packed; sdf/nurbs/geometry-bridge
// at opt-level=3 added size without speed, so they keep the size profile.
const geometryBytes = files.filter(file => /^assets\/geometry-kernel-bytes-[^/]+\.js$/.test(file.path))
if (geometryBytes.length !== 1 || geometryBytes[0].bytes > geometryChunkBudget) {
  throw new Error(`Expected one shared geometry kernel chunk within ${geometryChunkBudget} bytes`)
}
verifyPackedWasmChunk(
  readFileSync(new URL(geometryBytes[0].path, rootUrl), 'utf8'),
  readFileSync(new URL('../src/generated/geometry-kernels/kernel_bg.wasm', import.meta.url)),
  'Geometry kernel',
)
const languageBytes = files.filter(file => /^assets\/language-kernel-bytes-[^/]+\.js$/.test(file.path))
if (languageBytes.length !== 1) throw new Error('Expected one shared packed language kernel')
verifyPackedWasmChunk(
  readFileSync(new URL(languageBytes[0].path, rootUrl), 'utf8'),
  readFileSync(new URL('../src/generated/language-kernel/kernel_bg.wasm', import.meta.url)),
  'Language kernel',
)
let rawWasmBytes = 0
for (const [name, original] of [
  ['geometry-kernel', '../src/generated/geometry-kernels/kernel_bg.wasm'],
  ['language-kernel', '../src/generated/language-kernel/kernel_bg.wasm'],
  ['photogrammetry', '../crates/target/wasm32-unknown-unknown/release/photogrammetry_wasm.wasm'],
]) {
  rawWasmBytes += verifyRawWasm(
    readFileSync(new URL(`wasm/${name}.wasm`, rootUrl)),
    readFileSync(new URL(original, import.meta.url)),
    name,
  )
}
const harfBuzzBytes = files.filter(file => /^assets\/harfbuzz-bytes-[^/]+\.js$/.test(file.path))
if (harfBuzzBytes.length !== 1) throw new Error('Expected one shared packed HarfBuzz runtime')
verifyPackedWasmChunk(
  readFileSync(new URL(harfBuzzBytes[0].path, rootUrl), 'utf8'),
  readFileSync(createRequire(import.meta.url).resolve('harfbuzzjs/hb.wasm')),
  'HarfBuzz',
)
for (const [name, artifact, compression] of [
  ['photogrammetry-bytes', 'photogrammetry_wasm', 'brotli'],
  ['wasm-brotli-bytes', 'wasm_brotli', 'deflate'],
]) {
  const packed = files.filter(file => new RegExp(`^assets/${name}-[^/]+\\.js$`).test(file.path))
  if (packed.length !== 1) throw new Error(`Expected one shared ${name} runtime`)
  verifyPackedWasmChunk(
    readFileSync(new URL(packed[0].path, rootUrl), 'utf8'),
    readFileSync(new URL(`../crates/target/wasm32-unknown-unknown/release/${artifact}.wasm`, import.meta.url)),
    name,
    compression,
  )
}
// Integrated distribution: 3D lattice adds ~13 kB; the current photo worker
// adds ~26 kB independently. Field traits, record updates and ret functions add
// ~15 kB over the previous 2928506-byte build. Measured total: 2943117 bytes.
// Trait methods, defaults and associated types add ~18 kB (2960752 bytes total).
// Photogrammetry round-2 optimizations grow the kernel (~82 kB raw, +13 kB packed)
// but the worker no longer embeds the kernel: it loads lazily once per page
// (2976203 bytes total).
// Round-3 accuracy modes (joint refinement, outlier filter, depth priors,
// red-black propagation, fusion merge; opt-in, default path byte-identical)
// add ~22 kB packed (2998447 bytes total). Phase-3 browser WebGPU sweep adds
// the shared WGSL text and the worker/parse plumbing (3014297 bytes total).
// polygon-core opt-level=3 (measured -22.5% geometry time) adds ~68 kB
// packed (3082382 bytes total). The browser SDF WebGPU sweep adds the flat
// field payload ops and the runner (3098760 bytes total).
// Keep a bounded margin; individual chunk limits remain unchanged.
// The OpenSCAD language frontend (openscad-core lexer/parser/AST, migration
// stage 1) adds ~5 kB packed to the shared kernel (3115359 bytes total).
// The OpenSCAD value evaluator (openscad-core eval/builtins, migration stage
// 2) adds ~110 kB packed to the shared kernel chunk (1106620 bytes; chunk
// budget raised to 1200000) and ~110 kB to the total (3225631 bytes measured).
// SVG runtime, panel and complete font/dependency notices bring the measured
// distribution to ~4.5 MB. Keep the shared-code and per-artifact checks intact.
// The curve/ruled-surface intersection query (brep-core intersections.rs:
// 3x3 Newton (t,u,v) isolation, ruling/iso-v coincidence lifting) adds ~28 kB
// to the packed geometry chunk (2365958 -> 2393603 bytes; chunk budget
// unchanged at 2400000) and ~17 kB to the total (4695432 -> 4712137 bytes).
// The analytic sphere/sphere SS cell adds ~6.6 kB packed and the same to the
// total (4719922 -> 4726482 bytes measured), so both budgets move once.
// The axial analytic sphere/cylinder SS cell adds ~7.4 kB packed and the
// same to the total (4726482 -> 4733862 bytes measured).
// The parallel-axis analytic cylinder/cylinder SS cell adds ~1.3 kB packed
// and the same to the total (4733862 -> 4735172 bytes measured); both
// budgets unchanged.
// The analytic plane/sphere + plane/cylinder SS cells add ~8.6 kB packed and
// the same to the total (4735172 -> 4743767 bytes measured), so both
// budgets move once.
// The analytic plane/cone (frustum) SS cell adds ~12.5 kB packed and the
// same to the total (4743767 -> 4756257 bytes measured), so both budgets
// move once.
// The analytic plane/torus SS cell adds ~8.6 kB packed and the same to the
// total (4758452 -> 4767027 bytes measured), so both budgets move once.
// The coaxial analytic cone/torus SS cell adds ~5.7 kB packed and the same
// to the total (4771577 -> 4777287 bytes measured; chunk 2453043 ->
// 2458753 bytes), so both budgets move once.
// The coaxial analytic torus/torus SS cell (the last canonical-primitive
// analytic SS cell) adds ~0.6 kB packed and the same to the total
// (4777287 -> 4777922 bytes measured; chunk 2458753 -> 2459388 bytes);
// both budgets unchanged.
// Release-qualified-v2 measures 5137553 bytes across 68 artifacts. Geometry
// Closure V3 measures 5215998 bytes across the same 68 artifacts. Retain a
// bounded distribution margin alongside the chunk-specific gate. The bounded
// multi-span Boolean successor remains capped by the stricter chunk gate above.
// Mesh import/convert (STL/OBJ/PLY/OFF/AMF/3MF → any export format) adds a
// lazily loaded ~42 kB converter chunk that reuses the OpenSCAD import()
// decoders outside the geometry worker (5261863 -> 5308461 bytes measured;
// the eager index chunk and the geometry chunk are unchanged), so the total
// budget moves once.
// The workspace redesign (single top bar with an export dialog, dock tabs, per-mode command palettes,
// icon toolbars and the Solid WebGPU display layer) adds ~57 kB to the eager index chunk
// (5308461 -> 5406747 bytes measured; the geometry chunk is unchanged), so the total budget moves once.
// Splitting the OpenSCAD and RushGraph frontends into their own kernel (language-kernel-bytes)
// trades ~150 kB of total distribution for a geometry kernel that drops from 8811985 to 6591424
// bytes unpacked: it instantiates on the main thread again, and a session that never compiles
// source never fetches the 445 kB language chunk (5406202 -> 5557245 bytes measured).
// NURBS SS /1 adds general surface/surface to the packed kernel; the budget above already covers it.
// Raw streaming modules remain separately bounded above. Removing three inlined
// geometry payload copies reduces the JS/assets total to 5,776,741 bytes.
// Direct modeling grid, snapping, solid geometry, and extrusion controls add
// 254,082 bytes to the measured distribution; scene theming/material controls
// and the GPU SDF bridge grew it further (6,342,515 total). Retain a bounded
// margin for this feature family without removing the total-size gate.
// Textured matcap support adds four 256×256 capture PNGs (~89 kB) plus ~4 kB
// of renderer code (6,449,885 bytes measured); the budget moves once.
// PBR environment maps add three 512×256 equirect PNGs (~63 kB) plus ~6 kB of
// renderer code (6,519,598 bytes measured); the budget moves once.
// Current editor/tool bundle: 6.58 MB; retain a bounded 22 kB margin.
// Sketch slot kernel, validated dimension UI and constraint status: measured 6,602,602 bytes.
// Qualified exact edge authoring with capability checks: measured 6,639,504 bytes.
// Variable-radius and three-edge blend controls: bounded additional 10 kB.
// Curve/surface rebuild UI; updated native kernel remains within this bounded increment.
// IndexedDB fallback for large Solid drafts adds a bounded storage/recovery path.
// Patch gap markers and bilingual rational-boundary guidance: measured 6,720,201 bytes.
// Framed sweep native payload and UI: measured 6,728,937 bytes.
// Certified surface jets, regularity and preview: measured 6,764,071 asset bytes.
// Open/periodic seam preparation and UI: measured 6,776,051 asset bytes.
// Explicit periodic unlinking: measured 6,778,259 asset bytes.
// Curve G1 native proof, command preview and endpoint guides: measured 6,787,376 asset bytes.
// Profile preparation with gap markers and command panel: measured 6,797,062 asset bytes.
// Retained profile document/render/transform/extrusion bridge: measured 6,805,989 asset bytes including GPU loss fallback.
// Arc profile preparation in the existing command: measured 6,812,553 asset bytes including repeat after undo.
// Retained profile difference/intersection and role controls: measured 6,815,572 bytes after command panel layout.
// Round offset with holes and complete preview loops: measured 6,820,375 asset bytes.
// Worker extrusion imports and cancellation state: measured 6,835,576 asset bytes.
// Body edit worker dependencies and cancellation state: measured 6,856,284 bytes.
// Profile preparation report validation: measured 6,858,268 bytes.
// Surface construction worker and refinement report validation: measured 6,860,202 bytes.
// Matching and seam preparation worker reports: measured 6,862,039 bytes.
// Worker display preparation and cancellation: measured 6,889,592 asset bytes.
// Worker RushGraph import: measured 6,894,083 asset bytes.
// Cancellable surface-boundary report and protocol: measured 6,896,876 asset bytes.
// Worker surface display queue and retry controls: measured 6,900,283 asset bytes.
// Retained profile display queue and protocol add about 3 kB of UI/worker code.
// Topology worker protocol validates face/edge indices: measured 6,906,147 bytes.
// Authored edge worker protocol and UI: measured 6,908,108 bytes.
// Body snap worker preparation and descriptor validation add about 4 kB.
// Sketch snap worker and bounded protocol: measured 6,916,660 asset bytes.
// Retained NURBS snap intervals and world placement: measured 6,918,347 bytes.
// Point trim Rust/WASM operation and preview UI: measured 6,924,791 bytes.
// NURBS curve distance kernel, worker protocol and edge UI: measured 6,949,848 bytes.
// Complete NURBS surface distance: measured 6,962,603 asset bytes.
// Explicit audit completeness fields: measured 7,000,039 bytes after production build.
// Distance kernel bundles growth: measured 7,008,135 asset bytes after production build.
// Boundary diagnostics, protocol and panel: measured 7,023,018 asset bytes.
// Face-contact kernel, worker validation and panel: measured 7,039,890 asset bytes.
// Shared-edge report, protocol and panel: measured 7,045,011 asset bytes.
// Two shared-edge certificate variants: measured 7,049,056 asset bytes.
// Keyboard face selection adds 937 bytes; measured assets 7,057,203.
// Explicit quantity labels and linked errors: measured assets 7,062,786 bytes.
// WebXR stereo viewer and controls across all workspaces: measured 7,072,458 bytes.
// Separate VR core payload plus current CAD retry/error UI: measured 7,149,470 bytes.
// The same geometry allowance also applies to the aggregate asset budget.
// CAD commands and rational-surface kernels across all bundles: measured 7,685,057 bytes.
// Stable-profile projective transform operation: measured 7,686,457 bytes.
const totalBudget = 7_152_000 + 15_000 + 519_000 + 1_000
if (total > totalBudget) throw new Error(`dist totals ${total} bytes; budget is ${totalBudget}`)
console.log(`Verified ${files.length} dist artifacts (${total} asset bytes + ${rawWasmBytes} raw WASM bytes = ${total + rawWasmBytes} total bytes)`)
