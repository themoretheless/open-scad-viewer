import { readFileSync } from 'node:fs'
import { expect,it } from 'vitest'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {parseOpenSCAD} from '../src/services/openscadParser'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
const source=readFileSync('examples/modelgraph-text/nurbs-boolean.scad','utf8')
it('lowers compact parameters and mesh booleans to own Rust and renders directly',async()=>{
 const compiled=compileModelGraphText(source)
 expect(compiled.execution_target).toBe('own-nurbs')
 expect(compiled.document.language).toBe('modelgraph/nurbs-1')
 expect(compiled.customizer).toHaveLength(2)
 const result=buildOwnNurbs(compiled.document,{action:'build'})
 expect(result.report.mesh?.signedVolumeMm3).toBeCloseTo(1200,7)
 const scene=await parseOpenSCAD(source)
 expect(scene.meshes).toHaveLength(1)
 expect(scene.volume).toBeCloseTo(1200,7)
 expect(scene.meshes[0]!.topology.nonManifold).toBe(0)
 expect(scene.meshes[0]!.provenance[0]!.source).toBeNull()
 expect((await parseOpenSCAD(source.replace('20mm range','30mm range'))).volume).toBeCloseTo(2700,7)
})
it('supports all own boolean forms and rejects wrong types and mixed backends',()=>{
 for(const [call,volume] of [['mesh_union(body,cutter)',2800],['mesh_intersection(body,cutter)',400]] as const){
  const text=source.replace('show body |> mesh_subtract(cutter)',`show ${call}`)
  expect(buildOwnNurbs(compileModelGraphText(text).document,{action:'build'}).report.mesh?.signedVolumeMm3).toBeCloseTo(volume,7)
 }
 expect(()=>compileModelGraphText(source.replace('mesh_subtract(cutter)','mesh_subtract(patch)'))).toThrow() // unreachable cutter
 expect(()=>compileModelGraphText(source.replace('degree_u: 1','degree_u: 1mm'))).toThrow(/Expected/)
 expect(()=>compileModelGraphText(source.replace('show body |> mesh_subtract(cutter)','show union(body,cutter)'))).toThrow(/cannot be mixed/)
})

it('handles empty viewer results and cancellation',async()=>{
 const base=source.slice(0,source.indexOf('cutter ='))
 const empty=await parseOpenSCAD(base+'show body |> mesh_subtract(body)')
 expect(empty.meshes).toEqual([])
 expect(empty.volume).toBe(0)
 await expect(parseOpenSCAD(source,{shouldAbort:()=>true})).rejects.toThrow(/abort/i)
 expect(()=>buildOwnNurbs(compileModelGraphText(base+'show body |> mesh_subtract(patch)').document,{action:'build'})).toThrow(/mesh/i)
})

it('constructs polygons natively from a compact profile',()=>{
 const text='// @modelgraph-text/1\np = polygon_profile(outer: [[0mm,0mm],[2mm,0mm],[2mm,2mm],[0mm,2mm]])\nshow p |> extrude(3mm)'
 const result=buildOwnNurbs(compileModelGraphText(text).document,{action:'build'})
 expect(result.report.mesh?.signedVolumeMm3).toBeCloseTo(12)
 expect(result.report.kernel).toBe('own-rust-geometry')
})
