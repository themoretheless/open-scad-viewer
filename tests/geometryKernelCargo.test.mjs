import {test} from 'node:test'
import assert from 'node:assert/strict'
import {geometryKernelCargo} from '../scripts/geometry-kernel-cargo.mjs'
import {reproducibleCargo} from '../scripts/reproducible-cargo.mjs'

test('bounded geometry profile preserves encoded flags and reproducible paths',()=>{
 const result=geometryKernelCargo('/work/tree/',{CARGO_HOME:'/cargo-home',CARGO_ENCODED_RUSTFLAGS:'--cfg=owned\x1f-L/path with spaces'})
 assert.deepEqual(result.args,[])
 assert.deepEqual(result.env.CARGO_ENCODED_RUSTFLAGS.split('\x1f'),[
  '--cfg=owned','-L/path with spaces','-Zlocation-detail=none',
  '--remap-path-prefix=/cargo-home=/cargo','--remap-path-prefix=/work/tree=/src'])
})
test('geometry-only location policy does not change other kernel build flags',()=>{
 const env={CARGO_HOME:'/cargo-home',RUSTFLAGS:'--cfg=owned -Cpanic=abort'}
 const result=geometryKernelCargo('/work/tree',env)
 assert(result.env.CARGO_ENCODED_RUSTFLAGS.startsWith('--cfg=owned\x1f-Cpanic=abort\x1f-Zlocation-detail=none'))
 assert(!JSON.stringify(reproducibleCargo('/work/tree',env)).includes('location-detail'))
})
