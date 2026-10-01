import assert from 'node:assert/strict'
import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {cadRoadmapParts} from '../benchmarks/cad-roadmap-fixtures'
import {createBrepBox,exactValence3CornerBlend} from '../src/services/geometry/brep'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
import {importDirectStepV9} from '../src/services/cadNurbsStep'
const directory=resolve(process.argv[2]??'output/qualification/corner-blend')
mkdirSync(directory,{recursive:true})
const min=[-7,3,-2],max=[3,11,4],source=createBrepBox(min,max)
const body=cadRoadmapParts()[0].body,parts=[]
for(let mask=0;mask<8;mask++)for(const radius of [.25,1,2.5]){
 const corner=min.map((v,i)=>mask&(1<<i)?max[i]:v)
 const vertex=source.vertices.findIndex(v=>v.point.every((p,i)=>Math.abs(p-corner[i])<1e-9))
 assert.ok(vertex>=0)
 const edges=source.edges.flatMap((e,i)=>e.vertices.includes(vertex)?[i]:[])
 assert.equal(edges.length,3)
 const before=JSON.stringify(source),result=exactValence3CornerBlend(source,edges,radius)
 assert.ok(result.audit.ok&&result.namingComplete&&result.certificate.complete)
 assert.equal(JSON.stringify(source),before)
 const first=await exportSolidStepCurrent({...body,brep:result.model})
 const second=await exportSolidStepCurrent({...body,brep:importDirectStepV9(first).model})
 // Three rectangular edge prisms minus their quarter cylinders, plus
 // the common corner cube minus the sphere octant. The regions are disjoint.
 const volume=480-(1-Math.PI/4)*radius**2*(24-3*radius)-(1-Math.PI/6)*radius**3
 for(const [cycle,text] of [first,second].entries()){
  const name=`corner-${mask}-r${radius}-${cycle}`,file=name+'.step'
  writeFileSync(resolve(directory,file),text)
  parts.push({name,file,sha256:createHash('sha256').update(text).digest('hex'),expected:{volumeMm3:volume,boundsMm:[min,max]}})
 }
}
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts},null,2)+'\n')
console.log(`Exported ${parts.length} corner blend STEP models`)
