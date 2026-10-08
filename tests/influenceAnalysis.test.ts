import {expect, it} from 'vitest'
import {GeometryKernelError} from '../src/services/geometry/kernel'
import {solveFrameInfluence, type FrameInfluenceModel} from '../src/services/frameAnalysis'

// Simply supported beam, L = 1000 mm, two elements, planar restraints.
const beam = (): Omit<FrameInfluenceModel, 'target'> => ({
  nodesMm: [[0,0,0],[500,0,0],[1000,0,0]],
  members: [{nodes:[0,1],youngMpa:200000,poisson:0.3,areaMm2:100,iyyMm4:1e6,izzMm4:1e6,jMm4:2e6},
    {nodes:[1,2],youngMpa:200000,poisson:0.3,areaMm2:100,iyyMm4:1e6,izzMm4:1e6,jMm4:2e6}],
  restrained: [[true,true,true,true,true,false],
    [false,false,true,true,true,false],
    [false,true,true,true,true,false]],
  forceN: [0,-1,0],
  positions: [{member:0,atMm:0},{member:0,atMm:250},{member:0,atMm:500},
    {member:1,atMm:250},{member:1,atMm:500}],
})

it('finds the classical reaction and moment influence lines through real WASM', () => {
  // Reaction at A: the line is 1 − x/L.
  const reaction = solveFrameInfluence({...beam(), target: {type:'reaction', node:0, dof:1}})
  for (const [v, x] of reaction.values.map((v, i) => [v, [0,250,500,750,1000][i]] as const)) {
    expect(v).toBeCloseTo(1 - x/1000, 9)
  }
  // Mid-span moment: triangle peaking at a·b/L = 250 mm.
  const moment = solveFrameInfluence({...beam(),
    target: {type:'memberResultant', member:0, atMm:500, resultant:'momentZ'}})
  for (const [v, expected] of moment.values.map((v, i) => [v, [0,125,250,125,0][i]] as const)) {
    expect(Math.abs(v)).toBeCloseTo(expected, 6)
  }
})

it('matches Betti–Maxwell reciprocity and rejects bad input', () => {
  // Mid-span deflection for a load at x equals the deflection at x for a unit
  // load at mid-span: v = −x(3L²−4x²)/(48EI).
  const [E, I, L] = [2e5, 1e6, 1000]
  const exact = (x:number) => {const s = Math.min(x, L-x); return -s*(3*L*L-4*s*s)/(48*E*I)}
  const r = solveFrameInfluence({...beam(), target: {type:'displacement', node:1, dof:1}})
  for (const [v, x] of r.values.map((v, i) => [v, [0,250,500,750,1000][i]] as const)) {
    expect(v).toBeCloseTo(exact(x), 12)
  }
  const model = beam()
  expect(() => solveFrameInfluence({...model, positions: [{member:0,atMm:9000}],
    target: {type:'reaction', node:0, dof:1}})).toThrow(GeometryKernelError)
  expect(() => solveFrameInfluence({...model, positions: [],
    target: {type:'reaction', node:0, dof:1}})).toThrow(GeometryKernelError)
  expect(() => solveFrameInfluence({...model,
    target: {type:'reaction', node:1, dof:1}})).toThrow(GeometryKernelError)
})
