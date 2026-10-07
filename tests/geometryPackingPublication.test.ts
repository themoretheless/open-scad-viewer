import {expect,it} from 'vitest'
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync} from 'node:fs'
import {tmpdir} from 'node:os'
import {join} from 'node:path'
import {createHash} from 'node:crypto'
import {verifyPackedWasmChunk} from '../scripts/verify-packed-wasm.mjs'
import {packGeometryKernel} from '../scripts/pack-geometry-kernel.mjs'

it('preserves every installed artifact when an otherwise valid WASM exceeds the package budget',()=>{
 const root=mkdtempSync(join(tmpdir(),'nurbs-package-publication-'))
 try{
  const output=join(root,'generated'),publicWasm=join(root,'public');mkdirSync(output);mkdirSync(publicWasm)
  const files=[join(output,'kernel_bg.wasm'),join(output,'identity.ts'),join(output,'bytes.ts'),join(publicWasm,'geometry-kernel.wasm')]
  const before=files.map((file,i)=>{const bytes=Buffer.from(`previous-artifact-${i}`);writeFileSync(file,bytes);return bytes})
  const payload=Buffer.alloc(16*1024*1024+1),length:number[]=[]
  let n=payload.length;do{const byte=n&127;n>>>=7;length.push(byte|(n?128:0))}while(n)
  const wasm=Buffer.concat([Buffer.from([0,97,115,109,1,0,0,0,0,...length]),payload])
  expect(WebAssembly.validate(wasm)).toBe(true)
  expect(()=>packGeometryKernel(wasm,output,publicWasm)).toThrow('decompression output limit')
  files.forEach((file,i)=>expect(readFileSync(file)).toEqual(before[i]))
 }finally{rmSync(root,{recursive:true,force:true})}
})

it('publishes matching raw, embedded and fingerprinted artifacts after successful preparation',()=>{
 const root=mkdtempSync(join(tmpdir(),'nurbs-package-success-'))
 try{
  const output=join(root,'generated'),publicWasm=join(root,'public');mkdirSync(output);mkdirSync(publicWasm)
  const wasm=Buffer.from([0,97,115,109,1,0,0,0])
  packGeometryKernel(wasm,output,publicWasm)
  expect(readFileSync(join(output,'kernel_bg.wasm'))).toEqual(wasm)
  expect(readFileSync(join(publicWasm,'geometry-kernel.wasm'))).toEqual(wasm)
  const identitySource=readFileSync(join(output,'identity.ts'),'utf8')
  const identity=JSON.parse(identitySource.match(/Object\.freeze\((.*)\)/)![1]!)
  expect(identity).toEqual({sha256:createHash('sha256').update(wasm).digest('hex'),byteLength:wasm.length})
  expect(verifyPackedWasmChunk(readFileSync(join(output,'bytes.ts'),'utf8'),wasm,'prepared package')).toBe(wasm.length)
 }finally{rmSync(root,{recursive:true,force:true})}
})
