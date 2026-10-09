import {expect, it} from 'vitest'
import {GeometryKernelError} from '../src/services/geometry/kernel'
import {solveFrameBuckling, type FrameBucklingModel} from '../src/services/frameAnalysis'
import {solveTrussBuckling, type TrussModel} from '../src/services/trussAnalysis'

const column = (tipForce: number): FrameBucklingModel => ({
  nodesMm: [0,250,500,750,1000].map(x => [x,0,0]),
  members: [0,1,2,3].map(i => ({nodes:[i,i+1] as [number,number],youngMpa:200000,poisson:0.3,
    areaMm2:100,iyyMm4:1e6,izzMm4:1e6,jMm4:2e6})),
  restrained: [[true,true,true,true,true,true],[false,false,false,false,false,false],
    [false,false,false,false,false,false],[false,false,false,false,false,false],
    [false,false,false,false,false,false]],
  reference: {forcesN: [[0,0,0],[0,0,0],[0,0,0],[0,0,0],[tipForce,0,0]],
    momentsNmm: [[0,0,0],[0,0,0],[0,0,0],[0,0,0],[0,0,0]], loads: []},
  modes: 4,
})

it('buckles a cantilever column at the Euler load through real WASM', () => {
  const response = solveFrameBuckling(column(-1000))
  expect(response.modes).toHaveLength(4)
  expect(response.axialForcesN.map(n => Math.round(n))).toEqual([-1000,-1000,-1000,-1000])
  // P_cr = π²EI/(4L²); Iyy = Izz → degenerate pair, then the second mode.
  const pCr = Math.PI**2 * 2e5 * 1e6 / (4 * 1000 * 1000)
  const lambda = pCr / 1000
  expect(response.modes[0].loadFactor).toBeGreaterThan(0.995 * lambda)
  expect(response.modes[0].loadFactor).toBeLessThan(1.005 * lambda)
  expect(response.modes[1].loadFactor).toBeCloseTo(response.modes[0].loadFactor, 6)
  expect(response.modes[2].loadFactor).toBeGreaterThan(5 * lambda)
  expect(response.modes[0].relativeResidual).toBeLessThan(1e-8)
  // Lateral mode: transverse displacement attains the normalized peak.
  const peakY = Math.max(...response.modes[0].displacementsMm.map(d => Math.abs(d[1])),
    ...response.modes[0].displacementsMm.map(d => Math.abs(d[2])))
  expect(peakY).toBeCloseTo(1, 9)
  // Tension reference: buckling only under the reversed load.
  for (const mode of solveFrameBuckling(column(1000)).modes) {
    expect(mode.loadFactor).toBeLessThan(0)
  }
})

it('refuses unilateral supports, bad mode counts, and singular structures', () => {
  expect(() => solveFrameBuckling({...column(-1000), modes: 0}))
    .toThrowError(expect.objectContaining({code:'FRAME_INVALID_INPUT'}))
  expect(() => solveFrameBuckling({...column(-1000), modes: 9}))
    .toThrowError(expect.objectContaining({code:'FRAME_INVALID_INPUT'}))
  expect(() => solveFrameBuckling({...column(-1000),
    supports: [{type:'lowerBound', node:4, dof:1}]}))
    .toThrowError(expect.objectContaining({code:'FRAME_INVALID_INPUT'}))
  const free = column(-1000)
  free.restrained = free.restrained.map(() => [false,false,false,false,false,false])
  expect(() => solveFrameBuckling(free))
    .toThrowError(expect.objectContaining({code:'FRAME_SINGULAR'}))
  expect(() => solveFrameBuckling({...column(-1000), reference: {...column(-1000).reference,
    loads: [{type:'uniform', member:0, forceNPerMm:[0,0,-1], localAxes:false}]}})).not.toThrow()
})

it('buckles a von Mises toggle truss at the closed-form factor', () => {
  // Apex (10,5), span 20, sin²θ = 0.2, cos²θ = 0.8, N = −F·L/(2h).
  const model: TrussModel = {
    nodesMm: [[0,0,0],[10,5,0],[20,0,0]],
    members: [{nodes:[0,1],youngMpa:2000,areaMm2:2},{nodes:[1,2],youngMpa:2000,areaMm2:2}],
    restrained: [[true,true,true],[false,false,true],[true,true,true]],
    forcesN: [[0,0,0],[0,-100,0],[0,0,0]],
  }
  const L = Math.hypot(10,5)
  const lambda = (2*4000/L*0.2) / (2*(100*L/10)/L*0.8)
  const response = solveTrussBuckling(model, 2)
  expect(response.modes).toHaveLength(2)
  expect(response.modes[0].loadFactor).toBeCloseTo(lambda, 5)
  expect(response.modes[1].loadFactor).toBeCloseTo(16 * lambda, 4)
  expect(Math.abs(response.modes[0].displacementsMm[1][1])).toBeCloseTo(1, 9)
  expect(response.modes[0].relativeResidual).toBeLessThan(1e-10)
  // Zero loads → no axial force → no buckling modes.
  const idle = {...model, forcesN: model.forcesN.map(() => [0,0,0] as [number,number,number])}
  expect(solveTrussBuckling(idle, 2).modes).toEqual([])
  expect(() => solveTrussBuckling(model, 0)).toThrow(GeometryKernelError)
})
