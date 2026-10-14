import {expect, it} from 'vitest'
import {GeometryKernelError} from '../src/services/geometry/kernel'
import {solveFrameModal, type FrameModalModel} from '../src/services/frameAnalysis'
import {solveTrussModal} from '../src/services/trussAnalysis'

const frameCantilever = (): Omit<FrameModalModel, 'densitiesTMm3' | 'massModel' | 'modes'> => ({
  nodesMm: [0,250,500,750,1000].map(x => [x,0,0]),
  members: [0,1,2,3].map(i => ({nodes:[i,i+1] as [number,number],youngMpa:200000,poisson:0.3,
    areaMm2:100,iyyMm4:1e6,izzMm4:1e6,jMm4:2e6})),
  restrained: [[true,true,true,true,true,true],[false,false,false,false,false,false],
    [false,false,false,false,false,false],[false,false,false,false,false,false],
    [false,false,false,false,false,false]],
})

it('finds cantilever natural frequencies through real WASM', () => {
  const response = solveFrameModal({...frameCantilever(),
    densitiesTMm3: [8e-9,8e-9,8e-9,8e-9], massModel: 'consistent', modes: 2})
  // f1 = β₁²/(2π)·√(EI/(ρA))/L² ≈ 279.77 Hz; degenerate lateral pair.
  const f1 = 1.8751**2 / (2*Math.PI) * Math.sqrt(2e5*1e6 / (8e-9*100)) / 1e6
  expect(response.modes).toHaveLength(2)
  expect(response.modes[0].frequencyHz).toBeGreaterThan(0.995 * f1)
  expect(response.modes[0].frequencyHz).toBeLessThan(1.005 * f1)
  expect(response.modes[1].frequencyHz).toBeCloseTo(response.modes[0].frequencyHz, 3)
  expect(response.modes[0].omegaRadS).toBeCloseTo(2 * Math.PI * response.modes[0].frequencyHz, 9)
  expect(response.modes[0].relativeResidual).toBeLessThan(1e-8)
  expect(response.totalMassT).toBeCloseTo(8e-4, 12)
  // Lumped model lands in the same neighborhood.
  const lumped = solveFrameModal({...frameCantilever(),
    densitiesTMm3: [8e-9,8e-9,8e-9,8e-9], massModel: 'lumped', modes: 1})
  expect(lumped.modes[0].frequencyHz).toBeGreaterThan(0.85 * f1)
  expect(lumped.modes[0].frequencyHz).toBeLessThan(1.05 * f1)
  // Massless members leave no finite-frequency modes.
  expect(solveFrameModal({...frameCantilever(),
    densitiesTMm3: [0,0,0,0], massModel: 'lumped', modes: 2}).modes).toEqual([])
})

it('finds truss axial-rod frequencies and rejects bad input', () => {
  const structure = {
    nodesMm: [[0,0,0],[250,0,0],[500,0,0],[750,0,0],[1000,0,0]] as [number,number,number][],
    members: ([[0,1],[1,2],[2,3],[3,4]] as [number,number][]).map(nodes =>
      ({nodes, youngMpa:200000, areaMm2:100})),
    restrained: [[true,true,true],[false,true,true],[false,true,true],
      [false,true,true],[false,true,true]] as [boolean,boolean,boolean][],
  }
  // f₁ = √(E/ρ)/(4L) = 1250 Hz; f₃ = 3f₁.
  const response = solveTrussModal(structure, [8e-9,8e-9,8e-9,8e-9], 'consistent', 2)
  expect(response.modes[0].frequencyHz).toBeGreaterThan(0.98 * 1250)
  expect(response.modes[0].frequencyHz).toBeLessThan(1.02 * 1250)
  expect(response.modes[1].frequencyHz).toBeGreaterThan(0.92 * 3750)
  expect(response.modes[1].frequencyHz).toBeLessThan(1.08 * 3750)
  expect(response.modes[0].displacementsMm[4][0]).toBeCloseTo(1, 9)
  expect(() => solveTrussModal(structure, [8e-9,8e-9,8e-9,8e-9], 'consistent', 0))
    .toThrow(GeometryKernelError)
  expect(() => solveTrussModal(structure, [8e-9], 'consistent', 1))
    .toThrow(GeometryKernelError)
  expect(() => solveTrussModal(structure, [8e-9,8e-9,8e-9,8e-9], 'smeared' as 'lumped', 1))
    .toThrow(GeometryKernelError)
})
