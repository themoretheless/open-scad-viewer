import {expect, it} from 'vitest'
import {GeometryKernelError} from '../src/services/geometry/kernel'
import {solveTruss, type TrussModel} from '../src/services/trussAnalysis'
import {solveFrame, type FrameModel} from '../src/services/frameAnalysis'
import {diagnoseFrame, diagnoseTruss} from '../src/services/structuralDiagnostics'

const trussBar = (): TrussModel => ({
  nodesMm: [[0,0,0],[10,0,0]],
  members: [{nodes:[0,1],youngMpa:2000,areaMm2:2}],
  restrained: [[true,true,true],[false,true,true]],
  forcesN: [[0,0,0],[100,0,0]],
})
const cantilever = (): FrameModel => ({
  nodesMm: [[0,0,0],[1000,0,0]],
  members: [{nodes:[0,1],youngMpa:200000,poisson:0.3,areaMm2:100,iyyMm4:1e6,izzMm4:1e6,jMm4:2e6}],
  restrained: [[true,true,true,true,true,true],[false,false,false,false,false,false]],
  forcesN: [[0,0,0],[0,0,-1000]],
  momentsNmm: [[0,0,0],[0,0,0]],
  loads: [],
})

it('diagnoses an unrestrained truss node through real WASM', () => {
  const {forcesN: _, ...model} = trussBar()
  const stable = diagnoseTruss(model)
  expect(stable.stable).toBe(true)
  expect(stable.issues).toEqual([])
  expect(stable.minNormalizedPivot).toBeGreaterThan(1e-12)
  model.restrained[1] = [false,false,false]
  const report = diagnoseTruss(model)
  expect(report.stable).toBe(false)
  expect(report.minNormalizedPivot).toBeNull()
  // The bar runs along x: y and z have no stiffness at all.
  expect(report.issues).toEqual([
    {node:1,dof:1,dofName:'y',issue:'unrestrained'},
    {node:1,dof:2,dofName:'z',issue:'unrestrained'},
  ])
  // The solver error names the same DOFs.
  expect(() => solveTruss({...model, forcesN: [[0,0,0],[100,0,0]]}))
    .toThrowError(expect.objectContaining({code:'TRUSS_SINGULAR'}))
  try {
    solveTruss({...model, forcesN: [[0,0,0],[100,0,0]]})
    expect.unreachable()
  } catch (error) {
    expect((error as Error).message).toContain('node 1 y unrestrained')
  }
})

it('diagnoses a frame rigid-body mechanism and hinge-node rotations', () => {
  const {forcesN: _, momentsNmm: _m, loads: _l, ...structure} = cantilever()
  const stable = diagnoseFrame(structure)
  expect(stable.stable).toBe(true)
  expect(stable.minNormalizedPivot).toBeGreaterThan(1e-12)
  const free = {...structure, restrained: structure.restrained.map(() =>
    [false,false,false,false,false,false] as [boolean,boolean,boolean,boolean,boolean,boolean])}
  const report = diagnoseFrame(free)
  expect(report.stable).toBe(false)
  expect(report.issues.every(i => i.issue === 'mechanism')).toBe(true)
  expect(report.minNormalizedPivot).toBeLessThanOrEqual(1e-12)
  // Six springs standing in for the fixed end stabilize the beam.
  const springy = diagnoseFrame({...free, supports: [...Array(6)].map((_, dof) =>
    ({type:'spring' as const, node:0, dof, stiffness:1e9}))})
  expect(springy.stable).toBe(true)
  // The solver error carries the same diagnosis.
  const model = cantilever()
  model.restrained = [[false,false,false,false,false,false],[false,false,false,false,false,false]]
  try {
    solveFrame(model)
    expect.unreachable()
  } catch (error) {
    expect(error).toBeInstanceOf(GeometryKernelError)
    expect((error as GeometryKernelError).code).toBe('FRAME_SINGULAR')
    expect((error as Error).message).toContain('in a mechanism')
  }
})

it('rejects malformed diagnose requests', () => {
  const {forcesN: _, ...model} = trussBar()
  expect(() => diagnoseTruss({...model, nodesMm: Array.from({length:126}, () => [0,0,0])}))
    .toThrow(GeometryKernelError)
  expect(() => diagnoseTruss({...model, members: []})).toThrow(GeometryKernelError)
})
