import {expect, it} from 'vitest'
import {GeometryKernelError, callGeometryRust} from '../src/services/geometry/kernel'
import {solveFrame, solveFrameEnvelope, type FrameEnvelopeModel, type FrameModel} from '../src/services/frameAnalysis'

const L = 1000, E = 200000, I = 1e6
const cantilever = (): FrameModel => ({
  nodesMm: [[0,0,0],[L,0,0]],
  members: [{nodes:[0,1],youngMpa:E,poisson:0.3,areaMm2:100,iyyMm4:I,izzMm4:I,jMm4:2e6}],
  restrained: [[true,true,true,true,true,true],[false,false,false,false,false,false]],
  forcesN: [[0,0,0],[0,0,-1000]],
  momentsNmm: [[0,0,0],[0,0,0]],
  loads: [],
})

it('solves an analytical cantilever through real WASM without mutating the model', () => {
  const model = cantilever(), before = structuredClone(model)
  const r = solveFrame(model)
  expect(r.displacementsMm[1][2]).toBeCloseTo(-1000*L**3/(3*E*I), 9)
  expect(r.rotationsRad[1][1]).toBeCloseTo(1000*L**2/(2*E*I), 9)
  expect(r.reactionsN[0]).toEqual([0,0,1000])
  expect(r.reactionMomentsNmm[0][1]).toBeCloseTo(-1e6, 6)
  expect(r.members[0].stations).toHaveLength(21)
  expect(r.members[0].stations[0].momentYNmm).toBeCloseTo(1e6, 6)
  expect(r.members[0].stations[20].momentYNmm).toBeCloseTo(0, 6)
  expect(r.maxRelativeResidual).toBeLessThan(1e-12)
  expect(model).toEqual(before)
})

it('recovers closed-form diagrams for a simply supported UDL span', () => {
  const model = cantilever()
  model.restrained = [[true,true,true,true,false,true],[false,true,true,false,false,true]]
  model.forcesN = [[0,0,0],[0,0,0]]
  model.loads = [{type:'uniform',member:0,forceNPerMm:[0,0,-10],localAxes:false}]
  const r = solveFrame(model)
  expect(r.reactionsN[0][2]).toBeCloseTo(5000, 9)
  expect(r.reactionsN[1][2]).toBeCloseTo(5000, 9)
  const stations = r.members[0].stations
  expect(stations[10].momentYNmm).toBeCloseTo(-10*L*L/8, 6)
  expect(stations[0].shearZN).toBeCloseTo(-5000, 9)
})

it('models a propped cantilever through an end release', () => {
  const model = cantilever()
  model.restrained[1] = [true,true,true,true,true,true]
  model.members[0].releaseB = [false,true,false]
  model.forcesN = [[0,0,0],[0,0,0]]
  model.loads = [{type:'uniform',member:0,forceNPerMm:[0,0,-10],localAxes:true}]
  const r = solveFrame(model)
  expect(r.reactionsN[1][2]).toBeCloseTo(3*10*L/8, 9)
  expect(r.members[0].stations[0].momentYNmm).toBeCloseTo(10*L*L/8, 6)
  expect(r.members[0].stations[20].momentYNmm).toBeCloseTo(0, 6)
})

it('preserves typed failures and stays usable after refusals', () => {
  const mechanism = cantilever()
  mechanism.members[0].releaseA = [true,true,true]
  expect(() => solveFrame(mechanism)).toThrowError(expect.objectContaining({
    name: 'GeometryKernelError', code: 'FRAME_SINGULAR',
  }))
  const unstable = cantilever()
  unstable.members[0].releaseA = [true,false,false]
  unstable.members[0].releaseB = [true,false,false]
  expect(() => solveFrame(unstable)).toThrowError(expect.objectContaining({code:'FRAME_INVALID_INPUT'}))
  for (const poisson of [0.5, -1, NaN]) {
    const invalid = cantilever(); invalid.members[0].poisson = poisson
    expect(() => solveFrame(invalid)).toThrow()
  }
  expect(() => callGeometryRust('frame_solve', {...cantilever(), sections: []}))
    .toThrow(GeometryKernelError)
  expect(solveFrame(cantilever()).reactionsN[0][2]).toBeCloseTo(1000, 9)
})

