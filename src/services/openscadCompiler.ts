import type { SourceOperationId } from '../core/mesh'
import type { LanguageDiagnosticCode } from '../core/languageContract'
import { OpenSCADParseError } from './openscadErrors'

const MAX_AST_NODES = 25_000
const MAX_EXPRESSION_DEPTH = 256
const MAX_STATEMENT_DEPTH = 128

export const OPENSCAD_LANGUAGE_PROFILES = Object.freeze([
  'openscad-viewer-subset@1',
  'openscad/stable-2021.01',
] as const)
export type OpenScadLanguageProfile = typeof OPENSCAD_LANGUAGE_PROFILES[number]
export interface CompileOpenScadOptions {
  readonly languageProfile?: OpenScadLanguageProfile
}

export type ScalarValue = number | string | boolean | undefined

/**
 * Runtime representation of an OpenSCAD function value.  It deliberately
 * contains only language data (AST + an environment snapshot), never a
 * JavaScript function, so values cannot smuggle executable host code into the
 * evaluator.
 */
export interface FunctionValue {
  readonly kind: 'function-value'
  readonly name: string | null
  readonly params: readonly FunctionParam[]
  readonly body: Expr
  readonly closure: ReadonlyMap<string, Value>
}

/**
 * A range stays lazy at runtime in the full 2021.01 engine.  Keeping it
 * distinct from vectors is required for OpenSCAD's range-specific behaviour
 * (including `is_list()` and delayed expansion by consumers).
 */
export interface RangeValue {
  readonly kind: 'range-value'
  readonly start: number
  readonly step: number
  readonly end: number
}

export type Value = ScalarValue | Value[] | FunctionValue | RangeValue

export enum TT {
  Num, Str, Ident,
  LParen, RParen, LBrace, RBrace, LBracket, RBracket,
  Comma, Semi, Eq, Plus, Minus, Star, Slash, Percent, Caret,
  Hash, Dollar, Dot, Colon, Question,
  Lt, Gt, LtEq, GtEq, EqEq, NotEq, Not, And, Or,
  Eof, DirectivePath,
}

interface Token { t: TT; v: string; p: number; end: number; closed?: boolean }

