import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source=readFileSync('examples/rush/periodic-contour-regions.r','utf8')
const ratio=(n:number,d:number)=>({numerator:String(n),denominator:String(d)})
async function run(program:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(program).document)
  return scene.measurements!.map(row=>JSON.parse(row.report))
}
it('resolves hole parity, boundary contacts and exact moments in a periodic chart',async()=>{
  const [point,batch,region,simple,pair,relation,tree,area,centers,inertia]=await run(source)
  expect(point.location).toBe('inside')
  expect(batch.locations).toEqual(['inside','inside','outside','boundary'])
  expect(region.locations).toEqual(['inside','outside','outside','boundary'])
  expect(simple).toMatchObject({simple:true,contact:null})
  expect(pair).toMatchObject({disjoint:true,contact:null})
  expect(relation.relation.kind).toBe('firstContainsSecond')
  expect(tree.nodes).toEqual([{parent:null,depth:0},{parent:0,depth:1}])
  expect(area.area).toEqual(ratio(3,8))
  expect(centers.components).toHaveLength(1)
  expect(centers.components[0]).toMatchObject({outerContour:0,area:ratio(3,8),centroid:[ratio(4,1),ratio(1,2)]})
  expect(inertia.components[0].centralMoments).toEqual([ratio(5,128),ratio(0,1),ratio(5,512)])
})

it('locates contact across the seam and preserves a failed work certificate',async()=>{
  const path='[[3.5,0.25],[0.5,0.25],[0.5,0.75],[3.5,0.75],[3.5,0.25]]'
  const hole='[[3.75,0.375],[0.25,0.375],[0.25,0.625],[3.75,0.625],[3.75,0.375]]'
  const touching=source.replaceAll('second_path:'+hole,'second_path:'+path)
  const reports=await run(touching)
  expect(reports[4].disjoint).toBe(false)
  expect(reports[4].contact.edges).toHaveLength(2)
  expect(reports[5].relation.kind).toBe('boundaryContact')
  const exhausted=await run(source.replaceAll('max_work:1000000','max_work:1'))
  expect(exhausted[6].nodes).toBeNull()
  expect(exhausted[6].refusal).toBeTruthy()
})
