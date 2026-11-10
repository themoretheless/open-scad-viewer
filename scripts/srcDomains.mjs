// Domain layers of the TypeScript application (refactor plan M10, ARCHITECTURE section 8).
// A relative import from a lower layer to a higher one is an upward edge; existing edges are
// pinned in architecture-baseline.json and may only disappear.
import {readFileSync, existsSync} from 'node:fs'
import {resolve, posix} from 'node:path'

export const SRC_LAYERS = {
  core: 0, generated: 0, data: 0, i18n: 0, forma: 0,
  kernel: 1, language: 1, parser: 1, exchange: 1, render: 1, fabrication: 1,
  services: 2,
  composables: 3, components: 3, features: 3, ui: 3, dev: 3, workers: 3, mcp: 3, '(app)': 3,
}
const EXTS = ['', '.ts', '.tsx', '.mts', '.js', '.mjs', '.svt', '.vue', '/index.ts']
const SPEC = /(?:\bfrom\s*|\bimport\s*\(\s*|\bimport\s+|\bexport\s*\*\s*from\s*|\bnew\s+URL\s*\(\s*)(['"])(\.{1,2}\/[^'"?#]*)/g

function topOf(path) {
  const rest = path.slice('src/'.length)
  return rest.includes('/') ? rest.split('/')[0] : '(app)'
}

/** Sorted `from -> to` domain edges that point upward, for files under src/. */
export function upwardSrcEdges(root, files) {
  const present = new Set(files)
  const edges = new Set()
  for (const file of files) {
    if (!file.startsWith('src/') || !/\.(ts|tsx|mts|svt|vue)$/.test(file) || file.endsWith('.d.ts')) continue
    const from = topOf(file)
    if (SRC_LAYERS[from] === undefined) continue
    const text = readFileSync(resolve(root, file), 'utf8').replace(/\/\*[\s\S]*?\*\//g, ' ').replace(/^\s*\/\/[^\n]*/gm, ' ')
    for (const m of text.matchAll(SPEC)) {
      const base = posix.normalize(posix.join(posix.dirname(file), m[2]))
      const target = EXTS.map(e => base + e).find(c => present.has(c) || (c.startsWith('src/') && existsSync(resolve(root, c)) && !c.endsWith('/')))
      if (!target || !target.startsWith('src/')) continue
      const to = topOf(target)
      if (to === from || SRC_LAYERS[to] === undefined) continue
      if (SRC_LAYERS[to] > SRC_LAYERS[from]) edges.add(`${from} -> ${to}`)
    }
  }
  return [...edges].sort()
}
