import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string, transform=(source:string)=>source){
  const source=readFileSync('examples/rush/'+name+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(source)).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('clips to the shared and combined material and preserves open subtraction ends',async()=>{
  expect((await measure('clip-common-path')).spans).toEqual([[['2/5','3/5']]])
  expect((await measure('clip-union-path')).spans).toEqual([[['1/5','4/5']]])
  expect((await measure('subtract-path-cutters')).spans).toEqual([[{parameters:['1/5','2/5'],included:[true,false]}]])
  expect((await measure('classify-path-membership')).spans[0].map((p:{solids:number[]})=>p.solids)).toEqual([[],[0],[0,1],[1],[]])
  const three=(source:string)=>source.replace('measurement =','third = brep_box([2.5,2,3],[4.5,5,7])\nmeasurement =').replace('inputs:[other]','inputs:[third,other]')
  expect((await measure('clip-common-path',three)).spans).toEqual([[['1/2','3/5']]])
  const memberships=(await measure('classify-path-membership',three)).spans[0].map((p:{solids:number[]})=>p.solids)
  expect(memberships).toEqual([[],[0],[0,2],[0,1,2],[1,2],[1],[]])
  const touching=(source:string)=>source.replace('[2,2,3],[4,5,7]','[3,2,3],[4,5,7]')
  expect((await measure('clip-common-path',touching)).spans).toEqual([[['3/5','3/5']]])

})
it('measures coverage by count and reconstructs disconnected paths with exact endpoints',async()=>{
  expect((await measure('bound-overlap-lengths')).lengths).toEqual([{count:0,boundsMm:['2','2']},{count:1,boundsMm:['2','2']},{count:2,boundsMm:['1','1']}])
  expect((await measure('recover-overlap-paths')).components).toEqual([
    {parameters:['1/5','2/5'],included:[true,false],pointsMm:[['1','7/2','5'],['2','7/2','5']]},
    {parameters:['3/5','4/5'],included:[false,true],pointsMm:[['3','7/2','5'],['4','7/2','5']]},
  ])
})
