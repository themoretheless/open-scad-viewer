import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function build(name:string, before='', after='') {
  const text = readFileSync('examples/rush/exact-tensor-'+name+'.r','utf8').replace(before,after)
  const scene = await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return {report: JSON.parse(scene.measurements![0]!.report), vertices:Array.from(scene.meshes[0]!.vertices)}
}
it('renders both rational halves with the same seam and absolute UV parameters', async () => {
  const a = await build('split','part:1','part:0')
  const b = await build('split')
  expect(a.report.domain).toEqual([[2,4],[7,9]])
  expect(b.report.domain).toEqual([[4,6],[7,9]])
  expect(Math.max(...a.vertices.filter((_,i)=>i%6===0))).toBe(3)
  expect(Math.min(...b.vertices.filter((_,i)=>i%6===0))).toBe(3)
})
it('selects the V-major tile and refuses a boundary cut or an exhausted budget', async () => {
  expect((await build('tiling')).report).toMatchObject({domain:[[4,6],[7.5,9]],columns:2,rows:2,selectedPatch:3})
  await expect(build('tiling','cuts_u:[4]','cuts_u:[6]')).rejects.toThrow()
  await expect(build('tiling','max_work:1000000','max_work:1')).rejects.toThrow()
})