function tokenize(source: string): Token[] {
  const out: Token[] = []
  let i = 0
  let pendingDirective = false
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
    if (pendingDirective && ch === '<') {
      i++
      const pathStart = i
      while (i < source.length && source[i] !== '>') i++
      const closed = i < source.length
      const path = source.slice(pathStart, i)
      if (closed) i++
      out.push({ t: TT.DirectivePath, v: path, p, end: i, closed })
      pendingDirective = false
      continue
    }
    pendingDirective = false
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
      pendingDirective = value === 'include' || value === 'use'
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

export interface LetExpression {
  readonly kind: 'let'
  /** Ordered call arguments; OpenSCAD permits positional/named interleaving. */
  readonly args: ExpressionArgument[]
  readonly body: Expr
  readonly p: number
}

export interface AssertExpression {
  readonly kind: 'assert'
  readonly args: ExpressionArgument[]
  /** Absence is different from an explicit `undef` body. */
  readonly body?: Expr
  readonly p: number
}

export interface EchoExpression {
  readonly kind: 'echo'
  readonly args: ExpressionArgument[]
  /** Absence is different from an explicit `undef` body. */
  readonly body?: Expr
  readonly p: number
}

export interface ListComprehensionForExpression {
  readonly kind: 'lc-for'
  readonly args: ExpressionArgument[]
  readonly body: Expr
  readonly p: number
}

export interface ListComprehensionCForExpression {
  readonly kind: 'lc-for-c'
  readonly init: ExpressionArgument[]
  readonly condition: Expr
  readonly update: ExpressionArgument[]
  readonly body: Expr
  readonly p: number
}

export interface ListComprehensionIfExpression {
  readonly kind: 'lc-if'
  readonly condition: Expr
  readonly yes: Expr
  readonly no?: Expr
  readonly p: number
}

export interface ListComprehensionLetExpression {
  readonly kind: 'lc-let'
  readonly args: ExpressionArgument[]
  readonly body: ListComprehensionExpression
  readonly p: number
}

export interface ListComprehensionEachExpression {
  readonly kind: 'lc-each'
  readonly value: Expr
  readonly p: number
}

export type ListComprehensionExpression =
  | ListComprehensionForExpression
  | ListComprehensionCForExpression
  | ListComprehensionIfExpression
  | ListComprehensionLetExpression
  | ListComprehensionEachExpression

export type Expr =
  | { kind: 'literal'; value: Value; p: number }
  | { kind: 'identifier'; name: string; p: number }
  | { kind: 'vector'; items: Expr[]; p: number }
  | { kind: 'range'; start: Expr; step?: Expr; end: Expr; p: number }
  | { kind: 'unary'; op: TT; value: Expr; p: number }
  | { kind: 'binary'; op: TT; left: Expr; right: Expr; p: number }
  | { kind: 'ternary'; test: Expr; yes: Expr; no: Expr; p: number }
  | { kind: 'function'; params: FunctionParam[]; body: Expr; p: number }
  | {
    kind: 'call'
    /** Fast-path name for an identifier callee; null for a computed callee. */
    name: string | null
    callee: Expr
    args: ExpressionArgument[]
    p: number
  }
  | { kind: 'index'; value: Expr; index: Expr; p: number }
  | { kind: 'member'; value: Expr; name: string; p: number }
  | LetExpression
  | AssertExpression
  | EchoExpression
  | ListComprehensionExpression

export interface ExpressionArgument {
  readonly name?: string
  readonly value: Expr
  readonly p: number
  readonly end: number
}

export const OPENSCAD_VIEWPORT_MODIFIER_KINDS = Object.freeze([
  'disable',
  'highlight',
  'background',
  'root',
] as const)
export type OpenScadViewportModifierKind = typeof OPENSCAD_VIEWPORT_MODIFIER_KINDS[number]

/**
 * One authored viewport-modifier token.  The ordered token list is retained
 * instead of being collapsed to flags: chains are legal in OpenSCAD and the
 * exact token spans remain useful to editors and project-expansion errors.
 */
export interface OpenScadViewportModifier {
  readonly kind: OpenScadViewportModifierKind
  readonly token: '*' | '#' | '%' | '!'
  readonly span: { readonly start: number; readonly end: number }
  /** Defining project source file; absent for single-source compilation. */
  readonly sourcePath?: string
}

export function hasOpenScadViewportModifier(
  node: Pick<CallNode, 'viewportModifiers'>,
  kind: OpenScadViewportModifierKind,
): boolean {
  return node.viewportModifiers?.some(modifier => modifier.kind === kind) ?? false
}

export interface CallNode {
  type: 'call'
  name: string
  /** Complete authored order, including duplicate named arguments. */
  callArguments?: ExpressionArgument[]
  /** Last-value compatibility index used by legacy and simple call sites. */
  args: Record<string, Expr>
  argKinds: Record<string, 'named' | 'positional'>
  argSpans: Record<string, { start: number; end: number }>
  children: Statement[]
  alternative: Statement[]
  p: number
  end: number
  /** Defining project source file; absent for single-source compilation. */
  sourcePath?: string
  /** Present only when the full-profile source authored viewport modifiers. */
  viewportModifiers?: OpenScadViewportModifier[]
  operationId?: SourceOperationId
}

export interface AssignNode { type: 'assign'; name: string; value: Expr; p: number; end: number }
export interface ModuleParam { name: string; defaultValue?: Expr }
export type FunctionParam = ModuleParam
export interface ModuleNode {
  type: 'module'
  name: string
  params: ModuleParam[]
  children: Statement[]
  p: number
  end: number
}
export interface FunctionNode {
  type: 'function'
  name: string
  params: FunctionParam[]
  body: Expr
  p: number
  end: number
}
export type Statement = CallNode | AssignNode | ModuleNode | FunctionNode

/** A file dependency retained only by the full-profile compiler front-end. */
export interface DirectiveNode {
  readonly type: 'directive'
  readonly directive: 'include' | 'use'
  readonly path: string
  readonly pathSpan: { readonly start: number; readonly end: number }
  readonly p: number
  readonly end: number
  /**
   * A modifier immediately before include/use is carried through textual
   * project expansion until it reaches the first following statement.
   */
  readonly viewportModifiers?: readonly OpenScadViewportModifier[]
}

/** Full-profile parse output before a project compiler resolves dependencies. */
export type OpenScadParsedStatement = Statement | DirectiveNode

class Parser {
  private pos = 0
  private nodes = 0
  private expressionDepth = 0
  private statementDepth = 0
  private lastTokenEnd = 0

  constructor(
    private readonly tokens: Token[],
    private readonly source: string,
    private readonly languageProfile: OpenScadLanguageProfile,
  ) {}

  parseAll(): Statement[] {
    const result: Statement[] = []
    while (this.peek().t !== TT.Eof) {
      result.push(...this.statement())
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

  private statement(): Statement[] {
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

  private parseStatement(): Statement[] {
    if (this.match(TT.Semi)) return []
    if (this.peek().t === TT.LBrace) {
      if (this.languageProfile === 'openscad-viewer-subset@1') {
        this.fail(this.peek(), 'Expected a variable, module, or geometry call')
      }
      this.advance()
      this.countNode()
      const statements: Statement[] = []
      while (this.peek().t !== TT.RBrace && this.peek().t !== TT.Eof) {
        statements.push(...this.statement())
      }
      this.expect(TT.RBrace, 'Expected }')
      return statements
    }
    let disabled = false
    const viewportModifiers: OpenScadViewportModifier[] = []
    while ([TT.Hash, TT.Percent, TT.Star, TT.Not].includes(this.peek().t)) {
      const modifier = this.advance()
      if (this.languageProfile === 'openscad-viewer-subset@1') {
        if (modifier.t === TT.Star) disabled = true
        else this.fail(modifier, `Viewport modifier ${modifier.v} is not supported by ${'openscad-viewer-subset@1'}`, 'E_FEATURE_VIEWPORT_MODIFIER')
        continue
      }
      const kind: OpenScadViewportModifierKind = modifier.t === TT.Star
        ? 'disable'
        : modifier.t === TT.Hash
          ? 'highlight'
          : modifier.t === TT.Percent
            ? 'background'
            : 'root'
      viewportModifiers.push({
        kind,
        token: modifier.v as OpenScadViewportModifier['token'],
        span: { start: modifier.p, end: modifier.end },
      })
    }
    if (this.peek().t !== TT.Ident) this.fail(this.peek(), 'Expected a variable, module, or geometry call')

    let node: OpenScadParsedStatement
    if (this.peek().v === 'module') node = this.moduleDefinition()
    else if (this.peek().v === 'function') {
      if (this.languageProfile === 'openscad-viewer-subset@1') {
        this.fail(
          this.peek(),
          'function is not supported by openscad-viewer-subset@1',
          'E_FEATURE_USER_FUNCTION',
        )
      }
      node = this.functionDefinition()
    }
    else if (this.peek().v === 'include' || this.peek().v === 'use') {
      const feature = this.peek().v
      if (this.languageProfile === 'openscad-viewer-subset@1') {
        const code = feature === 'include' ? 'E_FEATURE_INCLUDE' : 'E_FEATURE_USE'
        this.fail(this.peek(), `${feature} is not supported by openscad-viewer-subset@1`, code)
      }
      node = this.directive()
    } else if (this.peek(1).t === TT.Eq) node = this.assignment()
    else node = this.call()
    if (viewportModifiers.length > 0) {
      if (node.type !== 'call' && node.type !== 'directive') {
        const first = viewportModifiers[0]
        this.fail(
          { t: TT.Eof, v: first.token, p: first.span.start, end: first.span.end },
          'Viewport modifiers may prefix only a module instantiation or an include/use directive',
        )
      }
      node = { ...node, viewportModifiers }
    }
    this.countNode()
    return this.languageProfile === 'openscad-viewer-subset@1' && disabled
      ? []
      : [node as Statement]
  }

  private assignment(): AssignNode {
    const name = this.expect(TT.Ident)
    this.expect(TT.Eq)
    const value = this.expression()
    const terminator = this.expect(TT.Semi, 'Expected ; after assignment')
    return { type: 'assign', name: name.v, value, p: name.p, end: terminator.end }
  }

  private directive(): DirectiveNode {
    const keyword = this.advance()
    const path = this.expect(
      TT.DirectivePath,
      `Expected <path> after ${keyword.v}`,
    )
    if (path.closed !== true) this.fail(path, `Unterminated ${keyword.v} path`)
    if (path.v.length === 0) this.fail(path, `${keyword.v} path cannot be empty`)
    if (/[\r\n]/u.test(path.v)) this.fail(path, `${keyword.v} path cannot contain a line break`)
    this.match(TT.Semi)
    return {
      type: 'directive',
      directive: keyword.v as DirectiveNode['directive'],
      path: path.v,
      pathSpan: { start: path.p + 1, end: path.end - 1 },
      p: keyword.p,
      end: this.lastTokenEnd,
    }
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

  private functionDefinition(): FunctionNode {
    const keyword = this.advance()
    const name = this.expect(TT.Ident, 'Expected function name')
    const params = this.functionParameters()
    this.expect(TT.Eq, 'Expected = before function body')
    const body = this.expression()
    const terminator = this.expect(TT.Semi, 'Expected ; after function definition')
    return { type: 'function', name: name.v, params, body, p: keyword.p, end: terminator.end }
  }

  private functionParameters(): FunctionParam[] {
    this.expect(TT.LParen, 'Expected ( before function parameters')
    const params: FunctionParam[] = []
    const names = new Set<string>()
    while (this.peek().t !== TT.RParen) {
      const param = this.expect(TT.Ident, 'Expected parameter name')
      if (names.has(param.v)) this.fail(param, `Duplicate parameter ${param.v}`)
      names.add(param.v)
      const defaultValue = this.match(TT.Eq) ? this.expression() : undefined
      params.push({ name: param.v, defaultValue })
      if (!this.match(TT.Comma)) break
      if (this.peek().t === TT.RParen) break
    }
    this.expect(TT.RParen, 'Expected ) after function parameters')
    return params
  }

  private call(): CallNode {
    const name = this.expect(TT.Ident)
    const callArguments: ExpressionArgument[] = []
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
        if (args[key] && this.languageProfile !== 'openscad/stable-2021.01') {
          this.fail(this.peek(), `Duplicate argument ${key}`)
        }
        const start = this.peek().p
        const value = this.expression()
        args[key] = value
        argKinds[key] = kind
        argSpans[key] = { start, end: this.lastTokenEnd }
        callArguments.push({
          ...(kind === 'named' ? { name: key } : {}),
          value,
          p: start,
          end: this.lastTokenEnd,
        })
        if (!this.match(TT.Comma) && this.peek().t !== TT.RParen) this.fail(this.peek(), 'Expected , or )')
      }
      this.advance()
    } else if (this.languageProfile === 'openscad/stable-2021.01') {
      this.fail(this.peek(), `Expected ( after module name ${name.v}`)
    }

    const children = this.body(false)
    let alternative: Statement[] = []
    if (name.v === 'if' && this.peek().t === TT.Ident && this.peek().v === 'else') {
      this.advance()
      alternative = this.body(true)
    }
    return { type: 'call', name: name.v, callArguments, args, argKinds, argSpans, children, alternative, p: name.p, end: this.lastTokenEnd }
  }

  private body(required: boolean): Statement[] {
    if (this.match(TT.LBrace)) {
      const children: Statement[] = []
      while (this.peek().t !== TT.RBrace && this.peek().t !== TT.Eof) {
        children.push(...this.statement())
      }
      this.expect(TT.RBrace, 'Expected }')
      return children
    }
    if (this.match(TT.Semi)) return []
    if (!required && [TT.RBrace, TT.Eof].includes(this.peek().t)) return []
    return this.statement()
  }

  private expression(): Expr {
    const position = this.peek().p
    return this.descendExpression(position, () => {
      if (this.peek().t === TT.Ident && this.peek(1).t === TT.LParen) {
        const keyword = this.peek().v
        if (keyword === 'function') {
          if (this.languageProfile === 'openscad-viewer-subset@1') {
            this.fail(
              this.peek(),
              'function is not supported by openscad-viewer-subset@1',
              'E_FEATURE_USER_FUNCTION',
            )
          }
          return this.anonymousFunctionExpression()
        }
        if (this.languageProfile === 'openscad/stable-2021.01'
          && (keyword === 'let' || keyword === 'assert' || keyword === 'echo')) {
          return this.wrapperExpression(keyword)
        }
      }
      return this.ternary()
    })
  }

  private anonymousFunctionExpression(): Expr {
    const keyword = this.expect(TT.Ident)
    const params = this.functionParameters()
    return this.expressionNode({ kind: 'function', params, body: this.expression(), p: keyword.p })
  }

  private wrapperExpression(keyword: 'let' | 'assert' | 'echo'): Expr {
    const start = this.expect(TT.Ident)
    this.expect(TT.LParen, `Expected ( after ${keyword}`)
    const args = this.expressionArguments(`Expected ) after ${keyword} arguments`)
    if (keyword === 'let') {
      return this.expressionNode({ kind: 'let', args, body: this.expression(), p: start.p })
    }
    const body = this.canStartExpression() ? this.expression() : undefined
    if (keyword === 'assert') {
      return body === undefined
        ? this.expressionNode({ kind: 'assert', args, p: start.p })
        : this.expressionNode({ kind: 'assert', args, body, p: start.p })
    }
    return body === undefined
      ? this.expressionNode({ kind: 'echo', args, p: start.p })
      : this.expressionNode({ kind: 'echo', args, body, p: start.p })
  }

  private canStartExpression(): boolean {
    const token = this.peek()
    if ([TT.Num, TT.Str, TT.LParen, TT.LBracket, TT.Plus, TT.Minus, TT.Not].includes(token.t)) return true
    if (token.t !== TT.Ident) return false
    if (this.languageProfile === 'openscad/stable-2021.01'
      && ['else', 'for', 'if', 'each'].includes(token.v)) return false
    return true
  }

  private ternary(): Expr {
    const test = this.binaryOr()
    if (!this.match(TT.Question)) return test
    const yes = this.expression()
    this.expect(TT.Colon, 'Expected : in conditional expression')
    return this.expressionNode({ kind: 'ternary', test, yes, no: this.expression(), p: test.p })
  }

  private binaryOr(): Expr { return this.binary(() => this.binaryAnd(), [TT.Or]) }
  private binaryAnd(): Expr {
    return this.binary(
      () => this.languageProfile === 'openscad/stable-2021.01'
        ? this.equality()
        : this.legacyComparison(),
      [TT.And],
    )
  }
  /** Frozen subset@1 level: equality and ordering operators remain combined. */
  private legacyComparison(): Expr {
    return this.binary(() => this.additive(), [TT.Lt, TT.Gt, TT.LtEq, TT.GtEq, TT.EqEq, TT.NotEq])
  }
  private equality(): Expr { return this.binary(() => this.comparison(), [TT.EqEq, TT.NotEq]) }
  private comparison(): Expr { return this.binary(() => this.additive(), [TT.Lt, TT.Gt, TT.LtEq, TT.GtEq]) }
  private additive(): Expr { return this.binary(() => this.multiplicative(), [TT.Plus, TT.Minus]) }
  private multiplicative(): Expr {
    return this.binary(
      () => this.languageProfile === 'openscad/stable-2021.01'
        ? this.fullUnary()
        : this.legacyPower(),
      [TT.Star, TT.Slash, TT.Percent],
    )
  }

  /** Frozen subset@1 precedence: retained so the versioned legacy route cannot drift. */
  private legacyPower(): Expr {
    const left = this.legacyUnary()
    if (!this.match(TT.Caret)) return left
    return this.expressionNode({
      kind: 'binary', op: TT.Caret, left,
      right: this.descendExpression(this.peek().p, () => this.legacyPower()),
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

  private legacyUnary(): Expr {
    if ([TT.Plus, TT.Minus, TT.Not].includes(this.peek().t)) {
      const op = this.advance()
      return this.expressionNode({
        kind: 'unary', op: op.t,
        value: this.descendExpression(op.p, () => this.legacyUnary()),
        p: op.p,
      })
    }
    return this.postfix()
  }

  /** OpenSCAD 2021.01 precedence: exponentiation binds tighter than unary. */
  private fullUnary(): Expr {
    if ([TT.Plus, TT.Minus, TT.Not].includes(this.peek().t)) {
      const op = this.advance()
      return this.expressionNode({
        kind: 'unary', op: op.t,
        value: this.descendExpression(op.p, () => this.fullUnary()),
        p: op.p,
      })
    }
    return this.fullPower()
  }

  private fullPower(): Expr {
    const left = this.postfix()
    if (!this.match(TT.Caret)) return left
    return this.expressionNode({
      kind: 'binary', op: TT.Caret, left,
      // Going through unary permits `2 ^ -2` and keeps `2 ^ 3 ^ 2`
      // right-associative, matching the pinned OpenSCAD grammar.
      right: this.descendExpression(this.peek().p, () => this.fullUnary()),
      p: left.p,
    })
  }

  private postfix(): Expr {
    let value = this.primary()
    while (true) {
      if (this.match(TT.LBracket)) {
        const p = value.p
        const index = this.expression()
        this.expect(TT.RBracket, 'Expected ] after index')
        value = this.expressionNode({ kind: 'index', value, index, p })
        continue
      }
      if (this.match(TT.Dot)) {
        const member = this.expect(TT.Ident, 'Expected member name after .')
        value = this.expressionNode({ kind: 'member', value, name: member.v, p: value.p })
        continue
      }
      if (this.match(TT.LParen)) {
        const args = this.expressionArguments('Expected ) after function arguments')
        value = this.expressionNode({
          kind: 'call',
          name: value.kind === 'identifier' ? value.name : null,
          callee: value,
          args,
          p: value.p,
        })
        continue
      }
      return value
    }
  }

  /** Parse OpenSCAD's ordered arguments_call production after `(`. */
  private expressionArguments(closingMessage: string, terminator: TT = TT.RParen): ExpressionArgument[] {
    const args: ExpressionArgument[] = []
    const named = new Set<string>()
    let sawNamed = false
    while (this.peek().t !== terminator) {
      const start = this.peek().p
      let name: string | undefined
      if (this.peek().t === TT.Ident && this.peek(1).t === TT.Eq) {
        const nameToken = this.advance()
        name = nameToken.v
        this.advance()
        if (named.has(name) && this.languageProfile === 'openscad-viewer-subset@1') {
          // Preserve the frozen subset@1 diagnostic position at the value.
          this.fail(this.peek(), `Duplicate argument ${name}`)
        }
        named.add(name)
        sawNamed = true
      } else if (sawNamed && this.languageProfile === 'openscad-viewer-subset@1') {
        this.fail(this.peek(), 'Positional arguments must precede named arguments')
      }
      const argument = this.expression()
      args.push({ name, value: argument, p: start, end: this.lastTokenEnd })
      if (!this.match(TT.Comma)) break
      if (this.peek().t === terminator) break
    }
    this.expect(terminator, closingMessage)
    return args
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
      if (token.v === 'function' && this.peek().t === TT.LParen) {
        if (this.languageProfile === 'openscad-viewer-subset@1') {
          this.fail(token, 'function is not supported by openscad-viewer-subset@1', 'E_FEATURE_USER_FUNCTION')
        }
        this.fail(token, 'Anonymous function must start an expression')
      }
      if (this.languageProfile === 'openscad/stable-2021.01') {
        if (['let', 'assert', 'echo'].includes(token.v) && this.peek().t === TT.LParen) {
          this.fail(token, `${token.v} expression must start an expression`)
        }
        if (['for', 'if', 'else', 'each'].includes(token.v)) {
          this.fail(token, `Expected expression, got ${token.v}`)
        }
      }
      return this.expressionNode({ kind: 'identifier', name: token.v, p: token.p })
    }
    this.fail(token, `Expected expression, got ${token.v || TT[token.t]}`)
  }

  private vectorOrRange(p: number): Expr {
    if (this.match(TT.RBracket)) return this.expressionNode({ kind: 'vector', items: [], p })
    const first = this.languageProfile === 'openscad/stable-2021.01' && this.isListComprehensionStart()
      ? this.listComprehension()
      : this.expression()
    if (this.match(TT.Colon)) {
      if (this.isListComprehension(first)) this.fail(this.peek(), 'A list comprehension cannot start a range')
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
      items.push(
        this.languageProfile === 'openscad/stable-2021.01' && this.isListComprehensionStart()
          ? this.listComprehension()
          : this.expression(),
      )
    }
    this.expect(TT.RBracket, 'Expected ]')
    return this.expressionNode({ kind: 'vector', items, p })
  }

  private isListComprehension(value: Expr): value is ListComprehensionExpression {
    return value.kind === 'lc-for'
      || value.kind === 'lc-for-c'
      || value.kind === 'lc-if'
      || value.kind === 'lc-let'
      || value.kind === 'lc-each'
  }

  private isListComprehensionStart(index = this.pos, recursion = 0): boolean {
    if (recursion > MAX_EXPRESSION_DEPTH) return false
    const token = this.tokens[index]
    if (token?.t !== TT.Ident) return false
    if (token.v === 'each') return true
    if ((token.v === 'for' || token.v === 'if') && this.tokens[index + 1]?.t === TT.LParen) return true
    if (token.v !== 'let' || this.tokens[index + 1]?.t !== TT.LParen) return false
    const close = this.matchingRightParen(index + 1)
    if (close === undefined) return false
    return this.isListComprehensionParenthesizedStart(close + 1, recursion + 1)
  }

  private isListComprehensionParenthesizedStart(index: number, recursion: number): boolean {
    if (this.isListComprehensionStart(index, recursion)) return true
    return this.tokens[index]?.t === TT.LParen
      && this.isListComprehensionParenthesizedStart(index + 1, recursion + 1)
  }

  private matchingRightParen(openIndex: number): number | undefined {
    let depth = 0
    for (let index = openIndex; index < this.tokens.length; index++) {
      const type = this.tokens[index].t
      if (type === TT.LParen) depth++
      else if (type === TT.RParen && --depth === 0) return index
    }
    return undefined
  }

  private listComprehension(): ListComprehensionExpression {
    const keyword = this.expect(TT.Ident)
    if (keyword.v === 'each') {
      return this.expressionNode({
        kind: 'lc-each',
        value: this.listComprehensionOrExpression(),
        p: keyword.p,
      })
    }

    this.expect(TT.LParen, `Expected ( after ${keyword.v}`)
    if (keyword.v === 'for') {
      if (this.hasTopLevelSemicolonBeforeRightParen()) {
        const init = this.expressionArguments('Expected ; after C-style for initializer', TT.Semi)
        const condition = this.expression()
        this.expect(TT.Semi, 'Expected ; after C-style for condition')
        const update = this.expressionArguments('Expected ) after C-style for update')
        return this.expressionNode({
          kind: 'lc-for-c',
          init,
          condition,
          update,
          body: this.listComprehensionOrExpression(),
          p: keyword.p,
        })
      }
      const args = this.expressionArguments('Expected ) after for bindings')
      return this.expressionNode({
        kind: 'lc-for',
        args,
        body: this.listComprehensionOrExpression(),
        p: keyword.p,
      })
    }

    if (keyword.v === 'if') {
      const condition = this.expression()
      this.expect(TT.RParen, 'Expected ) after list-comprehension condition')
      const yes = this.listComprehensionOrExpression()
      let no: Expr | undefined
      if (this.peek().t === TT.Ident && this.peek().v === 'else') {
        this.advance()
        no = this.listComprehensionOrExpression()
      }
      return no === undefined
        ? this.expressionNode({ kind: 'lc-if', condition, yes, p: keyword.p })
        : this.expressionNode({ kind: 'lc-if', condition, yes, no, p: keyword.p })
    }

    if (keyword.v === 'let') {
      const args = this.expressionArguments('Expected ) after list-comprehension let bindings')
      return this.expressionNode({
        kind: 'lc-let',
        args,
        body: this.parenthesizedListComprehension(),
        p: keyword.p,
      })
    }

    this.fail(keyword, `Expected list comprehension, got ${keyword.v}`)
  }

  private listComprehensionOrExpression(): Expr {
    if (this.isListComprehensionStart()) return this.listComprehension()
    if (this.peek().t === TT.LParen && this.isListComprehensionStart(this.pos + 1)) {
      return this.parenthesizedListComprehension()
    }
    return this.expression()
  }

  private parenthesizedListComprehension(): ListComprehensionExpression {
    if (!this.match(TT.LParen)) return this.listComprehension()
    const value = this.listComprehension()
    this.expect(TT.RParen, 'Expected ) after list comprehension')
    return value
  }

  private hasTopLevelSemicolonBeforeRightParen(): boolean {
    let parens = 0
    let brackets = 0
    let braces = 0
    for (let index = this.pos; index < this.tokens.length; index++) {
      const type = this.tokens[index].t
      if (type === TT.LParen) parens++
      else if (type === TT.RParen) {
        if (parens === 0 && brackets === 0 && braces === 0) return false
        parens--
      } else if (type === TT.LBracket) brackets++
      else if (type === TT.RBracket) brackets--
      else if (type === TT.LBrace) braces++
      else if (type === TT.RBrace) braces--
      else if (type === TT.Semi && parens === 0 && brackets === 0 && braces === 0) return true
    }
    return false
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
    if ((node as OpenScadParsedStatement).type === 'directive') continue
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

function findDirective(nodes: readonly Statement[]): DirectiveNode | undefined {
  for (const node of nodes) {
    const parsed = node as OpenScadParsedStatement
    if (parsed.type === 'directive') return parsed
    if (node.type === 'call') {
      const child = findDirective(node.children) ?? findDirective(node.alternative)
      if (child !== undefined) return child
    } else if (node.type === 'module') {
      const child = findDirective(node.children)
      if (child !== undefined) return child
    }
  }
  return undefined
}

/** Recompute call identities, then make an executable AST immutable. */
export function finalizeOpenScadProgram(statements: Statement[]): readonly Statement[] {
  assignOperationIds(statements)
  return deepFreeze(statements)
}

/** Parse one full-profile project source while deliberately retaining file directives. */
export function parseOpenScadProjectSource(source: string): readonly OpenScadParsedStatement[] {
  const statements = new Parser(
    tokenize(source),
    source,
    'openscad/stable-2021.01',
  ).parseAll()
  return finalizeOpenScadProgram(statements) as readonly OpenScadParsedStatement[]
}

/** Pure compiler front-end: source text to a typed operation IR. */
export function compileOpenSCAD(
  source: string,
  options: CompileOpenScadOptions = {},
): readonly Statement[] {
  const statements = new Parser(
    tokenize(source),
    source,
    options.languageProfile ?? 'openscad-viewer-subset@1',
  ).parseAll()
  const directive = findDirective(statements)
  if (directive !== undefined) {
    throw new OpenSCADParseError(
      source,
      directive.p,
      `${directive.directive} requires project compilation`,
      undefined,
      directive.end,
    )
  }
  return finalizeOpenScadProgram(statements)
}

function deepFreeze<T>(value: T): T {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    for (const child of Object.values(value as Record<string, unknown>)) deepFreeze(child)
    Object.freeze(value)
  }
  return value
}
