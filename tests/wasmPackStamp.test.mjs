import {test} from 'node:test'
import assert from 'node:assert/strict'
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync,readdirSync} from 'node:fs'
import {tmpdir} from 'node:os'
import {join} from 'node:path'
import {pathToFileURL} from 'node:url'
import {createHash} from 'node:crypto'
import {packStampHit,writePackStamp} from '../scripts/wasm-pack-stamp.mjs'
import {packGeometryKernel} from '../scripts/pack-geometry-kernel.mjs'
import {packLanguageKernel} from '../scripts/pack-language-kernel.mjs'
import {packPhotogrammetry} from '../scripts/pack-photogrammetry.mjs'
const wasm=Buffer.from([0,97,115,109,1,0,0,0])
function fixture(run) {
  const root=mkdtempSync(join(tmpdir(),'wasm-pack-integrity-')),output=join(root,'generated'),publicWasm=join(root,'public')
  mkdirSync(output);mkdirSync(publicWasm)
  const files=[join(output,'raw.wasm'),join(publicWasm,'public.wasm'),join(output,'identity.ts'),join(output,'bytes.ts')]
  files.forEach((p,i)=>writeFileSync(p,`complete artifact ${i}`))
  try {run({root,output,publicWasm,files,stamp:join(output,'.stamp')})}finally {rmSync(root,{recursive:true,force:true})}
}
test('all artifacts must match, even after same-size corruption of each file',()=>fixture(({stamp,files})=>{
  writePackStamp(stamp,wasm,files)
  assert.equal(packStampHit(stamp,wasm,files),true)
  assert.equal(packStampHit(stamp,wasm,[...files].reverse()),true)
  for(const file of files){const bytes=readFileSync(file),bad=Buffer.from(bytes);bad[0]^=1;writeFileSync(file,bad);assert.equal(packStampHit(stamp,wasm,files),false);writeFileSync(file,bytes)}
  assert.equal(packStampHit(stamp,wasm,files),true)
  assert.equal(packStampHit(stamp,Buffer.from('other source'),files),false)
  assert.equal(packStampHit(stamp,wasm,files.slice(1)),false)
}))
test('missing, truncated and oversized outputs invalidate the cache before reuse',()=>fixture(({stamp,files})=>{
  writePackStamp(stamp,wasm,files);const valid=readFileSync(files[0])
  for(const bytes of [valid.subarray(1),Buffer.concat([valid,Buffer.from('extra')])]){writeFileSync(files[0],bytes);assert.equal(packStampHit(stamp,wasm,files),false)}
  rmSync(files[0]);assert.equal(packStampHit(stamp,wasm,files),false)
}))
test('legacy, malformed, oversized and mismatched manifests are misses',()=>fixture(({stamp,files})=>{
  writePackStamp(stamp,wasm,files);const valid=JSON.parse(readFileSync(stamp,'utf8'))
  const legacy=createHash('sha256').update(wasm).digest('hex')+'\n'
  for(const text of [legacy,'not JSON','null',' '.repeat(65537),JSON.stringify({...valid,schema:'older'}),JSON.stringify({...valid,outputs:valid.outputs.slice(1)}),JSON.stringify({...valid,outputs:valid.outputs.map((o,i)=>i?o:{...o,path:'../../unrequested-file'})})]){writeFileSync(stamp,text);assert.equal(packStampHit(stamp,wasm,files),false)}
}))
test('file URLs work and failed snapshots preserve the last complete stamp',()=>fixture(({stamp,files,output})=>{
  writePackStamp(pathToFileURL(stamp),wasm,files.map(pathToFileURL));assert.equal(packStampHit(stamp,wasm,files),true)
  const before=readFileSync(stamp);rmSync(files[0]);assert.throws(()=>writePackStamp(stamp,wasm,files));assert.deepEqual(readFileSync(stamp),before)
  assert.equal(readdirSync(output).some(p=>p.endsWith('.tmp')),false)
  assert.throws(()=>writePackStamp(stamp,wasm,[]),/outputs/)
  assert.throws(()=>writePackStamp(stamp,wasm,[files[1],files[1]]),/unique/)
}))
for(const [name,pack,raw] of [['geometry',packGeometryKernel,'geometry-kernel.wasm'],['language',packLanguageKernel,'language-kernel.wasm'],['photogrammetry',packPhotogrammetry,'photogrammetry.wasm']]) {
  test(`${name} packer repairs every corrupt artifact and invalidates source-only stamps`,()=>fixture(({output,publicWasm})=>{
    pack(wasm,output,publicWasm)
    const files=[join(output,'bytes.ts'),join(publicWasm,raw)]
    if(name!=='photogrammetry')files.push(join(output,'identity.ts'),join(output,'kernel_bg.wasm'))
    const valid=files.map(p=>readFileSync(p))
    for(let i=0;i<files.length;i++){const bad=Buffer.from(valid[i]);bad[0]^=1;writeFileSync(files[i],bad);pack(wasm,output,publicWasm);files.forEach((p,j)=>assert.deepEqual(readFileSync(p),valid[j]))}
    const stamp=join(output,readdirSync(output).find(p=>p.startsWith('.wasm-pack-stamp')))
    writeFileSync(stamp,createHash('sha256').update(wasm).digest('hex')+'\n')
    pack(wasm,output,publicWasm);assert.equal(JSON.parse(readFileSync(stamp,'utf8')).schema,'wasm-pack-stamp-v2')
    pack(wasm,output,publicWasm);files.forEach((p,i)=>assert.deepEqual(readFileSync(p),valid[i]))
  }))
}

test('failed atomic publication removes temporary files without replacing its destination',()=>fixture(({stamp,files,output})=>{
  mkdirSync(stamp);const sentinel=join(stamp,'keep');writeFileSync(sentinel,'original directory contents')
  assert.throws(()=>writePackStamp(stamp,wasm,files));assert.equal(readFileSync(sentinel,'utf8'),'original directory contents')
  assert.equal(readdirSync(output).some(p=>p.endsWith('.tmp')),false)
}))
