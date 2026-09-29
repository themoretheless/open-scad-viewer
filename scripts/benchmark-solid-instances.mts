import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {performance} from 'node:perf_hooks'
import {emptyDirectDocument,DirectHistory,parseDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {resolveSolidInstances} from '../src/services/solidInstances'
import {stringifyMeshJson} from '../src/services/meshJson'

// Measures materialized instances. These numbers do not imply GPU instancing or browser FPS.
const directory=resolve(process.argv[2]??'output/qualification/solid-instances')
mkdirSync(directory,{recursive:true})
const measurements=[]
const counts=process.argv[3]?[Number(process.argv[3])]:[100,199,500,1000]
if(counts.some(n=>!Number.isInteger(n)||n<1||n>1000))throw Error('Instance count must be an integer from 1 to 1000')
for(const count of counts) {
 const memoryBefore=process.memoryUsage()
 const doc=emptyDirectDocument(),brep=createBrepBox([0,0,0],[2,3,4]),mesh=tessellateNurbsBrep(brep,1)
 const source={id:'source',name:'Source',brep,mesh};doc.bodies.push(source)
 for(let i=0;i<count;i++)doc.bodies.push({...structuredClone(source),id:`instance-${i}`,name:`Instance ${i}`,instance:{sourceId:'source',matrix:[[1,0,0,(i%25)*5],[0,1,0,Math.floor(i/25)*5],[0,0,1,0],[0,0,0,1]]}})
 const start=performance.now(),resolved=resolveSolidInstances(doc),resolveMs=performance.now()-start
 const text=stringifyMeshJson(resolved);writeFileSync(resolve(directory,`instances-${count}.json`),text)
 try {parseDirectDocument(text)} catch(error) {
  const rejected={instances:count,bodies:count+1,jsonBytes:Buffer.byteLength(text),resolveMs,rejected:error instanceof Error?error.message:String(error)}
  measurements.push(rejected);console.log(JSON.stringify(rejected));continue
 }
 const compact=serializeDirectDocument(resolved);writeFileSync(resolve(directory,`instances-${count}-compact.json`),compact)
 if(parseDirectDocument(compact).bodies.length!==count+1)throw Error('Compact restore lost bodies')
 const durations:number[]=[]
 for(let i=0;i<5;i++){const t=performance.now();parseDirectDocument(text);durations.push(performance.now()-t)}
 const history=new DirectHistory(resolved),edit=history.document,nextBrep=createBrepBox([0,0,0],[4,3,4]);edit.bodies[0]={...source,brep:nextBrep,mesh:tessellateNurbsBrep(nextBrep,1)}
 const t=performance.now();history.commit(edit);const commitMs=performance.now()-t
 const u=performance.now();history.undo();const undoMs=performance.now()-u
 const r=performance.now();history.redo();const redoMs=performance.now()-r
 const result=history.document
 if(result.bodies.length!==count+1||result.bodies[count].instance?.sourceId!=='source')throw Error('Instance identity lost')
 const positions=result.bodies[count].mesh.positions,xs=Array.from(positions).filter((_,i)=>i%3===0)
 if(Math.abs(Math.max(...xs)-Math.min(...xs)-4)>1e-8)throw Error('Source update failed')
 measurements.push({instances:count,bodies:count+1,memoryBefore,memoryAfter:process.memoryUsage(),jsonBytes:Buffer.byteLength(text),compactBytes:Buffer.byteLength(compact),resolveMs,parseSamplesMs:durations,commitMs,undoMs,redoMs,historyStorage:history.storageStats})
 console.log(JSON.stringify(measurements.at(-1)))
}
writeFileSync(resolve(directory,'measurements.json'),JSON.stringify({runtime:process.version,platform:process.platform,arch:process.arch,scope:'Node materialized B-rep instances; no browser/GPU claims',measurements},null,2)+'\n')
