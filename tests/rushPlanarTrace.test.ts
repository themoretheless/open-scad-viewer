import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string, transform=(source:string)=>source){
  const source=readFileSync('examples/rush/'+name+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(source)).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('nodes a crossing and integrates an inclined planar contour',async()=>{
  const r=await measure('node-spatial-path')
  expect(r.nodesMm).toHaveLength(5);expect(r.edges).toHaveLength(5)
  const cross=r.nodesMm.findIndex((p:string[])=>p.join(',')==='1,1,0')
  expect(r.edges.filter((e:{nodes:number[]})=>e.nodes.includes(cross))).toHaveLength(4)
  expect(await measure('measure-planar-path-area')).toMatchObject({areaVectorMm2:['-4','0','4'],areaSquaredMm4:'32'})
  expect((await measure('measure-planar-path-centroid')).centroidMm).toEqual(['1','1','1'])
})
it('classifies off-plane queries and clips transverse and coplanar spans',async()=>{
  expect((await measure('locate-planar-path-points')).locations).toEqual(['Interior','Boundary','Exterior','OffPlane'])
  expect((await measure('clip-path-to-planar-region')).spans).toEqual([[['1/4','3/4']]])
  const transverse=(s:string)=>s.replace('[[-1,1,-1],[3,1,3]]','[[0,1,2],[2,1,0]]')
  expect((await measure('clip-path-to-planar-region',transverse)).spans).toEqual([[['1/2','1/2']]])
  expect(await measure('project-to-planar-region')).toMatchObject({closestMm:['1','1','1'],location:'Interior',distanceSquaredMm2:'2'})
})
