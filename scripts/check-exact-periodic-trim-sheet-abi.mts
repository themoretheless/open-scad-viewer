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

function call(maxWork: number, op = request.op, contour?: number): Report {
  const options = contour === undefined ? {} : { contour }
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
const audit = call(request.maxWork, auditOp, 0)
assert.ok(audit.joins)
assert.equal(audit.joins.length, 6)
assert.deepEqual(audit.joins[1], {
  segment: 0, parameter: { numerator: '1', denominator: '2' }, kind: 'corner',
})
const refusedAudit = call(result.workUsed + 1, auditOp, 0)
assert.equal(refusedAudit.joins, null)
assert.equal(refusedAudit.refusalStage, 'audit')
console.log('WASM ABI returned exact contour joins and withheld an exhausted audit.')
