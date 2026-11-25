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
it('transfers the rational surface, outer boundary and hole under a single work limit',async()=>{
  expect((await build('reclock-trim-surface')).loopPieces).toEqual([5,4])
  expect(await build('reclock-trim-piece')).toMatchObject({loopPieces:[5,4],selectedLoop:1,selectedPiece:0,domain:[3,4]})
  await expect(build('reclock-trim-surface','hole1,hole2,hole3','hole2,hole1,hole3')).rejects.toThrow(/OriginalLoopClosure/)
})
it('certifies a whole UV curve touching the boundary and detects an interior excursion',async()=>{
  expect((await build('certify-uv-chart')).inside).toBe(true)
  expect((await build('certify-uv-chart','[0.5,2]','[0.5,3]')).inside).toBe(false)
  await expect(build('certify-uv-chart','max_work:1000000','max_work:1')).rejects.toThrow()
})
