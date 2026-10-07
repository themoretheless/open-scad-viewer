import {expect, it} from 'vitest'
import {GeometryKernelError} from '../src/services/geometry/kernel'
import {solveTrussNonlinear, type TrussModel} from '../src/services/trussAnalysis'

// von Mises toggle: supports at (±1000, 0), apex at (0, 30) pressed down.
const toggle = (apexLoad:number): TrussModel => ({
  nodesMm: [[-1000,0,0],[1000,0,0],[0,30,0]],
  members: [{nodes:[0,2],youngMpa:200000,areaMm2:100},
    {nodes:[1,2],youngMpa:200000,areaMm2:100}],
  restrained: [[true,true,true],[true,true,true],[false,false,true]],
  forcesN: [[0,0,0],[0,0,0],[0,-apexLoad,0]],
})
const options = {steps: 10, tolerance: 1e-9, maxIterations: 50}
// Exact path: P(y) = 2·EA·(L₀−L)/L₀·(y/L), y = current apex height.
const toggleLoad = (y:number) => {
  const l0 = Math.sqrt(1000**2 + 30**2), l = Math.sqrt(1000**2 + y*y)
  return 2*200000*100 * (l0-l)/l0 * (y/l)
}

it('tracks the exact toggle path and balances reactions through real WASM', () => {
  const p = toggleLoad(20) // apex height 30 → 20, i.e. v = 10 mm down
  const r = solveTrussNonlinear(toggle(p), options)
  expect(r.converged).toBe(true)
  expect(r.loadFactor).toBe(1)
  expect(r.displacementsMm[2][1]).toBeCloseTo(-10, 6)
  expect(Math.abs(r.displacementsMm[2][0])).toBeLessThan(1e-9)
  const l0 = Math.sqrt(1000**2 + 900), l = Math.sqrt(1000**2 + 400)
  const n = 200000*100 * (l-l0)/l0
  expect(r.axialForcesN[0]).toBeCloseTo(n, 6)
  expect(r.reactionsN[0][1] + r.reactionsN[1][1]).toBeCloseTo(p, 9)
})

it('detects snap-through and rejects bad input through real WASM', () => {
  // The load-controlled limit is ≈ 207.7 N; 400 N cannot be reached.
  const r = solveTrussNonlinear(toggle(400), options)
  expect(r.converged).toBe(false)
  expect(r.loadFactor).toBeGreaterThan(0.35)
  expect(r.loadFactor).toBeLessThan(0.65)
  expect(r.steps.length).toBeGreaterThan(0)
  expect(() => solveTrussNonlinear(toggle(100), {...options, steps: 0}))
    .toThrow(GeometryKernelError)
  expect(() => solveTrussNonlinear(toggle(100), {...options, tolerance: 0.01}))
    .toThrow(GeometryKernelError)
})
