import {readFileSync} from 'node:fs'
import {IDBFactory} from 'fake-indexeddb'
import {describe,expect,it} from 'vitest'
import {exportDirectStepV10,importDirectStepV10} from '../src/services/cadNurbsStep'
import {importTessellatedStepV10} from '../src/services/cadStep'
import {importStepForWorkbench} from '../src/services/cadStepRouting'
import {loadProjectStepModel,saveProjectStepModel} from '../src/services/cadStepIndexedDb'

const assembly=()=>readFileSync(new URL('./fixtures/step-v6/self-authored-ap242-assembly.step',import.meta.url),'utf8')
const reorderIds=(text:string)=>{
 const ids=[...new Set([...text.matchAll(/#(\d+)/g)].map(match=>Number(match[1])))].sort((a,b)=>a-b)
 const remap=new Map(ids.map((id,index)=>[id,900000+ids.length-index]))
 return text.replace(/#(\d+)/g,(_,id)=>`#${remap.get(Number(id))}`)
}
const tessellated=`ISO-10303-21;
HEADER;FILE_DESCRIPTION(('AP242 tessellation'),'2;1');FILE_NAME('x','',(''),(''),'','','');FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));ENDSEC;
DATA;
#1=CARTESIAN_POINT_LIST_3D('',((0.,0.,0.),(1.,0.,0.),(1.,1.,0.),(0.,1.,0.),(0.5,0.5,1.)));
#2=COMPLEX_TRIANGULATED_FACE_SET('item',#1,5,((0.,0.,1.)),(1,2,3,4,5),((1,2,3,4)),((5,1,2,3,4)));
#3=COLOUR_RGB('red',1.,0.,0.);
#4=PRESENTATION_LAYER_ASSIGNMENT('layer-a','',(#2));
#5=TEXTURE_COORDINATE('uv',#2,((0.,0.),(1.,0.),(1.,1.)));
#6=TESSELLATED_SHAPE_REPRESENTATION('rep',(#2),$);
ENDSEC;END-ISO-10303-21;`

describe('STEP /10 retained geometry products',()=>{
 it('preserves the authoritative occurrence graph without baking on export',()=>{
  const imported=importDirectStepV10(assembly())
  expect(imported.certificate.capability).toBe('step-interchange/10')
  expect(imported.model.bodies).toHaveLength(3)
  expect(imported.document.definitionIdentities).toHaveLength(1)
  expect(imported.document.occurrenceIdentities).toHaveLength(3)
  expect(imported.document.operatorIdentities.length).toBeGreaterThan(0)
  expect(exportDirectStepV10(imported.document).text).toBe(assembly())
  const reordered=importDirectStepV10(reorderIds(assembly()))
  expect(reordered.document.graphIdentity).toBe(imported.document.graphIdentity)
  expect(exportDirectStepV10(reordered.document).text).toBe(reorderIds(assembly()))
 })

 it('expands strips and fans while retaining normals, layers and texture presentation',()=>{
  const imported=importTessellatedStepV10(tessellated)
  expect(imported.bodies[0].mesh.indices.length).toBe(15)
  expect(imported.presentation[0]).toMatchObject({primitive:'strips-and-fans',layers:['#4']})
  expect(imported.presentation[0].normals).toEqual([0,0,1])
  expect(imported.presentation[0].colors).toEqual(['#3'])
  expect(imported.presentation[0].textureCoordinates.length).toBeGreaterThan(0)
  expect(imported.metadataLoss).toEqual([])
  expect(importStepForWorkbench(tessellated).report.identityLoss).toEqual([])
 })

 it('round-trips the retained graph through IndexedDB',async()=>{
  Object.defineProperty(globalThis,'indexedDB',{configurable:true,value:new IDBFactory()})
  const routed=importStepForWorkbench(assembly())
  await saveProjectStepModel(routed.retainedModel!,routed.report,routed.retainedDocument)
  const loaded=await loadProjectStepModel()
  expect(loaded?.document?.graphIdentity).toBe(routed.retainedDocument?.graphIdentity)
  expect(exportDirectStepV10(loaded!.document!).text).toBe(assembly())
 })

 it('keeps procedural CSG as a precise unsupported report',()=>{
  expect(()=>importStepForWorkbench("ISO-10303-21;DATA;#1=CSG_SOLID('',#2);ENDSEC;END-ISO-10303-21;"))
   .toThrow(/semantic-csg.*no explicit semantic evaluator.*no mesh relabeling/i)
 })

 it('refuses retained graph exports beyond the native resource boundary',()=>{
  const document=importDirectStepV10(assembly()).document
  expect(()=>exportDirectStepV10({...document,source:'x'.repeat(16*1024*1024+1)})).toThrow(/exceeds 16 MiB/)
 })
})
