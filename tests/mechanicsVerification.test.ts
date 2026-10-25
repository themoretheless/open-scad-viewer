/**
 * Consolidated mechanics verification suite (P2-15).
 *
 * Every case has a closed-form reference solution and runs through the real
 * WASM kernel — no mocks, no fitted expected values. References are the
 * classical textbook results (Timoshenko & Gere, "Mechanics of Materials";
 * Hibbeler, "Structural Analysis") in the spirit of NAFEMS verification
 * benchmarks: geometry, material and load are stated with the formula, and
 * discretization error is bounded explicitly where the model is mesh-based.
 *
 * Coverage map (kept non-duplicative with the per-feature test files):
 *  1. Simply supported UDL span — midspan deflection and end rotations.
 *  2. Timoshenko cantilever — bending + shear deflection decomposition.
 *  3. Statically indeterminate 3-bar truss — compatibility force split.
 *  4. Pin-pin Euler column — eigenvalue buckling, 4-element bound.
 *  5. Fixed-fixed axial rod — natural frequencies, consistent mass.
 *  6. Fixed-fixed beam — plastic hinge collapse under a midspan point load.
 */
import {expect, it} from 'vitest'
import {
  solveFrame, solveFrameBuckling, solveFrameCollapse,
  type FrameCollapseModel, type FrameModel,
} from '../src/services/frameAnalysis'
import {solveTruss, solveTrussModal} from '../src/services/trussAnalysis'

const E = 200_000, I = 1e6, L = 1000

/** Four-element frame span along +x, bending in the xz plane (local z up).
 * Interior nodes keep only ux, uz and ry free; node 0 anchors torsion. */
const span = (): FrameModel => ({
  nodesMm: [0,250,500,750,1000].map(x => [x,0,0]),
  members: [0,1,2,3].map(i => ({nodes:[i,i+1] as [number,number],youngMpa:E,poisson:0.3,
    areaMm2:100,iyyMm4:I,izzMm4:I,jMm4:2e6})),
  restrained: [
    [true,true,true,true,false,true],
    [false,true,false,true,false,true],
    [false,true,false,true,false,true],
    [false,true,false,true,false,true],
    [false,true,true,false,false,true],
  ],
  forcesN: [[0,0,0],[0,0,0],[0,0,0],[0,0,0],[0,0,0]],
  momentsNmm: [[0,0,0],[0,0,0],[0,0,0],[0,0,0],[0,0,0]],
  loads: [],
})

it('V1: simply supported UDL span matches 5qL⁴/384EI and qL³/24EI', () => {
  const model = span()
  const q = 10 // N/mm downward
  model.loads = [0,1,2,3].map(member => ({type:'uniform' as const, member,
    forceNPerMm:[0,0,-q] as [number,number,number], localAxes:false}))
  const r = solveFrame(model)
  // Timoshenko & Gere §9.3: δ_mid = 5qL⁴/(384EI), θ_end = qL³/(24EI).
  // Euler–Bernoulli members (no shear areas) → nodal values exact.
  expect(r.displacementsMm[2][2]).toBeCloseTo(-5*q*L**4/(384*E*I), 6)
  expect(r.rotationsRad[0][1]).toBeCloseTo(q*L**3/(24*E*I), 6)
  expect(r.rotationsRad[4][1]).toBeCloseTo(-q*L**3/(24*E*I), 6)
  expect(r.reactionsN[0][2]).toBeCloseTo(q*L/2, 9)
  expect(r.reactionsN[4][2]).toBeCloseTo(q*L/2, 9)
  expect(r.members[1].stations[20].momentYNmm).toBeCloseTo(-q*L*L/8, 6)
})

it('V2: Timoshenko cantilever adds the exact PL/GA_s shear deflection', () => {
  const model: FrameModel = {
    nodesMm: [[0,0,0],[L,0,0]],
    members: [{nodes:[0,1],youngMpa:E,poisson:0.3,areaMm2:100,iyyMm4:I,izzMm4:I,jMm4:2e6,
      shearAreaZMm2:50}],
    restrained: [[true,true,true,true,true,true],[false,false,false,false,false,false]],
    forcesN: [[0,0,0],[0,0,-1000]],
    momentsNmm: [[0,0,0],[0,0,0]],
    loads: [],
  }
  const G = E/(2*(1+0.3))
  const bending = 1000*L**3/(3*E*I), shear = 1000*L/(G*50)
  const r = solveFrame(model)
  expect(r.displacementsMm[1][2]).toBeCloseTo(-(bending+shear), 6)
  // Without a shear area the same model is exactly Euler–Bernoulli.
  const eb = structuredClone(model)
  delete eb.members[0].shearAreaZMm2
  expect(solveFrame(eb).displacementsMm[1][2]).toBeCloseTo(-bending, 9)
})

