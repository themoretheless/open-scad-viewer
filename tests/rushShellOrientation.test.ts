import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

it('independently proves that the authored box shell points outward',async()=>{
  const source=readFileSync('examples/rush/inspect-shell-orientation.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.volume).toBeCloseTo(24)
  const report=JSON.parse(scene.measurements!.find(row=>row.operation==='inspect_shell_orientation')!.report)
  expect(report.solidGeometryCertified).toBe(true)
  expect(report.boundaryEmbeddingCertified).toBe(true)
  expect(report.orientations).toHaveLength(1)
  expect(report.orientations[0]).toMatchObject({shell:0,expectedOutward:true,outward:true})
},180000)
