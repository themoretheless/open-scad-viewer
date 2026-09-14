import { readFileSync, readdirSync, statSync } from 'node:fs'
import { extname, join, relative } from 'node:path'
import { createRequire } from 'node:module'
import { verifyPackedWasmChunk } from './verify-packed-wasm.mjs'

const root = new URL('../dist/', import.meta.url)
const files = []
function walk(directory) {
  for (const name of readdirSync(directory)) {
    const path = join(directory, name)
    if (statSync(path).isDirectory()) walk(path)
    else files.push({ path: relative(root.pathname, path), bytes: statSync(path).size, extension: extname(path) })
  }
}
walk(root.pathname)

const limits = new Map([
  ['.html', 10_000],
  ['.css', 100_000],
  ['.js', 500_000],
  ['.wasm', 800_000],
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
const geometryChunkBudget = 2_460_000
// WASM is losslessly packed in JS chunks; validate its actual decoded module
// and source identity below instead of relying on an artifact's file suffix.
for (const required of ['.html', '.css', '.js']) {
  if (!files.some(file => file.extension === required)) throw new Error(`dist is missing a ${required} artifact`)
}
for (const file of files) {
  const limit = /^assets\/geometry-kernel-bytes-[^/]+\.js$/.test(file.path) ? geometryChunkBudget : limits.get(file.extension)
  if (file.bytes <= 0) throw new Error(`dist artifact ${file.path} is empty`)
  if (limit !== undefined && file.bytes > limit) {
    throw new Error(`dist artifact ${file.path} is ${file.bytes} bytes; budget is ${limit}`)
  }
}
const total = files.reduce((sum, file) => sum + file.bytes, 0)
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
  readFileSync(new URL(geometryBytes[0].path, root), 'utf8'),
  readFileSync(new URL('../src/generated/geometry-kernels/kernel_bg.wasm', import.meta.url)),
  'Geometry kernel',
)
const harfBuzzBytes = files.filter(file => /^assets\/harfbuzz-bytes-[^/]+\.js$/.test(file.path))
if (harfBuzzBytes.length !== 1) throw new Error('Expected one shared packed HarfBuzz runtime')
verifyPackedWasmChunk(
  readFileSync(new URL(harfBuzzBytes[0].path, root), 'utf8'),
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
    readFileSync(new URL(packed[0].path, root), 'utf8'),
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
const totalBudget = 4_780_000
if (total > totalBudget) throw new Error(`dist totals ${total} bytes; budget is ${totalBudget}`)
console.log(`Verified ${files.length} dist artifacts (${total} bytes)`)
