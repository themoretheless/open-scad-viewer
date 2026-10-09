/**
 * GPU OpenCSG Preview — WebGPU Depth/Stencil Boolean Evaluation.
 *
 * Implements real-time Goldfeather / SCS (Sequenced Convex Subtractions)
 * evaluation on WebGPU hardware. Avoids CPU B-Rep polygonal re-tessellation
 * bottlenecks during interactive preview and parameter editing.
 */

import { compileOpenSCAD, type Statement, type CallNode, type Expr } from './openscadCompiler'

/* ── CSG Types & Data Structures ──────────────────────── */

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

export function identityMat4(): Float32Array {
  const m = new Float32Array(16)
  m[0] = m[5] = m[10] = m[15] = 1
  return m
}

export function multiplyMat4(a: Float32Array, b: Float32Array): Float32Array {
  const out = new Float32Array(16)
  for (let r = 0; r < 4; r++) {
    const r4 = r * 4
    for (let c = 0; c < 4; c++) {
      out[r4 + c] =
        a[r4 + 0] * b[c] +
        a[r4 + 1] * b[4 + c] +
        a[r4 + 2] * b[8 + c] +
        a[r4 + 3] * b[12 + c]
    }
  }
  return out
}

export function translationMat4(tx: number, ty: number, tz: number): Float32Array {
  const m = identityMat4()
  m[3] = tx
  m[7] = ty
  m[11] = tz
  return m
}

export function scaleMat4(sx: number, sy: number, sz: number): Float32Array {
  const m = identityMat4()
  m[0] = sx
  m[5] = sy
  m[10] = sz
  return m
}

export function rotationXMat4(rad: number): Float32Array {
  const c = Math.cos(rad)
  const s = Math.sin(rad)
  const m = identityMat4()
  m[5] = c
  m[6] = -s
  m[9] = s
  m[10] = c
  return m
}

export function rotationYMat4(rad: number): Float32Array {
  const c = Math.cos(rad)
  const s = Math.sin(rad)
  const m = identityMat4()
  m[0] = c
  m[2] = s
  m[8] = -s
  m[10] = c
  return m
}

export function rotationZMat4(rad: number): Float32Array {
  const c = Math.cos(rad)
  const s = Math.sin(rad)
  const m = identityMat4()
  m[0] = c
  m[1] = -s
  m[4] = s
  m[5] = c
  return m
}

export function rotationEulerMat4(rxDeg: number, ryDeg: number, rzDeg: number): Float32Array {
  const deg2rad = Math.PI / 180
  const rx = rotationXMat4(rxDeg * deg2rad)
  const ry = rotationYMat4(ryDeg * deg2rad)
  const rz = rotationZMat4(rzDeg * deg2rad)
  // Euler Z * Y * X
  return multiplyMat4(rz, multiplyMat4(ry, rx))
}

export function normalMatrixFromModel(model: Float32Array): Float32Array {
  // Transpose of inverse of 3x3 upper-left block, padded to 4x4
  const n = identityMat4()
  const m00 = model[0], m01 = model[1], m02 = model[2]
  const m10 = model[4], m11 = model[5], m12 = model[6]
  const m20 = model[8], m21 = model[9], m22 = model[10]

  const c00 = m11 * m22 - m12 * m21
  const c01 = -(m10 * m22 - m12 * m20)
  const c02 = m10 * m21 - m11 * m20

  const det = m00 * c00 + m01 * c01 + m02 * c02
  if (Math.abs(det) < 1e-8) return n

  const invDet = 1 / det
  n[0] = c00 * invDet
  n[1] = (-(m01 * m22 - m02 * m21)) * invDet
  n[2] = (m01 * m12 - m02 * m11) * invDet

  n[4] = c01 * invDet
  n[5] = (m00 * m22 - m02 * m20) * invDet
  n[6] = (-(m00 * m12 - m02 * m10)) * invDet

  n[8] = c02 * invDet
  n[9] = (-(m00 * m21 - m01 * m20)) * invDet
  n[10] = (m00 * m11 - m01 * m10) * invDet
  return n
}

/* ── Primitive Geometry Generation ────────────────────── */

export interface GeometryData {
  /** Interleaved Float32Array [x, y, z, nx, ny, nz] */
  vertices: Float32Array
  /** Triangles index buffer */
  indices: Uint32Array
}

/**
 * Generates a unit cube [-0.5, 0.5]^3 or [0, 1]^3.
 * 24 vertices with sharp perpendicular face normals, 36 indices.
 */
