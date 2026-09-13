// Rebuild the authoritative sources and qualify the SVG cycle; do not hand-edit fingerprints.
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const tests = [
  'tests/svgDocument.test.ts', 'tests/svgGeometry.test.ts', 'tests/svgProjectionValidation.test.ts',
  'tests/svgWorker.test.ts', 'tests/svgWorkerRealBoundary.test.ts', 'tests/svgDraftStore.test.ts',
  'tests/svgPanel.test.ts', 'tests/mcpSvgTools.test.ts', 'tests/openScadImport.test.ts',
  'tests/path2dCurvex.test.ts', 'tests/directProfileTools.test.ts', 'tests/directModelingTools.test.ts',
]
const nativeChecks = [
  ['test', '--manifest-path', 'crates/Cargo.toml', '-p', 'geometry-bridge', '--lib', 'svg', '--no-default-features', '--offline'],
  ['test', '--manifest-path', 'crates/Cargo.toml', '-p', 'planar-geometry', '--offline'],
]
const hash = path => createHash('sha256').update(readFileSync(path)).digest('hex')
function sources(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    if (['target', 'node_modules', 'generated', '.git'].includes(entry.name)) return []
    const path = join(directory, entry.name)
    // Generated qualification metadata is bound separately by its WASM hash;
    // including it here would make evidence refreshes invalidate each other.
    if (path === 'src/core/ownRustCadEvidence.ts') return []
    return entry.isDirectory() ? sources(path) : entry.isFile() ? [path] : []
  })
}
function fingerprint() {
  const paths = [...sources('crates'), ...sources('src'), ...sources('tests'), ...sources('scripts'),
    'package.json', 'package-lock.json', 'rust-toolchain.toml', 'tsconfig.json', 'tsconfig.mcp.json',
    'vite.config.ts', 'vitest.config.ts', 'THIRD_PARTY_NOTICES.md'].sort()
  return createHash('sha256').update(paths.map(path => `${path}\0${hash(path)}\n`).join('')).digest('hex')
}
const sourceSha256 = fingerprint()
execFileSync(process.execPath, ['scripts/build-geometry-kernels.mjs'], { stdio: 'inherit' })
if (fingerprint() !== sourceSha256) throw new Error('Repository sources changed during the SVG build; run qualification again after edits finish.')
const wasmPath = 'src/generated/geometry-kernels/kernel_bg.wasm'
const packedPath = 'src/generated/geometry-kernels/bytes.ts'
const decoderPath = 'src/generated/wasm-brotli/bytes.ts'
const wasmSha256 = hash(wasmPath), packedSha256 = hash(packedPath), decoderSha256 = hash(decoderPath)
mkdirSync('output', { recursive: true })
const report = 'output/svg-cycle-tests.json'
for (const args of nativeChecks) execFileSync('cargo', args, { stdio: 'inherit' })
execFileSync(process.execPath, ['node_modules/vitest/vitest.mjs', 'run', ...tests, '--maxWorkers=2', '--reporter=json', `--outputFile=${report}`], { stdio: 'inherit' })
const result = JSON.parse(readFileSync(report, 'utf8'))
if (!result.success || result.numFailedTests || result.numPendingTests || result.numPassedTests === 0) throw new Error('SVG cycle qualification is incomplete.')
if (hash(wasmPath) !== wasmSha256 || hash(packedPath) !== packedSha256 || hash(decoderPath) !== decoderSha256 || fingerprint() !== sourceSha256) throw new Error('Repository or WASM changed during SVG qualification; results were not recorded.')
const requirements = {
  artwork: ['CSS cascade and geometry properties, use/symbol inheritance, viewports, gradients, clipping, masks, filters', 'Bundled and supplied font outlines, explicit raster silhouette conversion'],
  geometry: ['Physical units, curve tolerance, compound holes, strokes and transformed projections', 'Non-scaling strokes, dashes, markers and per-instance paint resources in outer viewport coordinates', 'SVG → solid → SVG with independent area, volume, bounds and topology checks', 'Optional editable ModelGraph, nested holes/islands and explicit graph admission limits'],
  interaction: ['Real isolated worker, responsive host, warm reuse, cancellation and hard timeout', 'Stale source, settings, scene and destination rejection; source-capacity admission'],
  persistence: ['SVG, settings and exact font bytes restored through IndexedDB', 'Unfinished numeric fields, rapid saves, cross-tab conflicts and unavailable-storage recovery'],
  vectors: ['Path2D operations, direct contour corner/revolve tools and direct modeling tools'],
}
const evidence = {
  id: 'svg-static-cycle-v1', generatedAt: new Date().toISOString(), sourceSha256, wasmSha256, packedSha256, decoderSha256,
  tests, passedTests: result.numPassedTests, nativeChecks: nativeChecks.map(args => ({ command: ['cargo', ...args], status: 'passed' })), requirements,
  environment: { platform: process.platform, architecture: process.arch, node: process.version },
  scope: 'The bounded static SVG viewing/import/extrusion/export workflows, browser worker protocol and persistence, MCP roundtrips and adjacent vector operations. This is not universal SVG 2 conformance or whole-application release qualification.',
  limitations: ['Scripts, animation, foreignObject, external resources, DTDs and vector effects other than none/non-scaling-stroke are explicitly rejected.', 'CSS math, custom variables, viewport-relative lengths and cascade at-rules require resolved values.', 'Effects use opt-in pixel silhouettes; text requires the correct supplied outline font for matching metrics. Custom SVG/COLR/bitmap font tables are unsupported.', 'Reference/marker/text and non-scaling resource expansion use conservative budgets before normalization or resource cloning.', 'ModelGraph uses its own profile/node budgets; SCAD source and general transport limits apply.'],
}
mkdirSync('docs/qualification', { recursive: true })
writeFileSync('docs/qualification/svg-static-cycle-v1.json', JSON.stringify(evidence, null, 2) + '\n')
console.log(`Qualified ${result.numPassedTests} SVG/vector workflow tests against WASM ${wasmSha256}`)
