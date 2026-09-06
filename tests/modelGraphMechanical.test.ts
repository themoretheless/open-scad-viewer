import { expect, it } from 'vitest'
import { buildModelGraphGear } from '../src/services/modelGraphGears'
import { buildModelGraphPlanetary } from '../src/services/modelGraphPlanetary'
import { buildModelGraphThread, threadRadiusAt } from '../src/services/modelGraphThreads'
import { GEAR_DEFAULTS, PLANETARY_DEFAULTS, THREAD_DEFAULTS, createMechanicalDocument } from '../src/services/mechanicalGeneratorContract'
import { compileModelGraph } from '../src/services/modelGraph'
import { HeadlessGeometryService } from '../src/mcp/geometryService'
import { DirectGeometrySupervisor } from '../src/mcp/directGeometrySupervisor'
import { defaultGeometryBuildEngine } from '../src/services/geometryBuildEngine'

it('builds all default generators through the production worker',async()=>{
  const runtime = new DirectGeometrySupervisor()
  try {
    const geometry = new HeadlessGeometryService(defaultGeometryBuildEngine,runtime)
    for (const kind of ['gear','planetary_gears','thread'] as const) {
      const compiled = compileModelGraph(createMechanicalDocument({kind}))
      const result = await geometry.analyze(compiled.source,'full').catch(e=>{throw new Error(kind,{cause:e})})
      expect(result.volume,kind).toBeGreaterThan(0)
      expect(result.meshCount,kind).toBe(kind==='planetary_gears'?5:1)
      expect(result.topology.boundary,kind).toBe(0)
      expect(result.topology.nonManifold,kind).toBe(0)
      expect(compiled.mechanical_reports).toHaveLength(1)
      expect(compiled.source.length).toBeLessThan(220000)
    }
  } finally {await runtime.close()}
},60000)

it('uses declared pitch geometry and preserves external/internal profile volumes',async()=>{
  const geometry = new HeadlessGeometryService()
  for (const options of [GEAR_DEFAULTS,{...GEAR_DEFAULTS,internal:true,bore:0,teeth:72}]) {
    const gear=buildModelGraphGear(options)
    expect(gear.report.pitch_diameter_mm).toBe(options.module*options.teeth)
    expect(gear.report.tooth_thickness_at_pitch_mm).toBeCloseTo(Math.PI*options.module/2-options.backlash,10)
    const measured=await geometry.analyze(gear.source,'full')
    expect(measured.volume).toBeCloseTo(gear.report.expected_volume_mm3,2)
  }
})

it('rejects undercut, impossible planets, invalid thread dimensions and excessive mesh work',()=>{
  expect(()=>buildModelGraphGear({...GEAR_DEFAULTS,teeth:10})).toThrow('undercut')
  expect(()=>buildModelGraphGear({...GEAR_DEFAULTS,bore:1000})).toThrow('bore')
  expect(()=>buildModelGraphPlanetary({...PLANETARY_DEFAULTS,planet_teeth:18})).toThrow('interference')
  expect(()=>buildModelGraphPlanetary({...PLANETARY_DEFAULTS,planet_count:5})).toThrow('integer')
  expect(()=>buildModelGraphThread({...THREAD_DEFAULTS,pitch:8})).toThrow('twice')
  expect(()=>buildModelGraphThread({...THREAD_DEFAULTS,length:96,segments_per_turn:96})).toThrow('triangles')
})

it('thread phase follows lead, handedness and matching clearance',()=>{
  const o={...THREAD_DEFAULTS,starts:2}
  const angle=.73,z=3.123
  expect(threadRadiusAt(o,angle,z)).toBeCloseTo(threadRadiusAt(o,angle+Math.PI,z+o.pitch),10)
  expect(threadRadiusAt({...o,left_handed:true},-angle,z)).toBeCloseTo(threadRadiusAt(o,angle,z),10)
  expect(threadRadiusAt({...o,internal:true},angle,z)-threadRadiusAt(o,angle,z)).toBeCloseTo(o.clearance,10)
  expect(buildModelGraphThread({...o,length:6}).report.lead_mm).toBe(3)
})

it('planetary kinematics and mesh phase remain disjoint at multiple carrier poses',async()=>{
  const geometry = new HeadlessGeometryService()
  for (const carrier_angle of [0,7,37]) {
    const generated=buildModelGraphPlanetary({...PLANETARY_DEFAULTS,carrier_angle})
    expect(generated.report.ring_teeth).toBe(72)
    expect(generated.report.sun_to_carrier_ratio).toBe(4)
    expect(generated.report.sun_angle_deg).toBe(4*carrier_angle)
    const [sun,ring,...planets]=generated.parts
    for (const planet of planets) for (const fixed of [sun,ring]) {
      const measured=await geometry.analyze(`intersection(){${fixed.source}\n${planet.source}}`,'full')
      expect(measured.volume,`${carrier_angle}: ${fixed.id}/${planet.id}`).toBeLessThan(1e-5)
    }
  }
},60000)

it('produces watertight left/right internal threads and multiple starts with no mating overlap',async()=>{
  const runtime=new DirectGeometrySupervisor()
  try {
    const geometry=new HeadlessGeometryService(defaultGeometryBuildEngine,runtime)
    for(const left_handed of [false,true]) {
      const options={...THREAD_DEFAULTS,length:6,left_handed,starts:2}
      const rod=buildModelGraphThread(options),nut=buildModelGraphThread({...options,internal:true})
      for(const generated of [rod,nut]) {
        const checked=await geometry.analyze(generated.source,'full')
        expect(checked.volume).toBeGreaterThan(0)
        expect(checked.topology.boundary).toBe(0)
        expect(checked.topology.nonManifold).toBe(0)
        expect(checked.topology.degenerate).toBe(0)
      }
      const overlap=await geometry.analyze(`intersection(){${rod.source}${nut.source}}`,'full')
      expect(overlap.volume).toBeLessThan(1e-7)
    }
  } finally {await runtime.close()}
},60000)

it('preserves strict units and editable generator parameters',()=>{
  const doc=createMechanicalDocument({kind:'gear'})
  doc.parameters.find(p=>p.id==='module')!.value=3
  const compiled=compileModelGraph(doc)
  expect(compiled.mechanical_reports[0].pitch_diameter_mm).toBe(72)
  doc.parameters.find(p=>p.id==='module')!.unit='deg'
  expect(()=>compileModelGraph(doc)).toThrow()
})

it('aligns odd-tooth planets against both mating members',async()=>{
  const geometry=new HeadlessGeometryService()
  for(const carrier_angle of [0,17]){
    const generated=buildModelGraphPlanetary({...PLANETARY_DEFAULTS,planet_teeth:23,planet_count:2,carrier_angle})
    const [sun,ring,...planets]=generated.parts
    for(const planet of planets)for(const fixed of [sun,ring]){
      const measured=await geometry.analyze(`intersection(){${fixed.source}${planet.source}}`,'full')
      expect(measured.volume,`${carrier_angle}/${fixed.id}`).toBeLessThan(1e-5)
    }
  }
})
