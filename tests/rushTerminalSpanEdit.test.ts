import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const program=readFileSync('examples/rush/terminal-span-edit.r','utf8')
const build=async(text:string)=>buildTextNurbsScene(compileRushFrontend(text).document)
it('removes an exact redundant control and grows opposite parameter ends by physical distances',async()=>{
  const scene=await build(program)
  const r=new Map(scene.measurements!.map(m=>[m.operation,JSON.parse(m.report)]))
  expect(r.get('simplify_linear_curve')).toMatchObject({retainedControls:[0,2],removedControls:[1],complete:true})
  expect(r.get('extend_terminal_curve')).toMatchObject({domain:[0,3],requestedContinuity:'G2',continuityCertified:false,stable:true})
  expect(r.get('extend_surface_boundary')).toMatchObject({requestedContinuity:'C2',continuityCertified:false,stable:true})
  const vertices=scene.meshes[0]!.vertices
  const x=Array.from(vertices).filter((_,i)=>i%6===0),y=Array.from(vertices).filter((_,i)=>i%6===1)
  expect(Math.max(...x)).toBeCloseTo(3,7)
  expect(Math.min(...y)).toBeCloseTo(-0.5,7)
})
it('preserves an original change in parameter speed and rejects nonpositive extension distances',async()=>{
  const scene=await build(program.replace('knots:[0,0,1,2,2]','knots:[0,0,0.75,2,2]'))
  expect(JSON.parse(scene.measurements![0]!.report).removedControls).toEqual([])
  await expect(build(program.replace('distance_mm:1mm','distance_mm:0mm'))).rejects.toThrow(/positive/i)
})