export function generateUnitCube(center = true): GeometryData {
  const min = center ? -0.5 : 0
  const max = center ? 0.5 : 1

  // 6 faces * 4 vertices = 24 vertices
  // Format: [px, py, pz, nx, ny, nz]
  const v = [
    // Front face (Z+)
    min, min, max,  0, 0, 1,
    max, min, max,  0, 0, 1,
    max, max, max,  0, 0, 1,
    min, max, max,  0, 0, 1,
    // Back face (Z-)
    max, min, min,  0, 0, -1,
    min, min, min,  0, 0, -1,
    min, max, min,  0, 0, -1,
    max, max, min,  0, 0, -1,
    // Right face (X+)
    max, min, min,  1, 0, 0,
    max, max, min,  1, 0, 0,
    max, max, max,  1, 0, 0,
    max, min, max,  1, 0, 0,
    // Left face (X-)
    min, min, max, -1, 0, 0,
    min, max, max, -1, 0, 0,
    min, max, min, -1, 0, 0,
    min, min, min, -1, 0, 0,
    // Top face (Y+)
    min, max, max,  0, 1, 0,
    max, max, max,  0, 1, 0,
    max, max, min,  0, 1, 0,
    min, max, min,  0, 1, 0,
    // Bottom face (Y-)
    min, min, min,  0, -1, 0,
    max, min, min,  0, -1, 0,
    max, min, max,  0, -1, 0,
    min, min, max,  0, -1, 0,
  ]

  const indices = new Uint32Array(36)
  for (let f = 0; f < 6; f++) {
    const base = f * 4
    const i = f * 6
    indices[i] = base
    indices[i + 1] = base + 1
    indices[i + 2] = base + 2
    indices[i + 3] = base
    indices[i + 4] = base + 2
    indices[i + 5] = base + 3
  }

  return { vertices: new Float32Array(v), indices }
}

/**
 * Generates a unit cylinder (radius = 1, height = 1, along Z-axis).
 */
export function generateUnitCylinder(segments = 32, center = true): GeometryData {
  const zMin = center ? -0.5 : 0
  const zMax = center ? 0.5 : 1

  const vertices: number[] = []
  const indices: number[] = []

  // Top cap center
  const topCenterIdx = 0
  vertices.push(0, 0, zMax,  0, 0, 1)
  // Top cap perimeter
  const topStartIdx = 1
  for (let i = 0; i < segments; i++) {
    const theta = (i / segments) * Math.PI * 2
    const x = Math.cos(theta)
    const y = Math.sin(theta)
    vertices.push(x, y, zMax,  0, 0, 1)
  }
  for (let i = 0; i < segments; i++) {
    const next = (i + 1) % segments
    indices.push(topCenterIdx, topStartIdx + i, topStartIdx + next)
  }

  // Bottom cap center
  const bottomCenterIdx = vertices.length / 6
  vertices.push(0, 0, zMin,  0, 0, -1)
  // Bottom cap perimeter
  const bottomStartIdx = bottomCenterIdx + 1
  for (let i = 0; i < segments; i++) {
    const theta = (i / segments) * Math.PI * 2
    const x = Math.cos(theta)
    const y = Math.sin(theta)
    vertices.push(x, y, zMin,  0, 0, -1)
  }
  for (let i = 0; i < segments; i++) {
    const next = (i + 1) % segments
    indices.push(bottomCenterIdx, bottomStartIdx + next, bottomStartIdx + i)
  }

  // Side wall (quads with radial normals)
  const sideStartIdx = vertices.length / 6
  for (let i = 0; i <= segments; i++) {
    const theta = (i / segments) * Math.PI * 2
    const x = Math.cos(theta)
    const y = Math.sin(theta)
    // Top vertex
    vertices.push(x, y, zMax,  x, y, 0)
    // Bottom vertex
    vertices.push(x, y, zMin,  x, y, 0)
  }
  for (let i = 0; i < segments; i++) {
    const iTop = sideStartIdx + i * 2
    const iBot = iTop + 1
    const nTop = sideStartIdx + (i + 1) * 2
    const nBot = nTop + 1
    indices.push(iTop, nTop, iBot)
    indices.push(iBot, nTop, nBot)
  }

  return { vertices: new Float32Array(vertices), indices: new Uint32Array(indices) }
}

/**
 * Generates a unit UV sphere (radius = 1).
 */
export function generateUnitSphere(segments = 32, rings = 16): GeometryData {
  const vertices: number[] = []
  const indices: number[] = []

  for (let r = 0; r <= rings; r++) {
    const phi = (r / rings) * Math.PI
    const sinPhi = Math.sin(phi)
    const cosPhi = Math.cos(phi)

    for (let s = 0; s <= segments; s++) {
      const theta = (s / segments) * Math.PI * 2
      const x = Math.cos(theta) * sinPhi
      const y = Math.sin(theta) * sinPhi
      const z = cosPhi
      // Normalized position doubles as normal
      vertices.push(x, y, z,  x, y, z)
    }
  }

  for (let r = 0; r < rings; r++) {
    for (let s = 0; s < segments; s++) {
      const first = r * (segments + 1) + s
      const second = first + segments + 1
      indices.push(first, second, first + 1)
      indices.push(second, second + 1, first + 1)
    }
  }

  return { vertices: new Float32Array(vertices), indices: new Uint32Array(indices) }
}

/* ── CSG Tree Flattening & Normalization ──────────────── */

