// Run with tsx and a freshly built geometry_wasm.wasm.
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { encodeBinary } from '../src/core/valueBinaryCodec'
import { decodePacked, writeLinear } from '../src/kernel/wasmHost'

interface Kernel extends WebAssembly.Exports {
  memory: WebAssembly.Memory
  abi_alloc: (length: number) => number
  abi_free: (pointer: number, length: number) => void
  abi_request: (op: number, pointer: number, length: number) => bigint
}

type Rational = { numerator: string; denominator: string }

interface Report {
  model: {
    faces: { holes: number[] }[]
    shells: { closed: boolean }[]
    bodies: unknown[]
    topologyIds: { edges: string[] }
  } | null
  edgeBindings?: { edgeId: string; contour: number; segment: number; interval?: [number, number]; parameters?: [Rational, Rational] }[]
  componentAreas?: { outerContour: number; boundsMm2: [number, number] }[]
  centroidBoundsMm?: [number, number][] | null
  centralInertiaBoundsMm4?: [number, number][][] | null
  principalMomentBoundsMm4?: [number, number][] | null
  boundsMm2?: [number, number] | [number, number][] | null
  stations?: { parameterBounds: [number, number] | null; pointBoundsMm: [number, number][] | null; status: string }[] | null
  pointBoundsMm?: [number, number][] | null
  parameterBounds?: [number, number] | null
  status?: string
  length?: { boundsMm: [number, number]; withinTolerance: boolean } | null
  minimumCells?: number | null
  joins?: { segment: number; parameter: Rational; kind: string }[] | null
  workUsed: number
  refusalStage: string | null
}

const file = process.argv[2]
assert.ok(file, 'Pass the path to geometry_wasm.wasm')
const kernel = new WebAssembly.Instance(new WebAssembly.Module(readFileSync(file))).exports as Kernel

