import {expect, it} from 'vitest'
import {GeometryKernelError, callGeometryRust} from '../src/services/geometry/kernel'
import {solveTruss, type TrussModel} from '../src/services/trussAnalysis'

const bar = (): TrussModel => ({
  nodesMm: [[0,0,0],[10,0,0]],
  members: [{nodes:[0,1],youngMpa:2000,areaMm2:2}],
  restrained: [[true,true,true],[false,true,true]],
  forcesN: [[0,0,0],[100,0,0]],
})

it('solves an analytical bar through real WASM without mutating the model', () => {
  const model = bar(), before = structuredClone(model)
  const response = solveTruss(model)
  expect(response.displacementsMm[1][0]).toBeCloseTo(0.25, 12)
  expect(response.reactionsN).toEqual([[-100,0,0],[0,0,0]])
  expect(response.axialForcesN[0]).toBeCloseTo(100, 10)
  expect(response.axialStressesMpa[0]).toBeCloseTo(50, 10)
  expect(response.maxDeflectionMm).toBeCloseTo(0.25, 12)
  expect(response.maxRelativeResidual).toBeLessThan(1e-12)
  expect(response.freeDofs).toBe(1)
  expect(model).toEqual(before)
})

it('preserves typed singular failure and remains usable after refusals', () => {
  const model = bar()
  model.restrained[1] = [false,false,false]
  expect(() => solveTruss(model)).toThrowError(expect.objectContaining({
    name: 'GeometryKernelError', code: 'TRUSS_SINGULAR',
  }))
  for (const youngMpa of [0, -1]) {
    const invalid = bar(); invalid.members[0].youngMpa = youngMpa
    expect(() => solveTruss(invalid)).toThrow(GeometryKernelError)
  }
  // Nonfinite numbers cannot enter the shared binary protocol at all.
  for (const youngMpa of [NaN, Infinity]) {
    const invalid = bar(); invalid.members[0].youngMpa = youngMpa
    expect(() => solveTruss(invalid)).toThrow('Nonfinite binary number')
  }
  expect(() => callGeometryRust('truss_solve', {...bar(), momentsNm: [[0,0,0],[0,0,1]]}))
    .toThrow(GeometryKernelError)
  expect(solveTruss(bar()).axialForcesN[0]).toBeCloseTo(100, 10)
})

it('rejects oversized collections and malformed supports before solving', () => {
  expect(() => solveTruss({...bar(), nodesMm: Array.from({length:126}, () => [0,0,0])}))
    .toThrow(GeometryKernelError)
  expect(() => callGeometryRust('truss_solve', {...bar(), restrained: [[1,1,1],[0,1,1]]}))
    .toThrow(GeometryKernelError)
})

it('balances signed forces and moments at the 125-node boundary', () => {
  const model: TrussModel = {nodesMm:[[10,0,0],[0,10,0],[0,0,0]], members:[],
    restrained:[[true,true,true],[true,true,true],[true,true,true]],
    forcesN:[[0,0,0],[0,0,0],[0,0,0]]}
  for(let i=0;i<122;i++) {
    model.nodesMm.push([1,2,10+i/10]); model.restrained.push([false,false,false])
    model.forcesN.push([1,-2,-3])
    for(let anchor=0;anchor<3;anchor++) {
      model.members.push({nodes:[anchor,i+3],youngMpa:2000,areaMm2:2})
    }
  }
  const response=solveTruss(model), force=[0,0,0], moment=[0,0,0]
  expect(response.freeDofs).toBe(366)
  expect(response.axialForcesN).toHaveLength(366)
  expect(response.maxRelativeResidual).toBeLessThan(1e-12)
  // Each free node is supported by a statically determinate tripod. Solve its
  // three force-balance equations analytically, independently of stiffness.
  for(let i=0;i<122;i++) {
    const z=10+i/10, total=-3/z, q0=(total-1)/10, q1=(2*total+2)/10
    const expected=[q0*Math.hypot(-9,2,z),q1*Math.hypot(1,-8,z),
      (total-q0-q1)*Math.hypot(1,2,z)]
    for(let k=0;k<3;k++) expect(response.axialForcesN[i*3+k]).toBeCloseTo(expected[k],9)
  }
  for(let i=0;i<model.nodesMm.length;i++) {
    const p=model.nodesMm[i], f=model.forcesN[i].map((x,k)=>x+response.reactionsN[i][k])
    for(let k=0;k<3;k++) force[k]+=f[k]
    moment[0]+=p[1]*f[2]-p[2]*f[1]
    moment[1]+=p[2]*f[0]-p[0]*f[2]
    moment[2]+=p[0]*f[1]-p[1]*f[0]
  }
  for(const value of [...force,...moment]) expect(Math.abs(value)).toBeLessThan(1e-8)
})
