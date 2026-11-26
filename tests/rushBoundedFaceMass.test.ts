import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function build(name:string,before='',after=''){
  const text=readFileSync('examples/rush/'+name+'.r','utf8').replace(before,after)
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}
function encloses(bounds:number[],expected:number){
  expect(bounds[0]).toBeLessThanOrEqual(expected)
  expect(bounds[1]).toBeGreaterThanOrEqual(expected)
}
it('encloses the area, XYZ centroid and central tensor of a rational ellipse',async()=>{
  const r=await build('bound-world-face-mass')
  expect(r.projectedConverged).toBe(true)
  encloses(r.signedArea,2*Math.PI)
  for(const [i,x] of [2,1,0].entries()) encloses(r.centroidal.point[i],x)
  encloses(r.centroidal.secondMoments[0][0],2*Math.PI)
  encloses(r.centroidal.secondMoments[1][1],Math.PI/2)
})
it('orders principal moments and encloses both unoriented principal axis projectors',async()=>{
  const s=await build('bound-principal-face-moments')
  encloses(s.spectrum.principal[0],Math.PI/2)
  encloses(s.spectrum.principal[1],2*Math.PI)
  const d=await build('bound-principal-face-directions')
  for(const [axis,coordinate] of [1,0].entries())for(let i=0;i<3;i++)for(let j=0;j<3;j++)
    encloses(d.axes[axis][i][j],i===coordinate&&j===coordinate?1:0)
})
