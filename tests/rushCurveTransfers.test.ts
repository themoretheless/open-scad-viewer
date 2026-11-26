import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function run(name:string){
  const code=readFileSync('examples/rush/'+name+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(code).document)
  expect(scene.meshes).toHaveLength(1)
  return scene
}

it('pulls back a world curve and lifts the certified UV curve onto its original panel',async()=>{
  const scene=await run('pullback-curve-to-uv')
  const report=JSON.parse(scene.measurements!.find(row=>row.operation==='pullback_curve_to_uv')!.report)
  expect(report).toMatchObject({withinTolerance:true,status:'within_tolerance'})
  const ys=Array.from(scene.meshes[0]!.vertices).filter((_,index)=>index%6===1)
  expect(ys.length).toBeGreaterThan(0)
  expect(ys.every(y=>Math.abs(y-1.5)<1e-8)).toBe(true)
},180000)

it('retains the authored bevel between two offset profile segments',async()=>{
  const scene=await run('bevel-offset-wire')
  const report=JSON.parse(scene.measurements!.find(row=>row.operation==='bevel_offset_wire')!.report)
  expect(report).toMatchObject({accepted:true,wholeWire:true,closed:false,piece:0,pieceCount:1})
  expect(report.cells.map((cell:{source:{kind:string}})=>cell.source.kind)).toEqual(['source-offset','bevel','source-offset'])
  expect(report.errorUpperMm).toBeLessThanOrEqual(0.001)
  const zs=Array.from(scene.meshes[0]!.vertices).filter((_,index)=>index%6===2)
  expect(Math.min(...zs)).toBeCloseTo(2,8)
  expect(Math.max(...zs)).toBeCloseTo(3,8)
  expect(scene.surfaceArea).toBeCloseTo(7+Math.SQRT2,6)
},180000)
