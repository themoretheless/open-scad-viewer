import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync} from 'node:fs'
import * as module from 'node:module'
import {load} from '../tests/fixtures/raw-import-loader.mjs'
import identity from '../src/generated/geometry-kernels/identity'
if(typeof(module as any).registerHooks==='function')(module as any).registerHooks({load})
else module.register(new URL('../tests/fixtures/raw-import-loader.mjs',import.meta.url),import.meta.url)
const {evaluateNurbsSurface}=await import('../src/kernel/nurbsSurface')
const bytes=readFileSync('public/wasm/geometry-kernel.wasm'),hash=createHash('sha256').update(bytes).digest('hex');assert.equal(hash,identity.sha256)
const surface={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]]}
for(let i=0;i<50;i++)assert.deepEqual(evaluateNurbsSurface(surface,.25,.75).point,[.25,.75,0])
const samples:number[]=[]
for(let batch=0;batch<7;batch++){const start=performance.now();for(let i=0;i<300;i++)assert.deepEqual(evaluateNurbsSurface(surface,.25,.75).point,[.25,.75,0]);samples.push((performance.now()-start)/300)}
assert.equal(createHash('sha256').update(readFileSync('public/wasm/geometry-kernel.wasm')).digest('hex'),hash)
const sorted=[...samples].sort((a,b)=>a-b),r={schema:'geometry-wire-eval-benchmark/1',wasmSha256:hash,bytes:bytes.length,samplesMsPerCall:samples,medianMsPerCall:sorted[3],scope:'Warm JSON surface_evaluate RPC on one plane; not all-command, browser FPS, cold-load or worker latency qualification.'}
writeFileSync(process.argv[2]??'/tmp/geometry-wire-eval.json',JSON.stringify(r,null,2)+'\n');console.log(JSON.stringify(r))
