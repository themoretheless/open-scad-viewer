import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function run(name:string, transform=(source:string)=>source) {
  const source=transform(readFileSync('examples/rush/'+name+'.r','utf8'))
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  const measurement=scene.measurements!.at(-1)!
  return {report:JSON.parse(measurement.report),step:measurement.step,volume:scene.volume}
}
it('retains tolerances, signed scale and coincident placements across actual STEP exchange',async()=>{
  expect((await run('make-step-assembly')).report.occurrenceCount).toBe(2)
  expect((await run('step-part-tolerances')).report.tolerances).toEqual([0.00001,0.0001])
  for (const [name,scale] of [['set-scaled-step-placement',-2],['export-scaled-step-instance',2],['export-mirrored-step-instance',-1]] as const){
    const {step,report}=await run(name)
    expect(report.placement[0][0]).toBe(scale)
    const back=await buildTextNurbsScene(compileRushFrontend('// @rush/1\na = open_step_file(step_text:'+JSON.stringify(step!)+',preview_index:1)\nshow a.brep_tessellate(3)').document)
    const imported=JSON.parse(back.measurements![0]!.report)
    expect(imported.occurrences[1].placement[0][0]).toBeCloseTo(scale,10)
    expect(imported.definitionCount).toBe(1)
  }
  const coincident=(await run('read-coincident-step-instances')).report
  expect(coincident.occurrences).toHaveLength(2)
  expect(coincident.occurrences[0]).toEqual(coincident.occurrences[1])
  await expect(run('set-scaled-step-placement',s=>s.replace('[-2,0,0,1]','[-2,1,0,1]'))).rejects.toThrow()
},180000)
it('measures boxes with a known overlap, treating each occurrence as a separate mass',async()=>{
  const {bounds}= (await run('step-occurrence-bounds')).report
  expect(bounds).toHaveLength(2)
  expect(bounds[1][0][0]).toBeCloseTo(1)
  expect(bounds[1][1][0]).toBeCloseTo(3)
  expect((await run('step-proximity-pairs')).report.pairs).toEqual([[0,1]])
  expect((await run('step-common-material')).volume).toBeCloseTo(4)
  expect((await run('step-interferences')).report.pairs).toEqual([{pair:[0,1],interferes:true}])
  expect((await run('step-material-volume')).report.volumeMm3).toBeCloseTo(16)
  const c=(await run('step-material-centroid')).report.centroidMm
  expect(c[0]).toBeCloseTo(1.5)
  expect(c[1]).toBeCloseTo(1)
  expect(c[2]).toBeCloseTo(1)
  expect((await run('step-material-inertia')).report.inertiaMm5[0][0]).toBeCloseTo(32/3)
  expect((await run('step-material-area')).report.areaMm2).toBeCloseTo(48)
  expect((await run('step-principal-inertia')).report.momentsMm5[0]).toBeCloseTo(44/3)
  expect((await run('step-mass-properties')).report.massKg).toBeCloseTo(0.024,8)
},180000)
