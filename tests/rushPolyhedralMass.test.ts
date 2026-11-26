import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function build(name:string,alter=(s:string)=>s){
  const source=alter(readFileSync('examples/rush/'+name+'.r','utf8'))
  return buildTextNurbsScene(compileRushFrontend(source).document)
}
async function measure(name:string){
  const scene=await build(name)
  expect(scene.volume).toBeCloseTo(24,6)
  return JSON.parse(scene.measurements![0]!.report)
}
it('returns exact translated mass properties and a centroidal tensor at unit density',async()=>{
  expect((await measure('measure-polyhedral-volume')).volumeMm3).toBe('24')
  expect((await measure('measure-polyhedral-centroid')).centroidMm).toEqual(['2','7/2','5'])
  expect((await measure('measure-polyhedral-inertia')).inertiaMm5).toEqual([['50','0','0'],['0','40','0'],['0','0','26']])
  await expect(build('measure-polyhedral-volume',s=>s.replace('max_work:1000000','max_work:1'))).rejects.toThrow()
})
it('measures each face and edge once and returns rational enclosures',async()=>{
  const squares=(await measure('measure-polyhedral-face-areas')).faceAreaSquaredMm4
  expect(squares.map(Number).sort((a:number,b:number)=>a-b)).toEqual([36,36,64,64,144,144])
  expect((await measure('bound-polyhedral-area')).areaBoundsMm2).toEqual(['52','52'])
  expect(await measure('bound-polyhedral-edge-length')).toMatchObject({edgeCount:12,edgeLengthBoundsMm:['36','36']})
})
