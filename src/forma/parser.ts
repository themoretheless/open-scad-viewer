/**
 * Forma declarative `.ui` parser and expression evaluator.
 */
export interface FormaActionCall {
  name: string
  args: string[]
}

export interface FormaElementNode {
  kind: 'element'
  type: string
  props: Record<string, string>
  bindings: Record<string, string>
  actions: Record<string, FormaActionCall>
  children: FormaAstNode[]
}

export interface FormaIfNode {
  kind: 'if'
  condition: string
  thenBranch: FormaAstNode[]
  elseBranch: FormaAstNode[]
}

export interface FormaForNode {
  kind: 'for'
  itemVar: string
  indexVar: string | null
  listExpr: string
  keyExpr: string | null
  body: FormaAstNode[]
}

export interface FormaLetNode {
  kind: 'let'
  name: string
  expr: string
}

export type FormaAstNode = FormaElementNode | FormaIfNode | FormaForNode | FormaLetNode

export interface FormaComponentAst {
  name: string
  tokens: Record<string, string>
  i18n: Record<string, Record<string, string>>
  defaults: Record<string, unknown>
  lets: FormaLetNode[]
  children: FormaAstNode[]
}

interface Token {
  kind: 'ident' | 'string' | 'number' | 'punct' | 'arrow' | 'bind'
  value: string
}

