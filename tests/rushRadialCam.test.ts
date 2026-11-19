import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {buildOwnNurbs} from '../src/language/rushGraphNurbsKernel'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
import {compileRushFrontend} from '../src/language/rushFrontend'
const program=readFileSync('examples/rush/radial-cam.r','utf8')
it('retains every profile wall and the original lift at the start and the middle of a turn',async()=>{
  const scene=await buildTextNurbsScene(compileRushFrontend(program).document)
  const r=JSON.parse(scene.measurements![0]!.report)
  expect(r.stop).toBe('SampleEstimateReached')
  expect(r.maxSampledErrorMm).toBeLessThanOrEqual(1e-5)
  expect(scene.surfaceArea).toBeGreaterThan(2*2*Math.PI*4)
  const source=program.replace('radial_cam_walls','radial_cam_profile').replace('vector:[0,0,2mm]','piece:0').replace('show cam.nurbs_patches_tessellate(16)','show cam')
  const doc=compileRushFrontend(source).document as {root:string}
  const q=buildOwnNurbs(doc,{action:'evaluate',evaluations:[{node:doc.root,u:0},{node:doc.root,u:Math.PI}]})
  expect(q.evaluations[0]!.point).toEqual([6,-3,5])
  q.evaluations[1]!.point.forEach((v,i)=>expect(v).toBeCloseTo([-4,-3,5][i]!,8))
})
it('reports an exhausted budget and rejects motion programs that fail to close',async()=>{
  const scene=await buildTextNurbsScene(compileRushFrontend(program.replace('max_spans:1024','max_spans:2')).document)
  expect(JSON.parse(scene.measurements![0]!.report).stop).toBe('SpanBudget')
  await expect(buildTextNurbsScene(compileRushFrontend(program.replace('end_lifts:[2mm,0mm]','end_lifts:[2mm,1mm]')).document)).rejects.toThrow(/close/i)
})
