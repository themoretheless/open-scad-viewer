import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function measure(name:string, transform=(source:string)=>source){
  const source=readFileSync('examples/rush/'+name+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(source)).document)
  return JSON.parse(scene.measurements![0]!.report)
}
it('preserves per-span contacts and exact closed material intervals',async()=>{
  const {contacts}=await measure('find-path-face-contacts')
  expect(contacts).toMatchObject([{span:0,intervals:[['1/2','1/2']]},{span:1,intervals:[['1/2','1/2']]}])
  expect(contacts[0].face).not.toBe(contacts[1].face)
  expect((await measure('clip-path-to-material')).spans).toEqual([[['1/2','1']],[['0','1/2']]])
})
it('preserves global parameters and distinguishes entry and exit',async()=>{
  expect((await measure('find-path-boundary-events')).events).toEqual([
    {parameters:['1/2','1/2'],before:'Exterior',after:'Interior'},
    {parameters:['3/2','3/2'],before:'Interior',after:'Exterior'},
  ])
})

it('joins boundary travel across consecutive spans and keeps boundary endpoints',async()=>{
  const onFace=(source:string)=>source.replace('[[0,"7/2",5],[2,"7/2",5],[4,"7/2",5]]','[[1,3,5],[1,"7/2",5],[1,4,5],[2,4,5]]')
  const {events}=await measure('find-path-boundary-events',onFace)
  expect(events).toEqual([{parameters:['0','2'],before:null,after:'Interior'}])
  const {spans}=await measure('clip-path-to-material',onFace)
  expect(spans).toEqual([[['0','1']],[['0','1']],[['0','1']]])
})
