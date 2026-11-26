import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string, transform=(source:string)=>source){
  const source=readFileSync('examples/rush/'+name+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(source)).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('triangulates oriented area and integrates normalized planar moments',async()=>{
  const {triangles}=await measure('triangulate-planar-path')
  expect(triangles).toHaveLength(2)
  const x=[0,6,6,0],y=[0,0,4,4]
  const areas=triangles.map(([i,j,k]:[number,number,number])=>(x[j]!-x[i]!)*(y[k]!-y[i]!)-(y[j]!-y[i]!)*(x[k]!-x[i]!))
  expect(areas).toEqual([24,24])
  expect((await measure('measure-path-second-moments')).normalizedMomentsMm2).toEqual([['3','0','0'],['0','4/3','0'],['0','0','0']])
  const {boundsMm2}=await measure('bound-path-principal-moments')
  for(const [i,v] of [4/3,3].entries()){
    const [lo,hi]=boundsMm2[i].map((s:string)=>{const [n,d='1']=s.split('/');return Number(n)/Number(d)})
    expect(lo).toBeLessThanOrEqual(v);expect(hi).toBeGreaterThanOrEqual(v);expect(hi-lo).toBeLessThanOrEqual(0.000001)
  }
})
it('subtracts hole interiors from area, moments and path coverage',async()=>{
  expect(await measure('measure-region-mass')).toMatchObject({centroidMm:['3','2','0'],areaVectorMm2:['0','0','20'],areaSquaredMm4:'400'})
  expect((await measure('clip-path-to-perforated-region')).spans).toEqual([[['1/8','3/8'],['5/8','7/8']]])
  expect((await measure('measure-region-second-moments')).normalizedMomentsMm2).toEqual([['53/15','0','0'],['0','23/15','0'],['0','0','0']])
  const twoHoles=(s:string)=>s.replace('[[[2,1,0],[4,1,0],[4,3,0],[2,3,0],[2,1,0]]]',
    '[[["1/2",1,0],["3/2",1,0],["3/2",3,0],["1/2",3,0],["1/2",1,0]],[["9/2",1,0],["9/2",3,0],["11/2",3,0],["11/2",1,0],["9/2",1,0]]]')
  expect(await measure('measure-region-mass',twoHoles)).toMatchObject({centroidMm:['3','2','0'],areaVectorMm2:['0','0','20']})
  expect((await measure('clip-path-to-perforated-region',twoHoles)).spans).toEqual([[['1/8','3/16'],['5/16','11/16'],['13/16','7/8']]])

})
