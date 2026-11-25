import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source=(name:string)=>readFileSync('examples/rush/'+name+'.r','utf8')
async function inspect(text:string) {
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}
it('cleans an overlapping slot into two components with zero transfer error',async()=>{
  const text=source('clean-offset-slot')
  expect(await inspect(text)).toMatchObject({stop:'Complete',components:2,errorUpperMm:0})
  await expect(inspect(text.replace('component:1','component:2'))).rejects.toThrow(/component/i)
  await expect(inspect(text.replace('max_pairs:10000','max_pairs:1'))).rejects.toThrow(/WorkLimit/i)
})
it('returns the selected pass only after verifying side, error and global clearance',async()=>{
  const text=source('square-toolpath')
  const r=await inspect(text)
  expect(r).toMatchObject({stop:'Complete',qualifiedPasses:3,outsideProven:true})
  expect(r.minimumClearanceMm[0]).toBeGreaterThanOrEqual(1.2)
  expect(r.nominalErrorUpperMm).toBeLessThanOrEqual(0.02)
  await expect(inspect(text.replace('pass_index:2','pass_index:3'))).rejects.toThrow(/pass/i)
})
