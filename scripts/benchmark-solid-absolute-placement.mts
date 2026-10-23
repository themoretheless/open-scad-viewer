import {performance} from 'node:perf_hooks'
import {mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import assert from 'node:assert/strict'
import {warmGeometryKernel} from '../src/services/geometry/kernel'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import type {DirectDocument} from '../src/services/directModeling'
import {serializeDirectDocument,parseDirectDocument} from '../src/services/directModeling'
import {applySolidSceneEdit} from '../src/services/solidSceneEdit'
import {SolidInstanceBatchCache} from '../src/services/solidInstanceBatchCache'
const directory=path.resolve(process.argv[2]??'/tmp/cad-absolute-placement-benchmark')
await mkdir(directory,{recursive:true});await warmGeometryKernel()
const rows=[]
for(const count of [100,1000]){
 const brep=createBrepBox([0,0,0],[2,3,4]),mesh=tessellateNurbsBrep(brep,1)
 const raw:DirectDocument={version:1,sketches:[],bodies:[{id:'source',name:'Source',brep,mesh},...Array.from({length:count},(_,i)=>({id:`linked-${i}`,name:`Instance ${i}`,mesh,brep,instance:{sourceId:'source',matrix:[[1,0,0,i*5],[0,1,0,0],[0,0,1,0],[0,0,0,1]]}}))]}
 const document=parseDirectDocument(serializeDirectDocument(raw))
 const before=serializeDirectDocument(document),cache=new SolidInstanceBatchCache()
 const options={operation:'instance-place' as const,id:'linked-0',ids:['linked-0'],createdId:'',x:7,y:8,z:9,axis:'z' as const,angle:0,scale:1}
 const expected=applySolidSceneEdit(document,options)
 applySolidSceneEdit(document,options,cache)
 const cached:number[]=[],uncached:number[]=[]
 for(let i=0;i<5;i++){
  for(const enabled of (i%2?[true,false]:[false,true])){
   const start=performance.now(),result=applySolidSceneEdit(document,options,enabled?cache:undefined)
   ;(enabled?cached:uncached).push(performance.now()-start)
   assert.equal(serializeDirectDocument(result),serializeDirectDocument(expected))
  }
 }
 const validated=parseDirectDocument(serializeDirectDocument(expected),cache)
 assert.equal(serializeDirectDocument(validated),serializeDirectDocument(expected))
 assert.equal(serializeDirectDocument(document),before)
 const median=(times:number[])=>[...times].sort((a,b)=>a-b)[Math.floor(times.length/2)]
 rows.push({instances:count,iterations:5,cachedMs:cached,uncachedMs:uncached,cachedMedianMs:median(cached),uncachedMedianMs:median(uncached),cacheBytes:cache.retainedBytes,cacheEntries:cache.size})
}
const report={scope:'synchronous absolute placement only; excludes worker transfer, history, rendering and UI',fixture:'box with linked translated instances; repeated identical target placement',rows}
await writeFile(path.join(directory,'benchmark.json'),JSON.stringify(report,null,2)+'\n');console.log(report)
