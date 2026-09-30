import {readFileSync,writeFileSync} from 'node:fs'
import {performance} from 'node:perf_hooks'
import {createHash} from 'node:crypto'
import {expect,it} from 'vitest'
import {parseDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
import {resolveSolidInstances} from '../src/services/solidInstances'
import {stringifyMeshJson} from '../src/services/meshJson'
import {SolidInstanceBatchCache} from '../src/services/solidInstanceBatchCache'

it.runIf(!!process.env.CAD_INSTANCE_PROFILE_INPUT)('profiles bounded instance restoration without changing serialized geometry',()=>{
 const text=readFileSync(process.env.CAD_INSTANCE_PROFILE_INPUT!,'utf8')
 const results:Array<Record<string,unknown>>=[]
 const baseline=parseDirectDocument(text)
 const expected=serializeDirectDocument(baseline),expectedGeometry=stringifyMeshJson(baseline)
 for(const mode of ['cold','warm'] as const){
  const cache=new SolidInstanceBatchCache()
  if(mode==='warm')parseDirectDocument(text,cache)
  for(let iteration=0;iteration<3;iteration++){
   const selected=mode==='cold'?new SolidInstanceBatchCache():cache
   const started=performance.now(),restored=parseDirectDocument(text,selected),restoreMs=performance.now()-started
   const serializedAt=performance.now(),serialized=serializeDirectDocument(restored),serializeMs=performance.now()-serializedAt
   expect(serialized).toBe(expected)
   expect(stringifyMeshJson(restored)).toBe(expectedGeometry)
   const wire=JSON.parse(text),resolveAt=performance.now()
   const geometry=resolveSolidInstances(wire,mode==='cold'?new SolidInstanceBatchCache():cache)
   const resolveMs=performance.now()-resolveAt
   expect(serializeDirectDocument(geometry)).toBe(expected)
   expect(stringifyMeshJson(geometry)).toBe(expectedGeometry)
   results.push({geometrySha256:createHash('sha256').update(expectedGeometry).digest('hex'),mode,iteration,restoreMs,serializeMs,resolveMs,instances:restored.bodies.length-1,cacheBytes:selected.retainedBytes})
  }
 }
 if(process.env.CAD_INSTANCE_PROFILE_OUTPUT)writeFileSync(process.env.CAD_INSTANCE_PROFILE_OUTPUT,JSON.stringify(results,null,2))
},120000)
