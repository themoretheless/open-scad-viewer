import {expect, it} from 'vitest'
import {GeometryKernelError} from '../src/services/geometry/kernel'
import {optimizeTrussTopology, solveTruss, type TrussMember} from '../src/services/trussAnalysis'

const member = (nodes:[number,number]):TrussMember => ({nodes, youngMpa:200_000, areaMm2:100})

it('meets the volume fraction with closed-form single-bar compliance through real WASM', () => {
  // C = F²L/(E·A·ρ^p); the volume constraint fixes ρ = 0.4 exactly.
  const r = optimizeTrussTopology({
    nodesMm: [[0,0,0],[1000,0,0]],
    members: [member([0,1])],
    restrained: [[true,true,true],[false,false,false]],
  }, [{forcesN: [[0,0,0],[1000,0,0]], weight: 1}], {volumeFraction: 0.4, penalty: 1})
  expect(r.converged).toBe(true)
  expect(r.densities[0]).toBeCloseTo(0.4, 3)
  expect(r.volumeMm3).toBeCloseTo(r.targetVolumeMm3, 6)
  expect(r.complianceNmm).toBeCloseTo(1000**2*1000/(200_000*100*0.4), 6)
  expect(r.axialForcesN[0][0]).toBeCloseTo(1000, 9)
  // ρ = 0.4 is below the 0.5 active-member threshold.
  expect(r.activeMembers).toEqual([])
})

it('vanishes zero-force members and matches the independent linear solver through real WASM', () => {
  // Bar 0-1 joins two fully fixed nodes and can never carry force.
  const model = {
    nodesMm: [[0,0,0],[1000,0,0],[500,1000,0]] as [number,number,number][],
    members: [member([0,1]), member([0,2]), member([1,2])],
    restrained: [[true,true,true],[true,true,true],[false,false,true]] as [boolean,boolean,boolean][],
  }
  const forcesN = [[0,0,0],[0,0,0],[1000,-500,0]] as [number,number,number][]
  const r = optimizeTrussTopology(model, [{forcesN, weight: 1}], {volumeFraction: 0.5})
  expect(r.converged).toBe(true)
  expect(r.densities[0]).toBeLessThanOrEqual(0.01)
  expect(r.activeMembers).toEqual([1,2])
  expect(r.complianceNmm).toBeLessThan(r.initialComplianceNmm)
  expect(r.volumeMm3).toBeLessThanOrEqual(r.targetVolumeMm3*1.000001)
  expect(r.volumeMm3).toBeCloseTo(r.targetVolumeMm3, 6)
  // The reported forces come from a final linear analysis at rho*A_ref:
  // the independent public linear solver must reproduce them.
  const linear = solveTruss({
    ...model,
    members: model.members.map((m, i) => ({...m, areaMm2: r.areasMm2[i]})),
    forcesN,
  })
  for (let i = 0; i < 3; i++)
    expect(linear.axialForcesN[i]).toBeCloseTo(r.axialForcesN[0][i], 6)
  expect(r.axialForcesN[0][0]).toBeCloseTo(0, 6)
})

it('rejects invalid options and unstable ground structures through real WASM', () => {
  const model = {
    nodesMm: [[0,0,0],[1000,0,0]] as [number,number,number][],
    members: [member([0,1])],
    restrained: [[true,true,true],[false,false,false]] as [boolean,boolean,boolean][],
  }
  const axial = [{forcesN: [[0,0,0],[1000,0,0]] as [number,number,number][], weight: 1}]
  expect(() => optimizeTrussTopology(model, axial, {volumeFraction: 0})).toThrowError(GeometryKernelError)
  try { optimizeTrussTopology(model, axial, {volumeFraction: 1.5}) } catch (error) {
    expect((error as GeometryKernelError).code).toBe('TRUSS_TOPOLOGY_INVALID_INPUT')
  }
  // Transverse load on a DOF without any connected bar: unstable.
  try {
    optimizeTrussTopology(model, [{forcesN: [[0,0,0],[0,1000,0]], weight: 1}])
    expect.unreachable()
  } catch (error) {
    expect((error as GeometryKernelError).code).toBe('TRUSS_TOPOLOGY_SINGULAR')
  }
  expect(() => optimizeTrussTopology(model, [])).toThrowError(GeometryKernelError)
})

it('accepts wrench load cases identical to nodal forces through real WASM', () => {
  const model = {
    nodesMm: [[0,0,0],[1000,0,0],[500,1000,0]] as [number,number,number][],
    members: [member([0,1]), member([0,2]), member([1,2])],
    restrained: [[true,true,true],[true,true,true],[false,false,true]] as [boolean,boolean,boolean][],
  }
  const forcesN = [[0,0,0],[0,0,0],[1000,-500,0]] as [number,number,number][]
  const nodal = optimizeTrussTopology(model, [{forcesN, weight: 1}], {volumeFraction: 0.5})
  const wrench = optimizeTrussTopology(model, [{weight: 1, loads: [
    {nodes: [2], originMm: [500,1000,0], forceN: [1000,-500,0], momentNmm: [0,0,0]},
  ]}], {volumeFraction: 0.5})
  expect(wrench.densities).toEqual(nodal.densities)
  expect(wrench.complianceNmm).toBeCloseTo(nodal.complianceNmm, 12)
  expect(wrench.axialForcesN).toEqual(nodal.axialForcesN)
})
