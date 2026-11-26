import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { encodeBinary } from '../../src/core/valueBinaryCodec'
import { decodePacked, writeLinear } from '../../src/kernel/wasmHost'

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
  edgeBindings?: {
    edgeId: string
    contour: number
    segment: number
    interval?: [number, number]
    parameters?: [Rational, Rational]
  }[]
  componentAreas?: { outerContour: number; boundsMm2: [number, number] }[]
  centroidBoundsMm?: [number, number][] | null
  centralInertiaBoundsMm4?: [number, number][][] | null
  withinMomentTolerance?: boolean
  referenceInertiaBoundsMm4?: [number, number][][] | null
  axisRadiusOfGyrationBoundsMm?: [number, number] | null
  componentMasses?: {
    outerContour: number
    boundsKg: [number, number]
    massFractionBounds: [number, number] | null
  }[]
  massBoundsKg?: [number, number] | null
  firstMassMomentAboutPlaneBoundsKgMm?: [number, number] | null
  centerOfMassPlaneDistanceBoundsMm?: [number, number] | null
  centerOfMassBoundsMm?: [number, number][] | null
  referenceInertiaBoundsKgMm2?: [number, number][][] | null
  centralInertiaBoundsKgMm2?: [number, number][][] | null
  twoPlaneCounterweightCapacity?: {
    status: string
    planes: { offsetMm: number; minimumRadiusBoundsMm: [number, number]; status: string }[]
  } | null
  twoPlaneCounterweights?: {
    offsetMm: number
    radiusMm: number
    massBoundsKg: [number, number]
    radialOffsetBoundsMm: [number, number][] | null
  }[] | null
  twoPlaneResidualBalance?: {
    status: string
    planes: { offsetMm: number; vectorBoundsKgMm: [number, number][]; status: string }[]
  } | null
  twoPlaneBalance?: {
    status: string
    planes: { offsetMm: number; magnitudeBoundsKgMm: [number, number]; status: string }[]
  } | null
  twoPlaneUnbalance?: { offsetMm: number; vectorBoundsKgMm: [number, number][] }[] | null
  coupleUnbalance?: { vectorBoundsKgMm2: [number, number][]; magnitudeBoundsKgMm2: [number, number] } | null
  staticUnbalance?: { vectorBoundsKgMm: [number, number][]; magnitudeBoundsKgMm: [number, number] } | null
  axisBalance?: { eccentricityBoundsMm: [number, number]; status: string } | null
  axisMomentBoundsKgMm2?: [number, number] | null
  axisMomentBoundsMm4?: [number, number] | null
  principalMomentBoundsMm4?: [number, number][] | null
  principalAxes?:
    | ({
        direction: [number, number, number]
        sinAngleUpper: number
        distanceBoundsMm?: [number, number]
        angleSineBounds?: [number, number]
      } | null)[]
    | null
  principalMomentBoundsKgMm2?: [number, number][] | null
  boundsMm2?: [number, number] | [number, number][] | null
  stations?:
    { parameterBounds: [number, number] | null; pointBoundsMm: [number, number][] | null; status: string }[] | null
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

const ring = [
  [0, 0],
  [1, 0],
  [1, 1],
  [0, 1],
  [0, 0],
]
export const request = {
  op: 'brep_build_exact_periodic_trim_sheet',
  surface: {
    degreeU: 1,
    degreeV: 1,
    knotsU: [-1, 0, 1, 2, 3, 4, 5],
    knotsV: [0, 0, 1, 1],
    controlPoints: ring.map(([x, y]) => [
      [x, y, 0],
      [x, y, 2],
    ]),
    weights: ring.map(() => [1, 1]),
    periodicU: true,
    periodicV: false,
  },
  contours: [
    [
      [3.5, 0.125],
      [0.5, 0.125],
      [0.5, 0.875],
      [3.5, 0.875],
      [3.5, 0.125],
    ],
    [
      [0.125, 0.25],
      [0.375, 0.25],
      [0.375, 0.75],
      [0.125, 0.75],
      [0.125, 0.25],
    ],
  ],
  toleranceMm: 1e-8,
  maxWork: 1_000_000,
}

export function call(maxWork: number, op = request.op, options = {}): Report {
  const bytes = encodeBinary({ ...request, ...options, maxWork, op })
  const ptr = writeLinear(kernel.memory, kernel.abi_alloc, bytes)
  assert.ok(ptr)
  try {
    const packed = kernel.abi_request(0, ptr, bytes.length)
    const response = decodePacked<{ ok: true; value: Report } | { ok: false; error: unknown }>(
      kernel.memory,
      kernel.abi_free,
      packed,
    )
    assert.ok(response.ok, JSON.stringify(response))
    return response.value
  } finally {
    kernel.abi_free(ptr, bytes.length)
  }
}

export const surfaceAreaOptions = { outerContour: 0, toleranceMm2: 0.05, toleranceUv: 1e-10 }