let nextCsgId = 1
export function resetCsgIdCounter() {
  nextCsgId = 1
}

export function createCsgPrimitive(
  kind: GpuCsgPrimitiveKind,
  transform: Float32Array = identityMat4(),
  color: [number, number, number, number] = [0.85, 0.7, 0.2, 1.0],
  customGeometry?: GeometryData,
): GpuCsgPrimitive {
  return {
    id: `csg_prim_${nextCsgId++}`,
    kind,
    transform,
    color,
    vertices: customGeometry?.vertices,
    indices: customGeometry?.indices,
  }
}

/**
 * Recursively flattens an arbitrary CSG tree into Sequenced Convex Subtraction (SCS) terms:
 * Scene = Term_1 U Term_2 U ...
 * where each Term = P \ (C_1 U C_2 U ... U C_k).
 */
export function flattenCsgNode(node: GpuCsgNode, parentTransform: Float32Array = identityMat4()): GpuCsgTerm[] {
  if (node.type === 'primitive') {
    const combinedTransform = multiplyMat4(parentTransform, node.primitive.transform)
    return [{
      positive: { ...node.primitive, transform: combinedTransform },
      cutters: [],
      operation: 'difference',
    }]
  }

  const currentTransform = node.transform ? multiplyMat4(parentTransform, node.transform) : parentTransform

  if (node.op === 'union') {
    const terms: GpuCsgTerm[] = []
    for (const child of node.children) {
      terms.push(...flattenCsgNode(child, currentTransform))
    }
    return terms
  }

  if (node.op === 'difference') {
    if (node.children.length === 0) return []
    // First child is positive base
    const baseTerms = flattenCsgNode(node.children[0], currentTransform)
    // Subsequent children are cutters
    const cutterPrimitives: GpuCsgPrimitive[] = []
    for (let i = 1; i < node.children.length; i++) {
      const cutterTerms = flattenCsgNode(node.children[i], currentTransform)
      for (const ct of cutterTerms) {
        cutterPrimitives.push(ct.positive)
      }
    }
    // Each base term receives the cutters
    return baseTerms.map(bt => ({
      positive: bt.positive,
      cutters: [...bt.cutters, ...cutterPrimitives],
      operation: 'difference',
    }))
  }

  if (node.op === 'intersection') {
    if (node.children.length === 0) return []
    if (node.children.length === 1) return flattenCsgNode(node.children[0], currentTransform)

    // In SCS / Goldfeather, intersection A ∩ B is rendered as A clipped to B (and B clipped to A)
    const termsA = flattenCsgNode(node.children[0], currentTransform)
    const termsB = flattenCsgNode(node.children[1], currentTransform)

    const result: GpuCsgTerm[] = []
    for (const a of termsA) {
      for (const b of termsB) {
        result.push({
          positive: a.positive,
          cutters: [b.positive],
          operation: 'intersection',
        })
      }
    }
    return result
  }

  return []
}

/* ── OpenSCAD AST to GPU CSG Tree Converter ──────────── */

function evalScalar(expr?: Expr): number | undefined {
  if (!expr) return undefined
  if (expr.kind === 'literal' && typeof expr.value === 'number') return expr.value
  if (expr.kind === 'unary' && expr.op === 14 /* Minus */) {
    const inner = evalScalar(expr.value)
    return inner !== undefined ? -inner : undefined
  }
  return undefined
}

function evalVec3(expr?: Expr): [number, number, number] | undefined {
  if (!expr) return undefined
  if (expr.kind === 'vector' && expr.items.length >= 3) {
    const x = evalScalar(expr.items[0])
    const y = evalScalar(expr.items[1])
    const z = evalScalar(expr.items[2])
    if (x !== undefined && y !== undefined && z !== undefined) return [x, y, z]
  }
  return undefined
}

function evalBool(expr?: Expr): boolean | undefined {
  if (!expr) return undefined
  if (expr.kind === 'literal' && typeof expr.value === 'boolean') return expr.value
  return undefined
}

function getCallArg(call: CallNode, name: string, posIndex?: number): Expr | undefined {
  if (call.args[name]) return call.args[name]
  if (posIndex !== undefined) {
    if (call.args[`_${posIndex}`]) return call.args[`_${posIndex}`]
    if (call.args[`${posIndex}`]) return call.args[`${posIndex}`]
    if (call.callArguments && call.callArguments[posIndex]) {
      return call.callArguments[posIndex].value
    }
  }
  return undefined
}

/**
 * Converts OpenSCAD parsed AST statements into a hierarchical GpuCsgNode.
 */
