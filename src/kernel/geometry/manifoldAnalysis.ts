import {decodeNurbsResult,GeometryKernelError,kernelRuntime,kernelVertexFormat} from './kernel'
import {copyBuffer} from './bufferTransport'
import manifoldReportSchema from './manifoldReportSchema.json'

/**
 * Versioned contract of the Manifold reports below. The same file is read by
 * a geometry-bridge test that compares it with the Rust `MANIFOLD_REPORT_SCHEMA`,
 * and by a TS test that compares it with the field sets of real kernel reports.
 */
export const MANIFOLD_REPORT_SCHEMA = manifoldReportSchema
/** Manifold WASM export surface version this TypeScript expects. */
export const MANIFOLD_ABI_VERSION: number = manifoldReportSchema.abiVersion

/** The loaded geometry kernel exports a different Manifold ABI than this TypeScript expects. */
export class ManifoldAbiMismatchError extends GeometryKernelError {
  constructor(public readonly expected: number, public readonly actual: number | null) {
    super('GEOMETRY_ABI_MISMATCH', actual === null
      ? `Geometry kernel has no Manifold ABI version export (expected v${expected}); rebuild the WASM kernel`
      : `Geometry kernel Manifold ABI v${actual} does not match TypeScript v${expected}; rebuild the WASM kernel or reload the app`)
    this.name = 'ManifoldAbiMismatchError'
  }
}

const verifiedManifoldExports = new WeakSet<object>()

/**
 * Refuse a kernel instance whose Manifold ABI differs from this build. Runs
 * once per instance, before its first Manifold call; a stale kernel paired
 * with newer TS (or the reverse) fails with a typed error instead of reading
 * misaligned result slots.
 */
export function assertManifoldAbi(exports: {abi_manifold_abi_version?: () => number}): void {
  if (verifiedManifoldExports.has(exports)) return
  const actual = typeof exports.abi_manifold_abi_version === 'function' ? exports.abi_manifold_abi_version() >>> 0 : null
  if (actual !== MANIFOLD_ABI_VERSION) throw new ManifoldAbiMismatchError(MANIFOLD_ABI_VERSION, actual)
  verifiedManifoldExports.add(exports)
}

function manifoldRuntime(): ReturnType<typeof kernelRuntime> {
  const runtime = kernelRuntime()
  assertManifoldAbi(runtime.exports)
  return runtime
}

export interface KernelManifoldReport {
  readonly vertexCount: number
  readonly triangleCount: number
  readonly componentCount: number
  readonly isManifold: boolean
  readonly isManifoldWithBoundary: boolean
  /** Flat [a, b] vertex pairs per edge. */
  readonly boundaryEdges: Uint32Array<ArrayBuffer>
  readonly nonManifoldEdges: Uint32Array<ArrayBuffer>
  readonly orientationEdges: Uint32Array<ArrayBuffer>
  readonly degenerateTriangles: Uint32Array<ArrayBuffer>
  readonly nonManifoldVertices: Uint32Array<ArrayBuffer>
  readonly isolatedVertices: Uint32Array<ArrayBuffer>
}

/** Repair result from the manifold-core kernel (abi_manifold_repair). */
export interface KernelManifoldRepair {
  readonly positions: Float64Array<ArrayBuffer>
  readonly indices: Uint32Array<ArrayBuffer>
  readonly weldedVertices: number
  readonly removedDegenerateTriangles: number
  readonly flippedTriangles: number
  /** Full mode: vertices duplicated when splitting non-manifold edges/fans. */
  readonly splitVertices: number
  /** Full mode: boundary loops triangulated. */
  readonly filledHoles: number
  /** Full mode: triangles added by hole filling. */
  readonly filledTriangles: number
  /** 0 = Conservative, 1 = Full. */
  readonly mode: number
  readonly isManifold: boolean
  readonly isManifoldWithBoundary: boolean
}

