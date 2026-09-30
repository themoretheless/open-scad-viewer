import assert from 'node:assert/strict'
import {mkdirSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {extrudeSketchProfile} from '../src/services/directExtrusion'
import {exactSimplePrismFillet,tessellateNurbsBrep,transformNurbsBrep} from '../src/services/geometry/brep'
import {exportSolidStepCurrent} from '../src/services/solidStepExchange'
import {DirectHistory,parseDirectDocument} from '../src/services/directModeling'
import {stringifyMeshJson} from '../src/services/meshJson'
import {importDirectStepV9} from '../src/services/cadNurbsStep'
const directory=resolve(process.argv[2]??'output/qualification/simple-prism-fillet')
mkdirSync(directory,{recursive:true})
const parts=[]
for(const shape of ['L','C'] as const)for(const rotated of [false,true]){
 const points: [number,number][]=shape==='L'?[[0,0],[40,0],[40,5],[5,5],[5,30],[0,30]]:[[0,0],[10,0],[10,2],[2,2],[2,8],[10,8],[10,10],[0,10]]
 const height=shape==='L'?20:5,radius=.5
 const base=extrudeSketchProfile([{id:'profile',name:shape,closed:true,points}],height)
 const edge=base.edges.findIndex(e=>e.vertices.every(i=>base.vertices[i].point[0]===0&&base.vertices[i].point[1]===0))
 assert.ok(edge>=0)
 const source=rotated?transformNurbsBrep(base,[[0,0,1,11],[1,0,0,-7],[0,1,0,3],[0,0,0,1]]):base
 const snapshot=stringifyMeshJson(source)
 const body={id:'part',name:shape,brep:source,mesh:tessellateNurbsBrep(source,8)}
 if(shape==='L'&&!rotated){
  writeFileSync(resolve(directory,'fixture.json'),stringifyMeshJson({version:1,sketches:[],bodies:[body]}))
  writeFileSync(resolve(directory,'edge-id.txt'),source.topologyIds!.edges[edge])
 }
 const result=exactSimplePrismFillet(source,[edge],radius)
 assert.ok(result.audit.ok&&result.namingComplete&&result.certificate.complete)
 const history=new DirectHistory({version:1,sketches:[],bodies:[body]})
 const rounded={...body,brep:result.model,mesh:tessellateNurbsBrep(result.model,8)}
 history.commit({...history.document,bodies:[rounded]})
 assert.deepEqual(history.undo().bodies[0].brep,source)
 const restored=parseDirectDocument(stringifyMeshJson(history.redo())).bodies[0]
 assert.deepEqual(restored.brep,result.model)
 assert.equal(stringifyMeshJson(source),snapshot)
 const text=await exportSolidStepCurrent(restored),file=`${shape}-${rotated?'placed':'base'}.step`
 writeFileSync(resolve(directory,file),text)
 const x=shape==='L'?40:10,y=shape==='L'?30:10
 parts.push({name:`${shape} R${radius}${rotated?' rotated':''}`,file,sha256:createHash('sha256').update(text).digest('hex'),expected:{volumeMm3:(shape==='L'?6500:260)-radius**2*(1-Math.PI/4)*height,boundsMm:rotated?[[11,-7,3],[11+height,-7+x,3+y]]:[[0,0,0],[x,y,height]]}})
 const imported=importDirectStepV9(text)
 const reexport=await exportSolidStepCurrent({...restored,brep:imported.model})
 const roundtripFile=file.replace('.step','-roundtrip.step')
 writeFileSync(resolve(directory,roundtripFile),reexport)
 parts.push({...parts[parts.length-1],name:`${shape} ${rotated?'placed':'base'} STEP roundtrip`,file:roundtripFile,sha256:createHash('sha256').update(reexport).digest('hex')})
}
writeFileSync(resolve(directory,'manifest.json'),JSON.stringify({schema:'cad-roadmap-step/1',units:'mm',toleranceMm:1e-6,relativeVolumeTolerance:1e-8,parts},null,2)+'\n')
console.log(`Exported ${parts.length} restored fillet models`)