export function astStatementToGpuCsgNode(stmt: Statement): GpuCsgNode | null {
  if (stmt.type !== 'call') return null
  const call = stmt as CallNode
  const name = call.name.toLowerCase()

  // 1. Primitive: cube
  if (name === 'cube') {
    let size: [number, number, number] = [1, 1, 1]
    const sizeExpr = getCallArg(call, 'size', 0)
    const vecSize = evalVec3(sizeExpr)
    const numSize = evalScalar(sizeExpr)
    if (vecSize) size = vecSize
    else if (numSize !== undefined) size = [numSize, numSize, numSize]

    const center = evalBool(getCallArg(call, 'center', 1)) ?? false
    // Scale unit cube to dimensions
    const scale = scaleMat4(size[0], size[1], size[2])
    const translate = center ? identityMat4() : translationMat4(size[0] / 2, size[1] / 2, size[2] / 2)
    const transform = multiplyMat4(translate, scale)

    return {
      type: 'primitive',
      primitive: createCsgPrimitive('cube', transform),
    }
  }

  // 2. Primitive: sphere
  if (name === 'sphere') {
    let r = 1
    const rVal = evalScalar(getCallArg(call, 'r', 0))
    const dVal = evalScalar(getCallArg(call, 'd'))
    if (rVal !== undefined) r = rVal
    else if (dVal !== undefined) r = dVal / 2

    const scale = scaleMat4(r, r, r)
    return {
      type: 'primitive',
      primitive: createCsgPrimitive('sphere', scale),
    }
  }

  // 3. Primitive: cylinder
  if (name === 'cylinder') {
    const h = evalScalar(getCallArg(call, 'h', 0)) ?? 1
    const r1 = evalScalar(getCallArg(call, 'r1')) ?? evalScalar(getCallArg(call, 'r', 1)) ?? 1
    const r2 = evalScalar(getCallArg(call, 'r2')) ?? r1
    const center = evalBool(getCallArg(call, 'center', 2)) ?? false

    const rAvg = (r1 + r2) / 2
    const scale = scaleMat4(rAvg, rAvg, h)
    const translate = center ? identityMat4() : translationMat4(0, 0, h / 2)
    const transform = multiplyMat4(translate, scale)

    return {
      type: 'primitive',
      primitive: createCsgPrimitive('cylinder', transform),
    }
  }

  // 4. Transforms
  if (name === 'translate') {
    const v = evalVec3(getCallArg(call, 'v', 0)) ?? [0, 0, 0]
    const tMat = translationMat4(v[0], v[1], v[2])
    const children = call.children.map(astStatementToGpuCsgNode).filter((c): c is GpuCsgNode => c !== null)
    return {
      type: 'operation',
      op: 'union',
      transform: tMat,
      children,
    }
  }

  if (name === 'scale') {
    const v = evalVec3(getCallArg(call, 'v', 0)) ?? [1, 1, 1]
    const sMat = scaleMat4(v[0], v[1], v[2])
    const children = call.children.map(astStatementToGpuCsgNode).filter((c): c is GpuCsgNode => c !== null)
    return {
      type: 'operation',
      op: 'union',
      transform: sMat,
      children,
    }
  }

  if (name === 'rotate') {
    const a = evalVec3(getCallArg(call, 'a', 0)) ?? [0, 0, 0]
    const rMat = rotationEulerMat4(a[0], a[1], a[2])
    const children = call.children.map(astStatementToGpuCsgNode).filter((c): c is GpuCsgNode => c !== null)
    return {
      type: 'operation',
      op: 'union',
      transform: rMat,
      children,
    }
  }

  // 5. CSG Operations
  if (name === 'union' || name === 'difference' || name === 'intersection') {
    const children = call.children.map(astStatementToGpuCsgNode).filter((c): c is GpuCsgNode => c !== null)
    return {
      type: 'operation',
      op: name as CsgOp,
      children,
    }
  }

  return null
}

/**
 * Top-level compiler that produces a normalized GpuCsgTree from statements.
 */
export function compileGpuCsgTree(statements: readonly Statement[]): GpuCsgTree {
  resetCsgIdCounter()
  const rootNodes: GpuCsgNode[] = []
  for (const stmt of statements) {
    const node = astStatementToGpuCsgNode(stmt)
    if (node) rootNodes.push(node)
  }

  const terms: GpuCsgTerm[] = []
  for (const node of rootNodes) {
    terms.push(...flattenCsgNode(node))
  }

  return { terms }
}

/**
 * Parses and compiles OpenSCAD source text into a GpuCsgTree for real-time GPU CSG preview.
 */
export function parseOpenScadCsgTree(source: string): GpuCsgTree | null {
  try {
    const statements = compileOpenSCAD(source)
    return compileGpuCsgTree(statements)
  } catch {
    return null
  }
}

/* ── WebGPU WGSL Shaders ──────────────────────────────── */

export const CSG_COMMON_WGSL = /* wgsl */`
struct CsgScene {
  vp: mat4x4f,
  eye: vec4f,
  light: vec4f,
  ambient: vec4f,
  cutTint: vec4f,
};

struct CsgObj {
  model: mat4x4f,
  nmat: mat4x4f,
  color: vec4f,
  params: vec4f, // x: isCutSurface (0 or 1), y: opacity
};

@group(0) @binding(0) var<uniform> sc: CsgScene;
@group(1) @binding(0) var<uniform> ob: CsgObj;
`

