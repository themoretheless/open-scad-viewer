import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function run(name:string, edit?:(source:string)=>string) {
  const source=readFileSync('examples/rush/'+name.replaceAll('_','-')+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(edit ? edit(source) : source).document)
  expect(scene.meshes).toHaveLength(1)
  expect(scene.surfaceArea).toBeGreaterThan(0)
  const report=JSON.parse(scene.measurements!.find(row=>row.operation===name)!.report)
  expect(report).toMatchObject({failure:null,continuousContactCertified:false,solidTopologyCertified:false})
  expect(report.spine.length).toBeGreaterThan(1)
  for (const station of report.spine) {
    expect(station.contactA[2]).toBeCloseTo(4,6)
    expect(station.contactB[0]).toBeCloseTo(2,6)
  }
  return {scene,report}
}

it('renders a cylindrical constant-radius fillet tangent to both supports',async()=>{
  const {scene,report}=await run('rolling_ball_fillet_surface')
  expect(report.sampledRadiusRangeMm).toEqual([1.25,1.25])
  const vertices=scene.meshes[0]!.vertices
  for(let i=0;i<vertices.length;i+=6){
    const r=Math.hypot(vertices[i]!-3.25,vertices[i+2]!-5.25)
    expect(r).toBeCloseTo(1.25,5)
  }
},180000)

it('preserves linear and spline radius laws and their original support contacts',async()=>{
  for (const edit of [undefined,(source:string)=>source.replace('[1,1.5mm]','[0.5,1.25mm],[1,1.5mm]')]){
    const {report}=await run('variable_radius_fillet_surface',edit)
    expect(report.sampledRadiusRangeMm[0]).toBeCloseTo(0.5,8)
    expect(report.sampledRadiusRangeMm[1]).toBeCloseTo(1.5,8)
    for(const station of report.spine){
      expect(Math.hypot(station.center[0]-2,station.center[2]-4)).toBeCloseTo(Math.SQRT2*station.radiusMm,6)
    }
  }
},180000)