it('V3: indeterminate 3-bar truss matches the compatibility force split', () => {
  // Loaded node at the origin; one vertical bar (L) and two diagonals at
  // θ = 45° from the vertical. Hibbeler §10 compatibility:
  // N_diag = N_center·cos²θ, N_center·(1+2cos³θ) = P.
  const c = Math.SQRT1_2, P = 10_000
  const model = {
    nodesMm: [[0,0,0],[0,1000,0],[-1000,1000,0],[1000,1000,0]] as [number,number,number][],
    members: [
      {nodes:[0,1] as [number,number], youngMpa:E, areaMm2:100},
      {nodes:[0,2] as [number,number], youngMpa:E, areaMm2:100},
      {nodes:[0,3] as [number,number], youngMpa:E, areaMm2:100},
    ],
    restrained: [[false,false,true],[true,true,true],[true,true,true],[true,true,true]] as [boolean,boolean,boolean][],
    forcesN: [[0,-P,0],[0,0,0],[0,0,0],[0,0,0]] as [number,number,number][],
  }
  const r = solveTruss(model)
  const center = P/(1+2*c**3), diagonal = center*c*c
  expect(r.axialForcesN[0]).toBeCloseTo(center, 9)
  expect(r.axialForcesN[1]).toBeCloseTo(diagonal, 9)
  expect(r.axialForcesN[2]).toBeCloseTo(diagonal, 9)
  // Equilibrium: N_center + 2·N_diag·cosθ = P.
  expect(r.axialForcesN[0] + 2*r.axialForcesN[1]*c).toBeCloseTo(P, 9)
  expect(r.displacementsMm[0][0]).toBeCloseTo(0, 12)
})

it('V4: pin-pin column buckles at π²EI/L² within the 4-element bound', () => {
  const model = span()
  model.restrained = [
    [true,true,true,true,false,false],
    [false,false,false,true,false,false],
    [false,false,false,true,false,false],
    [false,false,false,true,false,false],
    [false,true,true,true,false,false],
  ]
  // The buckling schema carries loads only inside `reference`.
  const {forcesN: _f, momentsNmm: _m, loads: _l, ...structure} = model
  const r = solveFrameBuckling({...structure,
    reference: {forcesN: [[0,0,0],[0,0,0],[0,0,0],[0,0,0],[-1000,0,0]],
      momentsNmm: [[0,0,0],[0,0,0],[0,0,0],[0,0,0],[0,0,0]], loads: []},
    modes: 2})
  // Euler pin-pin: P_cr = π²EI/L²; Iyy = Izz → degenerate lateral pair.
  const lambda = Math.PI**2*E*I/(L*L)/1000
  expect(r.modes[0].loadFactor).toBeGreaterThan(0.95*lambda)
  expect(r.modes[0].loadFactor).toBeLessThan(1.05*lambda)
  expect(r.modes[1].loadFactor).toBeCloseTo(r.modes[0].loadFactor, 3)
  expect(r.modes[0].relativeResidual).toBeLessThan(1e-8)
  expect(r.axialForcesN.map(n => Math.round(n))).toEqual([-1000,-1000,-1000,-1000])
})

it('V5: fixed-fixed axial rod matches f₁ = √(E/ρ)/2L and f₂ = 2f₁', () => {
  // Eight elements: consistent-mass overestimation is ≈0.65 % for f1 and
  // ≈2.6 % for f2, inside the 2 % / 5 % bounds asserted below.
  const structure = {
    nodesMm: [0,125,250,375,500,625,750,875,1000].map(x => [x,0,0] as [number,number,number]),
    members: Array.from({length: 8}, (_, i) =>
      ({nodes:[i,i+1] as [number,number], youngMpa:E, areaMm2:100})),
    restrained: [[true,true,true],
      ...Array.from({length: 7}, () => [false,true,true] as [boolean,boolean,boolean]),
      [true,true,true]] as [boolean,boolean,boolean][],
  }
  const rho = 8e-9
  const f1 = Math.sqrt(E/rho)/(2*L) // 2500 Hz
  const r = solveTrussModal(structure, Array(8).fill(rho), 'consistent', 2)
  expect(r.modes[0].frequencyHz).toBeGreaterThan(f1)
  expect(r.modes[0].frequencyHz).toBeLessThan(1.02*f1)
  expect(r.modes[1].frequencyHz).toBeGreaterThan(2*f1)
  expect(r.modes[1].frequencyHz).toBeLessThan(1.05*2*f1)
  // First mode of a fixed-fixed rod: sine half-wave, max at midspan.
  const amplitudes = r.modes[0].displacementsMm.map(d => Math.abs(d[0]))
  expect(amplitudes[0]).toBeCloseTo(0, 12)
  expect(amplitudes[8]).toBeCloseTo(0, 12)
  expect(amplitudes[4]).toBeCloseTo(1, 9)
})

it('V6: fixed-fixed beam collapses at P = 8Mp/L with three simultaneous hinges', () => {
  const model: FrameCollapseModel = {
    nodesMm: [[0,0,0],[500,0,0],[1000,0,0]],
    members: [
      {nodes:[0,1],youngMpa:E,poisson:0.3,areaMm2:100,iyyMm4:I,izzMm4:I,jMm4:2e6},
      {nodes:[1,2],youngMpa:E,poisson:0.3,areaMm2:100,iyyMm4:I,izzMm4:I,jMm4:2e6},
    ],
    restrained: [[true,true,true,true,true,true],
      [false,false,true,true,true,false],
      [true,true,true,true,true,true]],
    reference: {forcesN: [[0,0,0],[0,-1,0],[0,0,0]],
      momentsNmm: [[0,0,0],[0,0,0],[0,0,0]], loads: []},
    plasticMomentsNMm: [1e6,1e6],
    maxHinges: 16,
  }
  const r = solveFrameCollapse(model)
  // Fixed-fixed under a central point load: elastic end and midspan moments
  // are all PL/8, so all hinge positions yield together at λ = 8Mp/L = 8000.
  // Four hinge records: two fixed ends plus both member ends at midspan.
  expect(r.status).toBe('mechanism')
  expect(r.collapseLoadFactor).toBeCloseTo(8*1e6/L, 3)
  expect(r.hinges).toHaveLength(4)
  for (const hinge of r.hinges)
    expect(hinge.loadFactor).toBeCloseTo(8*1e6/L, 3)
})