export const CSG_DEPTH_WGSL = /* wgsl */`
${CSG_COMMON_WGSL}

struct VDepth {
  @builtin(position) p: vec4f,
};

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f) -> VDepth {
  let wp = (ob.model * vec4f(pos, 1.0)).xyz;
  return VDepth(sc.vp * vec4f(wp, 1.0));
}

@fragment fn fs() {
  // Empty fragment stage: updates depth and/or stencil buffer only
}
`

export const CSG_SHADE_WGSL = /* wgsl */`
${CSG_COMMON_WGSL}

struct VShade {
  @builtin(position) p: vec4f,
  @location(0) n: vec3f,
  @location(1) w: vec3f,
};

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f) -> VShade {
  let wp = (ob.model * vec4f(pos, 1.0)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm, 0.0)).xyz);
  return VShade(sc.vp * vec4f(wp, 1.0), wn, wp);
}

@fragment fn fs(v: VShade) -> @location(0) vec4f {
  let N = normalize(v.n);
  let L = normalize(sc.light.xyz);
  let V = normalize(sc.eye.xyz - v.w);
  let H = normalize(L + V);
  let diff = max(dot(N, L), 0.0);
  let spec = pow(max(dot(N, H), 0.0), 32.0) * 0.25;

  let base = select(ob.color.rgb, mix(ob.color.rgb, sc.cutTint.rgb, 0.6), ob.params.x > 0.5);
  let lit = base * (sc.ambient.rgb + diff * 0.75) + vec3f(spec);
  return vec4f(lit, ob.color.a);
}
`

/* ── WebGPU Pipeline Manager ──────────────────────────── */

export interface CsgPipelines {
  sceneBGL: GPUBindGroupLayout
  objBGL: GPUBindGroupLayout
  pipelineLayout: GPUPipelineLayout
  /** Pass 1: Renders base primitive into depth buffer */
  depthPipeline: GPURenderPipeline
  /** Pass 2: Evaluates cutter parity in stencil buffer */
  parityPipeline: GPURenderPipeline
  /** Pass 3: Shades base primitive where stencil == 0 */
  shadeBasePipeline: GPURenderPipeline
  /** Pass 4: Shades cut surfaces (back faces of cutters) */
  shadeCutPipeline: GPURenderPipeline
  /** Pass 3 (Intersection): Shades primitive where stencil > 0 */
  shadeIntersectPipeline: GPURenderPipeline
}

export class GpuCsgPipelineManager {
  private pipelines: CsgPipelines | null = null
  private colorFormat: GPUTextureFormat = 'bgra8unorm'
  private depthFormat: GPUTextureFormat = 'depth24plus-stencil8'

  constructor(colorFormat: GPUTextureFormat = 'bgra8unorm') {
    this.colorFormat = colorFormat
  }

  getPipelines(dev: GPUDevice): CsgPipelines {
    if (this.pipelines) return this.pipelines

    const sceneBGL = dev.createBindGroupLayout({
      entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
      ],
    })

    const objBGL = dev.createBindGroupLayout({
      entries: [
        { binding: 0, visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT, buffer: { type: 'uniform' } },
      ],
    })

    const pipelineLayout = dev.createPipelineLayout({
      bindGroupLayouts: [sceneBGL, objBGL],
    })

    const depthModule = dev.createShaderModule({ code: CSG_DEPTH_WGSL })
    const shadeModule = dev.createShaderModule({ code: CSG_SHADE_WGSL })

    const vertexBuffers: GPUVertexBufferLayout[] = [{
      arrayStride: 24, // 6 floats * 4 bytes
      attributes: [
        { shaderLocation: 0, offset: 0, format: 'float32x3' },  // position
        { shaderLocation: 1, offset: 12, format: 'float32x3' }, // normal
      ],
    }]

    // 1. Pass 1: Base Depth Pipeline
    const depthPipeline = dev.createRenderPipeline({
      layout: pipelineLayout,
      vertex: { module: depthModule, entryPoint: 'vs', buffers: vertexBuffers },
      fragment: { module: depthModule, entryPoint: 'fs', targets: [] },
      primitive: { topology: 'triangle-list', cullMode: 'back' },
      depthStencil: {
        format: this.depthFormat,
        depthWriteEnabled: true,
        depthCompare: 'less',
        stencilFront: { passOp: 'keep' },
        stencilBack: { passOp: 'keep' },
      },
    })

    // 2. Pass 2: Cutter Parity Pipeline (Counts Jordan winding in stencil)
    const parityPipeline = dev.createRenderPipeline({
      layout: pipelineLayout,
      vertex: { module: depthModule, entryPoint: 'vs', buffers: vertexBuffers },
      fragment: { module: depthModule, entryPoint: 'fs', targets: [] },
      primitive: { topology: 'triangle-list', cullMode: 'none' }, // Front and back faces
      depthStencil: {
        format: this.depthFormat,
        depthWriteEnabled: false,
        depthCompare: 'less',
        stencilFront: {
          compare: 'always',
          passOp: 'increment-wrap',
          failOp: 'keep',
          depthFailOp: 'keep',
        },
        stencilBack: {
          compare: 'always',
          passOp: 'decrement-wrap',
          failOp: 'keep',
          depthFailOp: 'keep',
        },
      },
    })

