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

interface Report {
  forwardBoundsMm: [number, number]
  reverseBoundsMm: [number, number]
  reverseStatus: string | null
  stopReason: string
}

const file = process.argv[2]
assert.ok(file, 'Pass the path to geometry_wasm.wasm')
const kernel = new WebAssembly.Instance(new WebAssembly.Module(readFileSync(file))).exports as Kernel

function measure(overrides = {}): Report {
  const bytes = encodeBinary({
    op: 'nurbs_measure_curve_directional_travel',
    curve: {
      degree: 2,
      knots: [0, 0, 0, 1, 1, 1],
      controlPoints: [[0, 0], [4, 0], [2, 0]],
      weights: [1, 1, 1],
      periodic: false,
    },
    direction: [1, 0],
    toleranceMm: 1e-7,
    maxCells: 4096,
    ...overrides,
  })
  const pointer = writeLinear(kernel.memory, kernel.abi_alloc, bytes)
  assert.ok(pointer)
  try {
    const packed = kernel.abi_request(0, pointer, bytes.length)
    const result = decodePacked<{ ok: true; value: Report } | { ok: false; error: unknown }>(
      kernel.memory, kernel.abi_free, packed,
    )
    assert.ok(result.ok, JSON.stringify(result))
    return result.value
  } finally {
    kernel.abi_free(pointer, bytes.length)
  }
}

const full = measure({ reverseLimitMm: 0.6 })
assert.equal(full.reverseStatus, 'exceeds')
for (const [[lo, hi], exact] of [[full.forwardBoundsMm, 8 / 3], [full.reverseBoundsMm, 2 / 3]] as const) {
  assert.ok(lo <= exact && exact <= hi)
}
assert.equal(measure({ reverseLimitMm: 0.6, parameterInterval: [0, 0.5] }).reverseStatus, 'within')
const coarse = measure({ reverseLimitMm: 0.7, maxCells: 1 })
assert.equal(coarse.reverseStatus, 'undetermined')
assert.equal(coarse.stopReason, 'workLimit')
console.log('WASM ABI enclosed signed directional travel and classified reverse travel.')
