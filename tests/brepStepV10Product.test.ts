import {readFileSync} from 'node:fs'
import {IDBFactory} from 'fake-indexeddb'
import {describe,expect,it} from 'vitest'
import {exportDirectStepV10,importDirectStepV10} from '../src/kernel/cadNurbsStep'
import {importTessellatedStepV10} from '../src/kernel/cadStep'
import {importStepForWorkbench} from '../src/kernel/cadStepRouting'
import {loadProjectStepModel,saveProjectStepModel} from '../src/kernel/cadStepIndexedDb'

const assembly=()=>readFileSync(new URL('./fixtures/step-v6/self-authored-ap242-assembly.step',import.meta.url),'utf8')
const identityRelationshipFixture=()=>readFileSync(new URL('./fixtures/step-v9/self-authored-ap242-cylinder.step',import.meta.url),'utf8')
 .replace('#298=SHAPE_DEFINITION_REPRESENTATION(#297,#290);','#298=SHAPE_DEFINITION_REPRESENTATION(#297,#299);')
 .replace('ENDSEC;\nEND-ISO',"#299=SHAPE_REPRESENTATION('',(),#289);\n#300=SHAPE_REPRESENTATION_RELATIONSHIP('','',#299,#290);\n#301=SHAPE_REPRESENTATION('PMI',(),#289);\n#302=SHAPE_REPRESENTATION_RELATIONSHIP('','',#299,#301);\nENDSEC;\nEND-ISO")
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
 it('selects one exact B-rep through identity relationships and retains PMI as metadata',()=>{
  const text=identityRelationshipFixture()
  const imported=importDirectStepV10(text)
  expect(imported.model.bodies).toHaveLength(1)
  expect(imported.document.source).toBe(text)
  expect(imported.document.metadataLoss).toContain('projection.non_exact_representations_retained_as_metadata')
  expect(exportDirectStepV10(imported.document).text).toBe(text)

  const multipleChildren=text.replace('ENDSEC;\nEND-ISO',"#303=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#286),#289);\n#304=SHAPE_REPRESENTATION_RELATIONSHIP('','',#299,#303);\nENDSEC;\nEND-ISO")
  expect(()=>importDirectStepV10(multipleChildren)).toThrow(/Ambiguous multiple exact/)
  const multipleRoots=text.replace('ENDSEC;\nEND-ISO',"#303=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#286),#289);\n#304=SHAPE_DEFINITION_REPRESENTATION(#297,#303);\nENDSEC;\nEND-ISO")
  expect(()=>importDirectStepV10(multipleRoots)).toThrow(/Ambiguous multiple exact/)
  const cycle=text.replace('ENDSEC;\nEND-ISO',"#303=SHAPE_REPRESENTATION('PMI cycle',(),#289);\n#304=SHAPE_REPRESENTATION_RELATIONSHIP('','',#301,#303);\n#305=SHAPE_REPRESENTATION_RELATIONSHIP('','',#303,#301);\nENDSEC;\nEND-ISO")
  expect(()=>importDirectStepV10(cycle)).toThrow(/Cycle in identity representation/)
 })

 it('retains presentation and PMI metadata without adding exact roots or LOD promises',()=>{
  const text=identityRelationshipFixture().replace('ENDSEC;\nEND-ISO',"#303=SURFACE_STYLE_PARAMETER_LINE(#301,(U_DIRECTION_COUNT(2),V_DIRECTION_COUNT(2)));\n#304=SURFACE_STYLE_TRANSPARENT(0.5);\n#305=TESSELLATED_SHAPE_REPRESENTATION('auxiliary',(),#289);\n#306=CONCENTRICITY_TOLERANCE('','',#301,#301,(#301));\n#307=PRODUCT_DEFINITION_CONTEXT_ROLE('additional context','');\n#308=SYMMETRY_TOLERANCE('','',#301,#301,(#301));\n#309=ROUNDNESS_TOLERANCE('','',#301,#301,(#301));\n#310=CYLINDRICITY_TOLERANCE('','',#301,#301,(#301));\n#311=APPLIED_DOCUMENT_REFERENCE(#301,'');\n#312=TOTAL_RUNOUT_TOLERANCE('','',#301,#301,(#301));\n#313=OBJECT_ROLE('PMI');\n#314=LIMITS_AND_FITS('','',#301,#301);\n#315=DOCUMENT_PRODUCT_EQUIVALENCE('','',#301,#301);\n#316=ROLE_ASSOCIATION(#313,#301);\nENDSEC;\nEND-ISO")
  const imported=importDirectStepV10(text)
  expect(imported.model.bodies).toHaveLength(1)
  expect(imported.document.source).toBe(text)
  expect(imported.document.metadataLoss).toContain('projection.non_exact_representations_retained_as_metadata')
  expect(exportDirectStepV10(imported.document).text).toBe(text)
 })

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
