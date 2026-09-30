import assert from 'node:assert/strict'
import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {cadRoadmapParts} from '../benchmarks/cad-roadmap-fixtures'
import {exactAnnularFillet,transformNurbsBrep} from '../src/services/geometry/brep'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
import {importDirectStepV9} from '../src/services/cadNurbsStep'
import {stringifyMeshJson} from '../src/services/meshJson'
const directory=resolve(process.argv[2]??'output/qualification/annular-fillet');mkdirSync(directory,{recursive:true})
const body=cadRoadmapParts().find(p=>p.body.id==='flange')!.body,source=body.brep!
const edges=source.edges.flatMap((e,i)=>e.curve.degree===2&&e.vertices.every(v=>{const p=source.vertices[v].point;return Math.abs(p[2]-6)<1e-8&&Math.abs(Math.hypot(p[0],p[1])-20)<1e-8})?[i]:[])
assert.equal(edges.length,4)
writeFileSync(resolve(directory,'fixture.json'),stringifyMeshJson({version:1,sketches:[],bodies:[body]}))
writeFileSync(resolve(directory,'edge-id.txt'),source.topologyIds!.edges[edges[0]])
writeFileSync(resolve(directory,'edge-ids.json'),JSON.stringify(edges.map(i=>source.topologyIds!.edges[i])))
const parts=[]
for(const rotated of [false,true])for(const radius of [.25,1,2.5]){
 const model=rotated?transformNurbsBrep(source,[[0,0,1,7],[1,0,0,-3],[0,1,0,11],[0,0,0,1]]):source
 const before=JSON.stringify(model),result=exactAnnularFillet(model,edges,radius)
 assert.ok(result.audit.ok&&result.namingComplete);assert.equal(JSON.stringify(model),before)
 const first=await exportSolidStepCurrent({...body,brep:result.model})
 const second=await exportSolidStepCurrent({...body,brep:importDirectStepV9(first).model})
 for(const [cycle,text] of [first,second].entries()){
  const name=`flange-r${radius}-${rotated?'placed':'base'}-${cycle}`,file=name+'.step'
  writeFileSync(resolve(directory,file),text)
  const moment=(20-radius)*(1-Math.PI/4)*radius**2+radius**3/6
  parts.push({name,file,sha256:createHash('sha256').update(text).digest('hex'),expected:{volumeMm3:2250*Math.PI-2*Math.PI*moment,boundsMm:rotated?[[7,-23,-9],[13,17,31]]:[[-20,-20,0],[20,20,6]]}})
 }
}
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts},null,2)+'\n')
console.log(`Exported ${parts.length} annular fillet STEP models`)
