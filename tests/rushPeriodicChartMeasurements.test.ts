import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const program=readFileSync('examples/rush/periodic-chart-measurements.r','utf8')
async function run(source:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  return scene.measurements!.map(row=>JSON.parse(row.report))
}
const ratio=(n:number,d:number)=>({numerator:String(n),denominator:String(d)})
it('wraps samples and preserves exact UV moments across the seam',async()=>{
  const [point,batch,lift,winding,area,center,inertia]=await run(program)
  expect(point.nativeUv).toEqual([0.5,0.5])
  expect(point.point).toEqual([0.5,0,1])
  expect(batch.samples.map((pair:number[][])=>pair[0])).toEqual([[3.5,0.25],[0.5,0.75]])
  expect(lift.parameters[1]).toEqual([4.5,0.25])
  expect(winding.winding).toEqual([0,0])
  expect(area.signedArea).toEqual(ratio(1,2))
  expect(center.centroid).toEqual([ratio(4,1),ratio(1,2)])
  expect(inertia.centralMoments).toEqual([ratio(1,24),ratio(0,1),ratio(1,96)])
})

it('reports the sample and axis of an ambiguous half-turn',async()=>{
  const path=program.replace('uv_points:[[3.5,0.25],[0.5,0.25]', 'uv_points:[[3.5,0.25],[1.5,0.25]')
  const reports=await run(path)
  expect(reports[2]).toMatchObject({parameters:null,refusal:'AmbiguousHalfTurn',refusedCoordinate:[1,0]})
})
