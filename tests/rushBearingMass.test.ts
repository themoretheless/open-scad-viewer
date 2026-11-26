import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

it('retains mass state through bearing measurements, balance and a point correction',async()=>{
  const initial=readFileSync('examples/rush/style-step-occurrences.r','utf8').split('\nshow ')[0]!
  const source=initial+`
mass = styled.step_branch_mass(product_path:["Fixture","Left"],densities:[2],relative_tolerance:0.000001,max_evaluations:100000,preview_index:0)
bearing = mass.measure_bearing_inertia(axis_point:[0mm,0mm,0mm],axis_direction:[0,0,7])
planes = bearing.balance_two_planes(axis_point:[0mm,0mm,0mm],axis_direction:[0,0,1],balance_planes:[0mm,2mm])
sites = planes.balance_fixed_sites(axis_point:[0mm,0mm,0mm],axis_direction:[0,0,1],balance_planes:[0mm,2mm],balance_sites:[[[-2mm,0mm,0mm],[0mm,-2mm,0mm]],[[-2mm,0mm,0mm],[0mm,-2mm,0mm]]])
weighted = sites.add_assembly_point_mass(mass_position:[-2mm,0mm,0mm],point_mass_kg:4)
show weighted.brep_tessellate(3)`
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  const report=(op:string)=>JSON.parse(scene.measurements!.find(row=>row.operation===op)!.report)
  const bearing=report('measure_bearing_inertia')
  expect(bearing.axisInertiaKgMm2).toBeCloseTo(128/3)
  expect(bearing.staticUnbalanceKgMm).toEqual([16,16,0].map(value=>expect.closeTo(value,8)))
  expect(report('balance_two_planes').planeCorrectionsKgMm.flat()).toEqual([-8,-8,0,-8,-8,0].map(value=>expect.closeTo(value,8)))
  expect(report('balance_fixed_sites').siteMassesKg.flat()).toEqual([4,4,4,4].map(value=>expect.closeTo(value,8)))
  const weighted=report('add_assembly_point_mass')
  expect(weighted.massKg).toBeCloseTo(20)
  expect(weighted.centroidMm).toEqual([0.4,0.8,0.8].map(value=>expect.closeTo(value,8)))
},180000)

it('composes measured components and recovers the residual after removing a known part',async()=>{
  const initial=readFileSync('examples/rush/style-step-occurrences.r','utf8').split('\nshow ')[0]!
  const source=initial+`
mass = styled.step_branch_mass(product_path:["Fixture","Left"],densities:[2],relative_tolerance:0.000001,max_evaluations:100000,preview_index:0)
placed = mass.place_assembly_mass(placement:[[0,-1,0,5],[1,0,0,0],[0,0,1,0],[0,0,0,1]])
scaled = placed.affine_assembly_mass(placement:[[2,0,0,0],[0,1,0,0],[0,0,-1,0],[0,0,0,1]])
combined = mass.combine_assembly_mass(target_geometry:scaled)
residual = combined.subtract_assembly_mass(target_geometry:scaled)
show residual.brep_tessellate(3)`
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  const report=(op:string)=>JSON.parse(scene.measurements!.find(row=>row.operation===op)!.report)
  const sourceMass=report('step_branch_mass')
  const placed=report('place_assembly_mass')
  expect(placed.massKg).toBeCloseTo(16)
  expect(placed.centroidMm).toEqual([4,1,1].map(value=>expect.closeTo(value,8)))
  const affine=report('affine_assembly_mass')
  expect(affine.massKg).toBeCloseTo(32)
  expect(affine.centroidMm).toEqual([8,1,-1].map(value=>expect.closeTo(value,8)))
  expect(report('combine_assembly_mass').massKg).toBeCloseTo(48)
  const residual=report('subtract_assembly_mass')
  expect(residual.massKg).toBeCloseTo(sourceMass.massKg,8)
  expect(residual.centroidMm).toEqual(sourceMass.centroidMm.map((value:number)=>expect.closeTo(value,8)))
  expect(residual.inertiaKgMm2.flat()).toEqual(sourceMass.inertiaKgMm2.flat().map((value:number)=>expect.closeTo(value,8)))
},180000)
