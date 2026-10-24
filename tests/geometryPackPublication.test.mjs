import {test} from 'node:test'
import assert from 'node:assert/strict'
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync} from 'node:fs'
import {tmpdir} from 'node:os'
import {join} from 'node:path'
import {packGeometryKernel} from '../scripts/pack-geometry-kernel.mjs'
function leb(value){const bytes=[];do{const byte=value&127;value=Math.floor(value/128);bytes.push(byte|(value?128:0))}while(value);return Buffer.from(bytes)}
test('refused oversized geometry package preserves every published artifact',()=>{
 const root=mkdtempSync(join(tmpdir(),'geometry-pack-publication-')),output=join(root,'generated'),publicWasm=join(root,'public')
 mkdirSync(output);mkdirSync(publicWasm)
 const files=[join(output,'kernel_bg.wasm'),join(publicWasm,'geometry-kernel.wasm'),join(output,'identity.ts'),join(output,'bytes.ts')]
 files.forEach((file,i)=>writeFileSync(file,`previous artifact ${i}`))
 try{
  // A valid, oversized custom section needs no large executable function.
  const payload=Buffer.alloc(16*1024*1024)
  const wasm=Buffer.concat([Buffer.from([0,97,115,109,1,0,0,0,0]),leb(payload.length),payload])
  assert.throws(()=>packGeometryKernel(wasm,output,publicWasm),/exceeds decompression output limit/)
  files.forEach((file,i)=>assert.equal(readFileSync(file,'utf8'),`previous artifact ${i}`))
 }finally{rmSync(root,{recursive:true,force:true})}
})
