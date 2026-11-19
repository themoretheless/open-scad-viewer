import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const source=readFileSync('examples/rush/periodic-trim-sheet.r','utf8')
it.each(['surface_trim_sheet','surface_trim_sheet_edge_bindings','surface_trim_sheet_input_bindings'])('constructs and tessellates an open periodic sheet using %s',async(op)=>{
  const program=source.replace('surface_trim_sheet_input_bindings',op)
  const scene=await buildTextNurbsScene(compileRushFrontend(program).document)
  expect(scene.meshes.length).toBe(1)
  expect(scene.meshes[0]!.indices.length).toBeGreaterThan(0)
  expect(scene.volume).toBe(0)
  // The two planar sides have area 1.5 mm² before subtracting the 0.25 mm² hole.
  // The display grid approximates the fold. Refinement must converge to the exact sheet area.
  const fine=await buildTextNurbsScene(compileRushFrontend(program.replace('brep_tessellate(8)','brep_tessellate(32)')).document)
  expect(Math.abs(fine.surfaceArea-1.25)).toBeLessThan(Math.abs(scene.surfaceArea-1.25)/3)
  expect(Math.abs(fine.surfaceArea-1.25)).toBeLessThan(0.006)
  const row=scene.measurements![0]!
  expect(row.operation).toBe(op)
  const p=JSON.parse(row.report)
  expect(p.refusalStage).toBeNull()
  if(op.endsWith('_bindings')){
    expect(new Set(p.edgeBindings.map((e:{edgeId:string})=>e.edgeId)).size).toBe(p.edgeBindings.length)
    if(op.endsWith('edge_bindings')){
      expect(p.edgeBindings.filter((b:{contour:number;segment:number})=>b.contour===0 && b.segment===0).map((b:{interval:number[]})=>b.interval)).toEqual([[0,0.5],[0.5,1]])
    }
    if(op.endsWith('input_bindings')){
      expect(p.edgeBindings[0].parameters).toEqual([{numerator:'0',denominator:'1'},{numerator:'1',denominator:'2'}])
      expect(p.edgeBindings.find((e:{contour:number})=>e.contour===1).segment).toBe(3)
    }
  }
})
it('surfaces native refusal when sheet construction has insufficient work',async()=>{
  await expect(buildTextNurbsScene(compileRushFrontend(source.replace('max_work:1000000','max_work:1')).document)).rejects.toThrow(/refused/i)
})

it('round-trips the sheet emitted by the editor with its hole through STEP',async()=>{
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  const artifact=scene.meshes[0]!.nativeGeometry!
  expect(artifact.kind).toBe('brep')
  const model=JSON.parse(artifact.geometryJson).geometry
  expect(model.faces[0].holes).toEqual([1])
  expect(model.shells[0].closed).toBe(false)
  const {exportDirectStepV9,importDirectStepV9}=await import('../src/kernel/cadNurbsStep')
  const step=exportDirectStepV9(model)
  const restored=importDirectStepV9(step.text)
  expect(restored.model.faces).toHaveLength(1)
  expect(restored.model.faces[0]!.holes).toHaveLength(1)
  expect(restored.model.shells[0]!.closed).toBe(false)
})
