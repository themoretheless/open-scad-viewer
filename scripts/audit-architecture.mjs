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
import {unreachableRustFiles} from './rustModuleTree.mjs'
import {FUNCTION_LINE_LIMIT, measureFunctions} from './functionSizes.mjs'
import {MODULE_LAYERS, upwardModuleEdges} from './moduleLayers.mjs'
import {upwardSrcEdges} from './srcDomains.mjs'

export const SIZE_LIMITS = {'.rs': 800, '.ts': 500, '.mts': 500, '.mjs': 500, '.svt': 400}
const SIZE_EXCLUDE = [/^crates\/vendor\//, /\/generated\//, /\.golden\.ts$/, /\.d\.ts$/, /(^|\/)tests?\//, /\.test\.[cm]?ts$/]

// Lower layers never depend on higher ones; same-layer dependencies are allowed.
export const CRATE_LAYERS = {
  'value-codec': 0, 'math-core': 0, 'cad-predicates': 0, 'vr-core': 0, 'wasm-brotli': 0,
  'gpu-compute': 1, 'compute-core': 1, 'tensor-core': 1, 'compute-cuda': 1, 'compute-mlx': 1,
  'raster-core': 1, 'math-compute': 1, 'mesh-topology': 1, 'mesh-query': 1, 'mesh-section': 1, 'mesh-io': 1,
  'geometry-ops': 2, 'planar-geometry': 2, 'polygon-core': 2, 'nurbs-core': 2, 'nurbs-intersect': 2, 'sdf-core': 2,
  'subdivision-core': 2, 'sketch-core': 2, 'brep-topology': 2, 'manifold-core': 2, 'manifold-ops': 2,
  'brep-core': 3,
  'cad-step': 4, 'brep-intersect': 4, 'rush-runtime': 4, 'rush-frontend': 4,
  'openscad-core': 4, 'modelgraph-runtime': 4, 'modelgraph-text': 4, 'mechanical-core': 4,
  'mechanics-core': 4, 'gcode-core': 4, 'gcode-optimize': 4, 'slicer-core': 4, 'laser-core': 4,
  'printer-core': 4, 'photogrammetry-core': 4,
  'bridge-codec': 0,
  'bridge-cam': 5, 'bridge-analysis': 5, 'bridge-svg': 5, 'geometry-bridge': 5, 'languages-bridge': 5, 'photogrammetry-ffi': 5, 'printer-cli': 5,
  'geometry-wasm': 6, 'languages-wasm': 6, 'photogrammetry-wasm': 6,
}

// Names produced by splitting a file to satisfy the size limit rather than by responsibility.
export const MECHANICAL_NAME = /(^|\/)(methods_[^/]*|[^/]*_(impl|cases|part)_\d+)\.rs$/

export function mechanicalNames(files) {
  return files.filter(path => path.startsWith('crates/') && !path.startsWith('crates/vendor/') && MECHANICAL_NAME.test(path)).sort()
}

/** Pinned path lists: a listed path may disappear, a new path fails. */
function auditPinnedList(observed = [], pinned = [], label, advice) {
  const errors = []
  for (const path of observed) if (!pinned.includes(path)) errors.push(`${path}: ${label}; ${advice}`)
  for (const path of pinned) if (!observed.includes(path)) errors.push(`${path}: no longer ${label}; remove it from the baseline (--update)`)
  return errors
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

export function auditArchitecture({sizes, layers, unreachable, mechanical, functions, moduleLayers, srcLayers}, baseline) {
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
  if (functions) {
    const pinned = baseline.functions ?? {}
    for (const [key, lines] of Object.entries(functions)) {
      if (pinned[key] === undefined) errors.push(`${key}: function has ${lines} lines, limit ${FUNCTION_LINE_LIMIT}; split it`)
      else if (lines > pinned[key]) errors.push(`${key}: function grew to ${lines} lines (pinned at ${pinned[key]}); long functions may only shrink`)
    }
    for (const key of Object.keys(pinned))
      if (functions[key] === undefined) errors.push(`${key}: function is now within limits or gone; remove it from the baseline (--update)`)
  }
  if (moduleLayers) errors.push(...auditPinnedList(moduleLayers, baseline.moduleLayers, 'upward module dependency (ADR 0012)', 'move the code down a layer or the dependency up'))
  if (srcLayers) errors.push(...auditPinnedList(srcLayers, baseline.srcLayers, 'upward src domain import (plan M10)', 'move the code down a domain or the import up'))
  if (unreachable) errors.push(...auditPinnedList(unreachable, baseline.unreachable, 'not compiled by any Cargo target', 'declare it with `mod` or delete it'))
  if (mechanical) errors.push(...auditPinnedList(mechanical, baseline.mechanical, 'mechanically split file name', 'name the module after its responsibility'))
  return errors
}

export function nextBaseline({sizes, layers, unreachable = [], mechanical = [], functions = {}, moduleLayers = [], srcLayers = []}, baseline) {
  const pinned = {}
  for (const [path, lines] of Object.entries(sizes).sort(([a], [b]) => a.localeCompare(b)))
    pinned[path] = Math.min(lines, baseline.sizes[path] ?? lines)
  const keep = (observed, list = []) => observed.filter(path => list.includes(path))
  // Keys owned by other audits (for example `clippy`) pass through unchanged.
  return {...baseline, sizes: pinned, layers: layers.filter(v => baseline.layers.includes(v)),
    unreachable: keep(unreachable, baseline.unreachable), mechanical: keep(mechanical, baseline.mechanical),
    moduleLayers: keep(moduleLayers, baseline.moduleLayers),
    srcLayers: keep(srcLayers, baseline.srcLayers),
    functions: Object.fromEntries(Object.entries(functions).filter(([key]) => baseline.functions?.[key] !== undefined)
      .map(([key, lines]) => [key, Math.min(lines, baseline.functions[key])]).sort(([a], [b]) => a.localeCompare(b)))}
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const root = resolve(fileURLToPath(import.meta.url), '../..')
  const baselinePath = resolve(root, 'architecture-baseline.json')
  const files = execFileSync('git', ['ls-files', '--cached', '--others', '--exclude-standard', '--', 'crates', 'src', 'scripts'], {cwd: root, encoding: 'utf8', maxBuffer: 1 << 28}).split('\n').filter(Boolean)
  const present = files.filter(path => existsSync(resolve(root, path)))
  const observed = {sizes: measureSizes(root, files), layers: layerViolations(crateEdges(root)),
    unreachable: unreachableRustFiles(root, present), mechanical: mechanicalNames(present),
    functions: measureFunctions(root, present, FROZEN_ARCHIVES),
    moduleLayers: Object.keys(MODULE_LAYERS).flatMap(crate => upwardModuleEdges(root, crate, present)),
    srcLayers: upwardSrcEdges(root, present)}
  const baseline = existsSync(baselinePath)
    ? JSON.parse(readFileSync(baselinePath, 'utf8'))
    : {sizes: {}, layers: []}
  if (process.argv.includes('--init')) {
    writeFileSync(baselinePath, JSON.stringify({sizes: Object.fromEntries(Object.entries(observed.sizes).sort()), layers: observed.layers,
      unreachable: observed.unreachable, mechanical: observed.mechanical,
      functions: Object.fromEntries(Object.entries(observed.functions).sort())}, null, 2) + '\n')
  } else if (process.argv.includes('--update')) {
    writeFileSync(baselinePath, JSON.stringify(nextBaseline(observed, baseline), null, 2) + '\n')
  } else {
    const errors = auditArchitecture(observed, baseline)
    for (const error of errors) console.error(error)
    if (errors.length) process.exit(1)
    console.log(`architecture ok: ${Object.keys(baseline.sizes).length} pinned oversized files, ${baseline.layers.length} pinned layer violations, `
      + `${(baseline.unreachable ?? []).length} pinned uncompiled Rust files, ${(baseline.mechanical ?? []).length} pinned mechanical file names, `
      + `${Object.keys(baseline.functions ?? {}).length} pinned long functions, ${(baseline.moduleLayers ?? []).length} pinned upward module edges, ${(baseline.srcLayers ?? []).length} pinned upward src domain imports`)
  }
}
