import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {inspectNurbsBrep,type NurbsBrep} from '../src/kernel/geometry/brep'
import {callGeometryRust} from '../src/kernel/geometry/kernel'
import {readNativeGeometryData} from '../src/kernel/nativeGeometrySnapshot'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function build(example:string, operation:string, edit?:(source:string)=>string){
  const source=readFileSync('examples/rush/'+example+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(edit ? edit(source) : source).document)
  expect(scene.meshes).toHaveLength(1)
  const report=JSON.parse(scene.measurements!.find(row=>row.operation===operation)!.report)
  return {scene,report}
}

it('keeps the capped blade as a closed rational B-rep after tessellation',async()=>{
  const {scene,report}=await build('section-blade','brep_section_blade')
  expect(report).toMatchObject({stations:3,closedOwnedIncidence:true,globalEmbeddingCertified:false})
  expect(scene.volume).toBeGreaterThan(0)
  const model=readNativeGeometryData(scene.meshes[0]!.nativeGeometry!).geometry
  expect(model.faces).toHaveLength(6)
  expect(inspectNurbsBrep(model)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
  const {text}=callGeometryRust<{text:string}>('brep_nurbs_export_step_v9',{model})
  const {model:restored}=callGeometryRust<{model:NurbsBrep}>('brep_nurbs_import_step_v9',{text})
  expect(restored.bodies).toHaveLength(1)
  expect(restored.faces).toHaveLength(6)
  expect(inspectNurbsBrep(restored)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
  await expect(build('section-blade','brep_section_blade',s=>s.replace('22mm','5mm'))).rejects.toThrow(/increasing Z/)
},180000)

it('lofts retained rational sections and offers a separately reported dome tip',async()=>{
  const open=await build('wing-loft-surface','wing_loft_surface')
  expect(open.report).toMatchObject({stations:3,tip:'open',grid:16,flipped:[],degenerate:[],globalInjectivityCertified:false})
  const tip=await build('wing-loft-surface','wing_loft_surface',s=>s.replace('wing_tip:"open",tip_height_mm:0mm','wing_tip:"dome",tip_height_mm:4mm'))
  expect(tip.report).toMatchObject({tip:'dome',solidTopologyCertified:false})
  for(const [result,top] of [[open,50],[tip,54]] as const){
    const z=Array.from(result.scene.meshes[0]!.vertices).filter((_,index)=>index%6===2)
    expect(Math.max(...z)).toBeCloseTo(top,5)
  }
},180000)
