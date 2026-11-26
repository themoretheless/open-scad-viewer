import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string, transform=(source:string)=>source){
  const source=readFileSync('examples/rush/'+name+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(source)).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('simplifies the directed path and retains certified deviation bounds',async()=>{
  expect((await measure('simplify-exact-path')).sourceVertices).toEqual([0,2,4])
  expect(await measure('simplify-path-with-tolerance')).toMatchObject({sourceVertices:[0,2],deviationSquaredMm2:'1/64'})
  expect((await measure('simplify-path-with-tolerance',s=>s.replace('deviation_mm:"1/8"','deviation_mm:"1/9"'))).sourceVertices).toEqual([0,1,2])
  expect((await measure('project-point-to-path')).hits).toEqual([{parameters:['1','2'],pointMm:['1','0','0']}])
})
it('retains reversed overlap parameters, isolated crossings and closest witnesses',async()=>{
  expect((await measure('intersect-spatial-paths')).intersections).toEqual([{spans:[0,0],parameters:[['1/4','3/4'],['1','0']],pointsMm:[['1','0','0'],['3','0','0']]}])
  expect((await measure('find-path-self-intersections')).intersections).toMatchObject([{spans:[0,2],pointsMm:[['1','1','0'],['1','1','0']]}])
  expect(await measure('measure-spatial-path-distance')).toMatchObject({distanceSquaredMm2:'9',parameters:['1/2','0']})
})