    // 3. Pass 3: Base Shade Pipeline (where stencil == 0)
    const shadeBasePipeline = dev.createRenderPipeline({
      layout: pipelineLayout,
      vertex: { module: shadeModule, entryPoint: 'vs', buffers: vertexBuffers },
      fragment: { module: shadeModule, entryPoint: 'fs', targets: [{ format: this.colorFormat }] },
      primitive: { topology: 'triangle-list', cullMode: 'back' },
      depthStencil: {
        format: this.depthFormat,
        depthWriteEnabled: true,
        depthCompare: 'equal',
        stencilFront: {
          compare: 'equal',
          passOp: 'keep',
        },
        stencilBack: {
          compare: 'equal',
          passOp: 'keep',
        },
      },
    })

    // 4. Pass 4: Cut Shade Pipeline (back faces of cutters)
    const shadeCutPipeline = dev.createRenderPipeline({
      layout: pipelineLayout,
      vertex: { module: shadeModule, entryPoint: 'vs', buffers: vertexBuffers },
      fragment: { module: shadeModule, entryPoint: 'fs', targets: [{ format: this.colorFormat }] },
      primitive: { topology: 'triangle-list', cullMode: 'front' }, // Inside cutter faces
      depthStencil: {
        format: this.depthFormat,
        depthWriteEnabled: true,
        depthCompare: 'less',
        stencilFront: { passOp: 'keep' },
        stencilBack: { passOp: 'keep' },
      },
    })

    // 5. Pass 3 (Intersection): Base Shade where stencil > 0
    const shadeIntersectPipeline = dev.createRenderPipeline({
      layout: pipelineLayout,
      vertex: { module: shadeModule, entryPoint: 'vs', buffers: vertexBuffers },
      fragment: { module: shadeModule, entryPoint: 'fs', targets: [{ format: this.colorFormat }] },
      primitive: { topology: 'triangle-list', cullMode: 'back' },
      depthStencil: {
        format: this.depthFormat,
        depthWriteEnabled: true,
        depthCompare: 'equal',
        stencilFront: {
          compare: 'not-equal', // stencil != 0 (i.e. > 0)
          passOp: 'keep',
        },
        stencilBack: {
          compare: 'not-equal',
          passOp: 'keep',
        },
      },
    })

    this.pipelines = {
      sceneBGL,
      objBGL,
      pipelineLayout,
      depthPipeline,
      parityPipeline,
      shadeBasePipeline,
      shadeCutPipeline,
      shadeIntersectPipeline,
    }
    return this.pipelines
  }
}

/* ── WebGPU Geometry Cache ────────────────────────────── */

export interface CachedGpuMesh {
  vb: GPUBuffer
  ib: GPUBuffer
  indexCount: number
}

export class GpuCsgGeometryCache {
  private cube: CachedGpuMesh | null = null
  private cylinder: CachedGpuMesh | null = null
  private sphere: CachedGpuMesh | null = null
  private customMeshes = new Map<string, CachedGpuMesh>()

  private uploadMesh(dev: GPUDevice, data: GeometryData): CachedGpuMesh {
    const vb = dev.createBuffer({
      size: data.vertices.byteLength,
      usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
      mappedAtCreation: true,
    })
    new Float32Array(vb.getMappedRange()).set(data.vertices)
    vb.unmap()

    const ib = dev.createBuffer({
      size: data.indices.byteLength,
      usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST,
      mappedAtCreation: true,
    })
    new Uint32Array(ib.getMappedRange()).set(data.indices)
    ib.unmap()

    return { vb, ib, indexCount: data.indices.length }
  }

  getPrimitiveMesh(dev: GPUDevice, prim: GpuCsgPrimitive): CachedGpuMesh {
    if (prim.kind === 'cube') {
      if (!this.cube) this.cube = this.uploadMesh(dev, generateUnitCube(true))
      return this.cube
    }
    if (prim.kind === 'cylinder') {
      if (!this.cylinder) this.cylinder = this.uploadMesh(dev, generateUnitCylinder(32, true))
      return this.cylinder
    }
    if (prim.kind === 'sphere') {
      if (!this.sphere) this.sphere = this.uploadMesh(dev, generateUnitSphere(32, 16))
      return this.sphere
    }

    // Polyhedron / custom mesh
    let cached = this.customMeshes.get(prim.id)
    if (!cached && prim.vertices && prim.indices) {
      cached = this.uploadMesh(dev, { vertices: prim.vertices, indices: prim.indices })
      this.customMeshes.set(prim.id, cached)
    }
    if (cached) return cached

    // Fallback to cube
    if (!this.cube) this.cube = this.uploadMesh(dev, generateUnitCube(true))
    return this.cube
  }

