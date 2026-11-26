import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string, transform=(source:string)=>source){
  const source=readFileSync('examples/rush/'+name+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(source)).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('recovers both clipped ends and the intermediate rational vertex',async()=>{
  const report=await measure('recover-material-path')
  expect(report.components).toEqual([{parameters:['1/2','3/2'],pointsMm:[['1','7/2','5'],['2','7/2','5'],['3','7/2','5']]}])
})
it('preserves an isolated vertex contact as one point',async()=>{
  const touch=(source:string)=>source.replace('[[0,"7/2",5],[2,"7/2",5],[4,"7/2",5]]','[[0,3,5],[1,2,5],[2,1,5]]')
  expect((await measure('recover-material-path',touch)).components).toEqual([{parameters:['1','1'],pointsMm:[['1','2','5']]}])
})