/** Run the manifold-core manifoldness check on an uploaded mesh. */
export function checkManifoldInKernel(
  vertices: Float32Array | Float64Array,
  indices: Uint32Array,
): KernelManifoldReport {
  const {exports: wasm, memory, takeResponse} = manifoldRuntime()
  let vp = 0
  let ip = 0
  let handle = 0
  try {
    vp = copyBuffer(wasm, new Uint8Array(vertices.buffer, vertices.byteOffset, vertices.byteLength))
    ip = copyBuffer(wasm, new Uint8Array(indices.buffer, indices.byteOffset, indices.byteLength))
    handle = decodeNurbsResult<number>(
      takeResponse(wasm.abi_manifold_check(vp, vertices.length, ip, indices.length, kernelVertexFormat(vertices))),
    )
    // Allocation above may grow memory. Never cache memory.buffer between calls.
    const buffer = memory.buffer
    const u32 = (slot: number) =>
      new Uint32Array(new Uint32Array(buffer, wasm.abi_array_field(handle, slot), wasm.abi_array_field(handle, slot + 1)))
    const flags = wasm.abi_array_field(handle, 15)
    return {
      boundaryEdges: u32(0),
      nonManifoldEdges: u32(2),
      orientationEdges: u32(4),
      degenerateTriangles: u32(6),
      nonManifoldVertices: u32(8),
      isolatedVertices: u32(10),
      vertexCount: wasm.abi_array_field(handle, 12),
      triangleCount: wasm.abi_array_field(handle, 13),
      componentCount: wasm.abi_array_field(handle, 14),
      isManifold: (flags & 1) !== 0,
      isManifoldWithBoundary: (flags & 2) !== 0,
    }
  } finally {
    if (handle) wasm.abi_array_free(handle)
    if (ip) wasm.abi_free(ip, indices.byteLength)
    if (vp) wasm.abi_free(vp, vertices.byteLength)
  }
}

/** Repair a mesh toward manifoldness in the kernel. `full` additionally splits
 * non-manifold edges/vertices and fills simple boundary loops. */
export function repairManifoldInKernel(
  vertices: Float32Array | Float64Array,
  indices: Uint32Array,
  epsilon: number,
  full = false,
): KernelManifoldRepair {
  const {exports: wasm, memory, takeResponse} = manifoldRuntime()
  let vp = 0
  let ip = 0
  let handle = 0
  try {
    vp = copyBuffer(wasm, new Uint8Array(vertices.buffer, vertices.byteOffset, vertices.byteLength))
    ip = copyBuffer(wasm, new Uint8Array(indices.buffer, indices.byteOffset, indices.byteLength))
    handle = decodeNurbsResult<number>(
      takeResponse(wasm.abi_manifold_repair(vp, vertices.length, ip, indices.length, kernelVertexFormat(vertices), epsilon, full ? 1 : 0)),
    )
    const buffer = memory.buffer
    const flags = wasm.abi_array_field(handle, 7)
    return {
      positions: new Float64Array(new Float64Array(buffer, wasm.abi_array_field(handle, 0), wasm.abi_array_field(handle, 1))),
      indices: new Uint32Array(new Uint32Array(buffer, wasm.abi_array_field(handle, 2), wasm.abi_array_field(handle, 3))),
      weldedVertices: wasm.abi_array_field(handle, 4),
      removedDegenerateTriangles: wasm.abi_array_field(handle, 5),
      flippedTriangles: wasm.abi_array_field(handle, 6),
      isManifold: (flags & 1) !== 0,
      isManifoldWithBoundary: (flags & 2) !== 0,
      splitVertices: wasm.abi_array_field(handle, 8),
      filledHoles: wasm.abi_array_field(handle, 9),
      filledTriangles: wasm.abi_array_field(handle, 10),
      mode: wasm.abi_array_field(handle, 11),
    }
  } finally {
    if (handle) wasm.abi_array_free(handle)
    if (ip) wasm.abi_free(ip, indices.byteLength)
    if (vp) wasm.abi_free(vp, vertices.byteLength)
  }
}

/** Per-component metrics from the manifold-core kernel (abi_manifold_metrics). */
export interface KernelManifoldComponentMetrics {
  readonly triangleCount: number
  readonly vertexCount: number
  readonly edgeCount: number
  readonly boundaryEdges: number
  readonly eulerCharacteristic: number
  /** Genus for closed components; `null` when undefined (open/non-manifold). */
  readonly genus: number | null
  readonly signedVolume: number
  readonly surfaceArea: number
}

/** Whole-mesh metrics from the manifold-core kernel (abi_manifold_metrics). */
export interface KernelManifoldMetrics {
  readonly vertexCount: number
  readonly triangleCount: number
  readonly edgeCount: number
  readonly eulerCharacteristic: number
  readonly componentCount: number
  readonly signedVolume: number
  readonly surfaceArea: number
  readonly watertight: boolean
  readonly components: KernelManifoldComponentMetrics[]
}

