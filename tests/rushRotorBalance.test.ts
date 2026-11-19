import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const source=readFileSync('examples/rush/two-plane-rotor-balance.r','utf8')
async function build(program:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(program).document)
  return JSON.parse(scene.measurements![0]!.report)
}
function contains(pair:number[],x:number){
  expect(pair[0]).toBeLessThanOrEqual(x)
  expect(pair[1]).toBeGreaterThanOrEqual(x)
}
it('measures eccentricity, two-plane corrections and the radius needed for each mass limit',async()=>{
  const p=await build(source)
  const moment=[0.34375-2.875,0.375,0], magnitude=Math.hypot(...moment)/2
  expect(p.axisBalance.status).toBe('outside')
  contains(p.axisBalance.eccentricityBoundsMm,2*magnitude/2.875)
  moment.forEach((v,i)=>contains(p.staticUnbalance.vectorBoundsKgMm[i],v))
  for(const [i,rad] of [2,4].entries()){
    const plane=p.twoPlaneUnbalance[i], weight=p.twoPlaneCounterweights[i]
    expect(plane.offsetMm).toBe(i*2)
    moment.forEach((v,k)=>contains(plane.vectorBoundsKgMm[k],v/2))
    contains(weight.massBoundsKg,magnitude/rad)
    moment.forEach((v,k)=>contains(weight.radialOffsetBoundsMm[k],-v*rad/(2*magnitude)))
    contains(p.twoPlaneCounterweightCapacity.planes[i].minimumRadiusBoundsMm,magnitude/[5,0.01][i]!)
  }
  expect(p.twoPlaneBalance.planes.map((x:{status:string})=>x.status)).toEqual(['within','outside'])
  expect(p.twoPlaneCounterweightCapacity.planes.map((x:{status:string})=>x.status)).toEqual(['within','outside'])
  for(const plane of p.twoPlaneResidualBalance.planes) for(const pair of plane.vectorBoundsKgMm) contains(pair,0)
  const loose=await build(source.replace('plane_tolerances:[5,0.1]','plane_tolerances:[5,5]'))
  expect(loose.twoPlaneResidualBalance.status).toBe('within')
},120000)
it('encloses the signed couple about the specified axis reference point',async()=>{
  const p=await build(source.replace('axis_point:[1,0,0],axis_vector:[0,0,1]','axis_point:[0,0,0],axis_vector:[0,0,-2]'))
  ;[0.375,-0.34375,0].forEach((v,i)=>contains(p.coupleUnbalance.vectorBoundsKgMm2[i],v))
},120000)
it('retains incomplete certification and rejects coincident balance planes',async()=>{
  const p=await build(source.replace('max_surface_cells:20000','max_surface_cells:2'))
  expect(p.twoPlaneResidualBalance).toBeNull()
  expect(p.twoPlaneCounterweightCapacity).toBeNull()
  await expect(build(source.replace('plane_offsets:[0,2]','plane_offsets:[0,0]'))).rejects.toThrow()
},120000)
