import {expect, it} from 'vitest'
import {GeometryKernelError} from '../src/services/geometry/kernel'
import {solveFrameCollapse, type FrameCollapseModel} from '../src/services/frameAnalysis'

// Propped cantilever, L = 1000 mm, two elements, unit force at mid-span.
// Planar model: out-of-plane and torsion DOFs restrained at the free nodes.
const proppedCantilever = (): FrameCollapseModel => ({
  nodesMm: [[0,0,0],[500,0,0],[1000,0,0]],
  members: [{nodes:[0,1],youngMpa:200000,poisson:0.3,areaMm2:100,iyyMm4:1e6,izzMm4:1e6,jMm4:2e6},
    {nodes:[1,2],youngMpa:200000,poisson:0.3,areaMm2:100,iyyMm4:1e6,izzMm4:1e6,jMm4:2e6}],
  restrained: [[true,true,true,true,true,true],
    [false,false,true,true,true,false],
    [false,true,true,true,true,false]],
  reference: {forcesN: [[0,0,0],[0,-1,0],[0,0,0]],
    momentsNmm: [[0,0,0],[0,0,0],[0,0,0]], loads: []},
  plasticMomentsNMm: [1e6,1e6],
  maxHinges: 16,
})

it('finds the classical propped-cantilever collapse load through real WASM', () => {
  const response = solveFrameCollapse(proppedCantilever())
  // First hinge at the fixed end at λ₁ = 16Mp/(3L); collapse at λ = 6Mp/L
  // once both mid-span ends yield.
  expect(response.status).toBe('mechanism')
  expect(response.hinges).toHaveLength(3)
  expect(response.hinges[0].member).toBe(0)
  expect(response.hinges[0].atNodeA).toBe(true)
  expect(response.hinges[0].loadFactor).toBeCloseTo(16 * 1e6 / (3 * 1000), 3)
  expect(response.collapseLoadFactor).toBeCloseTo(6 * 1e6 / 1000, 3)
  expect(response.hinges[1].loadFactor).toBeCloseTo(6 * 1e6 / 1000, 3)
})

it('reports elastic-unlimited and rejects bad input', () => {
  // Fixed-fixed variant with only member 0 yieldable: both its ends hinge at
  // λ = 8Mp/L, then elastic member 1 carries the load as a cantilever.
  const fixedFixed = proppedCantilever()
  fixedFixed.restrained[2] = [true,true,true,true,true,true]
  fixedFixed.plasticMomentsNMm = [1e6, null]
  const response = solveFrameCollapse(fixedFixed)
  expect(response.status).toBe('elasticUnlimited')
  expect(response.collapseLoadFactor).toBeNull()
  expect(response.hinges).toHaveLength(2)
  expect(response.hinges[0].loadFactor).toBeCloseTo(8 * 1e6 / 1000, 3)
  // Hinge budget of one stops after the fixed-end hinge.
  const limited = solveFrameCollapse({...proppedCantilever(), maxHinges: 1})
  expect(limited.status).toBe('hingeLimit')
  expect(limited.collapseLoadFactor).toBeNull()
  expect(limited.hinges).toHaveLength(1)
  // Validation propagates as typed kernel errors.
  expect(() => solveFrameCollapse({...proppedCantilever(), plasticMomentsNMm: [1e6]}))
    .toThrow(GeometryKernelError)
  expect(() => solveFrameCollapse({...proppedCantilever(), plasticMomentsNMm: [null, null]}))
    .toThrow(GeometryKernelError)
  expect(() => solveFrameCollapse({...proppedCantilever(), maxHinges: 0}))
    .toThrow(GeometryKernelError)
})
