/**
 * Strict, intentionally documented OpenSCAD subset backed by Manifold WASM.
 *
 * Supported language features: variables, arithmetic/boolean expressions,
 * ranges, for/if/let, user modules and children(). Supported geometry:
 * cube, sphere, cylinder, polyhedron, square, circle, polygon, transforms,
 * union/difference/intersection/hull, linear/rotate extrusion, projection and
 * 2D offset. Unsupported syntax fails loudly instead of rendering a wrong model.
 */
import Module, {
  type CrossSection as CrossSectionGeometry,
  type Manifold as ManifoldGeometry,
  type ManifoldToplevel,
  type Mat4 as ManifoldMatrix,
  type Polygons,
  type Vec2,
  type Vec3,
} from 'manifold-3d/manifold'
import {
  cleanup as cleanupManifold,
  garbageCollectManifold,
} from 'manifold-3d/lib/garbage-collector.js'
import type { GeometryQuality } from '../core/build'
import type {
  MeshData,
  MeshProvenanceRun,
  MeshSourceReference,
  SceneEntityId,
  SourceOperationId,
} from '../core/mesh'
import { identity, type Mat4 } from './math3d'
import { buildMeshBvh } from './meshBvh'
import { extractSemanticEdges } from './meshTopology'

export interface ParseOptions {
  quality?: GeometryQuality
}

export interface ParseResult {
  meshes: MeshData[]
  warnings: string[]
  volume: number
  surfaceArea: number
  quality: GeometryQuality
}

const MAX_SOURCE_LENGTH = 250_000
const MAX_AST_NODES = 25_000
const MAX_EXPRESSION_DEPTH = 256
const MAX_EVAL_DEPTH = 128
const MAX_EVALUATED_VALUE_UNITS = 500_000
const MAX_RANGE_ITEMS = 10_000
const MAX_SHAPES = 1_000
const MAX_TRIANGLES = 750_000
const MAX_FN = 256
/** Total evaluation steps (expressions + statements + loop iterations) before aborting —
 * bounds nested loops whose bodies produce no shapes, which MAX_SHAPES
 * can never catch (`for(i=[0:9999]) for(j=[0:9999]) x = i+j;`). */
const MAX_EVAL_OPS = 1_000_000
/** Cap on evaluated vector/string length — `a = concat(a, a);` repeated
 * ~40 times otherwise materializes 2^40 elements and OOMs the worker. */
const MAX_VALUE_ELEMENTS = 1_000_000
/** Cap on linear_extrude slices — passed straight into the Manifold kernel,
 * which allocates per-slice cross-sections before MAX_TRIANGLES can fire. */
const MAX_EXTRUDE_SLICES = 512

type Value = number | string | boolean | undefined | Value[]

enum TT {
  Num, Str, Ident,
  LParen, RParen, LBrace, RBrace, LBracket, RBracket,
  Comma, Semi, Eq, Plus, Minus, Star, Slash, Percent, Caret,
  Hash, Dollar, Dot, Colon, Question,
  Lt, Gt, LtEq, GtEq, EqEq, NotEq, Not, And, Or,
  Eof,
}

interface Token { t: TT; v: string; p: number; end: number }

export class OpenSCADParseError extends Error {
  readonly line: number
  readonly column: number

  constructor(source: string, position: number, message: string) {
    const safePosition = Math.max(0, Math.min(position, source.length))
    const before = source.slice(0, safePosition)
    const line = before.split('\n').length
    const lineStart = before.lastIndexOf('\n') + 1
    const lineEnd = source.indexOf('\n', safePosition)
    const excerpt = source.slice(lineStart, lineEnd < 0 ? source.length : lineEnd)
    const column = safePosition - lineStart + 1
    super(`Line ${line}, column ${column}: ${message}\n${excerpt}\n${' '.repeat(Math.max(0, column - 1))}^`)
    this.name = 'OpenSCADParseError'
    this.line = line
    this.column = column
  }
}

function tokenize(source: string): Token[] {
  const out: Token[] = []
  let i = 0
  while (i < source.length) {
    const ch = source[i]
    if (ch <= ' ') { i++; continue }
    if (ch === '/' && source[i + 1] === '/') {
      while (i < source.length && source[i] !== '\n') i++
      continue
    }
    if (ch === '/' && source[i + 1] === '*') {
      const start = i
      i += 2
      while (i < source.length - 1 && !(source[i] === '*' && source[i + 1] === '/')) i++
      if (i >= source.length - 1) throw new OpenSCADParseError(source, start, 'Unterminated block comment')
      i += 2
      continue
    }

    const p = i
    if (ch === '"') {
      i++
      let value = ''
      let closed = false
      while (i < source.length) {
        if (source[i] === '"') { i++; closed = true; break }
        if (source[i] === '\\') {
          i++
          const escaped = source[i]
          if (escaped == null) break
          value += escaped === 'n' ? '\n' : escaped === 't' ? '\t' : escaped
          i++
        } else {
          value += source[i++]
        }
      }
      if (!closed) throw new OpenSCADParseError(source, p, 'Unterminated string')
      out.push({ t: TT.Str, v: value, p, end: i })
      continue
    }

    if (isDigit(ch) || (ch === '.' && isDigit(source[i + 1]))) {
      let value = ''
      while (isDigit(source[i])) value += source[i++]
      if (source[i] === '.') {
        value += source[i++]
        while (isDigit(source[i])) value += source[i++]
      }
      if (source[i] === 'e' || source[i] === 'E') {
        value += source[i++]
        if (source[i] === '+' || source[i] === '-') value += source[i++]
        if (!isDigit(source[i])) throw new OpenSCADParseError(source, p, 'Invalid exponent')
        while (isDigit(source[i])) value += source[i++]
      }
      out.push({ t: TT.Num, v: value, p, end: i })
      continue
    }

    if (isIdentStart(ch)) {
      let value = ''
      while (isIdentPart(source[i])) value += source[i++]
      out.push({ t: TT.Ident, v: value, p, end: i })
      continue
    }

    const two = source.slice(i, i + 2)
    const doubles: Record<string, TT> = {
      '<=': TT.LtEq, '>=': TT.GtEq, '==': TT.EqEq, '!=': TT.NotEq,
      '&&': TT.And, '||': TT.Or,
    }
    if (doubles[two] !== undefined) {
      out.push({ t: doubles[two], v: two, p, end: i + 2 })
      i += 2
      continue
    }

    const singles: Record<string, TT> = {
      '(': TT.LParen, ')': TT.RParen, '{': TT.LBrace, '}': TT.RBrace,
      '[': TT.LBracket, ']': TT.RBracket, ',': TT.Comma, ';': TT.Semi,
      '=': TT.Eq, '+': TT.Plus, '-': TT.Minus, '*': TT.Star, '/': TT.Slash,
      '%': TT.Percent, '^': TT.Caret, '#': TT.Hash, '$': TT.Dollar,
      '.': TT.Dot, ':': TT.Colon, '?': TT.Question, '<': TT.Lt, '>': TT.Gt,
      '!': TT.Not,
    }
    const tokenType = singles[ch]
    if (tokenType === undefined) throw new OpenSCADParseError(source, p, `Unexpected character ${JSON.stringify(ch)}`)
    out.push({ t: tokenType, v: ch, p, end: i + 1 })
    i++
  }
  out.push({ t: TT.Eof, v: '', p: source.length, end: source.length })
  return out
}

