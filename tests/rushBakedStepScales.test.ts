import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {inspectNurbsBrep} from '../src/kernel/geometry/brep'
import {readNativeGeometryData} from '../src/kernel/nativeGeometrySnapshot'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const example=() => readFileSync('examples/rush/step-baked-scales.r','utf8')

it('converts reflected and nonuniform placements to reusable named STEP definitions',async()=>{
  const scene=await buildTextNurbsScene(compileRushFrontend(example()).document)
  const result=scene.measurements!.find(row=>row.operation==='bake_step_scales')!
  const report=JSON.parse(result.report)
  expect(report).toMatchObject({assemblyName:'Drive',occurrenceCount:4})
  expect(report.parts.map((part:{name:string;quantity:number})=>part.name+':'+part.quantity).sort()).toEqual(['Pin:1','Roller:1','Roller:2'])
  expect(result.step!.match(/MANIFOLD_SOLID_BREP\(/g)).toHaveLength(3)
  expect(result.step!.match(/ITEM_DEFINED_TRANSFORMATION\(/g)).toHaveLength(4)
  expect(result.step).not.toContain('CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM(')
  const model=readNativeGeometryData(scene.meshes[0]!.nativeGeometry!).geometry
  expect(inspectNurbsBrep(model)).toMatchObject({topologyValid:true,boundaryEdgeCount:0})
  expect(model.faces.some(face=>face.surface.weights.flat().some(weight=>weight!==1))).toBe(true)
  expect(Math.min(...model.vertices.map(vertex=>vertex.point[0]))).toBeCloseTo(56)
  expect(Math.max(...model.vertices.map(vertex=>vertex.point[0]))).toBeCloseTo(64)
  expect(Math.max(...model.vertices.map(vertex=>vertex.point[2]))).toBeCloseTo(12)
},180000)

it('keeps named groups and distinct instance colours on the shared reflected geometry',async()=>{
  const source=example().replace('baked = named.bake_step_scales(preview_index:3)',
    'baked = named.style_step_occurrences(group_names:["Rollers"],parent_indices:[0,-1,-1,0],selection:[0,3],style_names:["Red","Blue"],style_colours:[[1,0,0],[0,0,1]],preview_index:0)')
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  const result=scene.measurements!.find(row=>row.operation==='style_step_occurrences')!
  const report=JSON.parse(result.report)
  const colorAt=(x:number)=>report.colours[report.origins.findIndex((origin:number[])=>origin[0]===x)]
  expect(colorAt(0)).toEqual([1,0,0])
  expect(colorAt(60)).toEqual([0,0,1])
  expect(colorAt(20)).toBeNull()
  expect(report.definitionCount).toBe(3)
  expect(report.paths.some((path:string[])=>path.join('/').includes('Drive/Rollers/Roller'))).toBe(true)
},180000)
