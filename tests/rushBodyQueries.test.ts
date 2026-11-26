import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string,alter=(s:string)=>s){
  const source=alter(readFileSync('examples/rush/'+name+'.r','utf8'))
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('clips spatial and unused coplanar spans and classifies interior, boundary and exterior points',async()=>{
  expect((await measure('clip-polyhedral-segment')).intervals).toEqual([['1/4','3/4']])
  expect((await measure('clip-unused-face-plane')).intervals).toEqual([])
  expect((await measure('locate-polyhedral-point')).location.kind).toBe('material')
  expect((await measure('locate-polyhedral-point',s=>s.replace('[2,"7/2",5]','[3,3,5]'))).location.kind).toBe('boundary')
})
it('certifies nearest faces, ordered signed samples and exact sphere tangency',async()=>{
  expect((await measure('measure-polyhedral-boundary-distance')).distanceSquaredMm2).toBe('1')
  expect((await measure('measure-polyhedral-signed-distance')).sample.signedDistanceSquaredMm2).toBe('-1')
  expect((await measure('sample-polyhedral-signed-distances')).samples.map((s:{signedDistanceSquaredMm2:string})=>s.signedDistanceSquaredMm2)).toEqual(['-1','0','4'])
  expect((await measure('measure-polyhedral-ball-clearance')).relation).toBe('InsideTangent')
  await expect(measure('measure-polyhedral-ball-clearance',s=>s.replace('"1/1"','"-1/2"'))).rejects.toThrow()
})
