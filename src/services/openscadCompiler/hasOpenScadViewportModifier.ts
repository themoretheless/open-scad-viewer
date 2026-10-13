import { Parser, TT } from '../openscadCompiler'
import type { CallNode, CompileOpenScadOptions, DirectiveNode, OpenScadParsedStatement, OpenScadViewportModifierKind, Statement, Token } from '../openscadCompiler'
import { OpenSCADParseError } from '../openscadErrors'
import { deepFreeze } from '../../core/objects'

export function tokenize(source: string): Token[] {
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

export function isDigit(ch: string | undefined) { return ch !== undefined && ch >= '0' && ch <= '9' }

export function isIdentStart(ch: string | undefined) {
  return ch !== undefined && ((ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || ch === '_' || ch === '$')
}

export function isIdentPart(ch: string | undefined) { return isIdentStart(ch) || isDigit(ch) }

export function hasOpenScadViewportModifier(
  node: Pick<CallNode, 'viewportModifiers'>,
  kind: OpenScadViewportModifierKind,
): boolean {
  return node.viewportModifiers?.some(modifier => modifier.kind === kind) ?? false
}

/**
 * Give call sites structural identities after parsing. Occurrences are counted
 * per operation name, so whitespace, comments, unrelated siblings and quality
 * changes do not move an existing identity. Identical same-name siblings remain
 * positional because the language has no persistent user-authored node IDs.
 */
export function assignOperationIds(nodes: Statement[], parent: readonly string[] = ['root']) {
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

export function findDirective(nodes: readonly Statement[]): DirectiveNode | undefined {
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
