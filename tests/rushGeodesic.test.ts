import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const example=()=>readFileSync('examples/rush/geodesic-on-panel.r','utf8')

async function trace(source:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements!.find(row=>row.operation==='trace_geodesic')!.report)
}

it('builds a geodesic preview with the expected endpoint and achieved model length',async()=>{
  const report=await trace(example())
  expect(report).toMatchObject({stop:'length_reached',continuousBound:false})
  expect(report.lengthAchievedMm).toBeCloseTo(3,8)
  expect(report.uvPolyline.at(-1)).toEqual([expect.closeTo(0.5,8),expect.closeTo(0.3,8)])
  expect(report.maxSampledGeodesicCurvature).toBeCloseTo(0,8)
},180000)

it('retains distinct boundary and step-budget stops',async()=>{
  const source=example().replace('return_field:true','return_field:false').replace('path.surface_extrude([0mm,0mm,1mm])','path')
  const boundary=await trace(source.replace('length:3mm','length:20mm'))
  expect(boundary.stop).toBe('boundary_hit')
  expect(boundary.lengthAchievedMm).toBeCloseTo(8,5)
  expect((await trace(source.replace('max_steps:128','max_steps:1'))).stop).toBe('budget_exhausted')
},180000)
