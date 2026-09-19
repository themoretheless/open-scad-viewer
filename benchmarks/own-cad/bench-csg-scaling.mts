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
import { writeFileSync } from 'node:fs'
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

const args = process.argv.slice(2)
const outIndex = args.indexOf('--out')
const out = outIndex >= 0 ? args.splice(outIndex, 2)[1] : undefined
const selected = args.length ? args : Object.keys(cases)
const rows: Array<Record<string, unknown>> = []
for (const id of selected) {
  const item = cases[id]
  if (!item) throw new Error(`Unknown case ${id}`)
  // Warm-up run keeps kernel instantiation and JIT out of the measured sample.
  const samples: number[] = []
  let row: Record<string, unknown> | undefined
  for (let iteration = 0; iteration < 3 && !row; iteration++) {
    const start = performance.now()
    try {
      const result = await parseOpenSCAD(item.source, { quality: 'full' })
      const ms = performance.now() - start
      if (iteration === 0) continue
      samples.push(ms)
      if (iteration === 2) {
        const triangles = result.meshes.reduce((n, m) => n + m.indices.length / 3, 0)
        const volumeError = item.expectedVolume === undefined ? null : Math.abs(result.volume - item.expectedVolume) / item.expectedVolume
        row = {
          id, status: volumeError !== null && volumeError > 1e-6 ? 'wrong-volume' : 'ok',
          medianMs: Math.round(Math.min(...samples) * 10) / 10, triangles, meshes: result.meshes.length,
          evaluateMs: Math.round(result.timings.evaluateMs), analyzeMs: Math.round(result.timings.analyzeMs),
          volume: result.volume, ...(item.expectedVolume === undefined ? {} : { expectedVolume: item.expectedVolume, volumeError }),
        }
      }
    } catch (error) {
      row = { id, status: 'error', ms: Math.round((performance.now() - start) * 10) / 10, message: String((error as Error).message).split('\n')[0].slice(0, 160) }
    }
  }
  rows.push(row!)
  console.log(JSON.stringify(row))
}
const report = { kind: 'csg-scaling', node: process.version, recordedAt: new Date().toISOString(), rows }
if (out) writeFileSync(out, `${JSON.stringify(report, null, 2)}\n`)
