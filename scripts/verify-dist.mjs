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
const totalBudget = 2_200_000
if (total > totalBudget) throw new Error(`dist totals ${total} bytes; budget is ${totalBudget}`)
console.log(`Verified ${files.length} dist artifacts (${total} bytes)`)
