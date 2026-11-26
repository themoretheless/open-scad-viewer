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
it('distinguishes open common material from boundary points in the section',async()=>{
  const r=await build('classify-face-section')
  expect(r.status).toBe('transverse')
  expect(r.interior).toHaveLength(2)
  expect(r.commonBoundary).toEqual([['0','0'],['2','2']])
  expect(r.possible).toHaveLength(2)
})
it('keeps the side and endpoint ownership of boundary-only material intervals',async()=>{
  const r=await build('classify-face-section','[[[1,0,-1],[1,0,1]],[[1,2,-1],[1,2,1]]]','[[[1,0,0],[1,0,1]],[[1,2,0],[1,2,1]]]')
  expect(r.interior).toEqual([])
  expect(r.rightBoundaryInLeftInterior).toHaveLength(2)
  for(const interval of r.rightBoundaryInLeftInterior){
    expect(interval.closed).toEqual([false,false])
    expect(interval.parameter).toHaveLength(2)
  }
})
