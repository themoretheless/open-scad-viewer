import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const program=readFileSync('examples/rush/coordinate-frame-bounds.r','utf8')
const build=async(code:string)=>buildTextNurbsScene(compileRushFrontend(code).document)
it('round-trips a weighted curve through local coordinates and encloses mixed geometry',async()=>{
  const scene=await build(program)
  const reports=new Map(scene.measurements!.map(m=>[m.operation,JSON.parse(m.report)]))
  expect(reports.get('geometry_conditioning')).toMatchObject({weightRatio:2,minRelativeKnotSpan:[1],coordinateOffsetRatio:0.5,zeroExtent:false})
  expect(reports.get('geometry_bounds_union')).toMatchObject({min:[-1,0,0],max:[12,6,0]})
  const mesh=scene.meshes[0]!,xs=Array.from(mesh.vertices).filter((_,i)=>i%6===0)
  expect(Math.min(...xs)).toBe(8)
  expect(Math.max(...xs)).toBe(12)
})
it('inspects both surface knot directions and refuses an invalid coordinate frame',async()=>{
  const scene=await build(program.replace('show bounds.tessellate','show bounds.geometry_conditioning().tessellate'))
  const [row]=scene.measurements!.filter(r=>r.operation==='geometry_conditioning').slice(-1)
  expect(JSON.parse(row!.report).minRelativeKnotSpan).toEqual([1,1])
  await expect(build(program.replace('frame_scale:0.5','frame_scale:0'))).rejects.toThrow(/scale/i)
})
