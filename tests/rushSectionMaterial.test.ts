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
it('identifies the hole between two material intervals on the section',async()=>{
  const r=await build('classify-section-gaps')
  expect(r.gaps).toEqual([['Inside','Inside'],['Outside','Inside'],['Inside','Inside']])
  const material=await build('assemble-section-material')
  expect(material.proven).toEqual([[0,1],[2,3]])
  expect(material.possible).toEqual(material.proven)
})
it('measures the total length with the hole excluded',async()=>{
  expect(await build('measure-section-material')).toMatchObject({status:'transverse',clockLength:['1','1'],xyzLengthSquared:['1','1']})
})
