import assert from 'node:assert/strict'
import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {cadRoadmapParts} from '../benchmarks/cad-roadmap-fixtures'
import {exactLayeredPrismFillet,transformNurbsBrep} from '../src/services/geometry/brep'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
import {importDirectStepV9} from '../src/services/cadNurbsStep'
import {stringifyMeshJson} from '../src/services/meshJson'
const directory=resolve(process.argv[2]??'output/qualification/enclosure-fillet');mkdirSync(directory,{recursive:true})
const body=cadRoadmapParts().find(p=>p.body.id==='enclosure')!.body,source=body.brep!
const edges=source.edges.flatMap((e,i)=>e.vertices.every(v=>{const p=source.vertices[v].point;return p[0]===0&&p[1]===0})?[i]:[])
assert.ok(edges.length>1)
writeFileSync(resolve(directory,'fixture.json'),stringifyMeshJson({version:1,sketches:[],bodies:[body]}))
writeFileSync(resolve(directory,'edge-id.txt'),source.topologyIds!.edges[edges[0]])
writeFileSync(resolve(directory,'edge-ids.json'),JSON.stringify(edges.map(i=>source.topologyIds!.edges[i])))
const parts=[]
for(const rotated of [false,true])for(const radius of [1,4]){
 const model=rotated?transformNurbsBrep(source,[[0,0,1,7],[1,0,0,-3],[0,1,0,11],[0,0,0,1]]):source
 const before=JSON.stringify(model),result=exactLayeredPrismFillet(model,edges,radius)
 assert.ok(result.audit.ok&&result.namingComplete);assert.equal(JSON.stringify(model),before)
 const first=await exportSolidStepCurrent({...body,brep:result.model})
 const second=await exportSolidStepCurrent({...body,brep:importDirectStepV9(first).model})
 for(const [cycle,text] of [first,second].entries()){
  const name=`enclosure-r${radius}-${rotated?'placed':'base'}-${cycle}`,file=name+'.step'
  writeFileSync(resolve(directory,file),text)
  parts.push({name,file,sha256:createHash('sha256').update(text).digest('hex'),expected:{volumeMm3:7152-(1-Math.PI/4)*radius**2*20,boundsMm:rotated?[[7,-3,11],[27,37,41]]:[[0,0,0],[40,30,20]]}})
 }
}
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts},null,2)+'\n')
console.log(`Exported ${parts.length} enclosure fillet STEP models`)
