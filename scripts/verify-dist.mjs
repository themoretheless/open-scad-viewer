import { readdirSync, statSync } from 'node:fs'
import { extname, join, relative } from 'node:path'

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
for (const required of ['.html', '.css', '.js', '.wasm']) {
  if (!files.some(file => file.extension === required)) throw new Error(`dist is missing a ${required} artifact`)
}
for (const file of files) {
  const limit = /^assets\/geometry-kernel-bytes-[^/]+\.js$/.test(file.path) ? 900_000 : limits.get(file.extension)
  if (file.bytes <= 0) throw new Error(`dist artifact ${file.path} is empty`)
  if (limit !== undefined && file.bytes > limit) {
    throw new Error(`dist artifact ${file.path} is ${file.bytes} bytes; budget is ${limit}`)
  }
}
const total = files.reduce((sum, file) => sum + file.bytes, 0)
// Own CAD adds ~96 kB to the shared Rust payload but removes the separate
// Manifold WASM/JS assets. The complete distribution is smaller (~2.3 MB).
// Preserve the total release budget and reject any external Manifold artifact.
if (files.some(file => /manifold/i.test(file.path))) throw new Error('External Manifold artifact in dist')
const geometryBytes = files.filter(file => /^assets\/geometry-kernel-bytes-[^/]+\.js$/.test(file.path))
if (geometryBytes.length !== 1 || geometryBytes[0].bytes > 900_000) {
  throw new Error('Expected one shared geometry kernel chunk within 900000 bytes')
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
// Keep a bounded margin; individual chunk limits remain unchanged.
const totalBudget = 3_020_000
if (total > totalBudget) throw new Error(`dist totals ${total} bytes; budget is ${totalBudget}`)
console.log(`Verified ${files.length} dist artifacts (${total} bytes)`)
