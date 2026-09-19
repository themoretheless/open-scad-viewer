// CSG scaling probe through the production OpenSCAD evaluator and own-Rust kernel.
//
//   node --import tsx benchmarks/own-cad/bench-csg-scaling.mts [--out file.json] [case ...]
//
// Each case is one whole parseOpenSCAD() call at full quality, so timings include
// evaluation, kernel Booleans and mesh analysis but not Worker transport or GPU.
// A failing case is recorded, not fatal: the point of the probe is to find the
// input size at which the kernel refuses or slows down. Volumes are checked
// against the analytic value where one exists, so a faster wrong answer is
// reported as a failure.
import { readFileSync, writeFileSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import os from 'node:os'
import { parseArgs } from 'node:util'
import { parseOpenSCAD } from '../../src/services/openscadParser'

const polygonArea = (radius: number, segments: number) => segments / 2 * radius * radius * Math.sin(2 * Math.PI / segments)

interface Case { source: string; expectedVolume?: number }

function drilledPlate(count: number, segments: number): Case {
  const side = Math.ceil(Math.sqrt(count))
  const step = 80 / side
  const radius = Math.min(3, step / 2 - 0.5)
  const holes = Array.from({ length: count }, (_, i) =>
    `translate([${-40 + step / 2 + (i % side) * step},${-40 + step / 2 + Math.floor(i / side) * step},0]) cylinder(h=12, r=${radius}, center=true);`)
  return {
    source: `$fn=${segments};\ndifference() {\n  cube([86,86,8], center=true);\n  ${holes.join('\n  ')}\n}`,
    expectedVolume: 86 * 86 * 8 - count * polygonArea(radius, segments) * 8,
  }
}

function sphericalPockets(count: number, segments: number): Case {
  const side = Math.ceil(Math.sqrt(count))
  const step = 80 / side
  const pockets = Array.from({ length: count }, (_, i) =>
    `translate([${-40 + step / 2 + (i % side) * step},${-40 + step / 2 + Math.floor(i / side) * step},4]) sphere(r=3);`)
  return { source: `$fn=${segments};\ndifference() {\n  cube([86,86,8], center=true);\n  ${pockets.join('\n  ')}\n}` }
}

const separatedSpheres = (count: number, segments: number): Case => ({
  source: `$fn=${segments}; union() { ${Array.from({ length: count }, (_, i) => `translate([${i * 70},0,0]) sphere(r=30);`).join(' ')} }`,
})

const overlappingSpheres = (segments: number): Case => ({
  source: `$fn=${segments}; union() { sphere(r=30); translate([30,0,0]) sphere(r=30); }`,
})

const cases: Record<string, Case> = {}
for (const n of [1, 4, 16, 36, 64, 100]) cases[`difference/plate-${n}-holes-fn32`] = drilledPlate(n, 32)
for (const n of [16, 64]) cases[`difference/plate-${n}-spherical-pockets-fn16`] = sphericalPockets(n, 16)
for (const fn of [32, 64, 128, 192]) cases[`union/3-separated-spheres-fn${fn}`] = separatedSpheres(3, fn)
for (const fn of [32, 48, 64]) cases[`union/2-overlapping-spheres-fn${fn}`] = overlappingSpheres(fn)

const { values, positionals } = parseArgs({ allowPositionals: true, options: {
  out: { type: 'string' }, samples: { type: 'string', default: '7' }, warmups: { type: 'string', default: '2' },
} })
const sampleCount = Number(values.samples), warmups = Number(values.warmups)
if (!Number.isInteger(sampleCount) || sampleCount < 1 || sampleCount > 50 || !Number.isInteger(warmups) || warmups < 0 || warmups > 10) throw new Error('samples must be 1..50 and warmups 0..10')
const out = values.out
const selected = positionals.length ? positionals : Object.keys(cases)
const hash = (value: string | Uint8Array) => createHash('sha256').update(value).digest('hex')
const inputPaths = ['benchmarks/own-cad/bench-csg-scaling.mts', 'src/services/openscadParser.ts', 'src/services/cadKernelOps.ts', 'src/services/geometry/meshAnalysis.ts', 'src/services/geometry/kernel.ts', 'src/generated/geometry-kernels/bytes.ts', 'src/generated/geometry-kernels/kernel_bg.wasm', 'src/generated/wasm-brotli/bytes.ts', 'src/generated/language-kernel/bytes.ts']
const fingerprints = () => inputPaths.map(path => ({ path, sha256: hash(readFileSync(path)) }))
const files = fingerprints()
const source = { head: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(), files }
const median = (samples: number[]) => { const sorted = [...samples].sort((a,b) => a-b); const i = Math.floor(sorted.length / 2); return sorted.length % 2 ? sorted[i]! : (sorted[i-1]! + sorted[i]!) / 2 }
const rows: Array<Record<string, unknown>> = []
for (const id of selected) {
  const item = cases[id]
  if (!item) throw new Error(`Unknown case ${id}`)
  const samples: Array<{ wallMs: number; evaluateMs: number; analyzeMs: number }> = []
  let row: Record<string, unknown> | undefined
  for (let iteration = 0; iteration < warmups + sampleCount && !row; iteration++) {
    const start = performance.now()
    try {
      const result = await parseOpenSCAD(item.source, { quality: 'full' })
      const ms = performance.now() - start
      const volumeError = item.expectedVolume === undefined ? null : Math.abs(result.volume - item.expectedVolume) / item.expectedVolume
      if (!Number.isFinite(result.volume) || (volumeError !== null && volumeError > 1e-6)) throw new Error('Wrong or nonfinite volume')
      if (result.meshes.some(mesh => mesh.topology.boundary !== 0 || mesh.topology.nonManifold !== 0)) throw new Error('Output topology is not closed manifold')
      if (iteration < warmups) continue
      samples.push({ wallMs: ms, evaluateMs: result.timings.evaluateMs, analyzeMs: result.timings.analyzeMs })
      if (samples.length === sampleCount) {
        const triangles = result.meshes.reduce((n, m) => n + m.indices.length / 3, 0)
        row = {
          id, status: 'ok', sourceSha256: hash(item.source), samples,
          medianMs: median(samples.map(sample => sample.wallMs)), triangles, meshes: result.meshes.length,
          evaluateMs: median(samples.map(sample => sample.evaluateMs)), analyzeMs: median(samples.map(sample => sample.analyzeMs)),
          volume: result.volume, ...(item.expectedVolume === undefined ? {} : { expectedVolume: item.expectedVolume, volumeError }),
        }
      }
    } catch (error) {
      row = { id, sourceSha256: hash(item.source), status: 'error', iteration, warmup: iteration < warmups, samples, ms: performance.now() - start, message: String((error as Error).message).split('\n')[0].slice(0, 160) }
    }
  }
  rows.push(row!)
  console.log(JSON.stringify(row))
}
if (JSON.stringify(fingerprints()) !== JSON.stringify(files)) throw new Error('Inputs changed during benchmark')
const report = {
  kind: 'csg-scaling', schemaVersion: 2, node: process.version, v8: process.versions.v8,
  environment: { platform: process.platform, arch: process.arch, release: os.release(), cpuModels: [...new Set(os.cpus().map(cpu => cpu.model))] },
  recordedAt: new Date().toISOString(), source, warmups, sampleCount, rows,
  boundaries: ['Production parseOpenSCAD at full quality, including build and analysis; no Worker transport or GPU.', 'Sequential samples; no forced GC, CPU isolation or profiling. Validation outside timing. Run without other tests/builds.', 'Failure is recorded with its warmup/sample index, not treated as a fast successful result.', 'Selected host/artifact fingerprints, not a complete repository source manifest.'],
}
if (out) writeFileSync(out, `${JSON.stringify(report, null, 2)}\n`, { flag: 'wx' })