/** Compute geometric and topological metrics of a mesh in the kernel. */
export function metricsManifoldInKernel(
  vertices: Float32Array | Float64Array,
  indices: Uint32Array,
): KernelManifoldMetrics {
  const {exports: wasm, memory, takeResponse} = manifoldRuntime()
  let vp = 0
  let ip = 0
  let handle = 0
  try {
    vp = copyBuffer(wasm, new Uint8Array(vertices.buffer, vertices.byteOffset, vertices.byteLength))
    ip = copyBuffer(wasm, new Uint8Array(indices.buffer, indices.byteOffset, indices.byteLength))
    handle = decodeNurbsResult<number>(
      takeResponse(wasm.abi_manifold_metrics(vp, vertices.length, ip, indices.length, kernelVertexFormat(vertices))),
    )
    const buffer = memory.buffer
    const stats = new Uint32Array(new Uint32Array(buffer, wasm.abi_array_field(handle, 0), wasm.abi_array_field(handle, 1)))
    const componentFloats = new Float64Array(new Float64Array(buffer, wasm.abi_array_field(handle, 2), wasm.abi_array_field(handle, 3)))
    const floats = new Float64Array(new Float64Array(buffer, wasm.abi_array_field(handle, 4), wasm.abi_array_field(handle, 5)))
    const components: KernelManifoldComponentMetrics[] = []
    for (let c = 0; c < stats.length / 6; c++) {
      const genus = stats[c * 6 + 5]
      components.push({
        triangleCount: stats[c * 6],
        vertexCount: stats[c * 6 + 1],
        edgeCount: stats[c * 6 + 2],
        boundaryEdges: stats[c * 6 + 3],
        eulerCharacteristic: stats[c * 6 + 4] | 0,
        genus: genus === 0 ? null : genus - 1,
        signedVolume: componentFloats[c * 2],
        surfaceArea: componentFloats[c * 2 + 1],
      })
    }
    return {
      vertexCount: wasm.abi_array_field(handle, 6),
      triangleCount: wasm.abi_array_field(handle, 7),
      edgeCount: wasm.abi_array_field(handle, 8),
      eulerCharacteristic: wasm.abi_array_field(handle, 9) | 0,
      componentCount: wasm.abi_array_field(handle, 10),
      signedVolume: floats[0],
      surfaceArea: floats[1],
      watertight: (wasm.abi_array_field(handle, 11) & 1) !== 0,
      components,
    }
  } finally {
    if (handle) wasm.abi_array_free(handle)
    if (ip) wasm.abi_free(ip, indices.byteLength)
    if (vp) wasm.abi_free(vp, vertices.byteLength)
  }
}

/** Boolean operation kinds for the manifold-ops kernel (abi_manifold_boolean). */
export type ManifoldBooleanOp = 'union' | 'intersection' | 'difference' | 'xor'

const MANIFOLD_BOOLEAN_OP_TAG: Record<ManifoldBooleanOp, number> = {union: 0, intersection: 1, difference: 2, xor: 3}

export interface ManifoldBooleanOptions {
  /** Absolute mm epsilon for necessary pre/post repair; zero keeps exact welding. */
  readonly weldEpsilonMm?: number
  /** BSP tolerance relative to the longest combined input-bound side. */
  readonly relativeTolerance?: number
  /** BSP work budget per kernel step, 1..=64_000_000. */
  readonly maxWork?: number
}

export const MANIFOLD_BOOLEAN_DEFAULT_OPTIONS: Required<ManifoldBooleanOptions> = Object.freeze({
  weldEpsilonMm: 0,
  relativeTolerance: 1e-9,
  maxWork: 8_000_000,
})

/** Boolean result from the manifold-ops kernel (abi_manifold_boolean). */
export interface KernelManifoldBoolean {
  readonly positions: Float64Array<ArrayBuffer>
  readonly indices: Uint32Array<ArrayBuffer>
  /** True when an operand needed Full pre-repair before the boolean. */
  readonly operandRepaired: readonly [boolean, boolean]
  readonly kernel: {
    readonly toleranceMm: number
    readonly work: number
    readonly fragments: number
    readonly inputTriangles: readonly [number, number]
  }
  /** Strictly manifold output is the kernel contract; always true on success. */
  readonly isManifold: boolean
}

/**
 * Boolean `a OP b` with a guaranteed strictly manifold result: non-manifold
 * operands are pre-repaired (Full mode), unrepairable input and any
 * non-manifold kernel output reject with a GeometryKernelError.
 */
