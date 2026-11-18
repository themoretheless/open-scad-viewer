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

const file = process.argv[2]
assert.ok(file, 'Pass the path to geometry_wasm.wasm')
const kernel = new WebAssembly.Instance(new WebAssembly.Module(readFileSync(file))).exports as Kernel

function request<T>(payload: Record<string, unknown>): T {
  const bytes = encodeBinary(payload)
  const pointer = writeLinear(kernel.memory, kernel.abi_alloc, bytes)
  assert.ok(pointer)
  try {
    const packed = kernel.abi_request(0, pointer, bytes.length)
    const result = decodePacked<{ ok: true; value: T } | { ok: false; error: unknown }>(
      kernel.memory, kernel.abi_free, packed,
    )
    assert.ok(result.ok, JSON.stringify(result))
    return result.value
  } finally {
    kernel.abi_free(pointer, bytes.length)
  }
}

const block = request({ op: 'brep_nurbs_box', min: [0, 0, 0], max: [2, 3, 4] })
const placement = (x: number) => [[1, 0, 0, x], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]
const result = request<{ text: string }>({
  op: 'brep_nurbs_export_shared_step',
  definitions: [block],
  occurrences: [0, 17].map(x => ({ definition: 0, transform: placement(x) })),
})
assert.equal(result.text.match(/=MANIFOLD_SOLID_BREP\(/g)?.length, 1)
assert.equal(result.text.match(/=NEXT_ASSEMBLY_USAGE_OCCURRENCE\(/g)?.length, 2)
console.log('Shared STEP export verified on WASM: one definition and two placements.')
