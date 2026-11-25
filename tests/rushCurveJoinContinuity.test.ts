import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const source=readFileSync('examples/rush/curve-join-continuity.r','utf8')
async function measure(text:string) {
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  expect(scene.measurements![0]!.operation).toBe('curve_join_continuity')
  return JSON.parse(scene.measurements![0]!.report)
}

it('establishes G2 for unequal parameter speeds and reports the three independent bounds',async()=>{
  const report=await measure(source)
  expect(report).toMatchObject({g0:'WithinTolerance',g1:'WithinTolerance',g2:'WithinTolerance'})
  for(const bounds of [report.positionErrorMm,report.tangentError,report.curvatureErrorPerMm]){
    expect(bounds[0]).toBe(0)
    expect(bounds[1]).toBeLessThanOrEqual(0.0001)
  }
})

it('distinguishes failures of position, tangent and curvature',async()=>{
  const curvature=await measure(source.replace('[1mm,1mm,0mm]','[1mm,2mm,0mm]'))
  expect(curvature).toMatchObject({g0:'WithinTolerance',g1:'WithinTolerance',g2:'ExceedsTolerance'})
  const corner=await measure(source.replace('[0.5mm,0mm,0mm]','[0mm,0.5mm,0mm]'))
  expect(corner.g0).toBe('WithinTolerance')
  expect(corner.g1).toBe('ExceedsTolerance')
  const gap=await measure(source.replace('target_end:"start"','target_end:"end"'))
  expect(gap.g0).toBe('ExceedsTolerance')
  expect(()=>compileRushFrontend(source.replace('0.0001/1mm','0.0001mm'))).toThrow()
})
