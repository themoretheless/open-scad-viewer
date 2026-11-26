import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function build(example:string, operation:string, change?:(source:string)=>string) {
  const source = readFileSync('examples/rush/'+example+'.r','utf8')
  const scene = await buildTextNurbsScene(compileRushFrontend(change ? change(source) : source).document)
  expect(scene.meshes.length).toBeGreaterThan(0)
  expect(scene.surfaceArea).toBeGreaterThan(0)
  return JSON.parse(scene.measurements!.find(row=>row.operation===operation)!.report)
}

it('preserves all profile spring patches and reports exhausted refinement',async()=>{
  const measured = await build('profile-spring-surface','profile_spring_surface')
  expect(measured).toMatchObject({withinRealArithmeticEstimate:true,roundingCertified:false,solidTopologyCertified:false})
  expect(measured.spans).toBeGreaterThan(10)
  expect(measured.realArithmeticErrorEstimateMm).toBeLessThanOrEqual(0.001)
  const coarse = await build('profile-spring-surface','profile_spring_surface',s=>s.replace('max_spans:64','max_spans:1'))
  expect(coarse).toMatchObject({withinRealArithmeticEstimate:false,spans:1})
},180000)

it('uses a variable radius law and covers the audited characteristic circles',async()=>{
  const measured = await build('sphere-envelope-surface','sphere_envelope_surface')
  expect(measured).toMatchObject({spanwiseCharacteristicCirclesProven:true,angularChartProven:true,
    withinSampledTolerance:true,continuousApproximationBound:false,solidTopologyCertified:false,
    pathIntervals:[[0,1]],unresolvedIntervals:[]})
  await expect(build('sphere-envelope-surface','sphere_envelope_surface',s=>s.replace('end:[4mm,0mm,0mm]','end:[40mm,0mm,0mm]'))).rejects.toThrow(/domain/i)
},180000)
