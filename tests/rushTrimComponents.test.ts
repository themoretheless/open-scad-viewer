import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
import {evaluateNurbsCurve} from '../src/kernel/nurbsCurve'
import {evaluateNurbsSurface} from '../src/kernel/nurbsSurface'

const program=readFileSync('examples/rush/periodic-trim-components.r','utf8')
async function reports(source:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  return scene.measurements!.map(row=>JSON.parse(row.report))
}

it('retains a common exact chart for a seam-crossing component and its hole',async()=>{
  const [lift,loops,patches,edges]=await reports(program)
  const [component]=lift.components
  expect(component.outer.contour).toBe(0)
  expect(component.holes[0].contour).toBe(1)
  expect(component.outer.vertices[0]).toEqual([{numerator:'7',denominator:'2'},{numerator:'1',denominator:'8'}])
  expect(component.outer.vertices.at(-1)).toEqual(component.outer.vertices[0])
  expect(component.holes[0].translation).toEqual([1,0])
  expect(loops.components[0].contours).toEqual([0,1])
  for (const [i,loop] of loops.components[0].loops.entries()) {
    expect(loop).toHaveLength(1)
    const [boundary]=loop
    expect(boundary.degree).toBe(1)
    expect(boundary.controlPoints).toHaveLength(5)
    expect(boundary.controlPoints.at(-1)).toEqual(boundary.controlPoints[0])
    const pts=boundary.controlPoints as number[][]
    const twiceArea=pts.slice(1).reduce((sum,q,k)=>sum+pts[k]![0]!*q[1]!-pts[k]![1]!*q[0]!,0)
    expect(twiceArea).toBe(i===0?1.5:-0.25)
  }
  const [patch]=patches.components
  expect(patch.trim.contours).toEqual([0,1])
  const [geometry]=edges.components
  expect(geometry.patch).toEqual(patch)
  expect(geometry.loops.map((loop:unknown[])=>loop.length)).toEqual([6,4])
  for(const loop of geometry.loops){
    for(const {curve,pcurve} of loop){
      const lo=curve.knots[curve.degree],hi=curve.knots[curve.controlPoints.length]
      for(const t of [lo,(lo+hi)/2,hi]){
        const [u,v]=evaluateNurbsCurve(pcurve,t).point
        expect(evaluateNurbsSurface(patch.surface,u!,v!).point).toEqual(evaluateNurbsCurve(curve,t).point)
      }
    }
  }
})
it('returns a refusal without partial geometry when the shared budget is exhausted',async()=>{
  for(const row of await reports(program.replaceAll('max_work:1000000','max_work:1'))){
    expect(row.refusal).toBeTruthy()
    expect(row.components).toBeNull()
    expect(row.workUsed).toBeLessThanOrEqual(1)
  }
})