function isDigit(ch: string | undefined) { return ch !== undefined && ch >= '0' && ch <= '9' }
function isIdentStart(ch: string | undefined) {
  return ch !== undefined && ((ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || ch === '_' || ch === '$')
}
function isIdentPart(ch: string | undefined) { return isIdentStart(ch) || isDigit(ch) }

type Expr =
  | { kind: 'literal'; value: Value; p: number }
  | { kind: 'identifier'; name: string; p: number }
  | { kind: 'vector'; items: Expr[]; p: number }
  | { kind: 'range'; start: Expr; step?: Expr; end: Expr; p: number }
  | { kind: 'unary'; op: TT; value: Expr; p: number }
  | { kind: 'binary'; op: TT; left: Expr; right: Expr; p: number }
  | { kind: 'ternary'; test: Expr; yes: Expr; no: Expr; p: number }
  | { kind: 'call'; name: string; args: Expr[]; p: number }
  | { kind: 'index'; value: Expr; index: Expr; p: number }

interface CallNode {
  type: 'call'
  name: string
  args: Record<string, Expr>
  children: Statement[]
  alternative: Statement[]
  p: number
  end: number
  operationId?: SourceOperationId
}

interface AssignNode { type: 'assign'; name: string; value: Expr; p: number; end: number }
interface ModuleParam { name: string; defaultValue?: Expr }
interface ModuleNode {
  type: 'module'
  name: string
  params: ModuleParam[]
  children: Statement[]
  p: number
  end: number
}
type Statement = CallNode | AssignNode | ModuleNode

class Parser {
  private pos = 0
  private nodes = 0
  private expressionDepth = 0
  private statementDepth = 0
  private lastTokenEnd = 0

  constructor(private readonly tokens: Token[], private readonly source: string) {}

  parseAll(): Statement[] {
    const result: Statement[] = []
    while (this.peek().t !== TT.Eof) {
      const statement = this.statement()
      if (statement) result.push(statement)
    }
    return result
  }

  private peek(offset = 0) { return this.tokens[this.pos + offset] ?? this.tokens[this.tokens.length - 1] }
  private advance() {
    const token = this.tokens[this.pos++]
    this.lastTokenEnd = token.end
    return token
  }
  private match(type: TT) { if (this.peek().t === type) { this.advance(); return true } return false }
  private expect(type: TT, message?: string) {
    const token = this.advance()
    if (token.t !== type) this.fail(token, message ?? `Expected ${TT[type]}, got ${token.v || TT[token.t]}`)
    return token
  }
  private fail(token: Token, message: string): never { throw new OpenSCADParseError(this.source, token.p, message) }
  private countNode() {
    this.nodes++
    if (this.nodes > MAX_AST_NODES) this.fail(this.peek(), `Model exceeds the ${MAX_AST_NODES.toLocaleString()} syntax node limit`)
  }
  private expressionNode<T extends Expr>(node: T): T {
    this.countNode()
    return node
  }
  private descendExpression<T>(position: number, parse: () => T): T {
    if (this.expressionDepth >= MAX_EXPRESSION_DEPTH) {
      throw new OpenSCADParseError(this.source, position, `Expression exceeds ${MAX_EXPRESSION_DEPTH} nested levels`)
    }
    this.expressionDepth++
    try { return parse() } finally { this.expressionDepth-- }
  }

  private statement(): Statement | null {
    if (this.statementDepth >= MAX_EVAL_DEPTH) {
      this.fail(this.peek(), `Model exceeds ${MAX_EVAL_DEPTH} nested statements`)
    }
    this.statementDepth++
    try {
      return this.parseStatement()
    } finally {
      this.statementDepth--
    }
  }

  private parseStatement(): Statement | null {
    if (this.match(TT.Semi)) return null
    let disabled = false
    while ([TT.Hash, TT.Percent, TT.Star, TT.Not].includes(this.peek().t)) {
      if (this.advance().t === TT.Star) disabled = true
    }
    if (this.peek().t !== TT.Ident) this.fail(this.peek(), 'Expected a variable, module, or geometry call')

    let node: Statement
    if (this.peek().v === 'module') node = this.moduleDefinition()
    else if (this.peek().v === 'function' || this.peek().v === 'include' || this.peek().v === 'use') {
      this.fail(this.peek(), `${this.peek().v} is not supported by the permissive browser subset`)
    } else if (this.peek(1).t === TT.Eq) node = this.assignment()
    else node = this.call()
    this.countNode()
    return disabled ? null : node
  }

  private assignment(): AssignNode {
    const name = this.expect(TT.Ident)
    this.expect(TT.Eq)
    const value = this.expression()
    const terminator = this.expect(TT.Semi, 'Expected ; after assignment')
    return { type: 'assign', name: name.v, value, p: name.p, end: terminator.end }
  }

  private moduleDefinition(): ModuleNode {
    const keyword = this.advance()
    const name = this.expect(TT.Ident, 'Expected module name')
    this.expect(TT.LParen, 'Expected ( after module name')
    const params: ModuleParam[] = []
    while (this.peek().t !== TT.RParen) {
      const param = this.expect(TT.Ident, 'Expected parameter name')
      const defaultValue = this.match(TT.Eq) ? this.expression() : undefined
      params.push({ name: param.v, defaultValue })
      if (!this.match(TT.Comma) && this.peek().t !== TT.RParen) this.fail(this.peek(), 'Expected , or )')
    }
    this.advance()
    const children = this.body(true)
    return { type: 'module', name: name.v, params, children, p: keyword.p, end: this.lastTokenEnd }
  }

  private call(): CallNode {
    const name = this.expect(TT.Ident)
    const args: Record<string, Expr> = {}
    if (this.match(TT.LParen)) {
      let positional = 0
      while (this.peek().t !== TT.RParen) {
        let key: string
        if (this.peek().t === TT.Ident && this.peek(1).t === TT.Eq) {
          key = this.advance().v
          this.advance()
        } else key = `_${positional++}`
        if (args[key]) this.fail(this.peek(), `Duplicate argument ${key}`)
        args[key] = this.expression()
        if (!this.match(TT.Comma) && this.peek().t !== TT.RParen) this.fail(this.peek(), 'Expected , or )')
      }
      this.advance()
    }

    const children = this.body(false)
    let alternative: Statement[] = []
    if (name.v === 'if' && this.peek().t === TT.Ident && this.peek().v === 'else') {
      this.advance()
      alternative = this.body(true)
    }
    return { type: 'call', name: name.v, args, children, alternative, p: name.p, end: this.lastTokenEnd }
  }

  private body(required: boolean): Statement[] {
    if (this.match(TT.LBrace)) {
      const children: Statement[] = []
      while (this.peek().t !== TT.RBrace && this.peek().t !== TT.Eof) {
        const child = this.statement()
        if (child) children.push(child)
      }
      this.expect(TT.RBrace, 'Expected }')
      return children
    }
    if (this.match(TT.Semi)) return []
    if (!required && [TT.RBrace, TT.Eof].includes(this.peek().t)) return []
    const child = this.statement()
    return child ? [child] : []
  }

  private expression(): Expr { return this.descendExpression(this.peek().p, () => this.ternary()) }

  private ternary(): Expr {
    const test = this.binaryOr()
    if (!this.match(TT.Question)) return test
    const yes = this.expression()
    this.expect(TT.Colon, 'Expected : in conditional expression')
    return this.expressionNode({ kind: 'ternary', test, yes, no: this.expression(), p: test.p })
  }

  private binaryOr(): Expr { return this.binary(() => this.binaryAnd(), [TT.Or]) }
  private binaryAnd(): Expr { return this.binary(() => this.comparison(), [TT.And]) }
  private comparison(): Expr { return this.binary(() => this.additive(), [TT.Lt, TT.Gt, TT.LtEq, TT.GtEq, TT.EqEq, TT.NotEq]) }
  private additive(): Expr { return this.binary(() => this.multiplicative(), [TT.Plus, TT.Minus]) }
  private multiplicative(): Expr { return this.binary(() => this.power(), [TT.Star, TT.Slash, TT.Percent]) }

  private power(): Expr {
    const left = this.unary()
    if (!this.match(TT.Caret)) return left
    return this.expressionNode({
      kind: 'binary', op: TT.Caret, left,
      right: this.descendExpression(this.peek().p, () => this.power()),
      p: left.p,
    })
  }

  private binary(next: () => Expr, operators: TT[]): Expr {
    let left = next()
    while (operators.includes(this.peek().t)) {
      const op = this.advance()
      left = this.expressionNode({ kind: 'binary', op: op.t, left, right: next(), p: op.p })
    }
    return left
  }

  private unary(): Expr {
    if ([TT.Plus, TT.Minus, TT.Not].includes(this.peek().t)) {
      const op = this.advance()
      return this.expressionNode({
        kind: 'unary', op: op.t,
        value: this.descendExpression(op.p, () => this.unary()),
        p: op.p,
      })
    }
    return this.postfix()
  }

  private postfix(): Expr {
    let value = this.primary()
    while (this.match(TT.LBracket)) {
      const p = value.p
      const index = this.expression()
      this.expect(TT.RBracket, 'Expected ] after index')
      value = this.expressionNode({ kind: 'index', value, index, p })
    }
    return value
  }

  private primary(): Expr {
    const token = this.peek()
    if (this.match(TT.Num)) return this.expressionNode({ kind: 'literal', value: Number(token.v), p: token.p })
    if (this.match(TT.Str)) return this.expressionNode({ kind: 'literal', value: token.v, p: token.p })
    if (this.match(TT.LParen)) {
      const value = this.expression()
      this.expect(TT.RParen, 'Expected )')
      return value
    }
    if (this.match(TT.LBracket)) return this.vectorOrRange(token.p)
    if (this.match(TT.Ident)) {
      if (token.v === 'true' || token.v === 'false') return this.expressionNode({ kind: 'literal', value: token.v === 'true', p: token.p })
      if (token.v === 'undef') return this.expressionNode({ kind: 'literal', value: undefined, p: token.p })
      if (!this.match(TT.LParen)) return this.expressionNode({ kind: 'identifier', name: token.v, p: token.p })
      const args: Expr[] = []
      while (this.peek().t !== TT.RParen) {
        args.push(this.expression())
        if (!this.match(TT.Comma) && this.peek().t !== TT.RParen) this.fail(this.peek(), 'Expected , or )')
      }
      this.advance()
      return this.expressionNode({ kind: 'call', name: token.v, args, p: token.p })
    }
    this.fail(token, `Expected expression, got ${token.v || TT[token.t]}`)
  }

  private vectorOrRange(p: number): Expr {
    if (this.match(TT.RBracket)) return this.expressionNode({ kind: 'vector', items: [], p })
    const first = this.expression()
    if (this.match(TT.Colon)) {
      const second = this.expression()
      let step: Expr | undefined
      let end = second
      if (this.match(TT.Colon)) { step = second; end = this.expression() }
      this.expect(TT.RBracket, 'Expected ] after range')
      return this.expressionNode({ kind: 'range', start: first, step, end, p })
    }
    const items = [first]
    while (this.match(TT.Comma)) {
      if (this.peek().t === TT.RBracket) break
      items.push(this.expression())
    }
    this.expect(TT.RBracket, 'Expected ]')
    return this.expressionNode({ kind: 'vector', items, p })
  }
}

/**
 * Give call sites structural identities after parsing. Occurrences are counted
 * per operation name, so whitespace, comments, unrelated siblings and quality
 * changes do not move an existing identity. Identical same-name siblings remain
 * positional because the language has no persistent user-authored node IDs.
 */
function assignOperationIds(nodes: Statement[], parent: readonly string[] = ['root']) {
  const occurrences = new Map<string, number>()
  for (const node of nodes) {
    const key = `${node.type}:${node.name}`
    const occurrence = occurrences.get(key) ?? 0
    occurrences.set(key, occurrence + 1)
    const path = [...parent, `${key}#${occurrence}`]
    if (node.type === 'call') {
      node.operationId = `op:${path.map(encodeURIComponent).join('/')}`
      assignOperationIds(node.children, [...path, 'children'])
      assignOperationIds(node.alternative, [...path, 'alternative'])
    } else if (node.type === 'module') {
      assignOperationIds(node.children, [...path, 'body'])
    }
  }
}

interface EvalContext {
  wasm: ManifoldToplevel
  source: string
  env: Map<string, Value>
  modules: Map<string, ModuleNode>
  warnings: string[]
  quality: GeometryQuality
  sourceReferences: Map<number, MeshSourceReference>
  callChildren?: Statement[]
  depth: number
  /** Shared mutable evaluation budget — one object across all ctx spreads. */
  budget: { ops: number }
  instancePath: string
  valueBudget: { used: number }
  valueWeights: WeakMap<Value[], number>
  valueDepths: WeakMap<Value[], number>
}

interface Shape2D { dimension: 2; geometry: CrossSectionGeometry; color: RGBA; entityId: SceneEntityId }
interface Shape3D { dimension: 3; geometry: ManifoldGeometry; color: RGBA; entityId: SceneEntityId }
type Shape = Shape2D | Shape3D
type RGBA = [number, number, number, number]

function currentEntityId(ctx: EvalContext): SceneEntityId {
  return `entity:${ctx.instancePath}`
}

function staticOperationId(node: CallNode): SourceOperationId {
  return node.operationId ?? `op:legacy-offset-${node.p}`
}

function trackSource(geometry: ManifoldGeometry, node: CallNode, ctx: EvalContext) {
  const originalId = geometry.originalID()
  if (originalId < 0 || ctx.sourceReferences.has(originalId)) return
  ctx.sourceReferences.set(originalId, {
    id: node.p,
    operationId: staticOperationId(node),
    instanceId: currentEntityId(ctx),
    originalId,
    start: node.p,
    end: node.end,
    label: `${node.name}()`,
  })
}

function trackedSolid(geometry: ManifoldGeometry, color: RGBA, node: CallNode, ctx: EvalContext): Shape3D {
  // Eager geometry-generating operations such as hull() return a product
  // manifold (originalID() === -1). Promote those results to an original so
  // subsequent transforms/booleans retain a source ID for the operation that
  // actually generated the surface. Primitive constructors are already
  // originals, so this is a no-op for them.
  const trackedGeometry = geometry.originalID() < 0 ? geometry.asOriginal() : geometry
  trackSource(trackedGeometry, node, ctx)
  return { dimension: 3, geometry: trackedGeometry, color, entityId: currentEntityId(ctx) }
}

const PALETTE: RGBA[] = [
  [0.26, 0.52, 0.96, 1], [0.96, 0.52, 0.26, 1],
  [0.26, 0.86, 0.56, 1], [0.86, 0.26, 0.66, 1],
  [0.96, 0.86, 0.26, 1], [0.46, 0.76, 0.86, 1],
  [0.76, 0.56, 0.96, 1], [0.56, 0.86, 0.36, 1],
]
let paletteIndex = 0

const CSS_COLORS: Record<string, RGBA> = {
  red: [1, 0, 0, 1], green: [0, 0.5, 0, 1], blue: [0, 0, 1, 1],
  yellow: [1, 1, 0, 1], cyan: [0, 1, 1, 1], magenta: [1, 0, 1, 1],
  white: [1, 1, 1, 1], black: [0, 0, 0, 1], orange: [1, 0.65, 0, 1],
  gray: [0.5, 0.5, 0.5, 1], grey: [0.5, 0.5, 0.5, 1],
  pink: [1, 0.75, 0.8, 1], purple: [0.5, 0, 0.5, 1], brown: [0.65, 0.16, 0.16, 1],
  lime: [0, 1, 0, 1], navy: [0, 0, 0.5, 1], teal: [0, 0.5, 0.5, 1],
}

function nextColor(): RGBA { return [...PALETTE[paletteIndex++ % PALETTE.length]] as RGBA }
function warn(ctx: EvalContext, message: string) { if (!ctx.warnings.includes(message)) ctx.warnings.push(message) }
function evaluationError(ctx: EvalContext, p: number, message: string): never { throw new OpenSCADParseError(ctx.source, p, message) }

function valueWeight(value: Value, ctx: EvalContext): number {
  if (Array.isArray(value)) return ctx.valueWeights.get(value) ?? value.length + 1
  return typeof value === 'string' ? Math.max(1, value.length) : 1
}

function valueDepth(value: Value, ctx: EvalContext): number {
  return Array.isArray(value) ? ctx.valueDepths.get(value) ?? 1 : 0
}

function registerArrayValue<T extends Value[]>(
  value: T,
  ctx: EvalContext,
  p: number,
  limitLabel = 'Evaluated value',
): T {
  let weight = 1
  let depth = 1
  for (const item of value) {
    weight += valueWeight(item, ctx)
    depth = Math.max(depth, valueDepth(item, ctx) + 1)
    if (weight > MAX_EVALUATED_VALUE_UNITS) {
      evaluationError(ctx, p, `${limitLabel} exceeds ${MAX_EVALUATED_VALUE_UNITS.toLocaleString()} units`)
    }
  }
  if (depth > MAX_EXPRESSION_DEPTH) {
    evaluationError(ctx, p, `Evaluated value exceeds ${MAX_EXPRESSION_DEPTH} nested levels`)
  }
  ctx.valueBudget.used += weight
  if (ctx.valueBudget.used > MAX_EVALUATED_VALUE_UNITS) {
    evaluationError(ctx, p, `${limitLabel} exceeds the ${MAX_EVALUATED_VALUE_UNITS.toLocaleString()} value-allocation budget`)
  }
  ctx.valueWeights.set(value, weight)
  ctx.valueDepths.set(value, depth)
  return value
}

function registerStringValue(value: string, ctx: EvalContext, p: number, limitLabel = 'Evaluation'): string {
  ctx.valueBudget.used += Math.max(1, value.length)
  if (ctx.valueBudget.used > MAX_EVALUATED_VALUE_UNITS) {
    evaluationError(ctx, p, `${limitLabel} exceeds the ${MAX_EVALUATED_VALUE_UNITS.toLocaleString()} value-allocation budget`)
  }
  return value
}

function evalExpression(expr: Expr, ctx: EvalContext, depth = 0): Value {
  if (++ctx.budget.ops > MAX_EVAL_OPS) {
    evaluationError(ctx, expr.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
  }
  if (depth >= MAX_EXPRESSION_DEPTH) {
    evaluationError(ctx, expr.p, `Expression exceeds ${MAX_EXPRESSION_DEPTH} evaluated levels`)
  }
  const evaluate = (child: Expr) => evalExpression(child, ctx, depth + 1)
  switch (expr.kind) {
    case 'literal': return expr.value
    case 'identifier': {
      if (expr.name === 'PI') return Math.PI
      if (!ctx.env.has(expr.name)) evaluationError(ctx, expr.p, `Unknown variable ${expr.name}`)
      return ctx.env.get(expr.name)
    }
    case 'vector': return registerArrayValue(expr.items.map(evaluate), ctx, expr.p)
    case 'range': {
      const start = finiteNumber(evaluate(expr.start), ctx, expr.p, 'range start')
      const end = finiteNumber(evaluate(expr.end), ctx, expr.p, 'range end')
      const step = expr.step ? finiteNumber(evaluate(expr.step), ctx, expr.p, 'range step') : 1
      if (step === 0) evaluationError(ctx, expr.p, 'Range step cannot be zero')
      const values: number[] = []
      const forward = step > 0
      for (let value = start; forward ? value <= end + 1e-10 : value >= end - 1e-10; value += step) {
        values.push(value)
        if (values.length > MAX_RANGE_ITEMS) evaluationError(ctx, expr.p, `Range exceeds ${MAX_RANGE_ITEMS.toLocaleString()} items`)
      }
      return registerArrayValue(values, ctx, expr.p)
    }
    case 'unary': {
      const value = evaluate(expr.value)
      if (expr.op === TT.Not) return !truthy(value)
      const number = finiteNumber(value, ctx, expr.p, 'unary operand')
      return expr.op === TT.Minus ? -number : number
    }
    case 'binary': {
      if (expr.op === TT.And) return truthy(evaluate(expr.left)) && truthy(evaluate(expr.right))
      if (expr.op === TT.Or) return truthy(evaluate(expr.left)) || truthy(evaluate(expr.right))
      const left = evaluate(expr.left)
      const right = evaluate(expr.right)
      if (expr.op === TT.EqEq) return deepEqual(left, right)
      if (expr.op === TT.NotEq) return !deepEqual(left, right)
      if ([TT.Lt, TT.Gt, TT.LtEq, TT.GtEq].includes(expr.op)) {
        const a = finiteNumber(left, ctx, expr.p, 'comparison operand')
        const b = finiteNumber(right, ctx, expr.p, 'comparison operand')
        if (expr.op === TT.Lt) return a < b
        if (expr.op === TT.Gt) return a > b
        if (expr.op === TT.LtEq) return a <= b
        return a >= b
      }
      const a = finiteNumber(left, ctx, expr.p, 'arithmetic operand')
      const b = finiteNumber(right, ctx, expr.p, 'arithmetic operand')
      let result: number
      if (expr.op === TT.Plus) result = a + b
      else if (expr.op === TT.Minus) result = a - b
      else if (expr.op === TT.Star) result = a * b
      else if (expr.op === TT.Slash) result = a / b
      else if (expr.op === TT.Percent) result = a % b
      else result = a ** b
      if (!Number.isFinite(result)) evaluationError(ctx, expr.p, 'Expression produced a non-finite number')
      return result
    }
    case 'ternary': return truthy(evaluate(expr.test)) ? evaluate(expr.yes) : evaluate(expr.no)
    case 'index': {
      const value = evaluate(expr.value)
      const index = Math.trunc(finiteNumber(evaluate(expr.index), ctx, expr.p, 'index'))
      if (Array.isArray(value) || typeof value === 'string') return value[index] as Value
      evaluationError(ctx, expr.p, 'Only vectors and strings can be indexed')
    }
    case 'call': return evalBuiltin(expr, ctx, depth)
  }
}

function evalBuiltin(expr: Extract<Expr, { kind: 'call' }>, ctx: EvalContext, depth: number): Value {
  const values = expr.args.map(arg => evalExpression(arg, ctx, depth + 1))
  const nums = () => values.map(value => finiteNumber(value, ctx, expr.p, `${expr.name} argument`))
  const radians = (degrees: number) => degrees * Math.PI / 180
  const degrees = (radiansValue: number) => radiansValue * 180 / Math.PI
  const unary: Record<string, (value: number) => number> = {
    abs: Math.abs, ceil: Math.ceil, floor: Math.floor, round: Math.round,
    sqrt: Math.sqrt, exp: Math.exp, ln: Math.log, log: Math.log10,
    sin: value => Math.sin(radians(value)), cos: value => Math.cos(radians(value)),
    tan: value => Math.tan(radians(value)), asin: value => degrees(Math.asin(value)),
    acos: value => degrees(Math.acos(value)), atan: value => degrees(Math.atan(value)),
    sign: Math.sign,
  }
  if (unary[expr.name]) {
    const result = unary[expr.name](nums()[0] ?? 0)
    if (!Number.isFinite(result)) evaluationError(ctx, expr.p, `${expr.name} produced a non-finite number`)
    return result
  }
  if (expr.name === 'atan2') return degrees(Math.atan2(...nums().slice(0, 2) as [number, number]))
  if (expr.name === 'pow') return (nums()[0] ?? 0) ** (nums()[1] ?? 0)
  if (expr.name === 'min' || expr.name === 'max') {
    const input = values.length === 1 && Array.isArray(values[0]) ? values[0] : values
    const numbers = input.map(value => finiteNumber(value, ctx, expr.p, `${expr.name} argument`))
    return expr.name === 'min' ? Math.min(...numbers) : Math.max(...numbers)
  }
  if (expr.name === 'len') {
    const value = values[0]
    return Array.isArray(value) || typeof value === 'string' ? value.length : 0
  }
  if (expr.name === 'norm') {
    const vector = vectorValue(values[0], ctx, expr.p, 'norm vector')
    return Math.hypot(...vector)
  }
  if (expr.name === 'concat') {
    const result = values.flatMap(value => Array.isArray(value) ? value : [value])
    if (result.length > MAX_VALUE_ELEMENTS) evaluationError(ctx, expr.p, `concat() result exceeds ${MAX_VALUE_ELEMENTS.toLocaleString()} elements`)
    return registerArrayValue(result, ctx, expr.p, 'concat() result')
  }
  if (expr.name === 'str') {
    const result = values.map(value => valueToString(value)).join('')
    if (result.length > MAX_VALUE_ELEMENTS) evaluationError(ctx, expr.p, `str() result exceeds ${MAX_VALUE_ELEMENTS.toLocaleString()} characters`)
    return registerStringValue(result, ctx, expr.p, 'str() result')
  }
  evaluationError(ctx, expr.p, `Unsupported function ${expr.name}()`)
}

function valueToString(value: Value): string {
  if (Array.isArray(value)) return `[${value.map(valueToString).join(', ')}]`
  if (value === undefined) return 'undef'
  return String(value)
}
function truthy(value: Value) { return value !== false && value !== undefined && value !== 0 }
function deepEqual(a: Value, b: Value): boolean {
  if (Array.isArray(a) && Array.isArray(b)) return a.length === b.length && a.every((value, index) => deepEqual(value, b[index]))
  return a === b
}
function finiteNumber(value: Value, ctx: EvalContext, p: number, label: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) evaluationError(ctx, p, `${label} must be a finite number`)
  return value
}
function vectorValue(value: Value, ctx: EvalContext, p: number, label: string): number[] {
  if (!Array.isArray(value)) evaluationError(ctx, p, `${label} must be a vector`)
  return value.map(item => finiteNumber(item, ctx, p, label))
}
function arg(node: CallNode, name: string, position: number, fallback: Value, ctx: EvalContext): Value {
  const expression = node.args[name] ?? node.args[`_${position}`]
  return expression ? evalExpression(expression, ctx) : fallback
}

function collectModules(nodes: Statement[], modules: Map<string, ModuleNode>) {
  for (const node of nodes) {
    if (node.type === 'module') modules.set(node.name, node)
    if (node.type === 'call') {
      collectModules(node.children, modules)
      collectModules(node.alternative, modules)
    }
  }
}

function evalNodes(nodes: Statement[], parent: EvalContext, scoped = true): Shape[] {
  const ctx: EvalContext = { ...parent, env: scoped ? new Map(parent.env) : parent.env }
  const output: Shape[] = []
  for (const node of nodes) {
    if (++ctx.budget.ops > MAX_EVAL_OPS) evaluationError(ctx, node.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
    if (node.type === 'assign') { ctx.env.set(node.name, evalExpression(node.value, ctx)); continue }
    if (node.type === 'module') continue
    output.push(...evalNode(node, ctx))
    if (output.length > MAX_SHAPES) evaluationError(ctx, node.p, `Model exceeds the ${MAX_SHAPES.toLocaleString()} object limit`)
  }
  return output
}

function evalNode(node: CallNode, parent: EvalContext): Shape[] {
  if (parent.depth >= MAX_EVAL_DEPTH) evaluationError(parent, node.p, `Evaluation exceeds ${MAX_EVAL_DEPTH} nested calls`)
  const operationId = staticOperationId(node)
  const ctx: EvalContext = {
    ...parent,
    depth: parent.depth + 1,
    instancePath: `${parent.instancePath}>${operationId}`,
  }
  const childShapes = () => evalNodes(node.children, ctx)

  switch (node.name) {
    case 'cube': {
      const raw = arg(node, 'size', 0, 1, ctx)
      const size = Array.isArray(raw) ? vectorValue(raw, ctx, node.p, 'cube size') : [finiteNumber(raw, ctx, node.p, 'cube size')]
      const dimensions: Vec3 = [size[0] ?? 1, size[1] ?? size[0] ?? 1, size[2] ?? size[0] ?? 1]
      if (dimensions.some(value => value <= 0)) evaluationError(ctx, node.p, 'Cube dimensions must be positive')
      const geometry = ctx.wasm.Manifold.cube(dimensions, arg(node, 'center', 1, false, ctx) === true)
      return [trackedSolid(geometry, nextColor(), node, ctx)]
    }
    case 'sphere': {
      let radius = arg(node, 'r', 0, undefined, ctx)
      const diameter = arg(node, 'd', -1, undefined, ctx)
      if (radius === undefined) radius = diameter === undefined ? 1 : finiteNumber(diameter, ctx, node.p, 'sphere diameter') / 2
      const r = finiteNumber(radius, ctx, node.p, 'sphere radius')
      if (r <= 0) evaluationError(ctx, node.p, 'Sphere radius must be positive')
      return [trackedSolid(ctx.wasm.Manifold.sphere(r, segments(node, ctx, 32, 4)), nextColor(), node, ctx)]
    }
    case 'cylinder': return makeCylinder(node, ctx)
    case 'polyhedron': return makePolyhedron(node, ctx)
    case 'square': {
      const raw = arg(node, 'size', 0, 1, ctx)
      const size = Array.isArray(raw) ? vectorValue(raw, ctx, node.p, 'square size') : [finiteNumber(raw, ctx, node.p, 'square size')]
      const dimensions: Vec2 = [size[0] ?? 1, size[1] ?? size[0] ?? 1]
      if (dimensions.some(value => value <= 0)) evaluationError(ctx, node.p, 'Square dimensions must be positive')
      return [{
        dimension: 2,
        geometry: ctx.wasm.CrossSection.square(dimensions, arg(node, 'center', 1, false, ctx) === true),
        color: nextColor(),
        entityId: currentEntityId(ctx),
      }]
    }
    case 'circle': {
      let radius = arg(node, 'r', 0, undefined, ctx)
      const diameter = arg(node, 'd', -1, undefined, ctx)
      if (radius === undefined) radius = diameter === undefined ? 1 : finiteNumber(diameter, ctx, node.p, 'circle diameter') / 2
      const r = finiteNumber(radius, ctx, node.p, 'circle radius')
      if (r <= 0) evaluationError(ctx, node.p, 'Circle radius must be positive')
      return [{
        dimension: 2,
        geometry: ctx.wasm.CrossSection.circle(r, segments(node, ctx, 48, 3)),
        color: nextColor(),
        entityId: currentEntityId(ctx),
      }]
    }
    case 'polygon': return makePolygon(node, ctx)
    case 'translate': {
      const vector = vectorValue(arg(node, 'v', 0, [0, 0, 0], ctx), ctx, node.p, 'translate vector')
      return childShapes().map(shape => shape.dimension === 3
        ? { ...shape, geometry: shape.geometry.translate([vector[0] ?? 0, vector[1] ?? 0, vector[2] ?? 0]) }
        : { ...shape, geometry: shape.geometry.translate([vector[0] ?? 0, vector[1] ?? 0]) })
    }
    case 'rotate': return rotateShapes(childShapes(), node, ctx)
    case 'scale': {
      const raw = arg(node, 'v', 0, [1, 1, 1], ctx)
      const values = Array.isArray(raw) ? vectorValue(raw, ctx, node.p, 'scale vector') : [finiteNumber(raw, ctx, node.p, 'scale')]
      const sx = values[0] ?? 1, sy = values[1] ?? sx, sz = values[2] ?? sx
      if ([sx, sy, sz].some(value => value === 0)) evaluationError(ctx, node.p, 'Scale values cannot be zero')
      return childShapes().map(shape => shape.dimension === 3
        ? { ...shape, geometry: shape.geometry.scale([sx, sy, sz]) }
        : { ...shape, geometry: shape.geometry.scale([sx, sy]) })
    }
    case 'mirror': {
      const vector = vectorValue(arg(node, 'v', 0, [1, 0, 0], ctx), ctx, node.p, 'mirror normal')
      return childShapes().map(shape => shape.dimension === 3
        ? { ...shape, geometry: shape.geometry.mirror([vector[0] ?? 0, vector[1] ?? 0, vector[2] ?? 0]) }
        : { ...shape, geometry: shape.geometry.mirror([vector[0] ?? 0, vector[1] ?? 0]) })
    }
    case 'multmatrix': return transformByMatrix(childShapes(), arg(node, 'm', 0, undefined, ctx), node, ctx)
    case 'color': {
      const color = parseColor(arg(node, 'c', 0, [0.5, 0.5, 0.5], ctx), ctx, node.p)
      const alpha = arg(node, 'alpha', 1, undefined, ctx)
      if (alpha !== undefined) color[3] = clamp01(finiteNumber(alpha, ctx, node.p, 'color alpha'))
      return childShapes().map(shape => ({ ...shape, color: [...color] as RGBA }))
    }
    case 'union': return booleanShapes(childShapes(), 'union', ctx, node.p)
    case 'difference': return differenceChildren(node, ctx)
    case 'intersection': return booleanShapes(childShapes(), 'intersection', ctx, node.p)
    case 'hull': return hullShapes(childShapes(), node, ctx)
    case 'linear_extrude': return linearExtrude(node, ctx)
    case 'rotate_extrude': return rotateExtrude(node, ctx)
    case 'projection': {
      const cut = arg(node, 'cut', 0, false, ctx) === true
      return childShapes().map(shape => {
        if (shape.dimension !== 3) evaluationError(ctx, node.p, 'projection() requires 3D children')
        return {
          dimension: 2,
          geometry: cut ? shape.geometry.slice(0) : shape.geometry.project(),
          color: shape.color,
          entityId: currentEntityId(ctx),
        }
      })
    }
    case 'offset': {
      const distance = finiteNumber(arg(node, 'r', 0, arg(node, 'delta', 0, 1, ctx), ctx), ctx, node.p, 'offset distance')
      return childShapes().map(shape => {
        if (shape.dimension !== 2) evaluationError(ctx, node.p, 'offset() requires 2D children')
        return { ...shape, geometry: shape.geometry.offset(distance), entityId: currentEntityId(ctx) }
      })
    }
    case 'group': case 'render': return childShapes()
    case 'if': return evalNodes(truthy(arg(node, '_0', 0, false, ctx)) ? node.children : node.alternative, ctx)
    case 'let': {
      const env = new Map(ctx.env)
      for (const [name, expression] of Object.entries(node.args)) if (!name.startsWith('_')) env.set(name, evalExpression(expression, ctx))
      return evalNodes(node.children, { ...ctx, env }, false)
    }
    case 'for': return evalFor(node, ctx)
    case 'children': {
      const all = ctx.callChildren ?? []
      const index = arg(node, '_0', 0, undefined, ctx)
      if (index === undefined) return evalNodes(all, ctx)
      const childIndex = Math.trunc(finiteNumber(index, ctx, node.p, 'children index'))
      return all[childIndex] ? evalNodes([all[childIndex]], ctx) : []
    }
  }

  const module = ctx.modules.get(node.name)
  if (module) return evalUserModule(node, module, ctx)
  evaluationError(ctx, node.p, `Unsupported geometry operation ${node.name}()`)
}

function segments(node: CallNode, ctx: EvalContext, fallback: number, minimum: number): number {
  const local = arg(node, '$fn', -1, undefined, ctx)
  const global = ctx.env.get('$fn')
  const raw = local === undefined || local === 0 ? global : local
  const maxSegments = ctx.quality === 'preview' ? 48 : MAX_FN
  const previewFallback = ctx.quality === 'preview' ? Math.min(fallback, 24) : fallback
  let value = raw === undefined || raw === 0 ? previewFallback : Math.round(finiteNumber(raw, ctx, node.p, '$fn'))
  if (value > maxSegments) {
    warn(ctx, `$fn=${value} was clamped to ${maxSegments} for ${ctx.quality} rendering`)
    value = maxSegments
  }
  return Math.max(minimum, value)
}

function makeCylinder(node: CallNode, ctx: EvalContext): Shape[] {
  const height = finiteNumber(arg(node, 'h', 0, 1, ctx), ctx, node.p, 'cylinder height')
  if (height <= 0) evaluationError(ctx, node.p, 'Cylinder height must be positive')
  let low = arg(node, 'r1', 1, undefined, ctx)
  let high = arg(node, 'r2', 2, undefined, ctx)
  const radius = arg(node, 'r', 1, undefined, ctx)
  const diameter = arg(node, 'd', -1, undefined, ctx)
  const d1 = arg(node, 'd1', -1, undefined, ctx)
  const d2 = arg(node, 'd2', -1, undefined, ctx)
  if (d1 !== undefined) low = finiteNumber(d1, ctx, node.p, 'd1') / 2
  if (d2 !== undefined) high = finiteNumber(d2, ctx, node.p, 'd2') / 2
  if (low === undefined && high === undefined) {
    const base = diameter !== undefined ? finiteNumber(diameter, ctx, node.p, 'diameter') / 2
      : radius !== undefined ? finiteNumber(radius, ctx, node.p, 'radius') : 1
    low = high = base
  }
  if (low === undefined) low = high
  if (high === undefined) high = low
  const r1 = finiteNumber(low, ctx, node.p, 'r1')
  const r2 = finiteNumber(high, ctx, node.p, 'r2')
  if (r1 < 0 || r2 < 0 || (r1 === 0 && r2 === 0)) evaluationError(ctx, node.p, 'Cylinder radii must be non-negative and not both zero')
  const center = arg(node, 'center', 3, false, ctx) === true
  const fn = segments(node, ctx, 32, 3)
  let geometry: ManifoldGeometry
  if (r1 > 0) geometry = ctx.wasm.Manifold.cylinder(height, r1, r2, fn, center)
  else {
    geometry = ctx.wasm.Manifold.cylinder(height, r2, 0, fn, center).mirror([0, 0, 1])
    if (!center) geometry = geometry.translate([0, 0, height])
  }
  return [trackedSolid(geometry, nextColor(), node, ctx)]
}

function makePolyhedron(node: CallNode, ctx: EvalContext): Shape[] {
  const points = arg(node, 'points', 0, [], ctx)
  const faces = arg(node, 'faces', 1, arg(node, 'triangles', 1, [], ctx), ctx)
  if (!Array.isArray(points) || !Array.isArray(faces)) evaluationError(ctx, node.p, 'polyhedron points and faces must be vectors')
  const vertices: number[] = []
  for (const point of points) {
    const vector = vectorValue(point, ctx, node.p, 'polyhedron point')
    if (vector.length < 3) evaluationError(ctx, node.p, 'Each polyhedron point needs three coordinates')
    vertices.push(vector[0], vector[1], vector[2])
  }
  const indices: number[] = []
  for (const face of faces) {
    const polygon = vectorValue(face, ctx, node.p, 'polyhedron face').map(Math.trunc)
    if (polygon.length < 3) evaluationError(ctx, node.p, 'Each polyhedron face needs at least three vertices')
    for (const index of polygon) if (index < 0 || index >= points.length) evaluationError(ctx, node.p, 'Polyhedron face index is out of bounds')
    // OpenSCAD faces are wound clockwise viewed from outside; Manifold
    // requires counter-clockwise — reverse the fan so spec-correct
    // polyhedra build outward-facing instead of inside-out.
    for (let i = 1; i < polygon.length - 1; i++) indices.push(polygon[0], polygon[i + 1], polygon[i])
  }
  try {
    const mesh = new ctx.wasm.Mesh({ numProp: 3, vertProperties: new Float32Array(vertices), triVerts: new Uint32Array(indices) })
    // Weld duplicated coordinates first: OpenSCAD accepts point lists with
    // repeated positions, but Manifold's halfedge pairing rejects them.
    mesh.merge()
    return [trackedSolid(ctx.wasm.Manifold.ofMesh(mesh), nextColor(), node, ctx)]
  } catch (error) {
    evaluationError(ctx, node.p, `Invalid manifold polyhedron: ${error instanceof Error ? error.message : String(error)}`)
  }
}

function makePolygon(node: CallNode, ctx: EvalContext): Shape[] {
  const pointsValue = arg(node, 'points', 0, [], ctx)
  if (!Array.isArray(pointsValue)) evaluationError(ctx, node.p, 'polygon points must be a vector')
  const points = pointsValue.map(point => {
    const vector = vectorValue(point, ctx, node.p, 'polygon point')
    if (vector.length < 2) evaluationError(ctx, node.p, 'Each polygon point needs two coordinates')
    return [vector[0], vector[1]] as Vec2
  })
  const pathsValue = arg(node, 'paths', 1, undefined, ctx)
  let polygons: Polygons = points
  if (pathsValue !== undefined) {
    if (!Array.isArray(pathsValue)) evaluationError(ctx, node.p, 'polygon paths must be a vector')
    polygons = pathsValue.map(path => vectorValue(path, ctx, node.p, 'polygon path').map(index => {
      const point = points[Math.trunc(index)]
      if (!point) evaluationError(ctx, node.p, 'Polygon path index is out of bounds')
      return point
    }))
  }
  // EvenOdd: the default Positive fill rule silently yields an EMPTY shape
  // for clockwise-wound point lists, which are perfectly valid in OpenSCAD.
  return [{
    dimension: 2,
    geometry: ctx.wasm.CrossSection.ofPolygons(polygons, 'EvenOdd'),
    color: nextColor(),
    entityId: currentEntityId(ctx),
  }]
}

function rotateShapes(shapes: Shape[], node: CallNode, ctx: EvalContext): Shape[] {
  const angle = arg(node, 'a', 0, 0, ctx)
  const axis = arg(node, 'v', 1, undefined, ctx)
  return shapes.map(shape => {
    if (shape.dimension === 2) {
      const degrees = Array.isArray(angle) ? vectorValue(angle, ctx, node.p, 'rotation')[2] ?? 0 : finiteNumber(angle, ctx, node.p, 'rotation')
      return { ...shape, geometry: shape.geometry.rotate(degrees) }
    }
    if (Array.isArray(angle)) {
      const vector = vectorValue(angle, ctx, node.p, 'rotation')
      return { ...shape, geometry: shape.geometry.rotate([vector[0] ?? 0, vector[1] ?? 0, vector[2] ?? 0]) }
    }
    const degrees = finiteNumber(angle, ctx, node.p, 'rotation')
    if (axis === undefined) return { ...shape, geometry: shape.geometry.rotate([0, 0, degrees]) }
    const vector = vectorValue(axis, ctx, node.p, 'rotation axis')
    return { ...shape, geometry: shape.geometry.transform(axisAngleMatrix(vector, degrees, ctx, node.p)) }
  })
}

function axisAngleMatrix(axis: number[], degrees: number, ctx: EvalContext, p: number): ManifoldMatrix {
  const length = Math.hypot(axis[0] ?? 0, axis[1] ?? 0, axis[2] ?? 0)
  if (length === 0) evaluationError(ctx, p, 'Rotation axis cannot be zero')
  const x = (axis[0] ?? 0) / length, y = (axis[1] ?? 0) / length, z = (axis[2] ?? 0) / length
  const angle = degrees * Math.PI / 180, c = Math.cos(angle), s = Math.sin(angle), t = 1 - c
  // Manifold matrices are column-major.
  return [
    t*x*x+c, t*x*y+s*z, t*x*z-s*y, 0,
    t*x*y-s*z, t*y*y+c, t*y*z+s*x, 0,
    t*x*z+s*y, t*y*z-s*x, t*z*z+c, 0,
    0, 0, 0, 1,
  ]
}

function transformByMatrix(shapes: Shape[], value: Value, node: CallNode, ctx: EvalContext): Shape[] {
  if (!Array.isArray(value) || value.length < 3) evaluationError(ctx, node.p, 'multmatrix requires a 4x4 matrix')
  const rows = value.map(row => vectorValue(row, ctx, node.p, 'matrix row'))
  if (rows.some(row => row.length < 4)) evaluationError(ctx, node.p, 'multmatrix requires a 4x4 matrix')
  const matrix: number[] = []
  for (let column = 0; column < 4; column++) for (let row = 0; row < 4; row++) matrix.push(rows[row]?.[column] ?? (row === column ? 1 : 0))
  return shapes.map(shape => {
    if (shape.dimension !== 3) evaluationError(ctx, node.p, 'multmatrix currently supports 3D children only')
    return { ...shape, geometry: shape.geometry.transform(matrix as ManifoldMatrix) }
  })
}

function booleanShapes(shapes: Shape[], operation: 'union' | 'intersection', ctx: EvalContext, p: number): Shape[] {
  if (shapes.length === 0) return []
  const dimension = shapes[0].dimension
  if (shapes.some(shape => shape.dimension !== dimension)) evaluationError(ctx, p, `${operation}() cannot mix 2D and 3D children`)
  if (shapes.length === 1) return shapes
  if (dimension === 3) {
    const solids = shapes.map(shape => (shape as Shape3D).geometry)
    const geometry = operation === 'union' ? ctx.wasm.Manifold.union(solids) : ctx.wasm.Manifold.intersection(solids)
    return [{ dimension: 3, geometry, color: shapes[0].color, entityId: currentEntityId(ctx) }]
  }
  const sections = shapes.map(shape => (shape as Shape2D).geometry)
  const geometry = operation === 'union' ? ctx.wasm.CrossSection.union(sections) : ctx.wasm.CrossSection.intersection(sections)
  return [{ dimension: 2, geometry, color: shapes[0].color, entityId: currentEntityId(ctx) }]
}

function differenceChildren(node: CallNode, ctx: EvalContext): Shape[] {
  if (node.children.length === 0) return []
  const base = booleanShapes(evalNodes([node.children[0]], ctx), 'union', ctx, node.p)
  const cutters = booleanShapes(evalNodes(node.children.slice(1), ctx), 'union', ctx, node.p)
  if (base.length === 0 || cutters.length === 0) return base
  if (base[0].dimension !== cutters[0].dimension) evaluationError(ctx, node.p, 'difference() cannot mix 2D and 3D children')
  if (base[0].dimension === 3) {
    return [{
      dimension: 3,
      geometry: base[0].geometry.subtract((cutters[0] as Shape3D).geometry),
      color: base[0].color,
      entityId: currentEntityId(ctx),
    }]
  }
  return [{
    dimension: 2,
    geometry: base[0].geometry.subtract((cutters[0] as Shape2D).geometry),
    color: base[0].color,
    entityId: currentEntityId(ctx),
  }]
}

function hullShapes(shapes: Shape[], node: CallNode, ctx: EvalContext): Shape[] {
  if (shapes.length === 0) return []
  const dimension = shapes[0].dimension
  if (shapes.some(shape => shape.dimension !== dimension)) evaluationError(ctx, node.p, 'hull() cannot mix 2D and 3D children')
  if (dimension === 3) {
    const geometry = ctx.wasm.Manifold.hull(shapes.map(shape => (shape as Shape3D).geometry))
    return [trackedSolid(geometry, shapes[0].color, node, ctx)]
  }
  return [{
    dimension: 2,
    geometry: ctx.wasm.CrossSection.hull(shapes.map(shape => (shape as Shape2D).geometry)),
    color: shapes[0].color,
    entityId: currentEntityId(ctx),
  }]
}

function linearExtrude(node: CallNode, ctx: EvalContext): Shape[] {
  const sections = booleanShapes(evalNodes(node.children, ctx), 'union', ctx, node.p)
  if (sections.length === 0) return []
  if (sections[0].dimension !== 2) evaluationError(ctx, node.p, 'linear_extrude() requires 2D children')
  const height = finiteNumber(arg(node, 'height', 0, 1, ctx), ctx, node.p, 'extrusion height')
  if (height <= 0) evaluationError(ctx, node.p, 'linear_extrude() height must be positive')
  const twist = finiteNumber(arg(node, 'twist', -1, 0, ctx), ctx, node.p, 'extrusion twist')
  // Cap slices: the value goes straight into the Manifold kernel, which
  // allocates per-slice cross-sections long before MAX_TRIANGLES can fire.
  const slices = Math.min(MAX_EXTRUDE_SLICES, Math.max(0, Math.trunc(finiteNumber(arg(node, 'slices', -1, 0, ctx), ctx, node.p, 'extrusion slices'))))
  const rawScale = arg(node, 'scale', -1, [1, 1], ctx)
  const scaleValues = Array.isArray(rawScale) ? vectorValue(rawScale, ctx, node.p, 'extrusion scale') : [finiteNumber(rawScale, ctx, node.p, 'extrusion scale')]
  const scale: Vec2 = [scaleValues[0] ?? 1, scaleValues[1] ?? scaleValues[0] ?? 1]
  try {
    const geometry = ctx.wasm.Manifold.extrude(sections[0].geometry, height, slices, twist, scale, arg(node, 'center', -1, false, ctx) === true)
    return [trackedSolid(geometry, sections[0].color, node, ctx)]
  } catch (error) {
    evaluationError(ctx, node.p, `linear_extrude() failed: ${error instanceof Error ? error.message : String(error)}`)
  }
}

function rotateExtrude(node: CallNode, ctx: EvalContext): Shape[] {
  const sections = booleanShapes(evalNodes(node.children, ctx), 'union', ctx, node.p)
  if (sections.length === 0) return []
  if (sections[0].dimension !== 2) evaluationError(ctx, node.p, 'rotate_extrude() requires 2D children')
  const angle = finiteNumber(arg(node, 'angle', -1, 360, ctx), ctx, node.p, 'revolve angle')
  try {
    const geometry = ctx.wasm.Manifold.revolve(sections[0].geometry, segments(node, ctx, 48, 3), angle)
    return [trackedSolid(geometry, sections[0].color, node, ctx)]
  } catch (error) {
    evaluationError(ctx, node.p, `rotate_extrude() failed: ${error instanceof Error ? error.message : String(error)}`)
  }
}

function evalFor(node: CallNode, ctx: EvalContext): Shape[] {
  const entries = Object.entries(node.args).filter(([name]) => !name.startsWith('_'))
  if (entries.length !== 1) evaluationError(ctx, node.p, 'for() currently requires one named iterator')
  const [name, expression] = entries[0]
  const values = evalExpression(expression, ctx)
  if (!Array.isArray(values)) evaluationError(ctx, node.p, 'for() iterator must be a vector or range')
  const output: Shape[] = []
  const occurrences = new Map<string, number>()
  for (const value of values) {
    // Count each iteration even when the body produces no statements/shapes —
    // nested empty-bodied loops are otherwise invisible to every other limit.
    if (++ctx.budget.ops > MAX_EVAL_OPS) evaluationError(ctx, node.p, `Model exceeds the ${MAX_EVAL_OPS.toLocaleString()} evaluation step limit`)
    const env = new Map(ctx.env)
    env.set(name, value)
    const valueKey = encodeURIComponent(identityValue(value))
    const occurrence = occurrences.get(valueKey) ?? 0
    occurrences.set(valueKey, occurrence + 1)
    output.push(...evalNodes(node.children, {
      ...ctx,
      env,
      instancePath: `${ctx.instancePath}>loop:${encodeURIComponent(name)}=${valueKey}#${occurrence}`,
    }, false))
    if (output.length > MAX_SHAPES) evaluationError(ctx, node.p, `Model exceeds the ${MAX_SHAPES.toLocaleString()} object limit`)
  }
  return output
}

function identityValue(value: Value): string {
  if (Array.isArray(value)) return `[${value.map(identityValue).join(',')}]`
  if (value === undefined) return 'undef'
  if (typeof value === 'string') return JSON.stringify(value)
  return String(value)
}

function evalUserModule(call: CallNode, module: ModuleNode, ctx: EvalContext): Shape[] {
  const env = new Map(ctx.env)
  const positional = Object.entries(call.args).filter(([name]) => name.startsWith('_')).sort(([a], [b]) => Number(a.slice(1)) - Number(b.slice(1)))
  for (let i = 0; i < module.params.length; i++) {
    const param = module.params[i]
    const expression = call.args[param.name] ?? positional[i]?.[1] ?? param.defaultValue
    env.set(param.name, expression ? evalExpression(expression, { ...ctx, env }) : undefined)
  }
  return evalNodes(module.children, { ...ctx, env, callChildren: call.children }, false)
}

function parseColor(value: Value, ctx: EvalContext, p: number): RGBA {
  if (Array.isArray(value)) {
    const channels = vectorValue(value, ctx, p, 'color')
    return [clamp01(channels[0] ?? 0.5), clamp01(channels[1] ?? 0.5), clamp01(channels[2] ?? 0.5), clamp01(channels[3] ?? 1)]
  }
  if (typeof value !== 'string') evaluationError(ctx, p, 'color() expects a name or RGB(A) vector')
  const named = CSS_COLORS[value.toLowerCase()]
  if (named) return [...named]
  const match = value.match(/^#([0-9a-f]{6}|[0-9a-f]{8})$/i)
  if (match) {
    const hex = match[1]
    return [parseInt(hex.slice(0, 2), 16) / 255, parseInt(hex.slice(2, 4), 16) / 255, parseInt(hex.slice(4, 6), 16) / 255, hex.length === 8 ? parseInt(hex.slice(6, 8), 16) / 255 : 1]
  }
  evaluationError(ctx, p, `Unknown color ${value}`)
}
function clamp01(value: number) { return Math.max(0, Math.min(1, value)) }

let wasmPromise: Promise<ManifoldToplevel> | undefined
function getWasm() {
  if (!wasmPromise) {
    wasmPromise = Module().then(module => {
      module.setup()
      return garbageCollectManifold(module)
    })
  }
  return wasmPromise
}

async function parseInternal(source: string, options: ParseOptions): Promise<ParseResult> {
  if (source.length > MAX_SOURCE_LENGTH) throw new OpenSCADParseError(source, 0, `Source exceeds ${MAX_SOURCE_LENGTH.toLocaleString()} characters`)
  paletteIndex = 0
  const ast = new Parser(tokenize(source), source).parseAll()
  assignOperationIds(ast)
  const wasm = await getWasm()
  const warnings: string[] = []
  const modules = new Map<string, ModuleNode>()
  collectModules(ast, modules)
  const env = new Map<string, Value>([['$fn', 0], ['$fa', 12], ['$fs', 2]])
  const quality = options.quality ?? 'full'
  const sourceReferences = new Map<number, MeshSourceReference>()
  const ctx: EvalContext = {
    wasm,
    source,
    env,
    modules,
    warnings,
    quality,
    sourceReferences,
    depth: 0,
    budget: { ops: 0 },
    instancePath: 'root',
    valueBudget: { used: 0 },
    valueWeights: new WeakMap(),
    valueDepths: new WeakMap(),
  }

  try {
    const shapes = evalNodes(ast, ctx, false)
    const sections = shapes.filter(shape => shape.dimension === 2)
    if (sections.length) warn(ctx, `${sections.length} top-level 2D object(s) are not displayed; wrap them in linear_extrude() or rotate_extrude()`)
    const solids = shapes.filter((shape): shape is Shape3D => shape.dimension === 3 && !shape.geometry.isEmpty())
    const meshes: MeshData[] = []
    let volume = 0
    let surfaceArea = 0
    let triangleCount = 0
    for (const shape of solids) {
      volume += shape.geometry.volume()
      surfaceArea += shape.geometry.surfaceArea()
      const withNormals = shape.geometry.calculateNormals(0, 52.5)
      const mesh = withNormals.getMesh()
      triangleCount += mesh.numTri
      if (triangleCount > MAX_TRIANGLES) evaluationError(ctx, 0, `Rendered model exceeds ${MAX_TRIANGLES.toLocaleString()} triangles`)
      if (mesh.numProp < 6) evaluationError(ctx, 0, 'Geometry kernel did not produce normals')
      const vertices = new Float32Array(mesh.numVert * 6)
      for (let vertex = 0; vertex < mesh.numVert; vertex++) {
        const sourceOffset = vertex * mesh.numProp
        const targetOffset = vertex * 6
        for (let channel = 0; channel < 6; channel++) vertices[targetOffset + channel] = mesh.vertProperties[sourceOffset + channel]
      }
      const indices = new Uint32Array(mesh.triVerts)
      const bvh = buildMeshBvh(vertices, indices)
      const semanticEdges = extractSemanticEdges(vertices, indices, {
        creaseAngleDegrees: 30,
        mergeFromVert: mesh.mergeFromVert,
        mergeToVert: mesh.mergeToVert,
      })
      const provenance: MeshProvenanceRun[] = []
      for (let run = 0; run < mesh.runOriginalID.length; run++) {
        const triangleStart = (mesh.runIndex[run] ?? 0) / 3
        const triangleEnd = (mesh.runIndex[run + 1] ?? mesh.triVerts.length) / 3
        if (triangleEnd <= triangleStart) continue
        const originalId = mesh.runOriginalID[run]
        provenance.push({
          triangleStart,
          triangleEnd,
          source: sourceReferences.get(originalId) ?? null,
          backside: ((mesh.runFlags[run] ?? 0) & 1) !== 0,
        })
      }
      if (provenance.length === 0 && mesh.numTri > 0) {
        provenance.push({ triangleStart: 0, triangleEnd: mesh.numTri, source: null, backside: false })
      }
      meshes.push({
        entityId: shape.entityId,
        vertices,
        indices,
        bvh,
        edgeIndices: semanticEdges.indices,
        color: [...shape.color],
        transform: identity(),
        faceIds: new Uint32Array(mesh.faceID),
        provenance,
        topology: semanticEdges.diagnostics,
      })
    }
    return { meshes, warnings, volume, surfaceArea, quality }
  } finally {
    cleanupManifold()
  }
}

let parseQueue: Promise<void> = Promise.resolve()

/** Parse and evaluate the supported OpenSCAD subset in a serialized WASM scope. */
export function parseOpenSCAD(source: string, options: ParseOptions = {}): Promise<ParseResult> {
  const result = parseQueue.then(() => parseInternal(source, options), () => parseInternal(source, options))
  parseQueue = result.then(() => undefined, () => undefined)
  return result
}
