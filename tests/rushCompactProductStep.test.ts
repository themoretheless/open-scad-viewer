import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function build(name:string,transform=(s:string)=>s){
  const source=transform(readFileSync('examples/rush/'+name+'.r','utf8'))
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  const measurement=scene.measurements!.at(-1)!
  return {report:JSON.parse(measurement.report),step:measurement.step}
}
it('downloads a closed gear as one product with shared STEP points',async()=>{
  const {report,step}=await build('compact-body-step')
  expect(step).toMatch(/^ISO-10303-21;/)
  expect(report.bodies).toBe(1)
  expect(report.boundaryEdges).toBe(0)
  expect(report.sharedPoints).toBeGreaterThan(0)
  expect(step!.match(/CARTESIAN_POINT\(/g)).toHaveLength(report.sharedPoints)
  expect(report.stepBytes).toBe(new TextEncoder().encode(step).length)
},180000)
it('round-trips product, material and face presentation in compact STEP',async()=>{
  const {report,step}=await build('compact-product-step')
  expect(report.productName).toBe('Spinner')
  expect(report.materialName).toBe('Сталь')
  expect(report.layers).toContain('Контроль')
  expect(report.colourRgb).toEqual([0.2,0.4,0.8])
  expect(step).toContain('STYLED_ITEM(')
  await expect(build('compact-product-step',s=>s.replace('face_index:0','face_index:99999'))).rejects.toThrow(/face/i)
},180000)