  destroy() {
    this.cube?.vb.destroy()
    this.cube?.ib.destroy()
    this.cylinder?.vb.destroy()
    this.cylinder?.ib.destroy()
    this.sphere?.vb.destroy()
    this.sphere?.ib.destroy()
    for (const mesh of this.customMeshes.values()) {
      mesh.vb.destroy()
      mesh.ib.destroy()
    }
    this.customMeshes.clear()
    this.cube = null
    this.cylinder = null
    this.sphere = null
  }
}

/* ── WebGPU CSG Preview Renderer ──────────────────────── */

export interface RenderCsgSceneOptions {
  viewProjection: Float32Array
  eyePosition: [number, number, number]
  lightDirection?: [number, number, number]
  ambientColor?: [number, number, number]
  cutTintColor?: [number, number, number]
}

export class GpuCsgRenderer {
  readonly pipelineManager: GpuCsgPipelineManager
  readonly geometryCache = new GpuCsgGeometryCache()
  private sceneUB: GPUBuffer | null = null
  private sceneBG: GPUBindGroup | null = null

  constructor(colorFormat: GPUTextureFormat = 'bgra8unorm') {
    this.pipelineManager = new GpuCsgPipelineManager(colorFormat)
  }

  private ensureSceneUniforms(dev: GPUDevice, pipelines: CsgPipelines, opts: RenderCsgSceneOptions): GPUBindGroup {
    if (!this.sceneUB) {
      this.sceneUB = dev.createBuffer({
        size: 256, // Padded to uniform alignment
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
      })
      this.sceneBG = dev.createBindGroup({
        layout: pipelines.sceneBGL,
        entries: [{ binding: 0, resource: { buffer: this.sceneUB } }],
      })
    }

    const data = new Float32Array(64)
    // viewProjection: mat4x4f (16 floats)
    data.set(opts.viewProjection, 0)
    // eyePosition: vec4f (4 floats)
    data[16] = opts.eyePosition[0]
    data[17] = opts.eyePosition[1]
    data[18] = opts.eyePosition[2]
    data[19] = 1.0
    // lightDirection: vec4f
    const light = opts.lightDirection ?? [0.5, 0.8, 1.0]
    data[20] = light[0]
    data[21] = light[1]
    data[22] = light[2]
    data[23] = 0.0
    // ambientColor: vec4f
    const ambient = opts.ambientColor ?? [0.25, 0.25, 0.25]
    data[24] = ambient[0]
    data[25] = ambient[1]
    data[26] = ambient[2]
    data[27] = 1.0
    // cutTintColor: vec4f
    const cutTint = opts.cutTintColor ?? [0.95, 0.45, 0.15]
    data[28] = cutTint[0]
    data[29] = cutTint[1]
    data[30] = cutTint[2]
    data[31] = 1.0

    dev.queue.writeBuffer(this.sceneUB, 0, data)
    return this.sceneBG!
  }

  private createObjectBindGroup(
    dev: GPUDevice,
    pipelines: CsgPipelines,
    prim: GpuCsgPrimitive,
    isCutSurface: boolean,
  ): GPUBindGroup {
    const obUB = dev.createBuffer({
      size: 256,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    })

    const data = new Float32Array(64)
    // model: mat4x4f (16 floats)
    data.set(prim.transform, 0)
    // normalMatrix: mat4x4f (16 floats)
    data.set(normalMatrixFromModel(prim.transform), 16)
    // color: vec4f
    data[32] = prim.color[0]
    data[33] = prim.color[1]
    data[34] = prim.color[2]
    data[35] = prim.color[3]
    // params: vec4f
    data[36] = isCutSurface ? 1.0 : 0.0
    data[37] = prim.color[3]
    data[38] = 0.0
    data[39] = 0.0

    dev.queue.writeBuffer(obUB, 0, data)

    return dev.createBindGroup({
      layout: pipelines.objBGL,
      entries: [{ binding: 0, resource: { buffer: obUB } }],
    })
  }

