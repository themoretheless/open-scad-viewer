import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const source=readFileSync('examples/rush/quadric-support.r','utf8')
async function inspect(text: string) {
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}

it('proves regular implicit support for a bilinear saddle and detects a changed corner',async()=>{
  const report=await inspect(source)
  expect(report).toMatchObject({exactMembership:true,gradientNonzero:true,regularImplicitSupport:true})
  expect(report.gradientLower).toBeGreaterThan(0.9)
  expect((await inspect(source.replace('[1mm,1mm,1mm]','[1mm,1mm,1.01mm]'))).exactMembership).toBe(false)
})
it('reports exhausted proof work and rejects a nonsymmetric matrix',async()=>{
  expect((await inspect(source.replace('max_work:1000000','max_work:0'))).regularImplicitSupport).toBe(false)
  await expect(inspect(source.replace('[0,0.5,0,0]','[0,1,0,0]'))).rejects.toThrow()
})
