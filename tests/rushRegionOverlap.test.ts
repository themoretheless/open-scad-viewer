import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string, transform=(source:string)=>source){
  const source=readFileSync('examples/rush/'+name+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(source)).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('preserves holes in transverse intersections and planar overlap integrals',async()=>{
  expect((await measure('intersect-transverse-profiles')).segmentsMm).toEqual([[['0','2','0'],['2','2','0']],[['4','2','0'],['6','2','0']]])
  expect(await measure('measure-profile-overlap-area')).toMatchObject({areaVectorMm2:['0','0','8'],areaSquaredMm4:'64'})
  expect((await measure('measure-profile-overlap-centroid')).centroidMm).toEqual(['37/8','2','0'])
  expect((await measure('classify-profile-overlap')).relation).toBe('PartialOverlap')
})
it('decomposes the full positive overlap and distinguishes boundary-only contact',async()=>{
  const {cellsMm}=await measure('decompose-profile-overlap')
  let twice=0
  for(const cell of cellsMm){
    expect([3,4]).toContain(cell.length)
    const xy=cell.map((p:string[])=>p.map(s=>{const [n,d='1']=s.split('/');return Number(n)/Number(d)}))
    let signed=0
    for(let i=0;i<xy.length;i++){const a=xy[i],b=xy[(i+1)%xy.length];signed+=a[0]*b[1]-b[0]*a[1]}
    expect(signed).toBeGreaterThan(0);twice+=signed
  }
  expect(twice).toBe(16)
  const touch=(s:string)=>s.replace('[[3,0,0],[7,0,0],[7,4,0],[3,4,0],[3,0,0]]','[[6,0,0],[8,0,0],[8,4,0],[6,4,0],[6,0,0]]').replace(/other_holes_mm:.*?,max_work:/,'other_holes_mm:[],max_work:')
  expect((await measure('classify-profile-overlap',touch)).relation).toBe('TouchingAlongEdges')
  expect((await measure('measure-profile-overlap-centroid',touch)).centroidMm).toBeNull()
})
