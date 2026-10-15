/** Shared GPU CSG preview tree types. */
export type CsgOp = 'union' | 'difference' | 'intersection'

export type GpuCsgPrimitiveKind = 'cube' | 'cylinder' | 'sphere' | 'polyhedron'

export interface GpuCsgPrimitive {
  id: string
  kind: GpuCsgPrimitiveKind
  /** 4x4 row-major transformation matrix */
  transform: Float32Array
  /** Inverse 4x4 transformation matrix (optional, computed on demand) */
  inverseTransform?: Float32Array
  /** RGBA color [r, g, b, a] in 0..1 range */
  color: [number, number, number, number]
  /** Interleaved vertex data [x, y, z, nx, ny, nz] (stride 6 floats = 24 bytes) */
  vertices?: Float32Array
  /** Triangle indices */
  indices?: Uint32Array
}

export interface GpuCsgTerm {
  /** Positive target base primitive: P */
  positive: GpuCsgPrimitive
  /** Subtracted cutters: C_1, C_2, ..., C_k */
  cutters: GpuCsgPrimitive[]
  /** Term boolean operation (default: 'difference') */
  operation: CsgOp
}

export interface GpuCsgTree {
  /** Disjunctive Normal Form / Sequenced terms: Scene = Term_1 U Term_2 U ... */
  terms: GpuCsgTerm[]
}

export type GpuCsgNode =
  | { type: 'primitive'; primitive: GpuCsgPrimitive }
  | { type: 'operation'; op: CsgOp; children: GpuCsgNode[]; transform?: Float32Array; color?: [number, number, number, number] }

/* ── Matrix Math Utilities (Pure JS/TS, zero-dependency) ── */