export function booleanManifoldInKernel(
  aVertices: Float32Array | Float64Array,
  aIndices: Uint32Array,
  bVertices: Float32Array | Float64Array,
  bIndices: Uint32Array,
  op: ManifoldBooleanOp,
  options: ManifoldBooleanOptions = {},
): KernelManifoldBoolean {
  const weldEpsilonMm = options.weldEpsilonMm ?? MANIFOLD_BOOLEAN_DEFAULT_OPTIONS.weldEpsilonMm
  const relativeTolerance = options.relativeTolerance ?? MANIFOLD_BOOLEAN_DEFAULT_OPTIONS.relativeTolerance
  const maxWork = options.maxWork ?? MANIFOLD_BOOLEAN_DEFAULT_OPTIONS.maxWork
  if (!Number.isInteger(maxWork) || maxWork < 1 || maxWork > 64_000_000) {
    throw new RangeError('maxWork must be an integer within 1..=64000000')
  }
  const {exports: wasm, memory, takeResponse} = manifoldRuntime()
  let avp = 0
  let aip = 0
  let bvp = 0
  let bip = 0
  let handle = 0
  try {
    avp = copyBuffer(wasm, new Uint8Array(aVertices.buffer, aVertices.byteOffset, aVertices.byteLength))
    aip = copyBuffer(wasm, new Uint8Array(aIndices.buffer, aIndices.byteOffset, aIndices.byteLength))
    bvp = copyBuffer(wasm, new Uint8Array(bVertices.buffer, bVertices.byteOffset, bVertices.byteLength))
    bip = copyBuffer(wasm, new Uint8Array(bIndices.buffer, bIndices.byteOffset, bIndices.byteLength))
    handle = decodeNurbsResult<number>(
      takeResponse(wasm.abi_manifold_boolean(
        avp, aVertices.length, aip, aIndices.length, kernelVertexFormat(aVertices),
        bvp, bVertices.length, bip, bIndices.length, kernelVertexFormat(bVertices),
        MANIFOLD_BOOLEAN_OP_TAG[op], weldEpsilonMm, relativeTolerance, maxWork,
      )),
    )
    const buffer = memory.buffer
    const floats = new Float64Array(new Float64Array(buffer, wasm.abi_array_field(handle, 4), wasm.abi_array_field(handle, 5)))
    return {
      positions: new Float64Array(new Float64Array(buffer, wasm.abi_array_field(handle, 0), wasm.abi_array_field(handle, 1))),
      indices: new Uint32Array(new Uint32Array(buffer, wasm.abi_array_field(handle, 2), wasm.abi_array_field(handle, 3))),
      operandRepaired: [wasm.abi_array_field(handle, 7) !== 0, wasm.abi_array_field(handle, 8) !== 0],
      kernel: {
        toleranceMm: floats[0],
        work: wasm.abi_array_field(handle, 10),
        fragments: wasm.abi_array_field(handle, 9),
        inputTriangles: [wasm.abi_array_field(handle, 11), wasm.abi_array_field(handle, 12)],
      },
      isManifold: (wasm.abi_array_field(handle, 13) & 1) !== 0,
    }
  } finally {
    if (handle) wasm.abi_array_free(handle)
    if (bip) wasm.abi_free(bip, bIndices.byteLength)
    if (bvp) wasm.abi_free(bvp, bVertices.byteLength)
    if (aip) wasm.abi_free(aip, aIndices.byteLength)
    if (avp) wasm.abi_free(avp, aVertices.byteLength)
  }
}

/** Shell pair closer than the clearance threshold (abi_manifold_diagnostics). */
export interface KernelManifoldClearancePair {
  readonly componentA: number
  readonly componentB: number
  /** Closest triangle distance; 0 when the shells touch or intersect. */
  readonly distance: number
  readonly triangleA: number
  readonly triangleB: number
}

/** Mass properties at the requested density (abi_manifold_diagnostics). */
export interface KernelManifoldMass {
  readonly volume: number
  readonly mass: number
  readonly centerOfMass: readonly [number, number, number]
  /** Row-major 3x3 inertia tensor about the center of mass, scaled by density. */
  readonly inertia: readonly number[]
}

/** Geometric diagnostics from the manifold-core kernel (abi_manifold_diagnostics). */
export interface KernelManifoldDiagnostics {
  /** Flat [i, j] intersecting triangle pairs, i < j. */
  readonly selfIntersectionPairs: Uint32Array<ArrayBuffer>
  readonly selfIntersectionCandidatePairs: number
  /** True when the pair list hit the kernel's reporting cap. */
  readonly selfIntersectionTruncated: boolean
  /** Smallest sampled wall thickness; null when no inward ray hit. */
  readonly minThickness: number | null
  readonly maxThickness: number | null
  readonly minThicknessTriangle: number | null
  readonly thicknessSamples: number
  readonly thicknessMissed: number
  /** Triangles whose thickness sample is below the `minThickness` option. */
  readonly thinTriangles: Uint32Array<ArrayBuffer>
  /** Smallest free distance in front of a face (outward ray); null when all rays escape. */
  readonly minGap: number | null
  readonly minGapTriangle: number | null
  readonly componentCount: number
  readonly clearancePairs: KernelManifoldClearancePair[]
  /** Null for open or zero-volume meshes. */
  readonly mass: KernelManifoldMass | null
}

