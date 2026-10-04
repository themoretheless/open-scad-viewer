import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs,buildOwnNurbsAsync} from '../src/services/modelGraphNurbsKernel'

it('refuses and diagnoses an authored continuous bound despite zero sampled error through Rush and async previews',async()=>{
 const source=readFileSync('examples/rush/authored-progressive-sweep.r','utf8')
  .replace('values: [0deg,90deg]','values: [0deg,0deg]')
  .replace('max_sections: 129','max_sections: 3')
  .replace('max_deviation: 0.001mm','max_deviation: 0.000000000000000000000000000001mm')
 const {document}=compileModelGraphText(source)
 expect(()=>buildOwnNurbs(document,{action:'build'})).toThrow(/Progressive sweep continuous retained-patch error/)
 let previews=0
 await expect(buildOwnNurbsAsync(document,{action:'build'},{onSweepPreview:(_id,preview)=>{
  previews++
  expect(preview.report).toMatchObject({accepted:false,sampledControlDeviation:0})
  expect(preview.report.continuousErrorUpper!).toBeGreaterThan(preview.report.budget)
  expect('knownProfileErrorUpper' in preview.report&&preview.report.knownProfileErrorUpper).toBe(preview.report.continuousErrorUpper)
 }})).rejects.toThrow(/Progressive sweep continuous retained-patch error/)
 expect(previews).toBe(1)
})
