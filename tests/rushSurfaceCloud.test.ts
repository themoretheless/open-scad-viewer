import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

it('fits an unstructured cloud and retains its finite-data error bounds',async()=>{
  const source=readFileSync('examples/rush/fit-surface-cloud.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  const {certificate}=JSON.parse(scene.measurements!.find(row=>row.operation==='fit_surface_cloud')!.report)
  expect(certificate).toMatchObject({classification:'approximate_cloud_fit',fittedToExactPromotion:false,siteCount:6})
  expect(certificate.dataSiteErrorUpper).toBeLessThan(1e-8)
  expect(certificate.hausdorffErrorUpper).toBeGreaterThanOrEqual(Math.sqrt(13))
  expect(Array.from(scene.meshes[0]!.positions).filter((_,index)=>index%3===2).every(z=>Math.abs(z-5)<1e-8)).toBe(true)
},180000)
