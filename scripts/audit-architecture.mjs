import {createHash} from 'node:crypto'
import {FROZEN_ARCHIVES} from './qualificationFrozenArchives.mjs'
// Architecture ratchet: source-file size limits and Rust crate layering.
// Existing violations are pinned in `architecture-baseline.json`; a pinned
// file may shrink but never grow, and a fixed violation must leave the
// baseline (`--update` rewrites it, lowering pins only).
import {execFileSync} from 'node:child_process'
import {readFileSync, writeFileSync, readdirSync, existsSync} from 'node:fs'
import {resolve, extname} from 'node:path'
import {fileURLToPath} from 'node:url'

export const SIZE_LIMITS = {'.rs': 800, '.ts': 500, '.mts': 500, '.mjs': 500, '.vue': 400}
const SIZE_EXCLUDE = [/^crates\/vendor\//, /\/generated\//, /\.golden\.ts$/, /\.d\.ts$/, /(^|\/)tests?\//, /\.test\.[cm]?ts$/]

// Lower layers never depend on higher ones; same-layer dependencies are allowed.
export const CRATE_LAYERS = {
  'value-codec': 0, 'math-core': 0, 'cad-predicates': 0, 'vr-core': 0, 'wasm-brotli': 0,
  'gpu-compute': 1, 'compute-core': 1, 'tensor-core': 1, 'compute-cuda': 1, 'compute-mlx': 1,
  'raster-core': 1, 'math-compute': 1,
  'geometry-ops': 2, 'planar-geometry': 2, 'polygon-core': 2, 'nurbs-core': 2, 'sdf-core': 2,
  'subdivision-core': 2, 'sketch-core': 2, 'brep-topology': 2,
  'brep-core': 3,
  'openscad-core': 4, 'modelgraph-runtime': 4, 'modelgraph-text': 4, 'mechanical-core': 4,
  'mechanics-core': 4, 'gcode-core': 4, 'gcode-optimize': 4, 'slicer-core': 4, 'laser-core': 4,
  'printer-core': 4, 'photogrammetry-core': 4,
  'bridge-codec': 0,
  'bridge-cam': 5, 'bridge-analysis': 5, 'bridge-svg': 5, 'geometry-bridge': 5, 'languages-bridge': 5, 'photogrammetry-ffi': 5, 'printer-cli': 5,
  'geometry-wasm': 6, 'languages-wasm': 6, 'photogrammetry-wasm': 6,
}

const countLines = text => text.length === 0 ? 0 : text.split('\n').length - (text.endsWith('\n') ? 1 : 0)

export function measureSizes(root, files) {
  const sizes = {}
  for (const path of files) {
    const limit = SIZE_LIMITS[extname(path)]
    if (limit === undefined || SIZE_EXCLUDE.some(pattern => pattern.test(path))) continue
    const full = resolve(root, path)
    if (!existsSync(full)) continue
    const bytes = readFileSync(full)
    // Historical executors are immutable evidence, not evolving source.
    if (FROZEN_ARCHIVES[path] !== undefined) {
      if (FROZEN_ARCHIVES[path] !== createHash('sha256').update(bytes).digest('hex'))
        throw new Error(`${path}: immutable qualification archive hash changed`)
      continue
    }
    const lines = countLines(bytes.toString('utf8'))
    if (lines > limit) sizes[path] = lines
  }
  return sizes
}

export function crateEdges(root) {
  const edges = []
  for (const crate of readdirSync(resolve(root, 'crates'))) {
    const manifest = resolve(root, 'crates', crate, 'Cargo.toml')
    if (crate === 'vendor' || !existsSync(manifest)) continue
    for (const [, dep] of readFileSync(manifest, 'utf8').matchAll(/path\s*=\s*"\.\.\/([\w-]+)"/g))
      edges.push([crate, dep])
  }
  return edges
}

export function layerViolations(edges) {
  const violations = new Set()
  for (const [from, to] of edges) {
    if (CRATE_LAYERS[from] === undefined) violations.add(`${from}: crate has no layer`)
    else if (CRATE_LAYERS[to] !== undefined && CRATE_LAYERS[to] > CRATE_LAYERS[from])
      violations.add(`${from} -> ${to}`)
  }
  return [...violations].sort()
}

export function auditArchitecture({sizes, layers}, baseline) {
  const errors = []
  for (const [path, lines] of Object.entries(sizes)) {
    const pinned = baseline.sizes[path]
    if (pinned === undefined) errors.push(`${path}: ${lines} lines exceeds limit ${SIZE_LIMITS[extname(path)]}; split it`)
    else if (lines > pinned) errors.push(`${path}: grew to ${lines} lines (pinned at ${pinned}); oversized files may only shrink`)
  }
  for (const path of Object.keys(baseline.sizes))
    if (sizes[path] === undefined) errors.push(`${path}: now within limits; remove it from the baseline (--update)`)
  for (const violation of layers)
    if (!baseline.layers.includes(violation)) errors.push(`layering: ${violation} depends upward`)
  for (const violation of baseline.layers)
    if (!layers.includes(violation)) errors.push(`layering: ${violation} is fixed; remove it from the baseline (--update)`)
  return errors
}

export function nextBaseline({sizes, layers}, baseline) {
  const pinned = {}
  for (const [path, lines] of Object.entries(sizes).sort(([a], [b]) => a.localeCompare(b)))
    pinned[path] = Math.min(lines, baseline.sizes[path] ?? lines)
  return {sizes: pinned, layers: layers.filter(v => baseline.layers.includes(v))}
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const root = resolve(fileURLToPath(import.meta.url), '../..')
  const baselinePath = resolve(root, 'architecture-baseline.json')
  const files = execFileSync('git', ['ls-files', '--', 'crates', 'src', 'scripts'], {cwd: root, encoding: 'utf8', maxBuffer: 1 << 28}).split('\n').filter(Boolean)
  const observed = {sizes: measureSizes(root, files), layers: layerViolations(crateEdges(root))}
  const baseline = existsSync(baselinePath)
    ? JSON.parse(readFileSync(baselinePath, 'utf8'))
    : {sizes: {}, layers: []}
  if (process.argv.includes('--init')) {
    writeFileSync(baselinePath, JSON.stringify({sizes: Object.fromEntries(Object.entries(observed.sizes).sort()), layers: observed.layers}, null, 2) + '\n')
  } else if (process.argv.includes('--update')) {
    writeFileSync(baselinePath, JSON.stringify(nextBaseline(observed, baseline), null, 2) + '\n')
  } else {
    const errors = auditArchitecture(observed, baseline)
    for (const error of errors) console.error(error)
    if (errors.length) process.exit(1)
    console.log(`architecture ok: ${Object.keys(baseline.sizes).length} pinned oversized files, ${baseline.layers.length} pinned layer violations`)
  }
}
