import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const program=readFileSync('examples/rush/trim-lengths-and-projections.r','utf8')
async function run(text:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  return scene.measurements!.map(row=>JSON.parse(row.report))
}
function contains(b:number[],x:number){expect(b[0]).toBeLessThanOrEqual(x);expect(b[1]).toBeGreaterThanOrEqual(x)}
it('measures complete, partial and inverted contour lengths using the original segment order',async()=>{
  const [full,seg,range,loc,table,multi,division]=await run(program)
  contains(full.length.boundsMm,5)
  contains(seg.length.boundsMm,1)
  contains(range.length.boundsMm,0.5)
  contains(loc.parameterBounds,0.5)
  ;[0,1,2.5,3.5,5].forEach((v,i)=>contains(table.vertexPrefixBoundsMm[i],v))
  ;[0,0.5,3.5].forEach((v,i)=>contains(multi.stations[i].parameterBounds,v))
  expect(multi.stations[3].parameterBounds).toBeNull()
  expect(division.stations).toHaveLength(6)
  expect(division.stations.at(-1).parameterBounds).toEqual([4,4])
})
it('separates exact contour area from the maximum projection of the patch with its hole',async()=>{
  const reports=await run(program)
  const q=(n:string,d:string)=>({numerator:n,denominator:d})
  expect(reports[7].exactAreaVector).toEqual([q('-3','4'),q('-3','4'),q('0','1')])
  expect(reports[8].exactAreaVector).toEqual(reports[7].exactAreaVector)
  ;[-0.75,-0.75,0].forEach((v,i)=>contains(reports[9].areaVectorBoundsMm2[i],v))
  const max=Math.hypot(0.75,0.5)
  contains(reports[10].maximumBoundsMm2,max)
  ;[-0.75/max,-0.5/max,0].forEach((v,i)=>contains(reports[11].normalBounds[i],v))
})

it('integrates a curved polynomial edge on a bilinear surface without rounding',async()=>{
  // On the first span x=u and z=(1+u)v. The signed Y projection
  // of the UV triangle is -area*(1+centroidU) = -35/192.
  const chart='chart = nurbs_surface(degree_u:1,degree_v:1,knots_u:[-1,0,1,2,3,4,5],knots_v:[0,0,1,1],control_points:[[[0,0,0],[0,0,1]],[[1,0,0],[1,0,2]],[[1,1,0],[1,1,2]],[[0,1,0],[0,1,1]],[[0,0,0],[0,0,1]]],weights:[[1,1],[1,1],[1,1],[1,1],[1,1]],periodic_u:true,periodic_v:false)'
  const contour='contours:[[[0.125,0.125],[0.625,0.125],[0.625,0.625],[0.125,0.125]]],contour:0,max_work:1000000'
  const source=['// @rush/1',chart,'measured = chart.exact_input_contour_polynomial_projected_area('+contour+')','show measured.tessellate(8,4)'].join('\n')
  const [report]=await run(source)
  expect(report.exactAreaVector[1]).toEqual({numerator:'-35',denominator:'192'})
})

it('distinguishes insufficient subdivision work from an invalid contour index',async()=>{
  const reports=await run(program.replace('max_cells:2048','max_cells:1'))
  expect(reports[0].length).toBeNull()
  expect(reports[0].minimumCells).toBeGreaterThan(1)
  await expect(run(program.replace('contour:0','contour:8'))).rejects.toThrow(/contour index/i)
})
