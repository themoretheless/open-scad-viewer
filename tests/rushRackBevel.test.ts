import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const rack=readFileSync('examples/rush/exact-gear-rack.r','utf8')
const bevel=readFileSync('examples/rush/verified-bevel-gear.r','utf8')
const scene=(s:string)=>buildTextNurbsScene(compileRushFrontend(s).document)
it('makes one closed rack body with the analytic pitch and material volume',async()=>{
  const r=await scene(rack)
  const report=JSON.parse(r.measurements![0]!.report)
  const pitch=2*Math.PI
  const toothWidth=pitch/2-0.1
  const area=12*pitch*3+12*4.5*(toothWidth+0.5*Math.tan(Math.PI/9))
  expect(report.pitchMm).toBeCloseTo(pitch,12)
  expect(report.toothThicknessAtPitchMm).toBeCloseTo(toothWidth,12)
  expect(report.toothCentersMm).toHaveLength(12)
  expect(report.toothCentersMm.at(-1)).toBeCloseTo(11.5*pitch,12)
  expect(JSON.parse(r.meshes[0]!.nativeGeometry!.geometryJson).geometry.bodies).toHaveLength(1)
  expect(r.volume).toBeCloseTo(5*area,5)
})
it('uses the same degree-one rack outline as an editable curve',async()=>{
  const profile=rack.replace('brep_rack(','rack_profile(').replace(',z_min:-1mm,z_max:4mm','')
    .replace('show rack.brep_tessellate(2)','wall = rack.surface_extrude(vector:[0,0,5mm])\nshow wall.tessellate(32,2)')
  const r=await scene(profile)
  expect(r.surfaceArea).toBeGreaterThan(0)
  expect(JSON.parse(r.measurements![0]!.report).toothCentersMm).toHaveLength(12)
  await expect(scene(profile.replace('backlash:0.1mm','backlash:4mm'))).rejects.toThrow(/backlash|thickness/i)
})
it('retains the certified bevel deviation with the closed B-rep in the editor',async()=>{
  const r=await scene(bevel)
  const report=JSON.parse(r.measurements![0]!.report)
  expect(report.worldErrorUpperMm).toBeGreaterThan(0)
  expect(report.worldErrorUpperMm).toBeLessThanOrEqual(1e-6)
  expect(report.solidGeometryCertified).toBe(false)
  const model=JSON.parse(r.meshes[0]!.nativeGeometry!.geometryJson).geometry
  expect(model.bodies).toHaveLength(1)
  expect(model.shells[0].closed).toBe(true)
},240000)
