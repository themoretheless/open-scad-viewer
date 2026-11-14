import { describe, expect, it } from 'vitest'
import {
  MANIFOLD_ABI_VERSION,
  MANIFOLD_REPORT_SCHEMA,
  ManifoldAbiMismatchError,
  assertManifoldAbi,
  type KernelManifoldBoolean,
  type KernelManifoldComponentMetrics,
  type KernelManifoldMetrics,
  type KernelManifoldRepair,
  type KernelManifoldReport,
} from '../src/kernel/geometry/manifoldAnalysis'
import { GeometryKernelError } from '../src/kernel/geometry/kernel'

// `Record<keyof T, true>` makes the compiler reject a missing or extra key, so
// these objects are exact field sets of the TS report types. The schema file is
// also compared with the Rust MANIFOLD_REPORT_SCHEMA by a geometry-bridge test.
const fields = <T>(keys: Record<keyof T, true>): string[] => Object.keys(keys)
const tsReports: Record<string, string[]> = {
  check: fields<KernelManifoldReport>({
    vertexCount: true, triangleCount: true, componentCount: true, isManifold: true, isManifoldWithBoundary: true,
    boundaryEdges: true, nonManifoldEdges: true, orientationEdges: true, degenerateTriangles: true,
    nonManifoldVertices: true, isolatedVertices: true,
  }),
  repair: fields<KernelManifoldRepair>({
    positions: true, indices: true, weldedVertices: true, removedDegenerateTriangles: true, flippedTriangles: true,
    splitVertices: true, filledHoles: true, filledTriangles: true, mode: true, isManifold: true, isManifoldWithBoundary: true,
  }),
  metrics: fields<KernelManifoldMetrics>({
    vertexCount: true, triangleCount: true, edgeCount: true, eulerCharacteristic: true, componentCount: true,
    signedVolume: true, surfaceArea: true, watertight: true, components: true,
  }),
  metricsComponent: fields<KernelManifoldComponentMetrics>({
    triangleCount: true, vertexCount: true, edgeCount: true, boundaryEdges: true, eulerCharacteristic: true,
    genus: true, signedVolume: true, surfaceArea: true,
  }),
  boolean: fields<KernelManifoldBoolean>({
    positions: true, indices: true, operandRepaired: true, kernel: true, isManifold: true,
  }),
}

describe('Manifold report schema', () => {
  it('is versioned and matches the TS report field sets', () => {
    expect(MANIFOLD_REPORT_SCHEMA.schemaVersion).toBeGreaterThanOrEqual(1)
    expect(MANIFOLD_ABI_VERSION).toBe(MANIFOLD_REPORT_SCHEMA.abiVersion)
    expect(Object.keys(MANIFOLD_REPORT_SCHEMA.reports).sort()).toEqual(Object.keys(tsReports).sort())
    for (const [kind, keys] of Object.entries(tsReports)) {
      const schemaFields = (MANIFOLD_REPORT_SCHEMA.reports as Record<string, string[]>)[kind]
      expect([...schemaFields].sort(), kind).toEqual([...keys].sort())
    }
  })
})

describe('Manifold ABI startup check', () => {
  it('accepts a kernel exporting the expected version and checks it once', () => {
    let calls = 0
    const exports = { abi_manifold_abi_version: () => { calls++; return MANIFOLD_ABI_VERSION } }
    expect(() => assertManifoldAbi(exports)).not.toThrow()
    expect(() => assertManifoldAbi(exports)).not.toThrow()
    expect(calls).toBe(1)
  })

  it('refuses a kernel with a different version with a typed error', () => {
    const exports = { abi_manifold_abi_version: () => MANIFOLD_ABI_VERSION + 1 }
    let error: unknown
    try { assertManifoldAbi(exports) } catch (caught) { error = caught }
    expect(error).toBeInstanceOf(ManifoldAbiMismatchError)
    expect(error).toBeInstanceOf(GeometryKernelError)
    expect(error).toMatchObject({ code: 'GEOMETRY_ABI_MISMATCH', expected: MANIFOLD_ABI_VERSION, actual: MANIFOLD_ABI_VERSION + 1 })
    // A refused instance stays refused.
    expect(() => assertManifoldAbi(exports)).toThrow(ManifoldAbiMismatchError)
  })

  it('refuses a stale kernel without the version export', () => {
    expect(() => assertManifoldAbi({})).toThrow(expect.objectContaining({ code: 'GEOMETRY_ABI_MISMATCH', actual: null }))
  })
})
