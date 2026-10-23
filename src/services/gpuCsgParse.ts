/** Lowers OpenSCAD statements into a flat GPU CSG preview tree. */
import { compileOpenSCAD, type Statement, type CallNode, type Expr } from './openscadCompiler'
import type { CsgOp, GpuCsgNode, GpuCsgPrimitive, GpuCsgTerm, GpuCsgTree } from './gpuCsgTypes'
import { createCsgPrimitive, resetCsgIdCounter, identityMat4, multiplyMat4, rotationEulerMat4, scaleMat4, translationMat4 } from './gpuCsgPrimitives'

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
