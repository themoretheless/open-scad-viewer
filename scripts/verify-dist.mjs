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
  const limit = limits.get(file.extension)
  if (file.bytes <= 0) throw new Error(`dist artifact ${file.path} is empty`)
  if (limit !== undefined && file.bytes > limit) {
    throw new Error(`dist artifact ${file.path} is ${file.bytes} bytes; budget is ${limit}`)
  }
}
const total = files.reduce((sum, file) => sum + file.bytes, 0)
// ModelGraph Text adds the bounded compiler to the geometry worker as well as the UI.
// Keep per-artifact limits above; allow the measured ~140 KiB compiler addition.
// The independent Rust geometry libraries add one shared, gzip-packed WASM
// chunk (~294 kB including mesh CSG and shared B-rep topology). Preserve every per-artifact limit and bound its allowance.
const geometryBytes = files.filter(file => /^assets\/geometry-kernel-bytes-[^/]+\.js$/.test(file.path))
if (geometryBytes.length !== 1 || geometryBytes[0].bytes > 480_000) {
  throw new Error('Expected one shared geometry kernel chunk within 480000 bytes')
}
// Direct NURBS text compilation and mesh publication add ~28 kB of host code.
// Typed block functions and generic records add ~21 kB across UI and worker.
const totalBudget = 2_200_000 + 480_000 + 30_000 + 30_000
if (total > totalBudget) throw new Error(`dist totals ${total} bytes; budget is ${totalBudget}`)
console.log(`Verified ${files.length} dist artifacts (${total} bytes)`)
