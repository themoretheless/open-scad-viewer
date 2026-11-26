import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {callGeometryRust} from '../src/kernel/geometry/kernel'
import {inspectNurbsBrep,type NurbsBrep} from '../src/kernel/geometry/brep'
import {readNativeGeometryData} from '../src/kernel/nativeGeometrySnapshot'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

it.each([
  ['intersection',1,38],
  ['union',1,null],
  ['extract',1,34],
  ['xor',2,null],
] as const)('builds a closed spinner bore %s in WASM',async(name,bodies,faces)=>{
  const source=readFileSync('examples/rush/spinner-bore-'+name+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  const model=readNativeGeometryData(scene.meshes[0]!.nativeGeometry!).geometry
  expect(model.bodies).toHaveLength(bodies)
  if(faces!==null) expect(model.faces).toHaveLength(faces)
  expect(inspectNurbsBrep(model)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
},240000)

it('shares identical STEP UV curves on the edited spinner without opening the shell',async()=>{
  const source=readFileSync('examples/rush/cut-bore-cylinder.r','utf8').replace(
    'result = sun.cut_bore_cylinder(radius_mm:22mm,height_mm:10mm)',
    'cutter = brep_cylinder(radius:22mm,height:10mm)\nresult = brep_subtract(sun,cutter)',
  )
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  const model=readNativeGeometryData(scene.meshes[0]!.nativeGeometry!).geometry
  const {text}=callGeometryRust<{text:string}>('brep_nurbs_export_step_v9',{model})
  const references=[...text.matchAll(/=DEFINITIONAL_REPRESENTATION\('',\(#(\d+)\),#/g)].map(match=>match[1])
  expect(references).toHaveLength(model.loops.reduce((total,wire)=>total+wire.coedges.length,0))
  expect(new Set(references).size*2).toBeLessThan(references.length)
  const restored=callGeometryRust<{model:NurbsBrep}>('brep_nurbs_import_step_v9',{text}).model
  expect(restored.bodies).toHaveLength(1)
  expect(restored.faces).toHaveLength(model.faces.length)
  expect(inspectNurbsBrep(restored)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
},240000)
