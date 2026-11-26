import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string,alter=(s:string)=>s){
  const source=alter(readFileSync('examples/rush/'+name+'.r','utf8'))
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('resolves rational point membership and distinct contour and material distances',async()=>{
  expect((await measure('locate-section-point')).location.kind).toBe('material')
  expect((await measure('locate-section-point',s=>s.replace('"7/2"','2'))).location.kind).toBe('boundary')
  expect((await measure('measure-section-contour-distance')).nearest.distanceSquaredMm2).toBe('9/4')
  expect((await measure('measure-section-material-distance')).nearest).toMatchObject({kind:'interior',distanceSquaredMm2:'0'})
  await expect(measure('locate-section-point',s=>s.replace('"7/2"','"1/0"'))).rejects.toThrow()
})
it('retains exact per-span clip intervals and counts repeated material traversal',async()=>{
  expect((await measure('clip-section-segment')).intervals).toEqual([['1/3','5/6']])
  expect((await measure('clip-section-polyline')).spanIntervals).toEqual([[['1/3','5/6']],[['1/6','2/3']]])
  expect((await measure('measure-section-path-length')).lengthBoundsMm).toEqual(['6','6'])
})
