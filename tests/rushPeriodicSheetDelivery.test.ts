import {readFileSync,writeFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
import {buildOwnNurbs} from '../src/language/rushGraphNurbsKernel'
import {importDirectStepV9} from '../src/kernel/cadNurbsStep'

const display=readFileSync('examples/rush/periodic-sheet-display.r','utf8')
const step=readFileSync('examples/rush/periodic-sheet-step.r','utf8')

it('keeps exact sheet topology and honors the final requested display resolution',async()=>{
  const scene=await buildTextNurbsScene(compileRushFrontend(display).document)
  expect(scene.meshes[0]!.indices.length).toBeGreaterThan(0)
  const native=scene.meshes[0]!.nativeGeometry!
  expect(JSON.parse(native.geometryJson).geometry.faces[0].holes).toEqual([1])
  const refined=await buildTextNurbsScene(compileRushFrontend(display.replace('brep_tessellate(8)','brep_tessellate(16)')).document)
  expect(refined.meshes[0]!.indices.length).toBeGreaterThan(scene.meshes[0]!.indices.length)
})

it('exports the open shell and the current placement after editing',()=>{
  const exportModel=(source:string,output?:string)=>{
    const built=buildOwnNurbs(compileRushFrontend(source).document,{action:'export',format:'step'})
    if (!('artifact' in built) || !('text' in built.artifact)) throw new Error('Missing STEP text')
    if(output) writeFileSync(output,built.artifact.text)
    return importDirectStepV9(built.artifact.text).model
  }
  const initial=exportModel(step)
  const moved=exportModel(step.replace('show sheet','show sheet.transform(matrix:[[1,0,0,7],[0,1,0,0],[0,0,1,0],[0,0,0,1]])'),
    process.env.RUSH_PERIODIC_SHEET_STEP_OUTPUT)
  expect(initial.faces[0]!.holes).toHaveLength(1)
  expect(moved.faces[0]!.holes).toHaveLength(1)
  expect(moved.shells[0]!.closed).toBe(false)
  const minimumX=(m:typeof initial)=>Math.min(...m.vertices.map(v=>v.point[0]))
  expect(minimumX(moved)-minimumX(initial)).toBeCloseTo(7,9)
})

it('refuses STEP for a standalone curve root',()=>{
  const source='// @rush/1\nshow line_curve(start:[0mm,0mm,0mm],end:[1mm,0mm,0mm])\n'
  const {document}=compileRushFrontend(source)
  expect(()=>buildOwnNurbs(document,{action:'export',format:'step'}))
    .toThrow(/STEP export requires an exact B-rep root/)
})