const ring = [[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]
const request = {
  op: 'brep_build_exact_periodic_trim_sheet',
  surface: {
    degreeU: 1, degreeV: 1,
    knotsU: [-1, 0, 1, 2, 3, 4, 5], knotsV: [0, 0, 1, 1],
    controlPoints: ring.map(([x, y]) => [[x, y, 0], [x, y, 2]]),
    weights: ring.map(() => [1, 1]), periodicU: true, periodicV: false,
  },
  contours: [
    [[3.5, 0.125], [0.5, 0.125], [0.5, 0.875], [3.5, 0.875], [3.5, 0.125]],
    [[0.125, 0.25], [0.375, 0.25], [0.375, 0.75], [0.125, 0.75], [0.125, 0.25]],
  ],
  toleranceMm: 1e-8,
  maxWork: 1_000_000,
}

function call(maxWork: number, op = request.op, options = {}): Report {
  const bytes = encodeBinary({ ...request, ...options, maxWork, op })
  const ptr = writeLinear(kernel.memory, kernel.abi_alloc, bytes)
  assert.ok(ptr)
  try {
    const packed = kernel.abi_request(0, ptr, bytes.length)
    const response = decodePacked<{ ok: true; value: Report } | { ok: false; error: unknown }>(
      kernel.memory, kernel.abi_free, packed,
    )
    assert.ok(response.ok, JSON.stringify(response))
    return response.value
  } finally {
    kernel.abi_free(ptr, bytes.length)
  }
}

const result = call(request.maxWork)
assert.ok(result.model)
assert.equal(result.refusalStage, null)
assert.ok(result.workUsed > 0)
assert.equal(result.model.faces.length, 1)
assert.deepEqual(result.model.faces[0].holes, [1])
assert.equal(result.model.shells.length, 1)
assert.equal(result.model.shells[0].closed, false)
assert.equal(result.model.bodies.length, 0)

const refusal = call(1)
assert.equal(refusal.model, null)
assert.equal(refusal.refusalStage, 'patch')
console.log('WASM ABI preserved the open periodic trim sheet and hole and refused exhausted work.')

const bindingOp = 'brep_build_exact_periodic_trim_sheet_with_bindings'
const bound = call(request.maxWork, bindingOp)
assert.deepEqual(bound.model, result.model)
assert.ok(bound.edgeBindings)
assert.deepEqual(bound.edgeBindings.map(binding => binding.edgeId), result.model.topologyIds.edges)
assert.deepEqual(
  bound.edgeBindings.filter(binding => binding.contour === 0 && binding.segment === 0)
    .map(binding => binding.interval),
  [[0, 0.5], [0.5, 1]],
)
const refusedBound = call(1, bindingOp)
assert.equal(refusedBound.model, null)
assert.deepEqual(refusedBound.edgeBindings, [])
console.log('WASM ABI retained persistent edge IDs and source intervals.')

const inputOp = 'brep_build_exact_periodic_trim_sheet_with_input_bindings'
const directed = call(request.maxWork, inputOp)
assert.deepEqual(directed.model, result.model)
assert.ok(directed.edgeBindings)
assert.deepEqual(directed.edgeBindings.map(binding => binding.edgeId), result.model.topologyIds.edges)
assert.deepEqual(directed.edgeBindings[0].parameters, [
  { numerator: '0', denominator: '1' }, { numerator: '1', denominator: '2' },
])
const holeEdge = directed.edgeBindings.find(binding => binding.contour === 1)
assert.equal(holeEdge?.segment, 3)
assert.deepEqual(holeEdge?.parameters, [
  { numerator: '1', denominator: '1' }, { numerator: '0', denominator: '1' },
])
const refusedDirected = call(1, inputOp)
assert.equal(refusedDirected.model, null)
assert.deepEqual(refusedDirected.edgeBindings, [])
console.log('WASM ABI retained exact rational parameters in original contour direction.')

const auditOp = 'brep_audit_exact_periodic_trim_contour'
const audit = call(request.maxWork, auditOp, { contour: 0 })
assert.ok(audit.joins)
assert.equal(audit.joins.length, 6)
assert.deepEqual(audit.joins[1], {
  segment: 0, parameter: { numerator: '1', denominator: '2' }, kind: 'corner',
})
const refusedAudit = call(result.workUsed + 1, auditOp, { contour: 0 })
assert.equal(refusedAudit.joins, null)
assert.equal(refusedAudit.refusalStage, 'audit')
console.log('WASM ABI returned exact contour joins and withheld an exhausted audit.')

const lengthOp = 'brep_measure_exact_periodic_trim_range'
const selection = {
  contour: 0, segment: 0,
  range: [{ numerator: '4', denominator: '5' }, { numerator: '1', denominator: '3' }],
}
const measured = call(request.maxWork, lengthOp, { ...selection, maxCells: 100 })
assert.ok(measured.length?.withinTolerance)
assert.ok(measured.length.boundsMm[0] <= 7 / 15 && 7 / 15 <= measured.length.boundsMm[1])
const short = call(request.maxWork, lengthOp, { ...selection, maxCells: 3 })
assert.equal(short.length, null)
assert.equal(short.minimumCells, 4)
assert.equal(short.refusalStage, 'measure')
console.log('WASM ABI enclosed a rational trim length and enforced its cell budget.')

const stationOp = 'brep_locate_exact_periodic_trim_contour_station'
const stationOptions = { contour: 0, distanceMm: 0.5, parameterTolerance: 1e-6 }
const station = call(request.maxWork, stationOp, { ...stationOptions, maxCells: 1024 })
assert.equal(station.status, 'tolerance')
assert.ok(station.parameterBounds)
assert.ok(station.parameterBounds[0] <= 0.5 && 0.5 <= station.parameterBounds[1])
assert.ok(station.parameterBounds[1] - station.parameterBounds[0] <= 1e-6)
const limitedStation = call(request.maxWork, stationOp, { ...stationOptions, maxCells: 6 })
assert.equal(limitedStation.status, 'workLimit')
assert.deepEqual(limitedStation.parameterBounds, [0, 1])
console.log('WASM ABI located a seam station and retained its proved bracket at the cell limit.')

const stationBoundsOp = 'brep_enclose_exact_periodic_trim_contour_station'
const worldStation = call(request.maxWork, stationBoundsOp, { ...stationOptions, maxCells: 1024 })
assert.deepEqual(worldStation.parameterBounds, station.parameterBounds)
assert.ok(worldStation.pointBoundsMm)
assert.equal(worldStation.pointBoundsMm.length, 3)
for (const [index, [lo, hi]] of worldStation.pointBoundsMm.entries()) {
  const value = [0, 0, 0.25][index]
  assert.ok(lo <= value && value <= hi && hi - lo < 1e-5)
}
const outsideStation = call(request.maxWork, stationBoundsOp, {
  ...stationOptions, distanceMm: 6, maxCells: 6,
})
assert.equal(outsideStation.status, 'outside')
assert.equal(outsideStation.pointBoundsMm, null)
console.log('WASM ABI returned a certified world box for an available seam station.')

const divisionOp = 'brep_divide_exact_periodic_trim_contour_by_length'
const divisionOptions = { contour: 0, divisions: 5, parameterTolerance: 1e-6 }
const divided = call(request.maxWork, divisionOp, { ...divisionOptions, maxCells: 2048 })
assert.ok(divided.stations)
assert.equal(divided.stations.length, 6)
assert.deepEqual(divided.stations[0].parameterBounds, [0, 0])
assert.deepEqual(divided.stations[5].parameterBounds, [4, 4])
assert.ok(divided.stations.every(station => station.status === 'tolerance' && station.pointBoundsMm?.length === 3))
const coarseDivision = call(request.maxWork, divisionOp, { ...divisionOptions, maxCells: 6 })
assert.equal(coarseDivision.stations?.[1].status, 'workLimit')
assert.ok(coarseDivision.stations?.[1].pointBoundsMm)
console.log('WASM ABI returned ordered equal-length contour station boxes and closure.')

const patchAreaOp = 'brep_bound_exact_periodic_trim_patch_area'
const patchArea = call(request.maxWork, patchAreaOp, { outerContour: 0, maxCells: 100, toleranceMm2: 1e-8 })
assert.equal(patchArea.status, 'tolerance')
assert.ok(patchArea.boundsMm2 && patchArea.boundsMm2.every(Array.isArray))
assert.equal(patchArea.boundsMm2.length, 3)
for (const [index, [lo, hi]] of (patchArea.boundsMm2 as [number, number][]).entries()) {
  const value = [-0.75, -0.5, 0][index]
  assert.ok(lo <= value && value <= hi)
}
const patchAreaLimited = call(request.maxWork, patchAreaOp, { outerContour: 0, maxCells: 9, toleranceMm2: 1e-8 })
assert.equal(patchAreaLimited.status, 'workLimit')
assert.equal(patchAreaLimited.boundsMm2, null)
console.log('WASM ABI enclosed signed projected areas with hole subtraction.')

const directionalAreaOp = 'brep_bound_exact_periodic_trim_directional_area'
const directional = call(request.maxWork, directionalAreaOp, {
  outerContour: 0, normal: [1, 1, 0], maxCells: 100, toleranceMm2: 1e-8,
})
assert.equal(directional.status, 'tolerance')
assert.ok(directional.boundsMm2)
const [areaLo, areaHi] = directional.boundsMm2
assert.equal(typeof areaLo, 'number')
assert.equal(typeof areaHi, 'number')
assert.ok(areaLo <= -1.25 / Math.sqrt(2) && -1.25 / Math.sqrt(2) <= areaHi)
console.log('WASM ABI enclosed the signed area in an oblique projection.')

const surfaceAreaOp = 'brep_measure_exact_periodic_trim_surface_area'
const surfaceAreaOptions = { outerContour: 0, toleranceMm2: 0.05, toleranceUv: 1e-10 }
const surfaceArea = call(request.maxWork, surfaceAreaOp, {
  ...surfaceAreaOptions, maxSurfaceCells: 10_000, maxTrimCells: 1_000_000,
})
assert.equal(surfaceArea.status, 'tolerance')
assert.ok(surfaceArea.boundsMm2)
const [surfaceLo, surfaceHi] = surfaceArea.boundsMm2
assert.ok(typeof surfaceLo === 'number' && typeof surfaceHi === 'number')
assert.ok(surfaceLo <= 1.25 && 1.25 <= surfaceHi)
const incompleteArea = call(request.maxWork, surfaceAreaOp, {
  ...surfaceAreaOptions, maxSurfaceCells: 1, maxTrimCells: 1,
})
assert.equal(incompleteArea.status, 'workLimit')
assert.equal(incompleteArea.boundsMm2, null)
console.log('WASM ABI enclosed true trimmed surface area and refused incomplete coverage.')

const massOp = 'brep_measure_exact_periodic_trim_surface_mass_properties'
const massOptions = { ...surfaceAreaOptions, toleranceMm2: 0.005, momentToleranceMm4: 1e-8 }
const mass = call(request.maxWork, massOp, {
  ...massOptions, maxSurfaceCells: 10_000, maxTrimCells: 1_000_000,
})
assert.ok(mass.centroidBoundsMm)
assert.equal(mass.centroidBoundsMm.length, 3)
for (const [axis, [lo, hi]] of mass.centroidBoundsMm.entries()) {
  const value = [0.1, 0.15, 1][axis]
  assert.ok(lo <= value && value <= hi)
}
assert.deepEqual(mass.centralInertiaBoundsMm4?.[0][1], mass.centralInertiaBoundsMm4?.[1][0])
assert.equal(mass.principalMomentBoundsMm4?.length, 3)
const coarseMass = call(request.maxWork, massOp, { ...massOptions, maxSurfaceCells: 2, maxTrimCells: 1 })
assert.equal(coarseMass.centroidBoundsMm, null)
assert.equal(coarseMass.principalMomentBoundsMm4, null)
console.log('WASM ABI returned trimmed area centroid, central inertia and principal moment bounds.')

const windowMassOp = 'brep_measure_exact_periodic_trim_window_mass_properties'
const windowOptions = { ...massOptions, toleranceMm2: 0.001, maxSurfaceCells: 10_000, maxTrimCells: 500_000 }
const windowMass = call(request.maxWork, windowMassOp, { ...windowOptions, uvWindow: [[3.75, 4.25], [0.375, 0.625]] })
assert.ok(windowMass.boundsMm2)
const [windowLo, windowHi] = windowMass.boundsMm2
assert.ok(typeof windowLo === 'number' && typeof windowHi === 'number' && windowLo <= 0.1875 && 0.1875 <= windowHi)
assert.ok(windowMass.centroidBoundsMm)
assert.ok(windowMass.centroidBoundsMm[0][0] <= 1 / 48 && 1 / 48 <= windowMass.centroidBoundsMm[0][1])
const emptyWindow = call(request.maxWork, windowMassOp, { ...windowOptions, uvWindow: [[5, 6], [0.25, 0.5]] })
assert.deepEqual(emptyWindow.boundsMm2, [0, 0])
assert.equal(emptyWindow.centroidBoundsMm, null)
console.log('WASM ABI enclosed mass properties inside a clipped UV window.')

const patchSetOp = 'brep_measure_exact_periodic_trim_patch_set_mass_properties'
const island = [[0.1875, 0.375], [0.3125, 0.375], [0.3125, 0.625], [0.1875, 0.625], [0.1875, 0.375]]
const patchSetOptions = {
  ...surfaceAreaOptions, contours: [...request.contours, island], maxTrimCells: 1_000_000,
}
const patchSet = call(request.maxWork, patchSetOp, { ...patchSetOptions, maxSurfaceCells: 20_000 })
assert.equal(patchSet.componentAreas?.length, 2)
assert.equal(patchSet.componentAreas?.[1].outerContour, 2)
const islandBounds = patchSet.componentAreas?.[1].boundsMm2
assert.ok(islandBounds && islandBounds[0] <= 0.0625 && 0.0625 <= islandBounds[1])
assert.ok(patchSet.centroidBoundsMm)
assert.equal(patchSet.centralInertiaBoundsMm4?.length, 3)
const incompleteSet = call(request.maxWork, patchSetOp, { ...patchSetOptions, maxSurfaceCells: 2 })
assert.equal(incompleteSet.status, 'workLimit')
assert.deepEqual(incompleteSet.componentAreas, [])
assert.equal(incompleteSet.boundsMm2, null)
console.log('WASM ABI returned complete multi-component trimmed surface mass properties.')
