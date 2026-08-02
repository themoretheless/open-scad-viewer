import type { SourceOperationId } from '../core/mesh'
import type { LanguageDiagnosticCode } from '../core/languageContract'
import { OpenSCADParseError } from './openscadErrors'

const MAX_AST_NODES = 25_000
const MAX_EXPRESSION_DEPTH = 256
const MAX_STATEMENT_DEPTH = 128

export type Value = number | string | boolean | undefined | Value[]

export enum TT {
  Num, Str, Ident,
  LParen, RParen, LBrace, RBrace, LBracket, RBracket,
  Comma, Semi, Eq, Plus, Minus, Star, Slash, Percent, Caret,
  Hash, Dollar, Dot, Colon, Question,
  Lt, Gt, LtEq, GtEq, EqEq, NotEq, Not, And, Or,
  Eof,
}

interface Token { t: TT; v: string; p: number; end: number }

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

export type Expr =
  | { kind: 'literal'; value: Value; p: number }
  | { kind: 'identifier'; name: string; p: number }
  | { kind: 'vector'; items: Expr[]; p: number }
  | { kind: 'range'; start: Expr; step?: Expr; end: Expr; p: number }
  | { kind: 'unary'; op: TT; value: Expr; p: number }
  | { kind: 'binary'; op: TT; left: Expr; right: Expr; p: number }
  | { kind: 'ternary'; test: Expr; yes: Expr; no: Expr; p: number }
  | { kind: 'call'; name: string; args: Expr[]; p: number }
  | { kind: 'index'; value: Expr; index: Expr; p: number }

export interface CallNode {
  type: 'call'
  name: string
  args: Record<string, Expr>
  argKinds: Record<string, 'named' | 'positional'>
  argSpans: Record<string, { start: number; end: number }>
  children: Statement[]
  alternative: Statement[]
  p: number
  end: number
  operationId?: SourceOperationId
}

export interface AssignNode { type: 'assign'; name: string; value: Expr; p: number; end: number }
export interface ModuleParam { name: string; defaultValue?: Expr }
export interface ModuleNode {
  type: 'module'
  name: string
  params: ModuleParam[]
  children: Statement[]
  p: number
  end: number
}
export type Statement = CallNode | AssignNode | ModuleNode

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
  private fail(token: Token, message: string, code?: LanguageDiagnosticCode): never {
    throw new OpenSCADParseError(this.source, token.p, message, code, token.end)
  }
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
    if (this.statementDepth >= MAX_STATEMENT_DEPTH) {
      this.fail(this.peek(), `Model exceeds ${MAX_STATEMENT_DEPTH} nested statements`)
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
      const modifier = this.advance()
      if (modifier.t === TT.Star) disabled = true
      else this.fail(modifier, `Viewport modifier ${modifier.v} is not supported by ${'openscad-viewer-subset@1'}`, 'E_FEATURE_VIEWPORT_MODIFIER')
    }
    if (this.peek().t !== TT.Ident) this.fail(this.peek(), 'Expected a variable, module, or geometry call')

    let node: Statement
    if (this.peek().v === 'module') node = this.moduleDefinition()
    else if (this.peek().v === 'function' || this.peek().v === 'include' || this.peek().v === 'use') {
      const feature = this.peek().v
      const code = feature === 'function' ? 'E_FEATURE_USER_FUNCTION'
        : feature === 'include' ? 'E_FEATURE_INCLUDE' : 'E_FEATURE_USE'
      this.fail(this.peek(), `${feature} is not supported by openscad-viewer-subset@1`, code)
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
    const argKinds: CallNode['argKinds'] = {}
    const argSpans: CallNode['argSpans'] = {}
    if (this.match(TT.LParen)) {
      let positional = 0
      while (this.peek().t !== TT.RParen) {
        let key: string
        let kind: CallNode['argKinds'][string]
        if (this.peek().t === TT.Ident && this.peek(1).t === TT.Eq) {
          key = this.advance().v
          this.advance()
          kind = 'named'
        } else {
          key = `_${positional++}`
          kind = 'positional'
        }
        if (args[key]) this.fail(this.peek(), `Duplicate argument ${key}`)
        const start = this.peek().p
        args[key] = this.expression()
        argKinds[key] = kind
        argSpans[key] = { start, end: this.lastTokenEnd }
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
    return { type: 'call', name: name.v, args, argKinds, argSpans, children, alternative, p: name.p, end: this.lastTokenEnd }
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


/** Pure compiler front-end: source text to a typed operation IR. */
export function compileOpenSCAD(source: string): readonly Statement[] {
  const statements = new Parser(tokenize(source), source).parseAll()
  assignOperationIds(statements)
  return deepFreeze(statements)
}

function deepFreeze<T>(value: T): T {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    for (const child of Object.values(value as Record<string, unknown>)) deepFreeze(child)
    Object.freeze(value)
  }
  return value
}