const twoCaseEnvelope = (): FrameEnvelopeModel => ({
  nodesMm: [[0,0,0],[L,0,0]],
  members: [{nodes:[0,1],youngMpa:E,poisson:0.3,areaMm2:100,iyyMm4:I,izzMm4:I,jMm4:2e6}],
  restrained: [[true,true,true,true,true,true],[false,false,false,false,false,false]],
  cases: [
    {forcesN: [[0,0,0],[0,0,-1000]], momentsNmm: [[0,0,0],[0,0,0]], loads: []},
    {forcesN: [[0,0,0],[0,0,600]], momentsNmm: [[0,0,0],[0,0,0]], loads: []},
  ],
  combinations: [
    {name: 'G', factors: [1,0]},
    {name: 'Q', factors: [0,1]},
    {name: '1.35G+1.5Q', factors: [1.35,1.5]},
    {name: 'G-Q', factors: [1,-1]},
  ],
})

it('envelopes load combinations through real WASM with governing indices', () => {
  const r = solveFrameEnvelope(twoCaseEnvelope())
  const tipZ = r.displacementsMm[1][2]
  // Tip uz per combination: G −5/3, Q +1, 1.35G+1.5Q −0.75, G−Q −8/3.
  expect(tipZ.min).toBeCloseTo(-8/3, 9)
  expect(tipZ.max).toBeCloseTo(1, 9)
  expect(tipZ.minCombination).toBe(3)
  expect(tipZ.maxCombination).toBe(1)
  const rootMy = r.members[0].stations[0].momentYNmm
  expect(rootMy.min).toBeCloseTo(-6e5, 6)
  expect(rootMy.max).toBeCloseTo(1.6e6, 6)
  expect(r.maxDeflectionMm.max).toBeCloseTo(8/3, 9)
  expect(r.combinations).toEqual(['G','Q','1.35G+1.5Q','G-Q'])
  expect(r.loadCases).toBe(2)
  expect(r.maxRelativeResidual).toBeLessThan(1e-12)
})

it('rejects malformed envelope requests with typed failures', () => {
  const badCombo = twoCaseEnvelope()
  badCombo.combinations[0].factors = [1]
  expect(() => solveFrameEnvelope(badCombo))
    .toThrowError(expect.objectContaining({code: 'COMBO_INVALID_INPUT'}))
  const noCases = twoCaseEnvelope()
  noCases.cases = []
  expect(() => solveFrameEnvelope(noCases))
    .toThrowError(expect.objectContaining({code: 'COMBO_INVALID_INPUT'}))
  // A single combined envelope equals the plain solve of that combination.
  const single = twoCaseEnvelope()
  single.combinations = [{name: 'G-Q', factors: [1,-1]}]
  const r = solveFrameEnvelope(single)
  expect(r.displacementsMm[1][2].min).toBeCloseTo(-8/3, 9)
  expect(r.displacementsMm[1][2].min).toBe(r.displacementsMm[1][2].max)
})

it('solves spring and unilateral supports through real WASM', () => {
  const model = cantilever()
  model.supports = [{type: 'spring', node: 1, dof: 2, stiffness: 0.6}]
  const r = solveFrame(model)
  const k = 0.6, rb = 1000*k*L**3/(3*E*I + k*L**3)
  expect(r.reactionsN[1][2]).toBeCloseTo(rb, 9)
  expect(r.displacementsMm[1][2]).toBeCloseTo(-rb/k, 9)
  expect(r.reactionsN[0][2]).toBeCloseTo(1000 - rb, 9)
})

it('opens and closes a rigid contact by load direction', () => {
  const model = cantilever()
  model.forcesN = [[0,0,0],[0,0,0]]
  model.supports = [{type: 'lowerBound', node: 1, dof: 2}]
  model.loads = [{type:'uniform',member:0,forceNPerMm:[0,0,-10],localAxes:false}]
  const down = solveFrame(model)
  expect(down.reactionsN[1][2]).toBeCloseTo(3750, 9)
  expect(down.displacementsMm[1][2]).toBeCloseTo(0, 12)
  model.loads = [{type:'uniform',member:0,forceNPerMm:[0,0,10],localAxes:false}]
  const up = solveFrame(model)
  expect(up.reactionsN[1][2]).toBeCloseTo(0, 12)
  expect(up.displacementsMm[1][2]).toBeCloseTo(6.25, 9)
  const bad = cantilever()
  bad.supports = [{type: 'lowerBound', node: 0, dof: 2}]
  expect(() => solveFrame(bad))
    .toThrowError(expect.objectContaining({code: 'FRAME_INVALID_INPUT'}))
})