function tokenize(source: string): Token[] {
  const cleaned = source
    .replace(/\/\*[\s\S]*?\*\//g, ' ')
    .replace(/(^|[^:])\/\/.*$/gm, '$1 ')
  const tokens: Token[] = []
  let i = 0
  while (i < cleaned.length) {
    const ch = cleaned[i]
    if (/\s/.test(ch)) {
      i++
      continue
    }
    if (cleaned.startsWith('<->', i)) {
      tokens.push({ kind: 'bind', value: '<->' })
      i += 3
      continue
    }
    if (cleaned.startsWith('->', i) || cleaned.startsWith('=>', i)) {
      tokens.push({ kind: 'arrow', value: cleaned.slice(i, i + 2) })
      i += 2
      continue
    }
    if (ch === '"' || ch === "'") {
      const quote = ch
      i++
      let val = ''
      while (i < cleaned.length && cleaned[i] !== quote) {
        if (cleaned[i] === '\\' && i + 1 < cleaned.length) {
          i++
          val += cleaned[i]
        } else {
          val += cleaned[i]
        }
        i++
      }
      i++
      tokens.push({ kind: 'string', value: val })
      continue
    }
    if (/[0-9]/.test(ch) || (ch === '-' && /[0-9]/.test(cleaned[i + 1] ?? ''))) {
      let num = ch
      i++
      while (i < cleaned.length && /[0-9.a-zA-Z%]/.test(cleaned[i])) {
        num += cleaned[i]
        i++
      }
      tokens.push({ kind: 'number', value: num })
      continue
    }
    if (ch === '#' && /[0-9a-fA-F]/.test(cleaned[i + 1] ?? '')) {
      let hex = '#'
      i++
      while (i < cleaned.length && /[0-9a-fA-F]/.test(cleaned[i])) {
        hex += cleaned[i]
        i++
      }
      tokens.push({ kind: 'string', value: JSON.stringify(hex) })
      continue
    }
    if (/[a-zA-Z_$]/.test(ch)) {
      let id = ch
      i++
      while (i < cleaned.length && /[a-zA-Z0-9_$.-]/.test(cleaned[i])) {
        id += cleaned[i]
        i++
      }
      tokens.push({ kind: 'ident', value: id })
      continue
    }
    tokens.push({ kind: 'punct', value: ch })
    i++
  }
  return tokens
}

function readRawExpression(tokens: Token[], cursor: { idx: number }): string {
  let expr = ''
  let depthParen = 0
  let depthBracket = 0
  while (cursor.idx < tokens.length) {
    const tok = tokens[cursor.idx]
    if (depthParen === 0 && depthBracket === 0) {
      if (tok.kind === 'punct' && (tok.value === ';' || tok.value === ',' || tok.value === '}')) {
        if (tok.value === ';' || tok.value === ',') cursor.idx++
        break
      }
      if (
        tok.kind === 'ident' &&
        expr.trim().length > 0 &&
        !/[?:+\-*/|&=!<>]$/.test(expr.trim())
      ) {
        const next = tokens[cursor.idx + 1]
        if (
          next &&
          ((next.kind === 'punct' && (next.value === ':' || next.value === '{')) ||
            next.kind === 'arrow' ||
            next.kind === 'bind')
        ) {
          break
        }
        if (tok.value === 'if' || tok.value === 'for' || tok.value === 'let') {
          break
        }
      }
    }
    if (tok.kind === 'punct' && tok.value === '(') depthParen++
    if (tok.kind === 'punct' && tok.value === ')') depthParen = Math.max(0, depthParen - 1)
    if (tok.kind === 'punct' && tok.value === '[') depthBracket++
    if (tok.kind === 'punct' && tok.value === ']') depthBracket = Math.max(0, depthBracket - 1)
    if (tok.kind === 'string') {
      const s = tok.value.startsWith('"') ? tok.value : JSON.stringify(tok.value)
      expr += ` ${s}`
    } else {
      expr += tok.kind === 'punct' && '.()[]!'.includes(tok.value) ? tok.value : ` ${tok.value}`
    }
    cursor.idx++
  }
  return expr.trim()
}

function parseActionTarget(raw: string): FormaActionCall {
  const cleaned = raw.replace(/^actions\./, '').trim()
  const parenStart = cleaned.indexOf('(')
  if (parenStart === -1) return { name: cleaned, args: [] }
  const name = cleaned.slice(0, parenStart).trim()
  const inside = cleaned.slice(parenStart + 1, cleaned.lastIndexOf(')')).trim()
  if (!inside) return { name, args: [] }
  const args: string[] = []
  let cur = ''
  let depth = 0
  let inQuote: string | null = null
  for (let i = 0; i < inside.length; i++) {
    const ch = inside[i]
    if (inQuote) {
      cur += ch
      if (ch === inQuote && inside[i - 1] !== '\\') inQuote = null
      continue
    }
    if (ch === '"' || ch === "'") {
      inQuote = ch
      cur += ch
      continue
    }
    if (ch === '(' || ch === '[') depth++
    if (ch === ')' || ch === ']') depth--
    if (ch === ',' && depth === 0) {
      if (cur.trim()) args.push(cur.trim())
      cur = ''
    } else {
      cur += ch
    }
  }
  if (cur.trim()) args.push(cur.trim())
  return { name, args }
}

function parseBlockChildren(
  tokens: Token[],
  cursor: { idx: number },
  comp?: FormaComponentAst,
): FormaAstNode[] {
  const nodes: FormaAstNode[] = []
  while (cursor.idx < tokens.length) {
    const tok = tokens[cursor.idx]
    if (tok.kind === 'punct' && tok.value === '}') {
      cursor.idx++
      break
    }
    if (tok.kind === 'punct' && (tok.value === ';' || tok.value === ',')) {
      cursor.idx++
      continue
    }
    if (tok.kind === 'ident' && tok.value === 'tokens' && comp) {
      cursor.idx++
      if (tokens[cursor.idx]?.value === '{') cursor.idx++
      while (cursor.idx < tokens.length && tokens[cursor.idx]?.value !== '}') {
        const key = tokens[cursor.idx]?.value
        cursor.idx++
        if (tokens[cursor.idx]?.value === ':') cursor.idx++
        const val = readRawExpression(tokens, cursor)
        if (key) comp.tokens[key] = val.replace(/^"|"$/g, '')
      }
      if (tokens[cursor.idx]?.value === '}') cursor.idx++
      continue
    }
    if (tok.kind === 'ident' && tok.value === 'i18n' && comp) {
      cursor.idx++
      const locale = tokens[cursor.idx]?.value ?? 'ru'
      cursor.idx++
      if (tokens[cursor.idx]?.value === '{') cursor.idx++
      comp.i18n[locale] = comp.i18n[locale] ?? {}
      while (cursor.idx < tokens.length && tokens[cursor.idx]?.value !== '}') {
        const key = tokens[cursor.idx]?.value
        cursor.idx++
        if (tokens[cursor.idx]?.value === ':') cursor.idx++
        const val = readRawExpression(tokens, cursor)
        if (key) comp.i18n[locale][key] = val.replace(/^"|"$/g, '')
      }
      if (tokens[cursor.idx]?.value === '}') cursor.idx++
      continue
    }
    if (tok.kind === 'ident' && (tok.value === 'state' || tok.value === 'prop') && comp) {
      cursor.idx++
      const field = tokens[cursor.idx]?.value ?? ''
      cursor.idx++
      if (tokens[cursor.idx]?.value === ':') {
        cursor.idx++
        if (tokens[cursor.idx]?.kind === 'ident' && tokens[cursor.idx + 1]?.value === '=') {
          cursor.idx++
        }
      }
      if (tokens[cursor.idx]?.value === '=') {
        cursor.idx++
        const raw = readRawExpression(tokens, cursor)
        comp.defaults[field] = raw
      } else {
        readRawExpression(tokens, cursor)
      }
      continue
    }
    if (tok.kind === 'ident' && tok.value === 'let') {
      cursor.idx++
      const name = tokens[cursor.idx]?.value ?? ''
      cursor.idx++
      if (tokens[cursor.idx]?.value === '=') cursor.idx++
      const expr = readRawExpression(tokens, cursor)
      const letNode: FormaLetNode = { kind: 'let', name, expr }
      if (comp) comp.lets.push(letNode)
      nodes.push(letNode)
      continue
    }
    if (tok.kind === 'ident' && tok.value === 'if') {
      cursor.idx++
      let cond = ''
      while (cursor.idx < tokens.length && tokens[cursor.idx]?.value !== '{') {
        const t = tokens[cursor.idx]
        cond += t.kind === 'string' ? ` ${JSON.stringify(t.value)}` : ` ${t.value}`
        cursor.idx++
      }
      if (tokens[cursor.idx]?.value === '{') cursor.idx++
      const thenBranch = parseBlockChildren(tokens, cursor)
      let elseBranch: FormaAstNode[] = []
      if (tokens[cursor.idx]?.kind === 'ident' && tokens[cursor.idx]?.value === 'else') {
        cursor.idx++
        if (tokens[cursor.idx]?.kind === 'ident' && tokens[cursor.idx]?.value === 'if') {
          elseBranch = parseBlockChildren(tokens, cursor)
        } else {
          if (tokens[cursor.idx]?.value === '{') cursor.idx++
          elseBranch = parseBlockChildren(tokens, cursor)
        }
      }
      nodes.push({ kind: 'if', condition: cond.trim(), thenBranch, elseBranch })
      continue
    }
    if (tok.kind === 'ident' && tok.value === 'for') {
      cursor.idx++
      const itemVar = tokens[cursor.idx]?.value ?? 'item'
      cursor.idx++
      let indexVar: string | null = null
      if (tokens[cursor.idx]?.value === ',') {
        cursor.idx++
        indexVar = tokens[cursor.idx]?.value ?? 'index'
        cursor.idx++
      }
      if (tokens[cursor.idx]?.value === 'in') cursor.idx++
      let listExpr = ''
      let keyExpr: string | null = null
      while (cursor.idx < tokens.length && tokens[cursor.idx]?.value !== '{') {
        if (tokens[cursor.idx]?.kind === 'ident' && tokens[cursor.idx]?.value === 'key') {
          cursor.idx++
          let k = ''
          while (cursor.idx < tokens.length && tokens[cursor.idx]?.value !== '{') {
            k += ` ${tokens[cursor.idx].value}`
            cursor.idx++
          }
          keyExpr = k.trim()
          break
        }
        listExpr += ` ${tokens[cursor.idx].value}`
        cursor.idx++
      }
      if (tokens[cursor.idx]?.value === '{') cursor.idx++
      const body = parseBlockChildren(tokens, cursor)
      nodes.push({ kind: 'for', itemVar, indexVar, listExpr: listExpr.trim(), keyExpr, body })
      continue
    }
    if (tok.kind === 'ident' && tokens[cursor.idx + 1]?.value === '{') {
      const type = tok.value
      cursor.idx += 2
      nodes.push(parseElementBody(type, tokens, cursor))
      continue
    }
    cursor.idx++
  }
  return nodes
}

function parseElementBody(
  type: string,
  tokens: Token[],
  cursor: { idx: number },
): FormaElementNode {
  const props: Record<string, string> = {}
  const bindings: Record<string, string> = {}
  const actions: Record<string, FormaActionCall> = {}
  const children: FormaAstNode[] = []

  while (cursor.idx < tokens.length) {
    const tok = tokens[cursor.idx]
    if (tok.kind === 'punct' && tok.value === '}') {
      cursor.idx++
      break
    }
    if (tok.kind === 'punct' && (tok.value === ';' || tok.value === ',')) {
      cursor.idx++
      continue
    }
    if (tok.kind === 'ident' && (tok.value === 'if' || tok.value === 'for' || tok.value === 'let')) {
      const sub = parseBlockChildren(tokens, cursor)
      children.push(...sub)
      break
    }
    const next = tokens[cursor.idx + 1]
    if (tok.kind === 'ident' && next?.kind === 'bind') {
      const propName = tok.value
      cursor.idx += 2
      const target = readRawExpression(tokens, cursor)
      bindings[propName] = target.replace(/^state\./, '').trim()
      continue
    }
    if (tok.kind === 'ident' && next?.kind === 'arrow') {
      const eventName = tok.value
      cursor.idx += 2
      const rawTarget = readRawExpression(tokens, cursor)
      actions[eventName] = parseActionTarget(rawTarget)
      continue
    }
    if (tok.kind === 'ident' && next?.value === ':') {
      const propName = tok.value
      cursor.idx += 2
      props[propName] = readRawExpression(tokens, cursor)
      continue
    }
    if (tok.kind === 'ident' && next?.value === '{') {
      const childType = tok.value
      cursor.idx += 2
      children.push(parseElementBody(childType, tokens, cursor))
      continue
    }
    cursor.idx++
  }

  return { kind: 'element', type, props, bindings, actions, children }
}

const astCache = new Map<string, FormaComponentAst>()

export function parseFormaUi(source: string): FormaComponentAst {
  const cached = astCache.get(source)
  if (cached) return cached
  const tokens = tokenize(source)
  const cursor = { idx: 0 }
  let name = 'FormaComponent'
  while (cursor.idx < tokens.length) {
    const tok = tokens[cursor.idx]
    if (tok.kind === 'ident' && tok.value === 'component') {
      cursor.idx++
      name = tokens[cursor.idx]?.value ?? name
      cursor.idx++
      if (tokens[cursor.idx]?.value === '{') cursor.idx++
      break
    }
    cursor.idx++
  }
  const comp: FormaComponentAst = {
    name,
    tokens: {},
    i18n: {},
    defaults: {},
    lets: [],
    children: [],
  }
  comp.children = parseBlockChildren(tokens, cursor, comp)
  astCache.set(source, comp)
  return comp
}