export interface ManifoldDiagnosticsOptions {
  /** Material density (mass per unit volume), > 0. */
  readonly density?: number
  /** Wall thickness threshold for `thinTriangles`, >= 0. */
  readonly minThickness?: number
  /** Gap threshold between shells, > 0. */
  readonly clearanceThreshold?: number
  /** Work budget per pass (candidate pairs, rays), 1..=64_000_000. */
  readonly maxWork?: number
}

export const MANIFOLD_DIAGNOSTICS_DEFAULT_OPTIONS: Required<ManifoldDiagnosticsOptions> = Object.freeze({
  density: 1,
  minThickness: 0,
  clearanceThreshold: 0.2,
  maxWork: 4_000_000,
})

const finiteOrNull = (value: number): number | null => (Number.isNaN(value) ? null : value)

/**
 * Self-intersections, sampled wall thickness, clearance between shells and
 * mass properties of a mesh. A pass over `maxWork` rejects with a
 * GeometryKernelError (`GEOMETRY_RESOURCE_LIMIT`).
 */
export function diagnoseManifoldInKernel(
  vertices: Float32Array | Float64Array,
  indices: Uint32Array,
  options: ManifoldDiagnosticsOptions = {},
): KernelManifoldDiagnostics {
  const o = {...MANIFOLD_DIAGNOSTICS_DEFAULT_OPTIONS, ...options}
  if (!Number.isInteger(o.maxWork) || o.maxWork < 1 || o.maxWork > 64_000_000) {
    throw new RangeError('maxWork must be an integer within 1..=64000000')
  }
  const {exports: wasm, memory, takeResponse} = manifoldRuntime()
  let vp = 0
  let ip = 0
  let handle = 0
  try {
    vp = copyBuffer(wasm, new Uint8Array(vertices.buffer, vertices.byteOffset, vertices.byteLength))
    ip = copyBuffer(wasm, new Uint8Array(indices.buffer, indices.byteOffset, indices.byteLength))
    handle = decodeNurbsResult<number>(
      takeResponse(wasm.abi_manifold_diagnostics(
        vp, vertices.length, ip, indices.length, kernelVertexFormat(vertices),
        o.density, o.minThickness, o.clearanceThreshold, o.maxWork,
      )),
    )
    const buffer = memory.buffer
    const u32 = (slot: number) =>
      new Uint32Array(new Uint32Array(buffer, wasm.abi_array_field(handle, slot), wasm.abi_array_field(handle, slot + 1)))
    const clearance = u32(4)
    const floats = new Float64Array(new Float64Array(buffer, wasm.abi_array_field(handle, 6), wasm.abi_array_field(handle, 7)))
    const flags = wasm.abi_array_field(handle, 9)
    const minTriangle = wasm.abi_array_field(handle, 13)
    const gapTriangle = wasm.abi_array_field(handle, 14)
    const clearancePairs: KernelManifoldClearancePair[] = []
    for (let p = 0; p < clearance.length / 4; p++) {
      clearancePairs.push({
        componentA: clearance[p * 4],
        componentB: clearance[p * 4 + 1],
        distance: floats[17 + p],
        triangleA: clearance[p * 4 + 2],
        triangleB: clearance[p * 4 + 3],
      })
    }
    return {
      selfIntersectionPairs: u32(0),
      selfIntersectionCandidatePairs: wasm.abi_array_field(handle, 8),
      selfIntersectionTruncated: (flags & 1) !== 0,
      minThickness: finiteOrNull(floats[0]),
      maxThickness: finiteOrNull(floats[1]),
      minThicknessTriangle: minTriangle === 0 ? null : minTriangle - 1,
      thicknessSamples: wasm.abi_array_field(handle, 10),
      thicknessMissed: wasm.abi_array_field(handle, 11),
      thinTriangles: u32(2),
      minGap: finiteOrNull(floats[2]),
      minGapTriangle: gapTriangle === 0 ? null : gapTriangle - 1,
      componentCount: wasm.abi_array_field(handle, 12),
      clearancePairs,
      mass: (flags & 2) === 0 ? null : {
        volume: floats[3],
        mass: floats[4],
        centerOfMass: [floats[5], floats[6], floats[7]],
        inertia: Array.from(floats.subarray(8, 17)),
      },
    }
  } finally {
    if (handle) wasm.abi_array_free(handle)
    if (ip) wasm.abi_free(ip, indices.byteLength)
    if (vp) wasm.abi_free(vp, vertices.byteLength)
  }
}