  /**
   * Renders a GpuCsgTree into a render pass using the Goldfeather / SCS stencil algorithm.
   */
  renderCsg(
    dev: GPUDevice,
    encoder: GPUCommandEncoder,
    colorView: GPUTextureView,
    depthStencilView: GPUTextureView,
    tree: GpuCsgTree,
    opts: RenderCsgSceneOptions,
  ) {
    if (tree.terms.length === 0) return

    const pipelines = this.pipelineManager.getPipelines(dev)
    const sceneBG = this.ensureSceneUniforms(dev, pipelines, opts)

    for (let termIdx = 0; termIdx < tree.terms.length; termIdx++) {
      const term = tree.terms[termIdx]
      const isFirstTerm = termIdx === 0
      const baseMesh = this.geometryCache.getPrimitiveMesh(dev, term.positive)
      const baseBG = this.createObjectBindGroup(dev, pipelines, term.positive, false)

      // Fast path: pure union primitive without cutters
      if (term.cutters.length === 0) {
        const pass = encoder.beginRenderPass({
          colorAttachments: [{
            view: colorView,
            loadOp: isFirstTerm ? 'clear' : 'load',
            storeOp: 'store',
          }],
          depthStencilAttachment: {
            view: depthStencilView,
            depthClearValue: 1.0,
            depthLoadOp: isFirstTerm ? 'clear' : 'load',
            depthStoreOp: 'store',
            stencilClearValue: 0,
            stencilLoadOp: 'clear',
            stencilStoreOp: 'store',
          },
        })

        pass.setPipeline(pipelines.shadeBasePipeline)
        pass.setBindGroup(0, sceneBG)
        pass.setBindGroup(1, baseBG)
        pass.setVertexBuffer(0, baseMesh.vb)
        pass.setIndexBuffer(baseMesh.ib, 'uint32')
        pass.setStencilReference(0)
        pass.drawIndexed(baseMesh.indexCount)
        pass.end()
        continue
      }

      // Boolean Operation: Goldfeather / SCS Multi-Pass Evaluation

      // Pass 1: Base Depth
      const depthPass = encoder.beginRenderPass({
        colorAttachments: [],
        depthStencilAttachment: {
          view: depthStencilView,
          depthClearValue: 1.0,
          depthLoadOp: isFirstTerm ? 'clear' : 'load',
          depthStoreOp: 'store',
          stencilClearValue: 0,
          stencilLoadOp: 'clear',
          stencilStoreOp: 'store',
        },
      })
      depthPass.setPipeline(pipelines.depthPipeline)
      depthPass.setBindGroup(0, sceneBG)
      depthPass.setBindGroup(1, baseBG)
      depthPass.setVertexBuffer(0, baseMesh.vb)
      depthPass.setIndexBuffer(baseMesh.ib, 'uint32')
      depthPass.drawIndexed(baseMesh.indexCount)
      depthPass.end()

      // Pass 2: Cutter Parity Counting (depth write OFF, stencil parity wrap)
      const parityPass = encoder.beginRenderPass({
        colorAttachments: [],
        depthStencilAttachment: {
          view: depthStencilView,
          depthLoadOp: 'load',
          depthStoreOp: 'store',
          stencilLoadOp: 'load',
          stencilStoreOp: 'store',
        },
      })
      parityPass.setPipeline(pipelines.parityPipeline)
      parityPass.setBindGroup(0, sceneBG)
      for (const cutter of term.cutters) {
        const cutterMesh = this.geometryCache.getPrimitiveMesh(dev, cutter)
        const cutterBG = this.createObjectBindGroup(dev, pipelines, cutter, true)
        parityPass.setBindGroup(1, cutterBG)
        parityPass.setVertexBuffer(0, cutterMesh.vb)
        parityPass.setIndexBuffer(cutterMesh.ib, 'uint32')
        parityPass.drawIndexed(cutterMesh.indexCount)
      }
      parityPass.end()

      // Pass 3: Shade Visible Base Surface
      const shadePass = encoder.beginRenderPass({
        colorAttachments: [{
          view: colorView,
          loadOp: isFirstTerm ? 'clear' : 'load',
          storeOp: 'store',
        }],
        depthStencilAttachment: {
          view: depthStencilView,
          depthLoadOp: 'load',
          depthStoreOp: 'store',
          stencilLoadOp: 'load',
          stencilStoreOp: 'store',
        },
      })
      const isIntersection = term.operation === 'intersection'
      shadePass.setPipeline(isIntersection ? pipelines.shadeIntersectPipeline : pipelines.shadeBasePipeline)
      shadePass.setBindGroup(0, sceneBG)
      shadePass.setBindGroup(1, baseBG)
      shadePass.setVertexBuffer(0, baseMesh.vb)
      shadePass.setIndexBuffer(baseMesh.ib, 'uint32')
      shadePass.setStencilReference(0)
      shadePass.drawIndexed(baseMesh.indexCount)

      // Pass 4: Shade Cut Surfaces (back faces of cutters inside base volume)
      if (!isIntersection) {
        shadePass.setPipeline(pipelines.shadeCutPipeline)
        for (const cutter of term.cutters) {
          const cutterMesh = this.geometryCache.getPrimitiveMesh(dev, cutter)
          const cutterBG = this.createObjectBindGroup(dev, pipelines, cutter, true)
          shadePass.setBindGroup(1, cutterBG)
          shadePass.setVertexBuffer(0, cutterMesh.vb)
          shadePass.setIndexBuffer(cutterMesh.ib, 'uint32')
          shadePass.drawIndexed(cutterMesh.indexCount)
        }
      }

      shadePass.end()
    }
  }

  destroy() {
    this.geometryCache.destroy()
    this.sceneUB?.destroy()
    this.sceneUB = null
    this.sceneBG = null
  }
}
