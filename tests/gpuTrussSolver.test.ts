import { describe, it, expect } from 'vitest'
import {
  referenceSolveTrussPcg,
  runGpuTrussSolver,
  assembleTrussCsr,
  TRUSS_PCG_WGSL,
} from '../src/services/gpuTrussSolver'
import type { TrussModel } from '../src/services/trussAnalysis'

describe('WebGPU Compute Truss / Space-Frame Jacobi-PCG Solver', () => {
  it('solves a 1D axial bar and matches analytical elongation F*L/(E*A)', () => {
    const model: TrussModel = {
      nodesMm: [
        [0, 0, 0],
        [1000, 0, 0],
      ],
      members: [
        { nodes: [0, 1], youngMpa: 200_000, areaMm2: 100 }, // EA/L = 20,000 N/mm
      ],
      restrained: [
        [true, true, true],
        [false, true, true],
      ],
      forcesN: [
        [0, 0, 0],
        [10_000, 0, 0], // u_x = 10,000 / 20,000 = 0.5 mm
      ],
    }

    const res = referenceSolveTrussPcg(model)
    expect(res.freeDofs).toBe(1)
    expect(res.displacementsMm[1]![0]).toBeCloseTo(0.5, 8)
    expect(res.reactionsN[0]![0]).toBeCloseTo(-10_000, 6)
    expect(res.axialForcesN[0]).toBeCloseTo(10_000, 6)
    expect(res.axialStressesMpa[0]).toBeCloseTo(100, 6)
    expect(res.maxRelativeResidual).toBeLessThan(1e-10)
  })

  it('solves a 3D tetrahedral space-frame tripod under vertical apex load via runGpuTrussSolver', async () => {
    // 3 base nodes at radius 1000 mm on Z = 0, apex node 3 at (0, 0, 1000)
    const r = 1000
    const a1 = (2 * Math.PI) / 3
    const a2 = (4 * Math.PI) / 3
    const model: TrussModel = {
      nodesMm: [
        [r, 0, 0],
        [r * Math.cos(a1), r * Math.sin(a1), 0],
        [r * Math.cos(a2), r * Math.sin(a2), 0],
        [0, 0, 1000],
      ],
      members: [
        { nodes: [0, 3], youngMpa: 210_000, areaMm2: 200 },
        { nodes: [1, 3], youngMpa: 210_000, areaMm2: 200 },
        { nodes: [2, 3], youngMpa: 210_000, areaMm2: 200 },
      ],
      restrained: [
        [true, true, true],
        [true, true, true],
        [true, true, true],
        [false, false, false],
      ],
      forcesN: [
        [0, 0, 0],
        [0, 0, 0],
        [0, 0, 0],
        [0, 0, -30_000], // 30 kN downward load
      ],
    }

    const csr = assembleTrussCsr(model)
    expect(csr.freeDofs).toBe(3)

    const res = await runGpuTrussSolver(model)
    expect(res.freeDofs).toBe(3)
    // By symmetry, lateral displacements ux, uy are 0, and vertical reactions sum to +30,000 N
    expect(res.displacementsMm[3]![0]).toBeCloseTo(0, 7)
    expect(res.displacementsMm[3]![1]).toBeCloseTo(0, 7)
    expect(res.displacementsMm[3]![2]).toBeLessThan(0)

    const sumRz =
      res.reactionsN[0]![2] + res.reactionsN[1]![2] + res.reactionsN[2]![2]
    expect(sumRz).toBeCloseTo(30_000, 5)

    // Each bar carries -10,000 * sqrt(2) N in compression
    const expectedAxial = -10_000 * Math.SQRT2
    for (let i = 0; i < 3; i++) {
      expect(res.axialForcesN[i]).toBeCloseTo(expectedAxial, 4)
    }
    expect(res.maxRelativeResidual).toBeLessThan(1e-9)
  })

  it('declares valid WGSL compute shader for CSR SpMV and axial member stress', () => {
    expect(TRUSS_PCG_WGSL).toContain('@compute @workgroup_size(WG)')
    expect(TRUSS_PCG_WGSL).toContain('csr_row_ptr')
    expect(TRUSS_PCG_WGSL).toContain('elongation')
  })
})
