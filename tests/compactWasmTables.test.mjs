import test from 'node:test'
import assert from 'node:assert/strict'
import binaryen from 'binaryen'
import {compactWasmTables} from '../scripts/compact-wasm-tables.mjs'

function moduleWith(segments, start = '') {
  const parts = segments.map(([base, bytes]) => {
    const escaped = [...bytes].map(v => '\\' + v.toString(16).padStart(2,'0')).join('')
    return `(data (i32.const ${base}) "${escaped}")`
  })
  const wat = `(module (memory (export "memory") 1) (func $read (export "read") (param i32) (result i32) (i32.load (local.get 0))) ${start} ${parts.join(' ')})`
  const m = binaryen.parseText(wat)
  try { return Buffer.from(m.emitBinary()) } finally { m.dispose() }
}
function mixedTable() {
  const b = Buffer.alloc(8193)
  for(let i=0; i<2048; i++) {
    const v = i>500 && i<800 ? Math.imul(i,0x1365aaf3) ^ (i << 13) : Math.imul(i,i) - 1000*i
    b.writeInt32LE(v|0,i*4+1)
  }
  return b
}
test('restores mixed raw and second-difference runs and all exported memory at startup',()=>{
  const baseline=moduleWith([[67,mixedTable()],[12345,Buffer.from('untouched')],[15427,mixedTable()]])
  const candidate=compactWasmTables(baseline)
  assert.ok(candidate.length < baseline.length-2048)
  assert.deepEqual(candidate,compactWasmTables(baseline))
  const instantiate=b=>new WebAssembly.Instance(new WebAssembly.Module(b)).exports
  const a=instantiate(baseline),b=instantiate(candidate)
  assert.deepEqual(Object.keys(a),Object.keys(b))
  assert.deepEqual(Buffer.from(a.memory.buffer),Buffer.from(b.memory.buffer))
  for(const address of [0,64,68,2056,12345,15428]) assert.equal(a.read(address),b.read(address))
})
test('leaves short tables, preexisting initialization and overlapping data segments alone',()=>{
  for(const b of [
    moduleWith([[100,Buffer.alloc(100,23)]]),
    moduleWith([[67,mixedTable()]],'(func $init) (start $init)'),
    moduleWith([[67,mixedTable()],[70,Buffer.from('replacement')]])
  ]) assert.deepEqual(compactWasmTables(b),b)
})
test('restores overlapping backreferences, separated segments and a noisy literal suffix',()=>{
  const data = Buffer.alloc(40001)
  const sequence = Buffer.from('ordered rational surface patches')
  let seed = 9547
  for(let i=0;i<data.length;i++){
    seed = (Math.imul(seed,1664525) + 1013904223) >>> 0
    data[i] = i<500 || i>31009 || (i%2000)<65 ? seed>>>24 : sequence[i%sequence.length]
  }
  const original = moduleWith([[17,data],[43501,data.subarray(300,12000)]])
  const compact = compactWasmTables(original)
  assert.ok(compact.length < original.length - 20000)
  const memory = bytes => Buffer.from(new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports.memory.buffer)
  assert.deepEqual(memory(compact),memory(original))
  assert.deepEqual(compact,compactWasmTables(original))
})
