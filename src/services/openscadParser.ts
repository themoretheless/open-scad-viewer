/**
 * OpenSCAD subset parser + mesh generator.
 * Supports: cube, sphere, cylinder, translate, rotate, scale, color,
 *           mirror, multmatrix, union, difference, intersection,
 *           linear_extrude, rotate_extrude, hull,
 *           module definitions/calls, math expression evaluator
 */
import {
  identity, translate, rotateX, rotateY, rotateZ, scale, multiply,
  type Mat4, type Vec3,
} from './math3d'

/* ── Limits ───────────────────────────────────────── */

/** Upper bound on `$fn` (facet count) to prevent DoS via huge tessellation. */
const MAX_FN = 256

/* ── Tokens ───────────────────────────────────────── */

enum TT {
  Num, Str, Ident,
  LParen, RParen, LBrace, RBrace, LBracket, RBracket,
  Comma, Semi, Eq, Plus, Minus, Star, Slash, Percent,
  Hash, Dollar, Dot, Colon, Question,
  Lt, Gt, LtEq, GtEq, EqEq, NotEq, Not, And, Or,
  Eof,
}

interface Token { t: TT; v: string; p: number }

function tokenize(src: string): Token[] {
  const out: Token[] = []
  let i = 0
  const len = src.length
  while (i < len) {
    const ch = src[i]
    if (ch <= ' ') { i++; continue }
    if (ch === '/' && src[i + 1] === '/') { while (i < len && src[i] !== '\n') i++; continue }
    if (ch === '/' && src[i + 1] === '*') { i += 2; while (i < len - 1 && !(src[i] === '*' && src[i + 1] === '/')) i++; i += 2; continue }
    const p = i
    if (ch === '"') {
      i++; let s = ''
      while (i < len && src[i] !== '"') { if (src[i] === '\\') { i++; s += src[i] || '' } else s += src[i]; i++ }
      i++
      out.push({ t: TT.Str, v: s, p }); continue
    }
    if ((ch >= '0' && ch <= '9') || (ch === '.' && src[i + 1] >= '0' && src[i + 1] <= '9')) {
      let n = ''
      while (i < len && src[i] >= '0' && src[i] <= '9') { n += src[i]; i++ }
      if (i < len && src[i] === '.') { n += '.'; i++; while (i < len && src[i] >= '0' && src[i] <= '9') { n += src[i]; i++ } }
      if (i < len && (src[i] === 'e' || src[i] === 'E')) { n += src[i]; i++; if (src[i] === '+' || src[i] === '-') { n += src[i]; i++ }; while (i < len && src[i] >= '0' && src[i] <= '9') { n += src[i]; i++ } }
      out.push({ t: TT.Num, v: n, p }); continue
    }
    if ((ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || ch === '_' || ch === '$') {
      let id = ''
      while (i < len && ((src[i] >= 'a' && src[i] <= 'z') || (src[i] >= 'A' && src[i] <= 'Z') || (src[i] >= '0' && src[i] <= '9') || src[i] === '_' || src[i] === '$')) { id += src[i]; i++ }
      out.push({ t: TT.Ident, v: id, p }); continue
    }
    const tw = src.slice(i, i + 2)
    if (tw === '<=') { out.push({ t: TT.LtEq, v: tw, p }); i += 2; continue }
    if (tw === '>=') { out.push({ t: TT.GtEq, v: tw, p }); i += 2; continue }
    if (tw === '==') { out.push({ t: TT.EqEq, v: tw, p }); i += 2; continue }
    if (tw === '!=') { out.push({ t: TT.NotEq, v: tw, p }); i += 2; continue }
    if (tw === '&&') { out.push({ t: TT.And, v: tw, p }); i += 2; continue }
    if (tw === '||') { out.push({ t: TT.Or, v: tw, p }); i += 2; continue }
    const map: Record<string, TT> = {
      '(': TT.LParen, ')': TT.RParen, '{': TT.LBrace, '}': TT.RBrace,
      '[': TT.LBracket, ']': TT.RBracket, ',': TT.Comma, ';': TT.Semi,
      '=': TT.Eq, '+': TT.Plus, '-': TT.Minus, '*': TT.Star, '/': TT.Slash,
      '%': TT.Percent, '#': TT.Hash, '<': TT.Lt, '>': TT.Gt, '!': TT.Not,
      '?': TT.Question, ':': TT.Colon, '.': TT.Dot,
    }
    if (map[ch] !== undefined) { out.push({ t: map[ch], v: ch, p }); i++; continue }
    i++
  }
  out.push({ t: TT.Eof, v: '', p: i })
  return out
}

/* ── Expression AST node types ────────────────────── */

interface ExprBinary { __expr: true; op: string; left: any; right: any }
interface ExprUnary { __expr: true; op: 'neg' | 'not'; operand: any }
interface ExprTernary { __expr: true; op: 'ternary'; cond: any; ifTrue: any; ifFalse: any }
interface ExprVar { __expr: true; op: 'var'; name: string }
interface ExprCall { __expr: true; op: 'call'; name: string; args: any[] }
type Expr = ExprBinary | ExprUnary | ExprTernary | ExprVar | ExprCall

function isExpr(v: any): v is Expr {
  return v && typeof v === 'object' && v.__expr === true
}

/* ── Math functions map ───────────────────────────── */

const MATH_FUNCS: Record<string, (...args: number[]) => number> = {
  sin: (x: number) => Math.sin(x * Math.PI / 180),
  cos: (x: number) => Math.cos(x * Math.PI / 180),
  tan: (x: number) => Math.tan(x * Math.PI / 180),
  asin: (x: number) => Math.asin(x) * 180 / Math.PI,
  acos: (x: number) => Math.acos(x) * 180 / Math.PI,
  atan: (x: number) => Math.atan(x) * 180 / Math.PI,
  atan2: (y: number, x: number) => Math.atan2(y, x) * 180 / Math.PI,
  sqrt: Math.sqrt,
  abs: Math.abs,
  ceil: Math.ceil,
  floor: Math.floor,
  round: Math.round,
  min: (...a: number[]) => Math.min(...a),
  max: (...a: number[]) => Math.max(...a),
  pow: Math.pow,
  ln: Math.log,            // natural log (OpenSCAD ln())
  log: (x: number) => Math.log10(x), // base-10 log (OpenSCAD log())
  exp: Math.exp,
  sign: Math.sign,
  rands: (minv: number, maxv: number, count: number) => {
    // Returns a single value for simplicity (OpenSCAD returns vector)
    void count
    return minv + Math.random() * (maxv - minv)
  },
  // Note: len, norm, cross, lookup, str, chr, concat are intentionally NOT
  // defined here. They are reimplemented correctly in evalExprNode (they
  // operate on arrays/strings, not plain numbers). Keeping stubs here would
  // cause wrong constant-folding at parse time.
}

/**
 * Function names that must never be constant-folded at parse time.
 * These are handled with correct array/string-aware semantics in evalExprNode.
 */
const NON_FOLDABLE_FUNCS = new Set<string>([
  'len', 'norm', 'cross', 'lookup', 'str', 'chr', 'concat', 'ord', 'rands',
])

const MATH_CONSTANTS: Record<string, number> = {
  PI: Math.PI,
}

/* ── AST ──────────────────────────────────────────── */

export interface ASTNode {
  type: 'call'
  name: string
  args: Record<string, any>
  children: ASTNode[]
  pos: number
  endPos: number
  /** For module definitions: parameter info */
  params?: { name: string; defaultVal: any }[]
}

/* ── Parser ───────────────────────────────────────── */

class Parser {
  private tok: Token[]
  private pos = 0
  constructor(tok: Token[]) { this.tok = tok }

  private peek() { return this.tok[this.pos] }
  private adv() { return this.tok[this.pos++] }
  private expect(t: TT) {
    const tk = this.adv()
    if (tk.t !== t) throw new Error(`Expected ${TT[t]} got "${tk.v}" @${tk.p}`)
    return tk
  }
  private match(t: TT) { if (this.peek().t === t) { this.adv(); return true } return false }

  parseAll(): ASTNode[] {
    const out: ASTNode[] = []
    while (this.peek().t !== TT.Eof) { const n = this.stmt(); if (n) out.push(n) }
    return out
  }

  private stmt(): ASTNode | null {
    if (this.peek().t === TT.Semi) { this.adv(); return null }
    while (this.peek().t === TT.Hash || this.peek().t === TT.Percent ||
           this.peek().t === TT.Star || this.peek().t === TT.Not) this.adv()
    if (this.peek().t === TT.Ident) return this.call()
    this.adv(); return null
  }

  private call(): ASTNode {
    const nameTok = this.expect(TT.Ident)
    const name = nameTok.v
    const p = nameTok.p

    /* ── module definition ── */
    if (name === 'module') {
      const modNameTok = this.expect(TT.Ident)
      const modName = modNameTok.v
      const params: { name: string; defaultVal: any }[] = []
      if (this.match(TT.LParen)) {
        while (this.peek().t !== TT.RParen && this.peek().t !== TT.Eof) {
          const pName = this.expect(TT.Ident).v
          let defaultVal: any = undefined
          if (this.match(TT.Eq)) {
            defaultVal = this.parseExpr()
          }
          params.push({ name: pName, defaultVal })
          this.match(TT.Comma)
        }
        this.expect(TT.RParen)
      }
      const children: ASTNode[] = []
      if (this.peek().t === TT.LBrace) {
        this.adv()
        while (this.peek().t !== TT.RBrace && this.peek().t !== TT.Eof) {
          const c = this.stmt(); if (c) children.push(c)
        }
        this.match(TT.RBrace)
      } else if (this.peek().t !== TT.Semi && this.peek().t !== TT.Eof) {
        const c = this.stmt(); if (c) children.push(c)
      } else {
        this.match(TT.Semi)
      }
      return { type: 'call', name: 'module', args: { __name: modName }, children, pos: p, endPos: this.peek().p, params }
    }

    /* ── function definition (skip) ── */
    if (name === 'function') {
      this.skipExpr()
      this.match(TT.Semi)
      return { type: 'call', name: 'function', args: {}, children: [], pos: p, endPos: this.peek().p }
    }

    /* ── use / include ── */
    if (name === 'use' || name === 'include') {
      let fileName = ''
      if (this.peek().t === TT.Lt) {
        this.adv() // consume '<'
        while (this.peek().t !== TT.Gt && this.peek().t !== TT.Eof) {
          fileName += this.adv().v
        }
        this.match(TT.Gt)
      }
      this.match(TT.Semi)
      return { type: 'call', name, args: { __fileName: fileName }, children: [], pos: p, endPos: this.peek().p }
    }

    const args: Record<string, any> = {}

    /* ── variable assignment ── */
    if (this.peek().t === TT.Eq) {
      this.adv()
      const val = this.parseExpr()
      this.match(TT.Semi)
      return { type: 'call', name: '__assign', args: { __varName: name, __varValue: val }, children: [], pos: p, endPos: this.peek().p }
    }

    if (this.match(TT.LParen)) {
      this.parseArgs(args)
      this.expect(TT.RParen)
    }

    const children: ASTNode[] = []
    if (this.peek().t === TT.LBrace) {
      this.adv()
      while (this.peek().t !== TT.RBrace && this.peek().t !== TT.Eof) {
        const c = this.stmt(); if (c) children.push(c)
      }
      this.match(TT.RBrace)
    } else if (this.peek().t !== TT.Semi && this.peek().t !== TT.Eof) {
      const c = this.stmt(); if (c) children.push(c)
    } else {
      this.match(TT.Semi)
    }
    return { type: 'call', name, args, children, pos: p, endPos: this.peek().p }
  }

  private parseArgs(args: Record<string, any>) {
    let pi = 0
    while (this.peek().t !== TT.RParen && this.peek().t !== TT.Eof) {
      if (this.peek().t === TT.Ident && this.tok[this.pos + 1]?.t === TT.Eq &&
          this.tok[this.pos + 2]?.t !== TT.Eq) {
        const key = this.adv().v; this.adv()
        args[key] = this.parseExpr()
      } else {
        args[`_${pi++}`] = this.parseExpr()
      }
      this.match(TT.Comma)
    }
  }

  /* ── Expression parser with proper precedence ── */

  /** Parse expression: ternary (lowest precedence) */
  private parseExpr(): any {
    return this.parseTernary()
  }

  private parseTernary(): any {
    let left = this.parseOr()
    if (this.peek().t === TT.Question) {
      this.adv()
      const ifTrue = this.parseExpr()
      this.expect(TT.Colon)
      const ifFalse = this.parseExpr()
      if (isExpr(left) || isExpr(ifTrue) || isExpr(ifFalse)) {
        return { __expr: true, op: 'ternary', cond: left, ifTrue, ifFalse } as ExprTernary
      }
      return evalCondition(left) ? ifTrue : ifFalse
    }
    return left
  }

  private parseOr(): any {
    let left = this.parseAnd()
    while (this.peek().t === TT.Or) {
      this.adv()
      const right = this.parseAnd()
      if (isExpr(left) || isExpr(right)) {
        left = { __expr: true, op: '||', left, right } as ExprBinary
      } else {
        left = evalCondition(left) || evalCondition(right)
      }
    }
    return left
  }

  private parseAnd(): any {
    let left = this.parseComparison()
    while (this.peek().t === TT.And) {
      this.adv()
      const right = this.parseComparison()
      if (isExpr(left) || isExpr(right)) {
        left = { __expr: true, op: '&&', left, right } as ExprBinary
      } else {
        left = evalCondition(left) && evalCondition(right)
      }
    }
    return left
  }

  private parseComparison(): any {
    let left = this.parseAddSub()
    while (this.peek().t === TT.Lt || this.peek().t === TT.Gt ||
           this.peek().t === TT.LtEq || this.peek().t === TT.GtEq ||
           this.peek().t === TT.EqEq || this.peek().t === TT.NotEq) {
      const op = this.adv().v
      const right = this.parseAddSub()
      if (isExpr(left) || isExpr(right)) {
        left = { __expr: true, op, left, right } as ExprBinary
      } else {
        left = evalComparison(op, left, right)
      }
    }
    return left
  }

  private parseAddSub(): any {
    let left = this.parseMulDiv()
    while (this.peek().t === TT.Plus || this.peek().t === TT.Minus) {
      const op = this.adv().v
      const right = this.parseMulDiv()
      if (isExpr(left) || isExpr(right)) {
        left = { __expr: true, op, left, right } as ExprBinary
      } else if (typeof left === 'number' && typeof right === 'number') {
        left = op === '+' ? left + right : left - right
      } else {
        left = { __expr: true, op, left, right } as ExprBinary
      }
    }
    return left
  }

  private parseMulDiv(): any {
    let left = this.parseUnary()
    while (this.peek().t === TT.Star || this.peek().t === TT.Slash || this.peek().t === TT.Percent) {
      const op = this.adv().v
      const right = this.parseUnary()
      if (isExpr(left) || isExpr(right)) {
        left = { __expr: true, op, left, right } as ExprBinary
      } else if (typeof left === 'number' && typeof right === 'number') {
        if (op === '*') left = left * right
        else if (op === '/') left = right !== 0 ? left / right : (left === 0 ? NaN : (left > 0 ? Infinity : -Infinity))
        else left = right !== 0 ? left % right : NaN
      } else {
        left = { __expr: true, op, left, right } as ExprBinary
      }
    }
    return left
  }

  private parseUnary(): any {
    if (this.peek().t === TT.Minus) {
      this.adv()
      const v = this.parseUnary()
      if (typeof v === 'number') return -v
      if (isExpr(v)) return { __expr: true, op: 'neg', operand: v } as ExprUnary
      return { __expr: true, op: 'neg', operand: v } as ExprUnary
    }
    if (this.peek().t === TT.Not) {
      this.adv()
      const v = this.parseUnary()
      if (isExpr(v)) return { __expr: true, op: 'not', operand: v } as ExprUnary
      return !evalCondition(v)
    }
    if (this.peek().t === TT.Plus) {
      this.adv()
      return this.parseUnary()
    }
    return this.parsePrimary()
  }

  private parsePrimary(): any {
    const tk = this.peek()

    if (tk.t === TT.Num) { this.adv(); return parseFloat(tk.v) }
    if (tk.t === TT.Str) { this.adv(); return tk.v }

    if (tk.t === TT.Ident) {
      const name = tk.v

      // Boolean/special literals
      if (name === 'true') { this.adv(); return true }
      if (name === 'false') { this.adv(); return false }
      if (name === 'undef') { this.adv(); return undefined }

      // Constants
      if (name in MATH_CONSTANTS) { this.adv(); return MATH_CONSTANTS[name] }

      // Function calls: identifier followed by '('
      if (this.tok[this.pos + 1]?.t === TT.LParen) {
        this.adv() // consume ident
        this.expect(TT.LParen)
        const fnArgs: any[] = []
        while (this.peek().t !== TT.RParen && this.peek().t !== TT.Eof) {
          fnArgs.push(this.parseExpr())
          this.match(TT.Comma)
        }
        this.expect(TT.RParen)
        // If it's a known math function and all args are literal numbers, compute now
        if (name in MATH_FUNCS && !NON_FOLDABLE_FUNCS.has(name)) {
          if (fnArgs.every(a2 => typeof a2 === 'number')) {
            const fn = MATH_FUNCS[name]
            return fn(...fnArgs)
          }
          return { __expr: true, op: 'call', name, args: fnArgs } as ExprCall
        }
        // Unknown function call in expression context: return call node
        return { __expr: true, op: 'call', name, args: fnArgs } as ExprCall
      }

      // Variable reference
      this.adv()
      return { __expr: true, op: 'var', name } as ExprVar
    }

    if (tk.t === TT.LParen) {
      this.adv()
      const v = this.parseExpr()
      this.expect(TT.RParen)
      return v
    }

    if (tk.t === TT.LBracket) return this.parseVec()

    this.adv()
    return 0
  }

  private parseVec(): any {
    this.expect(TT.LBracket)
    // Check for list comprehension: [for ...]
    if (this.peek().t === TT.Ident && this.peek().v === 'for') {
      return this.parseListComprehension()
    }
    const vals: any[] = []
    while (this.peek().t !== TT.RBracket && this.peek().t !== TT.Eof) {
      if (this.peek().t === TT.Ident && this.peek().v === 'each') {
        this.adv() // consume 'each'
        const inner = this.parseExpr()
        vals.push({ __each: true, value: inner })
        this.match(TT.Comma)
        continue
      }
      vals.push(this.parseExpr())
      // Check for colon -> range syntax [start:end] or [start:step:end]
      if (this.peek().t === TT.Colon) {
        this.adv() // consume ':'
        const second = this.parseExpr()
        if (this.peek().t === TT.Colon) {
          this.adv() // consume second ':'
          const third = this.parseExpr()
          // [start : step : end]
          this.match(TT.RBracket)
          return { __range: true, start: vals[0], step: second, end: third }
        }
        // [start : end]
        this.match(TT.RBracket)
        return { __range: true, start: vals[0], step: 1, end: second }
      }
      this.match(TT.Comma)
    }
    this.expect(TT.RBracket)
    return vals
  }

  private parseListComprehension(): any {
    // We already consumed '[', and peek is 'for'
    this.adv() // consume 'for'
    this.expect(TT.LParen)
    const varName = this.expect(TT.Ident).v
    this.expect(TT.Eq)
    const iterVal = this.parseExpr()
    this.expect(TT.RParen)
    const body = this.parseExpr()
    this.expect(TT.RBracket)
    return { __listComp: true, varName, iterVal, body }
  }

  private skipExpr() {
    let depth = 0
    while (this.peek().t !== TT.Eof) {
      if (this.peek().t === TT.Semi && depth === 0) return
      if (this.peek().t === TT.LParen || this.peek().t === TT.LBrace || this.peek().t === TT.LBracket) depth++
      if (this.peek().t === TT.RParen || this.peek().t === TT.RBrace || this.peek().t === TT.RBracket) { if (depth === 0) return; depth-- }
      this.adv()
    }
  }
}

/* ── Mesh data ────────────────────────────────────── */

export interface MeshData {
  vertices: Float32Array   // interleaved pos(3) + normal(3)
  indices: Uint32Array
  color: [number, number, number, number]
  transform: Mat4
}

/* ── Bitmap font (5x7, ASCII 32-126) ─────────────── */

const BITMAP_FONT: Record<string, number[]> = {
  ' ': [0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0],
  '!': [0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,0,0,0, 0,0,1,0,0],
  '"': [0,1,0,1,0, 0,1,0,1,0, 0,1,0,1,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0],
  '#': [0,1,0,1,0, 1,1,1,1,1, 0,1,0,1,0, 0,1,0,1,0, 0,1,0,1,0, 1,1,1,1,1, 0,1,0,1,0],
  '$': [0,0,1,0,0, 0,1,1,1,1, 1,0,1,0,0, 0,1,1,1,0, 0,0,1,0,1, 1,1,1,1,0, 0,0,1,0,0],
  '%': [1,1,0,0,1, 1,1,0,1,0, 0,0,1,0,0, 0,0,1,0,0, 0,1,0,0,0, 0,1,0,1,1, 1,0,0,1,1],
  '&': [0,1,1,0,0, 1,0,0,1,0, 1,0,1,0,0, 0,1,0,0,0, 1,0,1,0,1, 1,0,0,1,0, 0,1,1,0,1],
  "'": [0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0],
  '(': [0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0, 0,1,0,0,0, 0,1,0,0,0, 0,0,1,0,0, 0,0,0,1,0],
  ')': [0,1,0,0,0, 0,0,1,0,0, 0,0,0,1,0, 0,0,0,1,0, 0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0],
  '*': [0,0,0,0,0, 0,0,1,0,0, 1,0,1,0,1, 0,1,1,1,0, 1,0,1,0,1, 0,0,1,0,0, 0,0,0,0,0],
  '+': [0,0,0,0,0, 0,0,1,0,0, 0,0,1,0,0, 1,1,1,1,1, 0,0,1,0,0, 0,0,1,0,0, 0,0,0,0,0],
  ',': [0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,1,0,0, 0,1,0,0,0],
  '-': [0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 1,1,1,1,1, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0],
  '.': [0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,1,1,0,0, 0,1,1,0,0],
  '/': [0,0,0,0,1, 0,0,0,1,0, 0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0, 0,1,0,0,0, 1,0,0,0,0],
  '0': [0,1,1,1,0, 1,0,0,0,1, 1,0,0,1,1, 1,0,1,0,1, 1,1,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  '1': [0,0,1,0,0, 0,1,1,0,0, 1,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 1,1,1,1,1],
  '2': [0,1,1,1,0, 1,0,0,0,1, 0,0,0,0,1, 0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0, 1,1,1,1,1],
  '3': [0,1,1,1,0, 1,0,0,0,1, 0,0,0,0,1, 0,0,1,1,0, 0,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  '4': [0,0,0,1,0, 0,0,1,1,0, 0,1,0,1,0, 1,0,0,1,0, 1,1,1,1,1, 0,0,0,1,0, 0,0,0,1,0],
  '5': [1,1,1,1,1, 1,0,0,0,0, 1,1,1,1,0, 0,0,0,0,1, 0,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  '6': [0,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  '7': [1,1,1,1,1, 0,0,0,0,1, 0,0,0,1,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0],
  '8': [0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  '9': [0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,1, 0,0,0,0,1, 0,0,0,0,1, 0,1,1,1,0],
  ':': [0,0,0,0,0, 0,1,1,0,0, 0,1,1,0,0, 0,0,0,0,0, 0,1,1,0,0, 0,1,1,0,0, 0,0,0,0,0],
  ';': [0,0,0,0,0, 0,1,1,0,0, 0,1,1,0,0, 0,0,0,0,0, 0,1,1,0,0, 0,0,1,0,0, 0,1,0,0,0],
  '<': [0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0, 1,0,0,0,0, 0,1,0,0,0, 0,0,1,0,0, 0,0,0,1,0],
  '=': [0,0,0,0,0, 0,0,0,0,0, 1,1,1,1,1, 0,0,0,0,0, 1,1,1,1,1, 0,0,0,0,0, 0,0,0,0,0],
  '>': [0,1,0,0,0, 0,0,1,0,0, 0,0,0,1,0, 0,0,0,0,1, 0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0],
  '?': [0,1,1,1,0, 1,0,0,0,1, 0,0,0,0,1, 0,0,0,1,0, 0,0,1,0,0, 0,0,0,0,0, 0,0,1,0,0],
  '@': [0,1,1,1,0, 1,0,0,0,1, 1,0,1,1,1, 1,0,1,0,1, 1,0,1,1,1, 1,0,0,0,0, 0,1,1,1,0],
  'A': [0,0,1,0,0, 0,1,0,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,1, 1,0,0,0,1, 1,0,0,0,1],
  'B': [1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,0],
  'C': [0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,1, 0,1,1,1,0],
  'D': [1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,0],
  'E': [1,1,1,1,1, 1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,1],
  'F': [1,1,1,1,1, 1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0],
  'G': [0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,0, 1,0,1,1,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  'H': [1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1],
  'I': [1,1,1,1,1, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 1,1,1,1,1],
  'J': [0,0,1,1,1, 0,0,0,1,0, 0,0,0,1,0, 0,0,0,1,0, 0,0,0,1,0, 1,0,0,1,0, 0,1,1,0,0],
  'K': [1,0,0,0,1, 1,0,0,1,0, 1,0,1,0,0, 1,1,0,0,0, 1,0,1,0,0, 1,0,0,1,0, 1,0,0,0,1],
  'L': [1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,1],
  'M': [1,0,0,0,1, 1,1,0,1,1, 1,0,1,0,1, 1,0,1,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1],
  'N': [1,0,0,0,1, 1,1,0,0,1, 1,0,1,0,1, 1,0,0,1,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1],
  'O': [0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  'P': [1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0],
  'Q': [0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,1,0,1, 1,0,0,1,0, 0,1,1,0,1],
  'R': [1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,0, 1,0,1,0,0, 1,0,0,1,0, 1,0,0,0,1],
  'S': [0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,0, 0,1,1,1,0, 0,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  'T': [1,1,1,1,1, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0],
  'U': [1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  'V': [1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,0,1,0, 0,1,0,1,0, 0,0,1,0,0],
  'W': [1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,1,0,1, 1,0,1,0,1, 1,1,0,1,1, 1,0,0,0,1],
  'X': [1,0,0,0,1, 1,0,0,0,1, 0,1,0,1,0, 0,0,1,0,0, 0,1,0,1,0, 1,0,0,0,1, 1,0,0,0,1],
  'Y': [1,0,0,0,1, 1,0,0,0,1, 0,1,0,1,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0],
  'Z': [1,1,1,1,1, 0,0,0,0,1, 0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0, 1,0,0,0,0, 1,1,1,1,1],
  '[': [0,1,1,1,0, 0,1,0,0,0, 0,1,0,0,0, 0,1,0,0,0, 0,1,0,0,0, 0,1,0,0,0, 0,1,1,1,0],
  '\\': [1,0,0,0,0, 0,1,0,0,0, 0,1,0,0,0, 0,0,1,0,0, 0,0,0,1,0, 0,0,0,1,0, 0,0,0,0,1],
  ']': [0,1,1,1,0, 0,0,0,1,0, 0,0,0,1,0, 0,0,0,1,0, 0,0,0,1,0, 0,0,0,1,0, 0,1,1,1,0],
  '^': [0,0,1,0,0, 0,1,0,1,0, 1,0,0,0,1, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0],
  '_': [0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 1,1,1,1,1],
  '`': [0,1,0,0,0, 0,0,1,0,0, 0,0,0,1,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0, 0,0,0,0,0],
  'a': [0,0,0,0,0, 0,0,0,0,0, 0,1,1,1,0, 0,0,0,0,1, 0,1,1,1,1, 1,0,0,0,1, 0,1,1,1,1],
  'b': [1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,0],
  'c': [0,0,0,0,0, 0,0,0,0,0, 0,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,1, 0,1,1,1,0],
  'd': [0,0,0,0,1, 0,0,0,0,1, 0,1,1,1,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,1],
  'e': [0,0,0,0,0, 0,0,0,0,0, 0,1,1,1,0, 1,0,0,0,1, 1,1,1,1,1, 1,0,0,0,0, 0,1,1,1,0],
  'f': [0,0,1,1,0, 0,1,0,0,1, 0,1,0,0,0, 1,1,1,1,0, 0,1,0,0,0, 0,1,0,0,0, 0,1,0,0,0],
  'g': [0,0,0,0,0, 0,1,1,1,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,1, 0,0,0,0,1, 0,1,1,1,0],
  'h': [1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1],
  'i': [0,0,1,0,0, 0,0,0,0,0, 0,1,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,1,1,1,0],
  'j': [0,0,0,1,0, 0,0,0,0,0, 0,0,1,1,0, 0,0,0,1,0, 0,0,0,1,0, 1,0,0,1,0, 0,1,1,0,0],
  'k': [1,0,0,0,0, 1,0,0,0,0, 1,0,0,1,0, 1,0,1,0,0, 1,1,0,0,0, 1,0,1,0,0, 1,0,0,1,0],
  'l': [0,1,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,1,1,1,0],
  'm': [0,0,0,0,0, 0,0,0,0,0, 1,1,0,1,0, 1,0,1,0,1, 1,0,1,0,1, 1,0,1,0,1, 1,0,0,0,1],
  'n': [0,0,0,0,0, 0,0,0,0,0, 1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1],
  'o': [0,0,0,0,0, 0,0,0,0,0, 0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0],
  'p': [0,0,0,0,0, 0,0,0,0,0, 1,1,1,1,0, 1,0,0,0,1, 1,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0],
  'q': [0,0,0,0,0, 0,0,0,0,0, 0,1,1,1,1, 1,0,0,0,1, 0,1,1,1,1, 0,0,0,0,1, 0,0,0,0,1],
  'r': [0,0,0,0,0, 0,0,0,0,0, 1,0,1,1,0, 1,1,0,0,1, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0],
  's': [0,0,0,0,0, 0,0,0,0,0, 0,1,1,1,1, 1,0,0,0,0, 0,1,1,1,0, 0,0,0,0,1, 1,1,1,1,0],
  't': [0,1,0,0,0, 0,1,0,0,0, 1,1,1,1,0, 0,1,0,0,0, 0,1,0,0,0, 0,1,0,0,1, 0,0,1,1,0],
  'u': [0,0,0,0,0, 0,0,0,0,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,1,1, 0,1,1,0,1],
  'v': [0,0,0,0,0, 0,0,0,0,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,0,1,0, 0,0,1,0,0],
  'w': [0,0,0,0,0, 0,0,0,0,0, 1,0,0,0,1, 1,0,1,0,1, 1,0,1,0,1, 1,0,1,0,1, 0,1,0,1,0],
  'x': [0,0,0,0,0, 0,0,0,0,0, 1,0,0,0,1, 0,1,0,1,0, 0,0,1,0,0, 0,1,0,1,0, 1,0,0,0,1],
  'y': [0,0,0,0,0, 0,0,0,0,0, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,1, 0,0,0,0,1, 0,1,1,1,0],
  'z': [0,0,0,0,0, 0,0,0,0,0, 1,1,1,1,1, 0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0, 1,1,1,1,1],
  '{': [0,0,0,1,0, 0,0,1,0,0, 0,0,1,0,0, 0,1,0,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,0,1,0],
  '|': [0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0],
  '}': [0,1,0,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,0,1,0, 0,0,1,0,0, 0,0,1,0,0, 0,1,0,0,0],
  '~': [0,0,0,0,0, 0,0,0,0,0, 0,1,0,0,0, 1,0,1,0,1, 0,0,0,1,0, 0,0,0,0,0, 0,0,0,0,0],
}

function makeText(text: string, size: number, spacing: number): { v: number[]; ix: number[] } {
  const v: number[] = []
  const ix: number[] = []
  const pixelSize = size / 7
  let vertexOffset = 0
  for (let ci = 0; ci < text.length; ci++) {
    const ch = text[ci]
    const bitmap = BITMAP_FONT[ch]
    if (!bitmap) continue
    const xOff = ci * (5 + spacing) * pixelSize
    for (let row = 0; row < 7; row++) {
      for (let col = 0; col < 5; col++) {
        if (bitmap[row * 5 + col] === 0) continue
        // Create a small cube for this pixel
        const px = xOff + col * pixelSize
        const py = (6 - row) * pixelSize // flip Y so top row is highest
        const pz = 0
        const ps = pixelSize
        // 6 faces, 4 verts each, 6 floats per vert (pos + normal)
        const faces: [number[],number[],number[],number[],number[]][] = [
          [[px,py,pz+ps],[px+ps,py,pz+ps],[px+ps,py+ps,pz+ps],[px,py+ps,pz+ps],[0,0,1]],
          [[px+ps,py,pz],[px,py,pz],[px,py+ps,pz],[px+ps,py+ps,pz],[0,0,-1]],
          [[px,py+ps,pz],[px,py+ps,pz+ps],[px+ps,py+ps,pz+ps],[px+ps,py+ps,pz],[0,1,0]],
          [[px,py,pz+ps],[px,py,pz],[px+ps,py,pz],[px+ps,py,pz+ps],[0,-1,0]],
          [[px+ps,py,pz+ps],[px+ps,py,pz],[px+ps,py+ps,pz],[px+ps,py+ps,pz+ps],[1,0,0]],
          [[px,py,pz],[px,py,pz+ps],[px,py+ps,pz+ps],[px,py+ps,pz],[-1,0,0]],
        ]
        for (const [a, b, c, d, n] of faces) {
          v.push(a[0],a[1],a[2],n[0],n[1],n[2])
          v.push(b[0],b[1],b[2],n[0],n[1],n[2])
          v.push(c[0],c[1],c[2],n[0],n[1],n[2])
          v.push(d[0],d[1],d[2],n[0],n[1],n[2])
          ix.push(vertexOffset, vertexOffset+1, vertexOffset+2, vertexOffset, vertexOffset+2, vertexOffset+3)
          vertexOffset += 4
        }
      }
    }
  }
  return { v, ix }
}

/* ── Mesh generators ──────────────────────────────── */

function makeCube(sx: number, sy: number, sz: number, center: boolean) {
  const x0 = center ? -sx / 2 : 0, x1 = center ? sx / 2 : sx
  const y0 = center ? -sy / 2 : 0, y1 = center ? sy / 2 : sy
  const z0 = center ? -sz / 2 : 0, z1 = center ? sz / 2 : sz
  const faces: [Vec3, Vec3, Vec3, Vec3, Vec3][] = [
    [[x0,y0,z1],[x1,y0,z1],[x1,y1,z1],[x0,y1,z1],[0,0,1]],
    [[x1,y0,z0],[x0,y0,z0],[x0,y1,z0],[x1,y1,z0],[0,0,-1]],
    [[x0,y1,z0],[x0,y1,z1],[x1,y1,z1],[x1,y1,z0],[0,1,0]],
    [[x0,y0,z1],[x0,y0,z0],[x1,y0,z0],[x1,y0,z1],[0,-1,0]],
    [[x1,y0,z1],[x1,y0,z0],[x1,y1,z0],[x1,y1,z1],[1,0,0]],
    [[x0,y0,z0],[x0,y0,z1],[x0,y1,z1],[x0,y1,z0],[-1,0,0]],
  ]
  const v: number[] = [], ix: number[] = []
  let vi = 0
  for (const [a, b, c, d, n] of faces) {
    v.push(a[0],a[1],a[2],n[0],n[1],n[2], b[0],b[1],b[2],n[0],n[1],n[2],
           c[0],c[1],c[2],n[0],n[1],n[2], d[0],d[1],d[2],n[0],n[1],n[2])
    ix.push(vi,vi+1,vi+2, vi,vi+2,vi+3); vi += 4
  }
  return { v, ix }
}

function makeSphere(r: number, seg: number) {
  const v: number[] = [], ix: number[] = []
  for (let ri = 0; ri <= seg; ri++) {
    const phi = Math.PI * ri / seg, sp = Math.sin(phi), cp = Math.cos(phi)
    for (let si = 0; si <= seg; si++) {
      const th = 2 * Math.PI * si / seg
      const nx = sp * Math.cos(th), ny = cp, nz = sp * Math.sin(th)
      v.push(r * nx, r * ny, r * nz, nx, ny, nz)
    }
  }
  for (let ri = 0; ri < seg; ri++)
    for (let si = 0; si < seg; si++) {
      const a = ri * (seg + 1) + si, b = a + seg + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  return { v, ix }
}

function makeCylinder(h: number, r1: number, r2: number, center: boolean, fn: number) {
  const v: number[] = [], ix: number[] = []
  const z0 = center ? -h / 2 : 0, z1 = center ? h / 2 : h
  const slopeLen = Math.sqrt(h * h + (r1 - r2) ** 2)
  const nzS = slopeLen > 0 ? (r1 - r2) / slopeLen : 0
  const nrS = slopeLen > 0 ? h / slopeLen : 1
  for (let i = 0; i <= fn; i++) {
    const a = (2 * Math.PI * i) / fn, ca = Math.cos(a), sa = Math.sin(a)
    v.push(r1*ca, r1*sa, z0, ca*nrS, sa*nrS, nzS)
    v.push(r2*ca, r2*sa, z1, ca*nrS, sa*nrS, nzS)
  }
  for (let i = 0; i < fn; i++) {
    const a = i * 2; ix.push(a, a+1, a+2, a+2, a+1, a+3)
  }
  const bi = v.length / 6
  v.push(0, 0, z0, 0, 0, -1)
  for (let i = 0; i <= fn; i++) {
    const a = (2 * Math.PI * i) / fn
    v.push(r1 * Math.cos(a), r1 * Math.sin(a), z0, 0, 0, -1)
  }
  for (let i = 0; i < fn; i++) ix.push(bi, bi+i+2, bi+i+1)
  const ti = v.length / 6
  v.push(0, 0, z1, 0, 0, 1)
  for (let i = 0; i <= fn; i++) {
    const a = (2 * Math.PI * i) / fn
    v.push(r2 * Math.cos(a), r2 * Math.sin(a), z1, 0, 0, 1)
  }
  for (let i = 0; i < fn; i++) ix.push(ti, ti+i+1, ti+i+2)
  return { v, ix }
}

function makePipe(h: number, r1: number, r2: number, center: boolean, fn: number) {
  const v: number[] = [], ix: number[] = []
  const z0 = center ? -h / 2 : 0, z1 = center ? h / 2 : h

  // Outer wall
  for (let i = 0; i <= fn; i++) {
    const a = (2 * Math.PI * i) / fn, ca = Math.cos(a), sa = Math.sin(a)
    v.push(r1*ca, r1*sa, z0, ca, sa, 0)
    v.push(r1*ca, r1*sa, z1, ca, sa, 0)
  }
  for (let i = 0; i < fn; i++) {
    const a = i * 2; ix.push(a, a+1, a+2, a+2, a+1, a+3)
  }

  // Inner wall (normals point inward)
  const innerBase = v.length / 6
  for (let i = 0; i <= fn; i++) {
    const a = (2 * Math.PI * i) / fn, ca = Math.cos(a), sa = Math.sin(a)
    v.push(r2*ca, r2*sa, z0, -ca, -sa, 0)
    v.push(r2*ca, r2*sa, z1, -ca, -sa, 0)
  }
  for (let i = 0; i < fn; i++) {
    const a = innerBase + i * 2; ix.push(a, a+2, a+1, a+1, a+2, a+3)
  }

  // Bottom ring cap (z0, normal pointing down)
  const botBase = v.length / 6
  for (let i = 0; i <= fn; i++) {
    const a = (2 * Math.PI * i) / fn, ca = Math.cos(a), sa = Math.sin(a)
    v.push(r1*ca, r1*sa, z0, 0, 0, -1)
    v.push(r2*ca, r2*sa, z0, 0, 0, -1)
  }
  for (let i = 0; i < fn; i++) {
    const a = botBase + i * 2
    ix.push(a, a+2, a+1, a+1, a+2, a+3)
  }

  // Top ring cap (z1, normal pointing up)
  const topBase = v.length / 6
  for (let i = 0; i <= fn; i++) {
    const a = (2 * Math.PI * i) / fn, ca = Math.cos(a), sa = Math.sin(a)
    v.push(r1*ca, r1*sa, z1, 0, 0, 1)
    v.push(r2*ca, r2*sa, z1, 0, 0, 1)
  }
  for (let i = 0; i < fn; i++) {
    const a = topBase + i * 2
    ix.push(a, a+1, a+2, a+2, a+1, a+3)
  }

  return { v, ix }
}

function makeWedge(sx: number, sy: number, sz: number) {
  // Wedge: triangular prism along Y axis
  // Bottom face is rectangle at z=0, top edge at z=sz
  // Vertices:
  //   0: (0,0,0)  1: (sx,0,0)  2: (sx,sy,0)  3: (0,sy,0)  -- bottom rectangle
  //   4: (0,0,sz) 5: (0,sy,sz) -- top edge (x=0 side)
  const v: number[] = []
  const ix: number[] = []

  // Front face (y=0): triangle 0,1,4
  const nf = [0, -1, 0]
  v.push(0,0,0, nf[0],nf[1],nf[2])
  v.push(sx,0,0, nf[0],nf[1],nf[2])
  v.push(0,0,sz, nf[0],nf[1],nf[2])
  ix.push(0,1,2)

  // Back face (y=sy): triangle 3,5,2
  const nb = [0, 1, 0]
  const bi = v.length / 6
  v.push(0,sy,0, nb[0],nb[1],nb[2])
  v.push(0,sy,sz, nb[0],nb[1],nb[2])
  v.push(sx,sy,0, nb[0],nb[1],nb[2])
  ix.push(bi, bi+1, bi+2)

  // Bottom face (z=0): rectangle 0,3,2,1
  const nd = [0, 0, -1]
  const di = v.length / 6
  v.push(0,0,0, nd[0],nd[1],nd[2])
  v.push(0,sy,0, nd[0],nd[1],nd[2])
  v.push(sx,sy,0, nd[0],nd[1],nd[2])
  v.push(sx,0,0, nd[0],nd[1],nd[2])
  ix.push(di, di+1, di+2, di, di+2, di+3)

  // Left face (x=0): rectangle 0,4,5,3
  const nl = [-1, 0, 0]
  const li = v.length / 6
  v.push(0,0,0, nl[0],nl[1],nl[2])
  v.push(0,0,sz, nl[0],nl[1],nl[2])
  v.push(0,sy,sz, nl[0],nl[1],nl[2])
  v.push(0,sy,0, nl[0],nl[1],nl[2])
  ix.push(li, li+1, li+2, li, li+2, li+3)

  // Slope face: from (sx,0,0)-(sx,sy,0) up to (0,0,sz)-(0,sy,sz)
  // Normal: cross product of edges
  // The slope connects: (sx,0,0), (sx,sy,0), (0,sy,sz), (0,0,sz)
  // Normal = normalize(cross((sx,sy,0)-(sx,0,0), (0,0,sz)-(sx,0,0)))
  //        = normalize(cross((0,sy,0), (-sx,0,sz)))
  //        = (sy*sz, 0, sy*sx) -> normalize -> (sz, 0, sx) / len
  const sn = Math.sqrt(sz*sz + sx*sx)
  const nsSlope = sn > 0 ? [sz/sn, 0, sx/sn] : [0, 0, 1]
  const si2 = v.length / 6
  v.push(sx,0,0, nsSlope[0],nsSlope[1],nsSlope[2])
  v.push(sx,sy,0, nsSlope[0],nsSlope[1],nsSlope[2])
  v.push(0,sy,sz, nsSlope[0],nsSlope[1],nsSlope[2])
  v.push(0,0,sz, nsSlope[0],nsSlope[1],nsSlope[2])
  ix.push(si2, si2+1, si2+2, si2, si2+2, si2+3)

  return { v, ix }
}

function makeTorus(r1: number, r2: number, fn: number) {
  const v: number[] = [], ix: number[] = []
  const ringSegs = fn
  const tubeSegs = Math.max(8, Math.floor(fn * r2 / r1))
  for (let i = 0; i <= ringSegs; i++) {
    const u = (2 * Math.PI * i) / ringSegs
    const cu = Math.cos(u), su = Math.sin(u)
    for (let j = 0; j <= tubeSegs; j++) {
      const vv = (2 * Math.PI * j) / tubeSegs
      const cv = Math.cos(vv), sv = Math.sin(vv)
      const x = (r1 + r2 * cv) * cu
      const y = r2 * sv
      const z = (r1 + r2 * cv) * su
      // Normal: direction from ring center to surface point
      const nx = cv * cu
      const ny = sv
      const nz = cv * su
      v.push(x, y, z, nx, ny, nz)
    }
  }
  for (let i = 0; i < ringSegs; i++) {
    for (let j = 0; j < tubeSegs; j++) {
      const a = i * (tubeSegs + 1) + j
      const b = a + tubeSegs + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }
  return { v, ix }
}

function makeHelix(r: number, pitch: number, turns: number, fn: number) {
  const v: number[] = [], ix: number[] = []
  const tubeR = pitch * 0.15 // tube radius = 15% of pitch
  turns = Math.min(Math.max(turns, 0), 200) // clamp to prevent unbounded vertex generation
  const totalHeight = pitch * turns
  const ringSegs = Math.min(20000, Math.max(16, fn * turns))
  const tubeSegs = Math.max(6, Math.floor(fn / 4))
  for (let i = 0; i <= ringSegs; i++) {
    const t = i / ringSegs
    const angle = 2 * Math.PI * turns * t
    const ca = Math.cos(angle), sa = Math.sin(angle)
    // Center of tube at this point on helix
    const cx = r * ca
    const cy = totalHeight * t
    const cz = r * sa
    // Tangent to helix path
    const tx = -r * sa * 2 * Math.PI * turns
    const ty = totalHeight
    const tz = r * ca * 2 * Math.PI * turns
    const tlen = Math.sqrt(tx * tx + ty * ty + tz * tz) || 1
    const ttx = tx / tlen, tty = ty / tlen, ttz = tz / tlen
    // Build local frame (normal, binormal)
    // Pick an arbitrary vector not parallel to tangent
    let upx = 0, upy = 1, upz = 0
    if (Math.abs(tty) > 0.9) { upx = 1; upy = 0; upz = 0 }
    // binormal = tangent x up
    let bx = tty * upz - ttz * upy
    let by = ttz * upx - ttx * upz
    let bz = ttx * upy - tty * upx
    const blen = Math.sqrt(bx * bx + by * by + bz * bz) || 1
    bx /= blen; by /= blen; bz /= blen
    // normal = binormal x tangent
    let nx = by * ttz - bz * tty
    let ny = bz * ttx - bx * ttz
    let nz = bx * tty - by * ttx
    const nlen = Math.sqrt(nx * nx + ny * ny + nz * nz) || 1
    nx /= nlen; ny /= nlen; nz /= nlen

    for (let j = 0; j <= tubeSegs; j++) {
      const phi = (2 * Math.PI * j) / tubeSegs
      const cp = Math.cos(phi), sp = Math.sin(phi)
      const px = cx + tubeR * (cp * nx + sp * bx)
      const py = cy + tubeR * (cp * ny + sp * by)
      const pz = cz + tubeR * (cp * nz + sp * bz)
      // Surface normal
      const snx = cp * nx + sp * bx
      const sny = cp * ny + sp * by
      const snz = cp * nz + sp * bz
      v.push(px, py, pz, snx, sny, snz)
    }
  }
  for (let i = 0; i < ringSegs; i++) {
    for (let j = 0; j < tubeSegs; j++) {
      const a = i * (tubeSegs + 1) + j
      const b = a + tubeSegs + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }
  return { v, ix }
}

function makeBezier(points: number[][], thickness: number, fn: number) {
  const v: number[] = [], ix: number[] = []
  const tubeR = thickness / 2
  const tubeSegs = Math.max(6, Math.floor(fn / 4))

  // Build cubic Bezier segments: points 0-3, 3-6, 6-9, ...
  const segments: number[][][] = []
  if (points.length <= 4) {
    segments.push(points)
  } else {
    for (let i = 0; i + 3 < points.length; i += 3) {
      segments.push(points.slice(i, i + 4))
    }
    // If remaining points didn't form a complete segment, last segment already covers them
  }

  // Evaluate composite Bezier curve at uniform t in [0, 1]
  const totalSegs = segments.length
  const ringSegs = fn
  const curvePts: number[][] = []
  for (let i = 0; i <= ringSegs; i++) {
    const tGlobal = i / ringSegs
    const segF = tGlobal * totalSegs
    const segIdx = Math.min(Math.floor(segF), totalSegs - 1)
    const t = segF - segIdx
    const seg = segments[segIdx]
    if (seg.length >= 4) {
      // Cubic Bezier: B(t) = (1-t)^3*P0 + 3(1-t)^2*t*P1 + 3(1-t)*t^2*P2 + t^3*P3
      const u = 1 - t
      const u2 = u * u, u3 = u2 * u
      const t2 = t * t, t3 = t2 * t
      curvePts.push([
        u3 * seg[0][0] + 3 * u2 * t * seg[1][0] + 3 * u * t2 * seg[2][0] + t3 * seg[3][0],
        u3 * seg[0][1] + 3 * u2 * t * seg[1][1] + 3 * u * t2 * seg[2][1] + t3 * seg[3][1],
        u3 * seg[0][2] + 3 * u2 * t * seg[1][2] + 3 * u * t2 * seg[2][2] + t3 * seg[3][2],
      ])
    } else if (seg.length === 3) {
      // Quadratic Bezier
      const u = 1 - t
      curvePts.push([
        u * u * seg[0][0] + 2 * u * t * seg[1][0] + t * t * seg[2][0],
        u * u * seg[0][1] + 2 * u * t * seg[1][1] + t * t * seg[2][1],
        u * u * seg[0][2] + 2 * u * t * seg[1][2] + t * t * seg[2][2],
      ])
    } else {
      // Linear interpolation
      const u = 1 - t
      curvePts.push([
        u * seg[0][0] + t * seg[seg.length - 1][0],
        u * seg[0][1] + t * seg[seg.length - 1][1],
        u * seg[0][2] + t * seg[seg.length - 1][2],
      ])
    }
  }

  // Generate tube using Frenet frame (same approach as makeHelix)
  for (let i = 0; i <= ringSegs; i++) {
    const cur = curvePts[i]
    // Compute tangent via finite differences
    let tx: number, ty: number, tz: number
    if (i < ringSegs) {
      const next = curvePts[i + 1]
      tx = next[0] - cur[0]; ty = next[1] - cur[1]; tz = next[2] - cur[2]
    } else {
      const prev = curvePts[i - 1]
      tx = cur[0] - prev[0]; ty = cur[1] - prev[1]; tz = cur[2] - prev[2]
    }
    const tlen = Math.sqrt(tx * tx + ty * ty + tz * tz) || 1
    const ttx = tx / tlen, tty = ty / tlen, ttz = tz / tlen
    // Pick an arbitrary vector not parallel to tangent
    let upx = 0, upy = 1, upz = 0
    if (Math.abs(tty) > 0.9) { upx = 1; upy = 0; upz = 0 }
    // binormal = tangent x up
    let bx = tty * upz - ttz * upy
    let by = ttz * upx - ttx * upz
    let bz = ttx * upy - tty * upx
    const blen = Math.sqrt(bx * bx + by * by + bz * bz) || 1
    bx /= blen; by /= blen; bz /= blen
    // normal = binormal x tangent
    let nx = by * ttz - bz * tty
    let ny = bz * ttx - bx * ttz
    let nz = bx * tty - by * ttx
    const nlen = Math.sqrt(nx * nx + ny * ny + nz * nz) || 1
    nx /= nlen; ny /= nlen; nz /= nlen

    for (let j = 0; j <= tubeSegs; j++) {
      const phi = (2 * Math.PI * j) / tubeSegs
      const cp = Math.cos(phi), sp = Math.sin(phi)
      const px = cur[0] + tubeR * (cp * nx + sp * bx)
      const py = cur[1] + tubeR * (cp * ny + sp * by)
      const pz = cur[2] + tubeR * (cp * nz + sp * bz)
      // Surface normal
      const snx = cp * nx + sp * bx
      const sny = cp * ny + sp * by
      const snz = cp * nz + sp * bz
      v.push(px, py, pz, snx, sny, snz)
    }
  }
  for (let i = 0; i < ringSegs; i++) {
    for (let j = 0; j < tubeSegs; j++) {
      const a = i * (tubeSegs + 1) + j
      const b = a + tubeSegs + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }
  return { v, ix }
}

function makeSweep(path: number[][], radius: number, fn: number) {
  const v: number[] = [], ix: number[] = []
  const tubeSegs = fn
  // Cap total ring*tube segment count at 20000 to prevent DoS
  const maxRings = Math.max(2, Math.floor(20000 / Math.max(1, tubeSegs)))
  if (path.length > maxRings) path = path.slice(0, maxRings)
  const ringSegs = path.length - 1

  // Compute tangents for each path point
  const tangents: number[][] = []
  for (let i = 0; i < path.length; i++) {
    let tx: number, ty: number, tz: number
    if (i < path.length - 1) {
      tx = path[i + 1][0] - path[i][0]
      ty = path[i + 1][1] - path[i][1]
      tz = path[i + 1][2] - path[i][2]
    } else {
      // Last point: reuse previous tangent
      tx = tangents[i - 1][0]; ty = tangents[i - 1][1]; tz = tangents[i - 1][2]
    }
    const tlen = Math.sqrt(tx * tx + ty * ty + tz * tz) || 1
    tangents.push([tx / tlen, ty / tlen, tz / tlen])
  }

  // Parallel transport frame
  const normals: number[][] = []
  const binormals: number[][] = []

  // Initialize first frame
  const t0 = tangents[0]
  let upx = 0, upy = 1, upz = 0
  if (Math.abs(t0[1]) > 0.9) { upx = 1; upy = 0; upz = 0 }
  // binormal = tangent x up
  let bx = t0[1] * upz - t0[2] * upy
  let by = t0[2] * upx - t0[0] * upz
  let bz = t0[0] * upy - t0[1] * upx
  let blen = Math.sqrt(bx * bx + by * by + bz * bz) || 1
  bx /= blen; by /= blen; bz /= blen
  // normal = binormal x tangent
  let nx = by * t0[2] - bz * t0[1]
  let ny = bz * t0[0] - bx * t0[2]
  let nz = bx * t0[1] - by * t0[0]
  let nlen = Math.sqrt(nx * nx + ny * ny + nz * nz) || 1
  nx /= nlen; ny /= nlen; nz /= nlen
  normals.push([nx, ny, nz])
  binormals.push([bx, by, bz])

  // Propagate frame along path using parallel transport
  for (let i = 1; i < path.length; i++) {
    const tPrev = tangents[i - 1]
    const tCur = tangents[i]
    // Rotation axis = tPrev x tCur
    const ax = tPrev[1] * tCur[2] - tPrev[2] * tCur[1]
    const ay = tPrev[2] * tCur[0] - tPrev[0] * tCur[2]
    const az = tPrev[0] * tCur[1] - tPrev[1] * tCur[0]
    const alen = Math.sqrt(ax * ax + ay * ay + az * az)
    if (alen < 1e-10) {
      // Tangents are parallel, keep previous frame
      normals.push([normals[i - 1][0], normals[i - 1][1], normals[i - 1][2]])
      binormals.push([binormals[i - 1][0], binormals[i - 1][1], binormals[i - 1][2]])
    } else {
      // Rotate previous normal by angle between tangents around rotation axis
      const dot = tPrev[0] * tCur[0] + tPrev[1] * tCur[1] + tPrev[2] * tCur[2]
      const angle = Math.acos(Math.max(-1, Math.min(1, dot)))
      const ux = ax / alen, uy = ay / alen, uz = az / alen
      const cosA = Math.cos(angle), sinA = Math.sin(angle)
      // Rodrigues' rotation formula on the previous normal
      const pn = normals[i - 1]
      const dotUN = ux * pn[0] + uy * pn[1] + uz * pn[2]
      const crossX = uy * pn[2] - uz * pn[1]
      const crossY = uz * pn[0] - ux * pn[2]
      const crossZ = ux * pn[1] - uy * pn[0]
      let rnx = pn[0] * cosA + crossX * sinA + ux * dotUN * (1 - cosA)
      let rny = pn[1] * cosA + crossY * sinA + uy * dotUN * (1 - cosA)
      let rnz = pn[2] * cosA + crossZ * sinA + uz * dotUN * (1 - cosA)
      const rnlen = Math.sqrt(rnx * rnx + rny * rny + rnz * rnz) || 1
      rnx /= rnlen; rny /= rnlen; rnz /= rnlen
      normals.push([rnx, rny, rnz])
      // binormal = tangent x normal
      const rbx = tCur[1] * rnz - tCur[2] * rny
      const rby = tCur[2] * rnx - tCur[0] * rnz
      const rbz = tCur[0] * rny - tCur[1] * rnx
      const rblen = Math.sqrt(rbx * rbx + rby * rby + rbz * rbz) || 1
      binormals.push([rbx / rblen, rby / rblen, rbz / rblen])
    }
  }

  // Generate tube vertices at each path point
  for (let i = 0; i < path.length; i++) {
    const cur = path[i]
    const n = normals[i], b = binormals[i]
    for (let j = 0; j <= tubeSegs; j++) {
      const phi = (2 * Math.PI * j) / tubeSegs
      const cp = Math.cos(phi), sp = Math.sin(phi)
      const px = cur[0] + radius * (cp * n[0] + sp * b[0])
      const py = cur[1] + radius * (cp * n[1] + sp * b[1])
      const pz = cur[2] + radius * (cp * n[2] + sp * b[2])
      // Surface normal
      const snx = cp * n[0] + sp * b[0]
      const sny = cp * n[1] + sp * b[1]
      const snz = cp * n[2] + sp * b[2]
      v.push(px, py, pz, snx, sny, snz)
    }
  }
  // Connect rings with triangles
  for (let i = 0; i < ringSegs; i++) {
    for (let j = 0; j < tubeSegs; j++) {
      const a = i * (tubeSegs + 1) + j
      const b = a + tubeSegs + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }
  return { v, ix }
}

function makeStar(points: number, r1: number, r2: number, h: number, _fn: number): { v: number[], ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const n2 = points * 2 // number of vertices around the star profile

  // Build 2D star profile
  const profile: number[][] = []
  for (let i = 0; i < n2; i++) {
    const angle = (i * Math.PI) / points
    const r = (i % 2 === 0) ? r1 : r2
    profile.push([r * Math.cos(angle), r * Math.sin(angle)])
  }

  // Bottom cap (z = 0), normal facing -Z
  const baseBot = v.length / 6
  for (let i = 0; i < n2; i++) {
    v.push(profile[i][0], profile[i][1], 0, 0, 0, -1)
  }
  const botTris = earClip(profile)
  // Reverse winding for bottom cap so normal faces -Z
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(baseBot + botTris[i], baseBot + botTris[i + 2], baseBot + botTris[i + 1])
  }

  // Top cap (z = h), normal facing +Z
  const baseTop = v.length / 6
  for (let i = 0; i < n2; i++) {
    v.push(profile[i][0], profile[i][1], h, 0, 0, 1)
  }
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(baseTop + botTris[i], baseTop + botTris[i + 1], baseTop + botTris[i + 2])
  }

  // Side walls
  for (let i = 0; i < n2; i++) {
    const i2 = (i + 1) % n2
    const x0 = profile[i][0], y0 = profile[i][1]
    const x1 = profile[i2][0], y1 = profile[i2][1]
    // Edge direction
    const ex = x1 - x0, ey = y1 - y0
    // Outward normal (perpendicular to edge, in XY plane)
    const len = Math.sqrt(ex * ex + ey * ey) || 1
    const nx = ey / len, ny = -ex / len

    const base = v.length / 6
    v.push(x0, y0, 0, nx, ny, 0) // bottom-left
    v.push(x1, y1, 0, nx, ny, 0) // bottom-right
    v.push(x1, y1, h, nx, ny, 0) // top-right
    v.push(x0, y0, h, nx, ny, 0) // top-left
    ix.push(base, base + 1, base + 2, base, base + 2, base + 3)
  }

  return { v, ix }
}

function makePrism(sides: number, r: number, h: number, center: boolean): { v: number[], ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const z0 = center ? -h / 2 : 0, z1 = center ? h / 2 : h

  // Build polygon profile
  const profile: number[][] = []
  for (let i = 0; i < sides; i++) {
    const angle = (2 * Math.PI * i) / sides
    profile.push([r * Math.cos(angle), r * Math.sin(angle)])
  }

  // Bottom cap (z0, normal -Z)
  const baseBot = v.length / 6
  for (let i = 0; i < sides; i++) {
    v.push(profile[i][0], profile[i][1], z0, 0, 0, -1)
  }
  const botTris = earClip(profile)
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(baseBot + botTris[i], baseBot + botTris[i + 2], baseBot + botTris[i + 1])
  }

  // Top cap (z1, normal +Z)
  const baseTop = v.length / 6
  for (let i = 0; i < sides; i++) {
    v.push(profile[i][0], profile[i][1], z1, 0, 0, 1)
  }
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(baseTop + botTris[i], baseTop + botTris[i + 1], baseTop + botTris[i + 2])
  }

  // Side walls
  for (let i = 0; i < sides; i++) {
    const i2 = (i + 1) % sides
    const x0 = profile[i][0], y0 = profile[i][1]
    const x1 = profile[i2][0], y1 = profile[i2][1]
    const ex = x1 - x0, ey = y1 - y0
    const len = Math.sqrt(ex * ex + ey * ey) || 1
    const nx = ey / len, ny = -ex / len

    const base = v.length / 6
    v.push(x0, y0, z0, nx, ny, 0)
    v.push(x1, y1, z0, nx, ny, 0)
    v.push(x1, y1, z1, nx, ny, 0)
    v.push(x0, y0, z1, nx, ny, 0)
    ix.push(base, base + 1, base + 2, base, base + 2, base + 3)
  }

  return { v, ix }
}

function makeCapsule(r: number, h: number, center: boolean, fn: number): { v: number[], ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const totalH = h + 2 * r
  const z0 = center ? -totalH / 2 : 0
  const cylBot = z0 + r
  const cylTop = cylBot + h

  // Cylinder body
  for (let i = 0; i <= fn; i++) {
    const a = (2 * Math.PI * i) / fn, ca = Math.cos(a), sa = Math.sin(a)
    v.push(r * ca, r * sa, cylBot, ca, sa, 0)
    v.push(r * ca, r * sa, cylTop, ca, sa, 0)
  }
  for (let i = 0; i < fn; i++) {
    const a2 = i * 2; ix.push(a2, a2 + 1, a2 + 2, a2 + 2, a2 + 1, a2 + 3)
  }

  // Bottom hemisphere (centered at cylBot)
  const halfSeg = Math.max(4, Math.floor(fn / 2))
  const botBase = v.length / 6
  for (let ri = 0; ri <= halfSeg; ri++) {
    const phi = Math.PI / 2 + (Math.PI / 2) * ri / halfSeg  // PI/2 to PI
    const sp = Math.sin(phi), cp = Math.cos(phi)
    for (let si = 0; si <= fn; si++) {
      const th = 2 * Math.PI * si / fn
      const nx = sp * Math.cos(th), ny = sp * Math.sin(th), nz = cp
      v.push(r * nx, r * ny, cylBot + r * nz, nx, ny, nz)
    }
  }
  for (let ri = 0; ri < halfSeg; ri++)
    for (let si = 0; si < fn; si++) {
      const a = botBase + ri * (fn + 1) + si, b = a + fn + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }

  // Top hemisphere (centered at cylTop)
  const topBase = v.length / 6
  for (let ri = 0; ri <= halfSeg; ri++) {
    const phi = (Math.PI / 2) * ri / halfSeg  // 0 to PI/2
    const sp = Math.sin(phi), cp = Math.cos(phi)
    for (let si = 0; si <= fn; si++) {
      const th = 2 * Math.PI * si / fn
      const nx = sp * Math.cos(th), ny = sp * Math.sin(th), nz = cp
      v.push(r * nx, r * ny, cylTop + r * nz, nx, ny, nz)
    }
  }
  for (let ri = 0; ri < halfSeg; ri++)
    for (let si = 0; si < fn; si++) {
      const a = topBase + ri * (fn + 1) + si, b = a + fn + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }

  return { v, ix }
}

function makeGear(teeth: number, mod: number, thickness: number, fn: number): { v: number[], ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const tipR = mod * teeth / 2
  const valleyR = tipR - mod
  const n2 = teeth * 2
  const segsPerSide = Math.max(1, fn)

  // Build 2D gear profile: alternating tip/valley arcs
  const profile: number[][] = []
  for (let i = 0; i < n2; i++) {
    const frac = i / n2
    const nextFrac = (i + 1) / n2
    const isTip = i % 2 === 0
    const r1g = isTip ? tipR : valleyR
    const r2g = isTip ? valleyR : tipR
    for (let s = 0; s < segsPerSide; s++) {
      const t = s / segsPerSide
      const angle = 2 * Math.PI * (frac + t * (nextFrac - frac))
      const rr = r1g + (r2g - r1g) * t
      profile.push([rr * Math.cos(angle), rr * Math.sin(angle)])
    }
  }

  const np = profile.length

  // Bottom cap (z=0, normal -Z)
  const baseBot = v.length / 6
  for (let i = 0; i < np; i++) {
    v.push(profile[i][0], profile[i][1], 0, 0, 0, -1)
  }
  const botTris = earClip(profile)
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(baseBot + botTris[i], baseBot + botTris[i + 2], baseBot + botTris[i + 1])
  }

  // Top cap (z=thickness, normal +Z)
  const baseTop = v.length / 6
  for (let i = 0; i < np; i++) {
    v.push(profile[i][0], profile[i][1], thickness, 0, 0, 1)
  }
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(baseTop + botTris[i], baseTop + botTris[i + 1], baseTop + botTris[i + 2])
  }

  // Side walls
  for (let i = 0; i < np; i++) {
    const i2 = (i + 1) % np
    const x0 = profile[i][0], y0 = profile[i][1]
    const x1 = profile[i2][0], y1 = profile[i2][1]
    const ex = x1 - x0, ey = y1 - y0
    const len = Math.sqrt(ex * ex + ey * ey) || 1
    const nx = ey / len, ny = -ex / len

    const base = v.length / 6
    v.push(x0, y0, 0, nx, ny, 0)
    v.push(x1, y1, 0, nx, ny, 0)
    v.push(x1, y1, thickness, nx, ny, 0)
    v.push(x0, y0, thickness, nx, ny, 0)
    ix.push(base, base + 1, base + 2, base, base + 2, base + 3)
  }

  return { v, ix }
}

function makeThread(d: number, pitch: number, length: number, fn: number): { v: number[], ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const baseR = d / 2
  const threadDepth = pitch * 0.3
  // Clamp length/pitch ratio and cap total slices to prevent unbounded vertex generation
  const ratio = pitch > 0 ? Math.min(Math.ceil(length / pitch), 20000) : 1
  const zSlices = Math.min(20000, ratio * Math.max(1, Math.floor(fn / 4)))
  const segs = fn // segments around circumference

  // Generate vertices
  for (let zi = 0; zi <= zSlices; zi++) {
    const z = (zi / zSlices) * length
    for (let ti = 0; ti <= segs; ti++) {
      const theta = (2 * Math.PI * ti) / segs
      const sinVal = Math.sin(2 * Math.PI * (z / pitch - theta / (2 * Math.PI)))
      const modulation = Math.max(0, sinVal)
      const r = baseR - threadDepth * modulation

      const x = r * Math.cos(theta)
      const y = r * Math.sin(theta)

      // Approximate normal by computing partial derivatives
      // Radial direction gives the main normal component
      const drdTheta = -threadDepth * Math.max(0, Math.cos(2 * Math.PI * (z / pitch - theta / (2 * Math.PI)))) * (sinVal > 0 ? 1 : 0) / (2 * Math.PI) * (2 * Math.PI) / (2 * Math.PI)
      const nx0 = Math.cos(theta)
      const ny0 = Math.sin(theta)
      // For simplicity, use the radial normal adjusted slightly
      // The tangent along theta: (-r*sin(theta) + drdTheta*cos(theta), r*cos(theta) + drdTheta*sin(theta), 0)
      // The tangent along z: (drdz*cos(theta), drdz*sin(theta), 1)
      // Normal = cross(tangent_theta, tangent_z)
      const drdz_raw = -threadDepth * Math.cos(2 * Math.PI * (z / pitch - theta / (2 * Math.PI))) * (2 * Math.PI / pitch) * (sinVal > 0 ? 1 : 0)
      const drdz = sinVal > 0 ? drdz_raw : 0

      // tangent along theta
      const ttx = -r * Math.sin(theta) + drdTheta * Math.cos(theta)
      const tty = r * Math.cos(theta) + drdTheta * Math.sin(theta)
      const ttz = 0
      // tangent along z
      const tzx = drdz * Math.cos(theta)
      const tzy = drdz * Math.sin(theta)
      const tzz = 1

      // cross product: tangent_theta x tangent_z
      let cnx = tty * tzz - ttz * tzy
      let cny = ttz * tzx - ttx * tzz
      let cnz = ttx * tzy - tty * tzx

      const clen = Math.sqrt(cnx * cnx + cny * cny + cnz * cnz) || 1
      cnx /= clen; cny /= clen; cnz /= clen

      // Ensure normal points outward (dot with radial direction should be positive)
      if (cnx * nx0 + cny * ny0 < 0) {
        cnx = -cnx; cny = -cny; cnz = -cnz
      }

      v.push(x, y, z, cnx, cny, cnz)
    }
  }

  // Generate indices
  for (let zi = 0; zi < zSlices; zi++) {
    for (let ti = 0; ti < segs; ti++) {
      const a = zi * (segs + 1) + ti
      const b = a + segs + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }

  // Bottom cap (z = 0)
  const botCenter = v.length / 6
  v.push(0, 0, 0, 0, 0, -1)
  const botRing = v.length / 6
  for (let ti = 0; ti < segs; ti++) {
    const theta = (2 * Math.PI * ti) / segs
    const sinVal = Math.sin(2 * Math.PI * (0 / pitch - theta / (2 * Math.PI)))
    const modulation = Math.max(0, sinVal)
    const r = baseR - threadDepth * modulation
    v.push(r * Math.cos(theta), r * Math.sin(theta), 0, 0, 0, -1)
  }
  for (let ti = 0; ti < segs; ti++) {
    const next = (ti + 1) % segs
    ix.push(botCenter, botRing + next, botRing + ti)
  }

  // Top cap (z = length)
  const topCenter = v.length / 6
  v.push(0, 0, length, 0, 0, 1)
  const topRing = v.length / 6
  for (let ti = 0; ti < segs; ti++) {
    const theta = (2 * Math.PI * ti) / segs
    const sinVal = Math.sin(2 * Math.PI * (length / pitch - theta / (2 * Math.PI)))
    const modulation = Math.max(0, sinVal)
    const r = baseR - threadDepth * modulation
    v.push(r * Math.cos(theta), r * Math.sin(theta), length, 0, 0, 1)
  }
  for (let ti = 0; ti < segs; ti++) {
    const next = (ti + 1) % segs
    ix.push(topCenter, topRing + ti, topRing + next)
  }

  return { v, ix }
}

function pointInTriangle(px: number, py: number, ax: number, ay: number, bx: number, by: number, cx: number, cy: number): boolean {
  // Strict containment: a point lying on an edge of the triangle does NOT count
  // as inside (so it never blocks an otherwise-valid ear). A point counts as
  // inside only if it is on the same side of every edge by more than epsilon.
  const eps = 1e-9
  const d1 = (px - bx) * (ay - by) - (ax - bx) * (py - by)
  const d2 = (px - cx) * (by - cy) - (bx - cx) * (py - cy)
  const d3 = (px - ax) * (cy - ay) - (cx - ax) * (py - ay)
  const hasNeg = (d1 < -eps) || (d2 < -eps) || (d3 < -eps)
  const hasPos = (d1 > eps) || (d2 > eps) || (d3 > eps)
  return !(hasNeg && hasPos)
}

function earClip(pts: number[][]): number[] {
  const n = pts.length
  if (n < 3) return []
  if (n === 3) return [0, 1, 2]

  // Determine winding order (positive = CCW)
  let area = 0
  for (let i = 0; i < n; i++) {
    const j = (i + 1) % n
    area += (pts[i][0] ?? 0) * (pts[j][1] ?? 0)
    area -= (pts[j][0] ?? 0) * (pts[i][1] ?? 0)
  }
  const ccw = area > 0

  const remaining = Array.from({ length: n }, (_, i) => i)
  const tris: number[] = []

  // Epsilon for treating a candidate ear's area as ~0 (collinear / degenerate).
  const areaEps = 1e-9

  while (remaining.length > 2) {
    let found = false
    for (let i = 0; i < remaining.length; i++) {
      const prev = remaining[(i - 1 + remaining.length) % remaining.length]
      const cur = remaining[i]
      const next = remaining[(i + 1) % remaining.length]

      const ax = pts[prev][0] ?? 0, ay = pts[prev][1] ?? 0
      const bx = pts[cur][0] ?? 0, by = pts[cur][1] ?? 0
      const cx = pts[next][0] ?? 0, cy = pts[next][1] ?? 0

      // Cross product to check if ear is convex
      const cross = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)

      // Skip near-degenerate (collinear / zero-area) candidate ears so the
      // algorithm doesn't stall on them.
      if (Math.abs(cross) < areaEps) continue

      const isConvex = ccw ? cross > 0 : cross < 0
      if (!isConvex) continue

      // Check no other point is strictly inside this triangle
      let hasInside = false
      for (const idx of remaining) {
        if (idx === prev || idx === cur || idx === next) continue
        const ppx = pts[idx][0] ?? 0, ppy = pts[idx][1] ?? 0
        if (pointInTriangle(ppx, ppy, ax, ay, bx, by, cx, cy)) {
          hasInside = true
          break
        }
      }
      if (hasInside) continue

      tris.push(prev, cur, next)
      remaining.splice(i, 1)
      found = true
      break
    }

    // No valid ear found in a full pass but vertices remain (self-intersecting,
    // collinear, or otherwise degenerate polygon). Fall back to a fan
    // triangulation of the remaining vertices so the cap is fully covered
    // (no holes) rather than returning a partial result.
    if (!found) {
      const anchor = remaining[0]
      for (let i = 1; i < remaining.length - 1; i++) {
        tris.push(anchor, remaining[i], remaining[i + 1])
      }
      break
    }
  }

  return tris
}

function makePolygon(points: number[][]) {
  if (points.length < 3) return { v: [] as number[], ix: [] as number[] }
  const h = 0.01
  const v: number[] = []
  const ix: number[] = []

  // Triangulate using ear clipping
  const indices = earClip(points)

  // Top face (z = h)
  for (const p of points) {
    v.push(p[0] ?? 0, p[1] ?? 0, h, 0, 0, 1)
  }
  for (let i = 0; i < indices.length; i += 3) {
    ix.push(indices[i], indices[i+1], indices[i+2])
  }

  // Bottom face (z = 0)
  const bOff = points.length
  for (const p of points) {
    v.push(p[0] ?? 0, p[1] ?? 0, 0, 0, 0, -1)
  }
  for (let i = 0; i < indices.length; i += 3) {
    ix.push(bOff + indices[i], bOff + indices[i+2], bOff + indices[i+1])
  }

  return { v, ix }
}

function makeHoneycomb(rows: number, cols: number, r: number, h: number, wall: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const sqrt3 = Math.sqrt(3)
  // Spacing between hex centers
  const xSpacing = r * 1.5 + wall
  const ySpacing = r * sqrt3 + wall
  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < cols; col++) {
      const cx = col * xSpacing
      const cy = row * ySpacing + (col % 2) * (ySpacing / 2)
      // Create hexagonal prism at (cx, cy)
      const hexVerts: number[][] = []
      for (let i = 0; i < 6; i++) {
        const angle = (Math.PI / 3) * i + Math.PI / 6
        hexVerts.push([cx + r * Math.cos(angle), cy + r * Math.sin(angle)])
      }
      // Bottom cap
      const baseBot = v.length / 6
      for (let i = 0; i < 6; i++) {
        v.push(hexVerts[i][0], hexVerts[i][1], 0, 0, 0, -1)
      }
      // Fan triangulation for bottom (reversed winding)
      for (let i = 1; i < 5; i++) {
        ix.push(baseBot, baseBot + i + 1, baseBot + i)
      }
      // Top cap
      const baseTop = v.length / 6
      for (let i = 0; i < 6; i++) {
        v.push(hexVerts[i][0], hexVerts[i][1], h, 0, 0, 1)
      }
      for (let i = 1; i < 5; i++) {
        ix.push(baseTop, baseTop + i, baseTop + i + 1)
      }
      // Side walls
      for (let i = 0; i < 6; i++) {
        const i2 = (i + 1) % 6
        const x0 = hexVerts[i][0], y0 = hexVerts[i][1]
        const x1 = hexVerts[i2][0], y1 = hexVerts[i2][1]
        const ex = x1 - x0, ey = y1 - y0
        const len = Math.sqrt(ex * ex + ey * ey) || 1
        const nx = ey / len, ny = -ex / len
        const base = v.length / 6
        v.push(x0, y0, 0, nx, ny, 0)
        v.push(x1, y1, 0, nx, ny, 0)
        v.push(x1, y1, h, nx, ny, 0)
        v.push(x0, y0, h, nx, ny, 0)
        ix.push(base, base + 1, base + 2, base, base + 2, base + 3)
      }
    }
  }
  return { v, ix }
}

function makeKnurl(d: number, h: number, pitch: number, depth: number, fn: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const baseR = d / 2
  const nTeeth = Math.max(1, Math.round(Math.PI * d / pitch))
  const nVertical = Math.max(1, Math.round(h / pitch))
  const zSlices = Math.max(8, fn)
  const segs = fn

  for (let zi = 0; zi <= zSlices; zi++) {
    const z = (zi / zSlices) * h
    for (let ti = 0; ti <= segs; ti++) {
      const theta = (2 * Math.PI * ti) / segs
      // Diamond knurl: two crossed sine waves
      const mod1 = Math.sin(nTeeth * theta)
      const mod2 = Math.sin(nVertical * 2 * Math.PI * z / h)
      const r = baseR + depth * mod1 * mod2

      const x = r * Math.cos(theta)
      const y = r * Math.sin(theta)

      // Approximate normal (radial)
      const nx0 = Math.cos(theta)
      const ny0 = Math.sin(theta)
      v.push(x, y, z, nx0, ny0, 0)
    }
  }

  for (let zi = 0; zi < zSlices; zi++) {
    for (let ti = 0; ti < segs; ti++) {
      const a = zi * (segs + 1) + ti
      const b = a + segs + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }

  // Bottom cap
  const botCenter = v.length / 6
  v.push(0, 0, 0, 0, 0, -1)
  const botRing = v.length / 6
  for (let ti = 0; ti < segs; ti++) {
    const theta = (2 * Math.PI * ti) / segs
    const mod1 = Math.sin(nTeeth * theta)
    const mod2 = Math.sin(0) // z=0
    const r = baseR + depth * mod1 * mod2
    v.push(r * Math.cos(theta), r * Math.sin(theta), 0, 0, 0, -1)
  }
  for (let ti = 0; ti < segs; ti++) {
    const next = (ti + 1) % segs
    ix.push(botCenter, botRing + next, botRing + ti)
  }

  // Top cap
  const topCenter = v.length / 6
  v.push(0, 0, h, 0, 0, 1)
  const topRing = v.length / 6
  for (let ti = 0; ti < segs; ti++) {
    const theta = (2 * Math.PI * ti) / segs
    const mod1 = Math.sin(nTeeth * theta)
    const mod2 = Math.sin(nVertical * 2 * Math.PI)
    const r = baseR + depth * mod1 * mod2
    v.push(r * Math.cos(theta), r * Math.sin(theta), h, 0, 0, 1)
  }
  for (let ti = 0; ti < segs; ti++) {
    const next = (ti + 1) % segs
    ix.push(topCenter, topRing + ti, topRing + next)
  }

  return { v, ix }
}

function makeChamferCube(sx: number, sy: number, sz: number, chamfer: number, center: boolean): { v: number[]; ix: number[] } {
  const c = Math.min(chamfer, sx / 2, sy / 2, sz / 2)
  const x0 = center ? -sx / 2 : 0
  const y0 = center ? -sy / 2 : 0
  const z0 = center ? -sz / 2 : 0
  const x1 = x0 + sx
  const y1 = y0 + sy
  const z1 = z0 + sz

  // 24 vertices: for each of 8 corners, 3 vertices offset inward along each edge
  const points: number[][] = []
  const corners: [number, number, number][] = [
    [x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0],
    [x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1],
  ]
  // For each corner, generate 3 chamfer vertices (offset along each adjacent edge)
  const dirs: [number, number, number][][] = [
    [[1,0,0],[0,1,0],[0,0,1]],   // corner 0 (x0,y0,z0)
    [[-1,0,0],[0,1,0],[0,0,1]],  // corner 1 (x1,y0,z0)
    [[-1,0,0],[0,-1,0],[0,0,1]], // corner 2 (x1,y1,z0)
    [[1,0,0],[0,-1,0],[0,0,1]],  // corner 3 (x0,y1,z0)
    [[1,0,0],[0,1,0],[0,0,-1]],  // corner 4 (x0,y0,z1)
    [[-1,0,0],[0,1,0],[0,0,-1]], // corner 5 (x1,y0,z1)
    [[-1,0,0],[0,-1,0],[0,0,-1]],// corner 6 (x1,y1,z1)
    [[1,0,0],[0,-1,0],[0,0,-1]], // corner 7 (x0,y1,z1)
  ]

  for (let ci = 0; ci < 8; ci++) {
    const [cx, cy, cz] = corners[ci]
    for (const [dx, dy, dz] of dirs[ci]) {
      points.push([cx + c * dx, cy + c * dy, cz + c * dz])
    }
  }

  // Build faces. Each original face is a rectangle with corners cut.
  // The 24 vertices are indexed as: corner i => vertices 3*i, 3*i+1, 3*i+2
  // where vertex 3*i+j is offset along direction j of that corner.
  // Faces:
  // Bottom (z0): corners 0,1,2,3 - use z-edge vertices (index +2 for each corner)
  //   but actually we need the vertices that lie on the bottom face.
  //   For corner 0: the x-offset (idx 0) and y-offset (idx 1) lie on z=z0
  //   For corner 1: the x-offset (idx 3) and y-offset (idx 4) lie on z=z0
  //   etc.
  const faceDefs: { verts: number[]; nx: number; ny: number; nz: number }[] = [
    // Bottom face (z=z0): vertices that have z=z0, i.e. x and y offsets of bottom corners
    { verts: [0,1, 4,3, 7,6, 10,11], nx: 0, ny: 0, nz: -1 },
    // Top face (z=z1): vertices that have z=z1
    { verts: [12,13, 16,15, 19,18, 22,23], nx: 0, ny: 0, nz: 1 },
    // Front face (y=y0): corners 0,1,5,4, y-offset vertices
    { verts: [1,2, 5,3, 14,15, 13,12], nx: 0, ny: -1, nz: 0 },
    // Wait, let me re-index properly...
  ]
  void faceDefs

  // Simpler approach: use convex hull on the 24 points
  const hullPts: [number, number, number][] = points.map(p => [p[0], p[1], p[2]])
  return convexHull3D(hullPts)
}

function makeLoft(profiles: number[][][], heights: number[], fn: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  if (profiles.length < 2 || heights.length < 2) return { v, ix }

  // Ensure all profiles have the same number of vertices by resampling
  const maxVerts = Math.max(...profiles.map(p => p.length))
  const targetVerts = Math.max(maxVerts, fn)

  // Resample a profile to have exactly n points
  function resampleProfile(profile: number[][], n: number): number[][] {
    if (profile.length === n) return profile
    const result: number[][] = []
    const len = profile.length
    for (let i = 0; i < n; i++) {
      const t = (i / n) * len
      const idx = Math.floor(t)
      const frac = t - idx
      const p0 = profile[idx % len]
      const p1 = profile[(idx + 1) % len]
      result.push([
        p0[0] + frac * (p1[0] - p0[0]),
        p0[1] + frac * (p1[1] - p0[1]),
      ])
    }
    return result
  }

  const resampledProfiles = profiles.map(p => resampleProfile(p, targetVerts))

  // Generate vertices for each profile at its height
  for (let pi = 0; pi < resampledProfiles.length; pi++) {
    const profile = resampledProfiles[pi]
    const z = heights[pi] ?? (pi * 10)
    for (let vi2 = 0; vi2 < targetVerts; vi2++) {
      const x = profile[vi2][0]
      const y = profile[vi2][1]
      // Approximate normal: outward from centroid
      let cx = 0, cy = 0
      for (const pt of profile) { cx += pt[0]; cy += pt[1] }
      cx /= profile.length; cy /= profile.length
      const dx = x - cx, dy = y - cy
      const nl = Math.sqrt(dx * dx + dy * dy) || 1
      v.push(x, y, z, dx / nl, dy / nl, 0)
    }
  }

  // Connect adjacent profiles with quads
  for (let pi = 0; pi < resampledProfiles.length - 1; pi++) {
    for (let vi2 = 0; vi2 < targetVerts; vi2++) {
      const nextVi = (vi2 + 1) % targetVerts
      const a = pi * targetVerts + vi2
      const b = pi * targetVerts + nextVi
      const c = (pi + 1) * targetVerts + vi2
      const d = (pi + 1) * targetVerts + nextVi
      ix.push(a, b, d, a, d, c)
    }
  }

  // Bottom cap
  const botOff = v.length / 6
  const botProfile = resampledProfiles[0]
  const botZ = heights[0] ?? 0
  for (let i = 0; i < targetVerts; i++) {
    v.push(botProfile[i][0], botProfile[i][1], botZ, 0, 0, -1)
  }
  const botTris = earClip(botProfile)
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(botOff + botTris[i], botOff + botTris[i + 2], botOff + botTris[i + 1])
  }

  // Top cap
  const topOff = v.length / 6
  const topProfile = resampledProfiles[resampledProfiles.length - 1]
  const topZ = heights[heights.length - 1] ?? ((resampledProfiles.length - 1) * 10)
  for (let i = 0; i < targetVerts; i++) {
    v.push(topProfile[i][0], topProfile[i][1], topZ, 0, 0, 1)
  }
  const topTris = earClip(topProfile)
  for (let i = 0; i < topTris.length; i += 3) {
    ix.push(topOff + topTris[i], topOff + topTris[i + 1], topOff + topTris[i + 2])
  }

  return { v, ix }
}

function makeSurface(data: number[][]): { v: number[]; ix: number[] } {
  const rows = data.length
  if (rows < 2) return { v: [], ix: [] }
  const cols = data[0].length
  if (cols < 2) return { v: [], ix: [] }

  const v: number[] = []
  const ix: number[] = []

  // Generate vertices with normals
  for (let y = 0; y < rows; y++) {
    for (let x = 0; x < cols; x++) {
      const z = typeof data[y][x] === 'number' ? data[y][x] : 0
      // Compute normal via finite differences
      const zL = x > 0 ? (typeof data[y][x - 1] === 'number' ? data[y][x - 1] : z) : z
      const zR = x < cols - 1 ? (typeof data[y][x + 1] === 'number' ? data[y][x + 1] : z) : z
      const zD = y > 0 ? (typeof data[y - 1][x] === 'number' ? data[y - 1][x] : z) : z
      const zU = y < rows - 1 ? (typeof data[y + 1][x] === 'number' ? data[y + 1][x] : z) : z
      const dx = (zR - zL) / (x > 0 && x < cols - 1 ? 2 : 1)
      const dy = (zU - zD) / (y > 0 && y < rows - 1 ? 2 : 1)
      // Normal = cross product of tangent vectors
      // tangent_x = (1, 0, dx), tangent_y = (0, 1, dy)
      // normal = (-dx, -dy, 1) normalized
      const len = Math.sqrt(dx * dx + dy * dy + 1)
      const nx = -dx / len, ny = -dy / len, nz = 1 / len
      v.push(x, y, z, nx, ny, nz)
    }
  }

  // Generate triangles - two per cell
  for (let y = 0; y < rows - 1; y++) {
    for (let x = 0; x < cols - 1; x++) {
      const i00 = y * cols + x
      const i10 = y * cols + (x + 1)
      const i01 = (y + 1) * cols + x
      const i11 = (y + 1) * cols + (x + 1)
      ix.push(i00, i10, i11)
      ix.push(i00, i11, i01)
    }
  }

  return { v, ix }
}

/* ── Expression evaluator ─────────────────────────── */

function evalComparison(op: string, left: any, right: any): boolean {
  // Equality / inequality: compare like-typed values directly.
  if (op === '==' || op === '!=') {
    let eq: boolean
    if (typeof left === 'number' && typeof right === 'number') {
      eq = left === right
    } else if (typeof left === 'string' && typeof right === 'string') {
      eq = left === right
    } else if (typeof left === 'boolean' && typeof right === 'boolean') {
      eq = left === right
    } else if (Array.isArray(left) && Array.isArray(right)) {
      eq = JSON.stringify(left) === JSON.stringify(right)
    } else {
      // Mixed/other types: strict structural comparison.
      eq = left === right
    }
    return op === '==' ? eq : !eq
  }
  // Ordering operators: numeric comparison.
  const l = typeof left === 'number' ? left : 0
  const r = typeof right === 'number' ? right : 0
  switch (op) {
    case '<': return l < r
    case '>': return l > r
    case '<=': return l <= r
    case '>=': return l >= r
    default: return false
  }
}

/** Evaluate an expression node with given variables */
function evalExprNode(val: any, vars: Record<string, number>): any {
  if (!isExpr(val)) return val

  switch (val.op) {
    case 'var': {
      const v = val as ExprVar
      if (v.name in vars) return vars[v.name]
      if (v.name in MATH_CONSTANTS) return MATH_CONSTANTS[v.name]
      return v.name // return as string for color names etc.
    }
    case 'neg': {
      const v = evalExprNode((val as ExprUnary).operand, vars)
      return typeof v === 'number' ? -v : v
    }
    case 'not': {
      const v = evalExprNode((val as ExprUnary).operand, vars)
      return !evalCondition(v)
    }
    case 'ternary': {
      const t = val as ExprTernary
      const c = evalExprNode(t.cond, vars)
      return evalCondition(c) ? evalExprNode(t.ifTrue, vars) : evalExprNode(t.ifFalse, vars)
    }
    case 'call': {
      const c = val as ExprCall
      // Special handling for len() which works on arrays
      if (c.name === 'len') {
        const argVal = evalExprNode(c.args[0], vars)
        if (Array.isArray(argVal)) return argVal.length
        if (typeof argVal === 'string') return argVal.length
        return 0
      }
      // Special handling for concat() which merges arrays or strings
      if (c.name === 'concat') {
        const evaled = c.args.map(a => evalExprNode(a, vars))
        // If any argument is a string, concatenate as strings
        if (evaled.some(v => typeof v === 'string')) {
          return evaled.map(v => {
            if (typeof v === 'string') return v
            if (typeof v === 'number') return String(v)
            if (Array.isArray(v)) return '[' + v.join(',') + ']'
            return String(v ?? '')
          }).join('')
        }
        // Otherwise concat as arrays
        const result: any[] = []
        for (const ev of evaled) {
          if (Array.isArray(ev)) result.push(...ev)
          else result.push(ev)
        }
        return result
      }
      // str() - convert to string and concatenate
      if (c.name === 'str') {
        const parts: string[] = []
        for (const a of c.args) {
          const ev = evalExprNode(a, vars)
          if (typeof ev === 'number') parts.push(String(ev))
          else if (typeof ev === 'string') parts.push(ev)
          else if (typeof ev === 'boolean') parts.push(String(ev))
          else if (Array.isArray(ev)) parts.push('[' + ev.join(',') + ']')
          else parts.push(String(ev ?? 'undef'))
        }
        return parts.join('')
      }
      // chr() - character from ASCII code
      if (c.name === 'chr') {
        const code = evalExprNode(c.args[0], vars)
        if (typeof code === 'number') return String.fromCharCode(Math.round(code))
        return ''
      }
      // ord() - ASCII code from character
      if (c.name === 'ord') {
        const s = evalExprNode(c.args[0], vars)
        if (typeof s === 'string' && s.length > 0) return s.charCodeAt(0)
        return 0
      }
      // lookup() - linear interpolation table lookup
      if (c.name === 'lookup') {
        const key = evalExprNode(c.args[0], vars)
        const table = evalExprNode(c.args[1], vars)
        if (typeof key === 'number' && Array.isArray(table)) {
          // Sort table by keys
          const sorted = table
            .filter(r => Array.isArray(r) && r.length >= 2)
            .map(r => [typeof r[0] === 'number' ? r[0] : 0, typeof r[1] === 'number' ? r[1] : 0])
            .sort((a, b) => a[0] - b[0])
          if (sorted.length === 0) return 0
          if (key <= sorted[0][0]) return sorted[0][1]
          if (key >= sorted[sorted.length - 1][0]) return sorted[sorted.length - 1][1]
          for (let i = 0; i < sorted.length - 1; i++) {
            if (key >= sorted[i][0] && key <= sorted[i + 1][0]) {
              const t = (key - sorted[i][0]) / (sorted[i + 1][0] - sorted[i][0])
              return sorted[i][1] + t * (sorted[i + 1][1] - sorted[i][1])
            }
          }
          return sorted[sorted.length - 1][1]
        }
        return 0
      }
      // cross() - cross product of two 3D vectors
      if (c.name === 'cross') {
        const v1 = evalExprNode(c.args[0], vars)
        const v2 = evalExprNode(c.args[1], vars)
        if (Array.isArray(v1) && Array.isArray(v2) && v1.length >= 3 && v2.length >= 3) {
          const x1 = typeof v1[0] === 'number' ? v1[0] : 0
          const y1 = typeof v1[1] === 'number' ? v1[1] : 0
          const z1 = typeof v1[2] === 'number' ? v1[2] : 0
          const x2 = typeof v2[0] === 'number' ? v2[0] : 0
          const y2 = typeof v2[1] === 'number' ? v2[1] : 0
          const z2 = typeof v2[2] === 'number' ? v2[2] : 0
          return [y1*z2 - z1*y2, z1*x2 - x1*z2, x1*y2 - y1*x2]
        }
        return [0, 0, 0]
      }
      // norm() - vector magnitude
      if (c.name === 'norm') {
        const v = evalExprNode(c.args[0], vars)
        if (Array.isArray(v)) {
          let sum = 0
          for (const x of v) sum += (typeof x === 'number' ? x : 0) ** 2
          return Math.sqrt(sum)
        }
        if (typeof v === 'number') return Math.abs(v)
        return 0
      }
      // rands() - generate random numbers as array
      if (c.name === 'rands') {
        const minv = evalExprNode(c.args[0], vars)
        const maxv = evalExprNode(c.args[1], vars)
        const count = evalExprNode(c.args[2], vars)
        const seed = c.args.length > 3 ? evalExprNode(c.args[3], vars) : undefined
        if (typeof minv === 'number' && typeof maxv === 'number' && typeof count === 'number') {
          const result: number[] = []
          // Simple seeded PRNG (if seed provided)
          let rng: () => number
          if (typeof seed === 'number') {
            let s = seed
            rng = () => { s = (s * 1103515245 + 12345) & 0x7fffffff; return s / 0x7fffffff }
          } else {
            rng = Math.random
          }
          for (let i = 0; i < Math.min(count, 1000); i++) {
            result.push(minv + rng() * (maxv - minv))
          }
          return result
        }
        return []
      }
      const fn = MATH_FUNCS[c.name]
      if (fn) {
        const args = c.args.map(a => {
          const ev = evalExprNode(a, vars)
          return typeof ev === 'number' ? ev : 0
        })
        return fn(...args)
      }
      return 0
    }
    default: {
      // Binary operator
      const b = val as ExprBinary
      const left = evalExprNode(b.left, vars)
      const right = evalExprNode(b.right, vars)
      const op = b.op

      if (op === '&&') return evalCondition(left) && evalCondition(right)
      if (op === '||') return evalCondition(left) || evalCondition(right)

      if (op === '<' || op === '>' || op === '<=' || op === '>=' || op === '==' || op === '!=') {
        return evalComparison(op, left, right)
      }

      // String concatenation if either side is a string
      if (op === '+' && (typeof left === 'string' || typeof right === 'string')) {
        return String(left) + String(right)
      }

      const l = typeof left === 'number' ? left : 0
      const r = typeof right === 'number' ? right : 0

      switch (op) {
        case '+': return l + r
        case '-': return l - r
        case '*': return l * r
        case '/': return r !== 0 ? l / r : (l === 0 ? NaN : (l > 0 ? Infinity : -Infinity))
        case '%': return r !== 0 ? l % r : NaN
        default: return l
      }
    }
  }
}

/* ── Evaluator ────────────────────────────────────── */

const PALETTE: [number,number,number,number][] = [
  [0.26,0.52,0.96,1],[0.96,0.52,0.26,1],[0.26,0.86,0.56,1],
  [0.86,0.26,0.66,1],[0.96,0.86,0.26,1],[0.46,0.76,0.86,1],
  [0.76,0.56,0.96,1],[0.56,0.86,0.36,1],
]
let cIdx = 0
let _resolveFile: ((name: string) => string | null) | null = null
let _profiling = false
let _profileEntries: ProfileEntry[] = []
let _profileDepth = 0
let _source = ''
function nextC(): [number,number,number,number] { return PALETTE[(cIdx++) % PALETTE.length] }

function arg(a: Record<string,any>, name: string, pos: number, def: any): any {
  return a[name] ?? a[`_${pos}`] ?? def
}

/** Resolve a parsed arg value: evaluate expressions, substitute variables */
function resolveArg(val: any, vars: Record<string, number>): any {
  if (isExpr(val)) return evalExprNode(val, vars)
  // String literals should stay as strings and not be treated as variable references.
  // Variable references come as ExprVar nodes (handled by isExpr above), not as plain strings.
  if (typeof val === 'string') return val
  if (Array.isArray(val)) {
    const result: any[] = []
    for (const v of val) {
      const resolved = resolveArg(v, vars)
      if (resolved && typeof resolved === 'object' && resolved.__each) {
        const inner = resolved.value
        if (Array.isArray(inner)) {
          result.push(...inner)
        } else {
          result.push(inner)
        }
      } else {
        result.push(resolved)
      }
    }
    return result
  }
  if (val && typeof val === 'object' && val.__range) {
    return { __range: true, start: resolveArg(val.start, vars), step: resolveArg(val.step, vars), end: resolveArg(val.end, vars) }
  }
  if (val && typeof val === 'object' && val.__listComp) {
    const iterResolved = resolveArg(val.iterVal, vars)
    let iterValues: any[] = []
    if (iterResolved && typeof iterResolved === 'object' && iterResolved.__range) {
      iterValues = expandRange(iterResolved)
    } else if (Array.isArray(iterResolved)) {
      iterValues = iterResolved
    }
    const result: any[] = []
    for (const iv of iterValues) {
      const newVars = { ...vars, [val.varName]: typeof iv === 'number' ? iv : 0 }
      result.push(resolveArg(val.body, newVars))
    }
    return result
  }
  if (val && typeof val === 'object' && val.__each) {
    return { __each: true, value: resolveArg(val.value, vars) }
  }
  return val
}

/** Resolve all args in a Record through variable substitution */
function resolveArgs(a: Record<string, any>, vars: Record<string, number>): Record<string, any> {
  const out: Record<string, any> = {}
  for (const [k, v] of Object.entries(a)) out[k] = resolveArg(v, vars)
  return out
}

/** Expand a range object into an array of numbers */
function expandRange(r: any): number[] {
  if (!r || typeof r !== 'object' || !r.__range) return []
  const start = typeof r.start === 'number' ? r.start : 0
  const step = typeof r.step === 'number' ? r.step : 1
  const end = typeof r.end === 'number' ? r.end : 0
  // Guard against invalid step (zero / non-finite) which would never terminate
  if (step === 0 || !Number.isFinite(step) || !Number.isFinite(start) || !Number.isFinite(end)) {
    throw new Error('Invalid range step')
  }
  // Reject degenerate ranges that would allocate huge arrays BEFORE iterating
  if (Math.abs((end - start) / step) > 10000) {
    throw new Error('Range too large (>10000 elements)')
  }
  const result: number[] = []
  if (step > 0) {
    for (let i = start; i <= end + 1e-9; i += step) { result.push(i); if (result.length > 10000) break }
  } else {
    for (let i = start; i >= end - 1e-9; i += step) { result.push(i); if (result.length > 10000) break }
  }
  return result
}

/** Evaluate a simple numeric condition for if() */
function evalCondition(val: any): boolean {
  if (typeof val === 'boolean') return val
  if (typeof val === 'number') return val !== 0
  if (val === undefined || val === null) return false
  if (val === 'true') return true
  if (val === 'false') return false
  if (val === 'undef') return false
  return !!val
}

/* ── Module storage ───────────────────────────────── */

interface ModuleDef {
  params: { name: string; defaultVal: any }[]
  children: ASTNode[]
}

/* ── Extrusion helpers ────────────────────────────── */

/** Extract XY positions from a flat mesh (z near 0) */
function extractFlatProfile(mesh: MeshData): { pts: [number, number][]; flat: boolean } {
  const verts = mesh.vertices
  const nv = verts.length / 6
  let minZ = Infinity, maxZ = -Infinity
  const pts: [number, number][] = []

  for (let i = 0; i < nv; i++) {
    const z = verts[i * 6 + 2]
    if (z < minZ) minZ = z
    if (z > maxZ) maxZ = z
  }

  const flat = (maxZ - minZ) < 0.5

  if (!flat) return { pts: [], flat: false }

  // Collect unique XY points from the mesh edges
  const idxs = mesh.indices
  const edgeSet = new Set<string>()
  const edgeVerts: [number, number][] = []

  for (let i = 0; i < nv; i++) {
    const x = verts[i * 6]
    const y = verts[i * 6 + 1]
    const key = `${x.toFixed(4)},${y.toFixed(4)}`
    if (!edgeSet.has(key)) {
      edgeSet.add(key)
      edgeVerts.push([x, y])
    }
  }

  // Build boundary edges: edges that appear in only one triangle
  const edgeCount = new Map<string, number>()
  const triCount = idxs.length / 3
  for (let t = 0; t < triCount; t++) {
    const i0 = idxs[t * 3], i1 = idxs[t * 3 + 1], i2 = idxs[t * 3 + 2]
    const edges = [[i0, i1], [i1, i2], [i2, i0]]
    for (const [a, b] of edges) {
      const key = a < b ? `${a},${b}` : `${b},${a}`
      edgeCount.set(key, (edgeCount.get(key) || 0) + 1)
    }
  }

  // Collect boundary vertex indices
  const boundaryVertSet = new Set<number>()
  for (const [key, count] of edgeCount) {
    if (count === 1) {
      const [a, b] = key.split(',').map(Number)
      boundaryVertSet.add(a)
      boundaryVertSet.add(b)
    }
  }

  // If we have boundary vertices, use them; otherwise use all unique verts
  const boundaryPts: [number, number][] = []
  if (boundaryVertSet.size >= 3) {
    // Build adjacency for boundary edges to order them
    const adj = new Map<number, number[]>()
    for (const [key, count] of edgeCount) {
      if (count === 1) {
        const [a, b] = key.split(',').map(Number)
        if (!adj.has(a)) adj.set(a, [])
        if (!adj.has(b)) adj.set(b, [])
        adj.get(a)!.push(b)
        adj.get(b)!.push(a)
      }
    }

    // Walk the boundary
    const visited = new Set<number>()
    let current = boundaryVertSet.values().next().value
    if (current !== undefined) {
      while (current !== undefined && !visited.has(current)) {
        visited.add(current)
        boundaryPts.push([verts[current * 6], verts[current * 6 + 1]])
        const nbrs: number[] = adj.get(current) || []
        let next: number | undefined = undefined
        for (const nb of nbrs) {
          if (!visited.has(nb)) { next = nb; break }
        }
        current = next
      }
    }
  }

  if (boundaryPts.length >= 3) {
    return { pts: boundaryPts, flat: true }
  }

  // Fallback: sort by angle around centroid
  if (edgeVerts.length < 3) return { pts: edgeVerts, flat: true }
  let cx = 0, cy = 0
  for (const [x, y] of edgeVerts) { cx += x; cy += y }
  cx /= edgeVerts.length; cy /= edgeVerts.length
  edgeVerts.sort((a, b) => Math.atan2(a[1] - cy, a[0] - cx) - Math.atan2(b[1] - cy, b[0] - cx))

  return { pts: edgeVerts, flat: true }
}

/** Generate extruded mesh from a 2D profile */
function extrudeMesh(profile: [number, number][], height: number, twist: number, slices: number): { v: number[]; ix: number[] } {
  const v: number[] = []
  const ix: number[] = []
  const n = profile.length
  if (n < 3) return { v, ix }

  const actualSlices = Math.max(1, twist !== 0 ? Math.max(slices, Math.ceil(Math.abs(twist) / 10)) : slices)

  // Generate vertices for each slice
  for (let s = 0; s <= actualSlices; s++) {
    const t = s / actualSlices
    const z = height * t
    const angle = (twist * t) * Math.PI / 180

    const ca = Math.cos(angle), sa = Math.sin(angle)

    for (let i = 0; i < n; i++) {
      const x = profile[i][0], y = profile[i][1]
      const rx = x * ca - y * sa
      const ry = x * sa + y * ca
      // Normal: pointing outward (approximate)
      const next = profile[(i + 1) % n]
      const dx = next[0] - profile[i][0]
      const dy = next[1] - profile[i][1]
      const nl = Math.sqrt(dx * dx + dy * dy) || 1
      let nx = dy / nl, ny = -dx / nl
      // Rotate normal too
      const rnx = nx * ca - ny * sa
      const rny = nx * sa + ny * ca
      v.push(rx, ry, z, rnx, rny, 0)
    }
  }

  // Side faces
  for (let s = 0; s < actualSlices; s++) {
    for (let i = 0; i < n; i++) {
      const ni = (i + 1) % n
      const a = s * n + i
      const b = s * n + ni
      const c = (s + 1) * n + i
      const d = (s + 1) * n + ni
      ix.push(a, b, d, a, d, c)
    }
  }

  // Bottom cap (z = 0)
  const bottomOff = v.length / 6
  for (let i = 0; i < n; i++) {
    v.push(profile[i][0], profile[i][1], 0, 0, 0, -1)
  }
  const bottomTris = earClip(profile.map(p => [p[0], p[1]]))
  for (let i = 0; i < bottomTris.length; i += 3) {
    ix.push(bottomOff + bottomTris[i], bottomOff + bottomTris[i + 2], bottomOff + bottomTris[i + 1])
  }

  // Top cap (z = height)
  const topOff = v.length / 6
  const topAngle = twist * Math.PI / 180
  const tca = Math.cos(topAngle), tsa = Math.sin(topAngle)
  const topProfile: [number, number][] = []
  for (let i = 0; i < n; i++) {
    const x = profile[i][0], y = profile[i][1]
    const rx = x * tca - y * tsa
    const ry = x * tsa + y * tca
    topProfile.push([rx, ry])
    v.push(rx, ry, height, 0, 0, 1)
  }
  const topTris = earClip(topProfile.map(p => [p[0], p[1]]))
  for (let i = 0; i < topTris.length; i += 3) {
    ix.push(topOff + topTris[i], topOff + topTris[i + 1], topOff + topTris[i + 2])
  }

  return { v, ix }
}

/** Generate a rotation-extruded mesh from a 2D XZ profile */
function rotateExtrudeMesh(profile: [number, number][], fn: number, angle: number): { v: number[]; ix: number[] } {
  const v: number[] = []
  const ix: number[] = []
  const n = profile.length
  if (n < 2) return { v, ix }

  const steps = Math.max(3, fn)
  const fullAngle = angle * Math.PI / 180

  // Generate vertices by rotating the profile around Z axis
  for (let s = 0; s <= steps; s++) {
    const t = s / steps
    const a = fullAngle * t
    const ca = Math.cos(a), sa = Math.sin(a)

    for (let i = 0; i < n; i++) {
      const r = profile[i][0]  // X coordinate = radius
      const z = profile[i][1]  // Y coordinate = Z height

      const x = r * ca
      const y = r * sa

      // Normal: approximate
      const prev = i > 0 ? i - 1 : i
      const next = i < n - 1 ? i + 1 : i
      const dr = profile[next][0] - profile[prev][0]
      const dz = profile[next][1] - profile[prev][1]
      const nl = Math.sqrt(dr * dr + dz * dz) || 1
      const nr = dz / nl   // outward in R direction
      const nz = -dr / nl  // Z component

      v.push(x, y, z, nr * ca, nr * sa, nz)
    }
  }

  // Generate faces
  for (let s = 0; s < steps; s++) {
    for (let i = 0; i < n - 1; i++) {
      const a0 = s * n + i
      const b0 = s * n + i + 1
      const a1 = (s + 1) * n + i
      const b1 = (s + 1) * n + i + 1
      ix.push(a0, b0, b1, a0, b1, a1)
    }
  }

  // If full 360, don't cap; if partial angle, add caps
  if (Math.abs(angle - 360) > 0.01) {
    // Start cap (angle = 0)
    const startOff = v.length / 6
    for (let i = 0; i < n; i++) {
      v.push(profile[i][0], 0, profile[i][1], 0, -1, 0)
    }
    const capTris = earClip(profile.map(p => [p[0], p[1]]))
    for (let i = 0; i < capTris.length; i += 3) {
      ix.push(startOff + capTris[i], startOff + capTris[i + 2], startOff + capTris[i + 1])
    }

    // End cap
    const endAngle = fullAngle
    const eca = Math.cos(endAngle), esa = Math.sin(endAngle)
    const endOff = v.length / 6
    for (let i = 0; i < n; i++) {
      const r = profile[i][0]
      const x = r * eca, y = r * esa
      // Normal perpendicular to the end face
      const nx = -esa, ny = eca
      v.push(x, y, profile[i][1], nx, ny, 0)
    }
    for (let i = 0; i < capTris.length; i += 3) {
      ix.push(endOff + capTris[i], endOff + capTris[i + 1], endOff + capTris[i + 2])
    }
  }

  return { v, ix }
}

/* ── 3D Convex Hull ───────────────────────────────── */

interface HullFace {
  a: number
  b: number
  c: number
  nx: number
  ny: number
  nz: number
  d: number // plane distance
}

function computeHullFaceNormal(pts: Vec3[], a: number, b: number, c: number): { nx: number; ny: number; nz: number; d: number } {
  const ax = pts[b][0] - pts[a][0], ay = pts[b][1] - pts[a][1], az = pts[b][2] - pts[a][2]
  const bx = pts[c][0] - pts[a][0], by = pts[c][1] - pts[a][1], bz = pts[c][2] - pts[a][2]
  let nx = ay * bz - az * by
  let ny = az * bx - ax * bz
  let nz = ax * by - ay * bx
  const len = Math.sqrt(nx * nx + ny * ny + nz * nz)
  if (len > 1e-10) { nx /= len; ny /= len; nz /= len }
  const d = nx * pts[a][0] + ny * pts[a][1] + nz * pts[a][2]
  return { nx, ny, nz, d }
}

function convexHull3D(points: Vec3[]): { v: number[]; ix: number[] } {
  const v: number[] = []
  const ix: number[] = []

  if (points.length < 4) {
    // Not enough for a 3D hull; make a degenerate shape
    if (points.length === 0) return { v, ix }
    // Build a tiny cube around center of given points
    let cx = 0, cy = 0, cz = 0
    for (const p of points) { cx += p[0]; cy += p[1]; cz += p[2] }
    cx /= points.length; cy /= points.length; cz /= points.length
    return makeCube(0.1, 0.1, 0.1, true)
  }

  // Remove duplicate points
  const uniqueMap = new Map<string, number>()
  const pts: Vec3[] = []
  for (const p of points) {
    const key = `${p[0].toFixed(6)},${p[1].toFixed(6)},${p[2].toFixed(6)}`
    if (!uniqueMap.has(key)) {
      uniqueMap.set(key, pts.length)
      pts.push(p)
    }
  }

  if (pts.length < 4) {
    // Degenerate: make bounding box
    let minX = Infinity, minY = Infinity, minZ = Infinity
    let maxX = -Infinity, maxY = -Infinity, maxZ = -Infinity
    for (const p of pts) {
      if (p[0] < minX) minX = p[0]; if (p[0] > maxX) maxX = p[0]
      if (p[1] < minY) minY = p[1]; if (p[1] > maxY) maxY = p[1]
      if (p[2] < minZ) minZ = p[2]; if (p[2] > maxZ) maxZ = p[2]
    }
    const sx = Math.max(maxX - minX, 0.1)
    const sy = Math.max(maxY - minY, 0.1)
    const sz = Math.max(maxZ - minZ, 0.1)
    const cube = makeCube(sx, sy, sz, false)
    // Offset vertices
    for (let i = 0; i < cube.v.length; i += 6) {
      cube.v[i] += minX; cube.v[i+1] += minY; cube.v[i+2] += minZ
    }
    return cube
  }

  // Find 4 non-coplanar points for initial tetrahedron
  let i0 = 0, i1 = -1, i2 = -1, i3 = -1

  // Find most distant point from i0
  let maxDist = 0
  for (let i = 1; i < pts.length; i++) {
    const dx = pts[i][0] - pts[0][0], dy = pts[i][1] - pts[0][1], dz = pts[i][2] - pts[0][2]
    const dist = dx*dx + dy*dy + dz*dz
    if (dist > maxDist) { maxDist = dist; i1 = i }
  }
  if (i1 === -1) i1 = 1

  // Find point most distant from line i0-i1
  maxDist = 0
  const lx = pts[i1][0] - pts[i0][0], ly = pts[i1][1] - pts[i0][1], lz = pts[i1][2] - pts[i0][2]
  for (let i = 0; i < pts.length; i++) {
    if (i === i0 || i === i1) continue
    const dx = pts[i][0] - pts[i0][0], dy = pts[i][1] - pts[i0][1], dz = pts[i][2] - pts[i0][2]
    // Cross product magnitude
    const cx2 = dy*lz - dz*ly, cy2 = dz*lx - dx*lz, cz2 = dx*ly - dy*lx
    const dist = cx2*cx2 + cy2*cy2 + cz2*cz2
    if (dist > maxDist) { maxDist = dist; i2 = i }
  }
  if (i2 === -1) i2 = 2 < pts.length ? 2 : i1

  // Find point most distant from plane i0-i1-i2
  const { nx: pnx, ny: pny, nz: pnz, d: pd } = computeHullFaceNormal(pts, i0, i1, i2)
  maxDist = 0
  for (let i = 0; i < pts.length; i++) {
    if (i === i0 || i === i1 || i === i2) continue
    const dist = Math.abs(pnx * pts[i][0] + pny * pts[i][1] + pnz * pts[i][2] - pd)
    if (dist > maxDist) { maxDist = dist; i3 = i }
  }
  if (i3 === -1) {
    // All coplanar - build a flat prism as approximation
    let minXf = Infinity, minYf = Infinity, minZf = Infinity
    let maxXf = -Infinity, maxYf = -Infinity, maxZf = -Infinity
    for (const p of pts) {
      if (p[0] < minXf) minXf = p[0]; if (p[0] > maxXf) maxXf = p[0]
      if (p[1] < minYf) minYf = p[1]; if (p[1] > maxYf) maxYf = p[1]
      if (p[2] < minZf) minZf = p[2]; if (p[2] > maxZf) maxZf = p[2]
    }
    const cube = makeCube(Math.max(maxXf - minXf, 0.1), Math.max(maxYf - minYf, 0.1), Math.max(maxZf - minZf, 0.1), false)
    for (let i = 0; i < cube.v.length; i += 6) {
      cube.v[i] += minXf; cube.v[i+1] += minYf; cube.v[i+2] += minZf
    }
    return cube
  }

  // Build initial tetrahedron with correct winding
  // Ensure i3 is on the positive side of face (i0,i1,i2)
  const side = pnx * pts[i3][0] + pny * pts[i3][1] + pnz * pts[i3][2] - pd
  let faces: HullFace[]
  if (side > 0) {
    // i3 above plane -> flip face orientation
    faces = [
      { ...computeHullFaceNormal(pts, i0, i2, i1), a: i0, b: i2, c: i1 },
      { ...computeHullFaceNormal(pts, i0, i1, i3), a: i0, b: i1, c: i3 },
      { ...computeHullFaceNormal(pts, i1, i2, i3), a: i1, b: i2, c: i3 },
      { ...computeHullFaceNormal(pts, i2, i0, i3), a: i2, b: i0, c: i3 },
    ]
  } else {
    faces = [
      { ...computeHullFaceNormal(pts, i0, i1, i2), a: i0, b: i1, c: i2 },
      { ...computeHullFaceNormal(pts, i0, i3, i1), a: i0, b: i3, c: i1 },
      { ...computeHullFaceNormal(pts, i1, i3, i2), a: i1, b: i3, c: i2 },
      { ...computeHullFaceNormal(pts, i2, i3, i0), a: i2, b: i3, c: i0 },
    ]
  }

  // Verify normals point outward: for each face, center of all other initial points should be inside
  const tetCenter: Vec3 = [
    (pts[i0][0]+pts[i1][0]+pts[i2][0]+pts[i3][0])/4,
    (pts[i0][1]+pts[i1][1]+pts[i2][1]+pts[i3][1])/4,
    (pts[i0][2]+pts[i1][2]+pts[i2][2]+pts[i3][2])/4,
  ]
  for (let fi = 0; fi < faces.length; fi++) {
    const f = faces[fi]
    const dot = f.nx * tetCenter[0] + f.ny * tetCenter[1] + f.nz * tetCenter[2] - f.d
    if (dot > 1e-10) {
      // Normal points inward, flip
      const tmp = f.b; f.b = f.c; f.c = tmp
      const nn = computeHullFaceNormal(pts, f.a, f.b, f.c)
      f.nx = nn.nx; f.ny = nn.ny; f.nz = nn.nz; f.d = nn.d
    }
  }

  const usedPts = new Set([i0, i1, i2, i3])

  // Incremental convex hull
  for (let pi = 0; pi < pts.length; pi++) {
    if (usedPts.has(pi)) continue
    const pt = pts[pi]

    // Find visible faces
    const visible: number[] = []
    for (let fi = 0; fi < faces.length; fi++) {
      const f = faces[fi]
      const dist = f.nx * pt[0] + f.ny * pt[1] + f.nz * pt[2] - f.d
      if (dist > 1e-10) visible.push(fi)
    }

    if (visible.length === 0) continue // point is inside hull

    usedPts.add(pi)

    // Find horizon edges (boundary of visible faces)
    const horizonEdges: [number, number][] = []
    const visibleSet = new Set(visible)

    for (const fi of visible) {
      const f = faces[fi]
      const edges: [number, number][] = [[f.a, f.b], [f.b, f.c], [f.c, f.a]]
      for (const [ea, eb] of edges) {
        // Check if the adjacent face sharing this edge is NOT visible
        let isHorizon = true
        for (let fj = 0; fj < faces.length; fj++) {
          if (fj === fi || visibleSet.has(fj)) continue
          const of2 = faces[fj]
          const hasEdge = (of2.a === eb && of2.b === ea) || (of2.b === eb && of2.c === ea) || (of2.c === eb && of2.a === ea) ||
                         (of2.a === ea && of2.b === eb) || (of2.b === ea && of2.c === eb) || (of2.c === ea && of2.a === eb)
          if (hasEdge) { isHorizon = true; break }
        }
        if (isHorizon) {
          // Check this edge isn't shared with another visible face
          let sharedWithVisible = false
          for (const fj of visible) {
            if (fj === fi) continue
            const of2 = faces[fj]
            const hasEdge = (of2.a === eb && of2.b === ea) || (of2.b === eb && of2.c === ea) || (of2.c === eb && of2.a === ea) ||
                           (of2.a === ea && of2.b === eb) || (of2.b === ea && of2.c === eb) || (of2.c === ea && of2.a === eb)
            if (hasEdge) { sharedWithVisible = true; break }
          }
          if (!sharedWithVisible) {
            horizonEdges.push([ea, eb])
          }
        }
      }
    }

    // Remove visible faces (in reverse order to preserve indices)
    const sortedVisible = [...visible].sort((a, b) => b - a)
    for (const fi of sortedVisible) {
      faces.splice(fi, 1)
    }

    // Create new faces from horizon edges to the point
    for (const [ea, eb] of horizonEdges) {
      const nn = computeHullFaceNormal(pts, ea, eb, pi)
      const newFace: HullFace = { a: ea, b: eb, c: pi, ...nn }

      // Check normal points outward (away from hull center)
      // Compute approximate center of remaining hull
      let hcx = 0, hcy = 0, hcz = 0, hcount = 0
      for (const uid of usedPts) {
        hcx += pts[uid][0]; hcy += pts[uid][1]; hcz += pts[uid][2]; hcount++
      }
      if (hcount > 0) { hcx /= hcount; hcy /= hcount; hcz /= hcount }

      const dot = newFace.nx * hcx + newFace.ny * hcy + newFace.nz * hcz - newFace.d
      if (dot > 1e-10) {
        // Flip
        newFace.b = eb; newFace.a = ea;
        // Actually swap a and b
        const tmpA = newFace.a
        newFace.a = newFace.b
        newFace.b = tmpA
        const nn2 = computeHullFaceNormal(pts, newFace.a, newFace.b, newFace.c)
        newFace.nx = nn2.nx; newFace.ny = nn2.ny; newFace.nz = nn2.nz; newFace.d = nn2.d
      }

      faces.push(newFace)
    }
  }

  // Convert faces to mesh
  // Each face gets its own vertices (flat shading)
  let vi = 0
  for (const f of faces) {
    const pa = pts[f.a], pb = pts[f.b], pc = pts[f.c]
    v.push(pa[0], pa[1], pa[2], f.nx, f.ny, f.nz)
    v.push(pb[0], pb[1], pb[2], f.nx, f.ny, f.nz)
    v.push(pc[0], pc[1], pc[2], f.nx, f.ny, f.nz)
    ix.push(vi, vi + 1, vi + 2)
    vi += 3
  }

  return { v, ix }
}

function makePolyhedron(points: number[][], faces: number[][]): { v: number[]; ix: number[] } {
  const v: number[] = []
  const ix: number[] = []
  if (points.length < 3 || faces.length < 1) return { v, ix }

  // Each face gets its own vertices (flat shading)
  let vi = 0
  for (const face of faces) {
    if (face.length < 3) continue
    // Compute face normal from first three vertices
    const p0 = points[face[0]] || [0,0,0]
    const p1 = points[face[1]] || [0,0,0]
    const p2 = points[face[2]] || [0,0,0]
    const e1x = p1[0] - p0[0], e1y = p1[1] - p0[1], e1z = p1[2] - p0[2]
    const e2x = p2[0] - p0[0], e2y = p2[1] - p0[1], e2z = p2[2] - p0[2]
    let nx = e1y * e2z - e1z * e2y
    let ny = e1z * e2x - e1x * e2z
    let nz = e1x * e2y - e1y * e2x
    const len = Math.sqrt(nx * nx + ny * ny + nz * nz)
    if (len > 1e-10) { nx /= len; ny /= len; nz /= len }

    // Emit vertices for each point in the face
    const baseVi = vi
    for (const idx of face) {
      const p = points[idx] || [0,0,0]
      v.push(p[0], p[1], p[2], nx, ny, nz)
      vi++
    }

    // Fan triangulation: anchor on the first vertex of the face
    for (let j = 1; j < face.length - 1; j++) {
      ix.push(baseVi, baseVi + j, baseVi + j + 1)
    }
  }
  return { v, ix }
}

/* ── Trapezoid / Pyramid / Donut generators ───────── */

function makeTrapezoid(top: number, bottom: number, h: number, depth: number) {
  const v: number[] = [], ix: number[] = []
  const bt = bottom / 2, tt = top / 2, d = depth / 2

  // 8 vertices of the trapezoid prism
  // Bottom face (z=0): bl_fl, bl_fr, bl_br, bl_bl
  // Top face (z=h):    tl_fl, tl_fr, tl_br, tl_bl
  const verts = [
    [-bt, -d, 0], [ bt, -d, 0], [ bt,  d, 0], [-bt,  d, 0],  // bottom: 0,1,2,3
    [-tt, -d, h], [ tt, -d, h], [ tt,  d, h], [-tt,  d, h],   // top: 4,5,6,7
  ]

  // 6 faces: [indices, normal]
  // Bottom face (z=0) - normal (0,0,-1)
  const faces: { verts: number[][]; normal: [number, number, number] }[] = [
    { verts: [verts[0], verts[3], verts[2], verts[1]], normal: [0, 0, -1] },  // bottom
    { verts: [verts[4], verts[5], verts[6], verts[7]], normal: [0, 0, 1] },   // top
    { verts: [verts[0], verts[1], verts[5], verts[4]], normal: [0, -1, 0] },  // front (y=-d)
    { verts: [verts[2], verts[3], verts[7], verts[6]], normal: [0, 1, 0] },   // back (y=+d)
  ]

  // Left slope: from (-bt,_,0) to (-tt,_,h)
  // Normal for left slope: perpendicular to the slope surface
  const ldx = -tt - (-bt)  // = bt - tt (change in x going up)
  const ldz = h             // change in z going up
  // Normal to left slope points leftward: (-ldz, 0, ldx) normalized... actually:
  // The left face goes from bottom-left to top-left. The slope direction is (ldx, 0, ldz).
  // Face normal perpendicular to slope, pointing left: (-h, 0, bt - tt) -> but we need outward
  // Outward normal for left face: (-h, 0, -(bt - tt)) ... let's compute properly
  // Edge along slope: (-tt - (-bt), 0, h) = (bt-tt, 0, h)
  // Edge along depth: (0, depth, 0)
  // Normal = slope x depth = (0*h - depth*0, depth*(bt-tt) - 0*0, 0*0 - 0*(bt-tt))... no
  // slope = (bt-tt, 0, h), depthEdge = (0, 1, 0)
  // cross = (0*0 - h*1, h*0 - (bt-tt)*0, (bt-tt)*1 - 0*0) = (-h, 0, bt-tt)
  const lnx = -h, lny = 0, lnz = bt - tt
  const lnLen = Math.sqrt(lnx * lnx + lnz * lnz) || 1
  faces.push({
    verts: [verts[0], verts[4], verts[7], verts[3]],
    normal: [lnx / lnLen, lny / lnLen, lnz / lnLen]
  })

  // Right slope: from (bt,_,0) to (tt,_,h)
  // By symmetry, normal is (h, 0, bt-tt) normalized
  const rnx = h, rny = 0, rnz = bt - tt
  const rnLen = Math.sqrt(rnx * rnx + rnz * rnz) || 1
  faces.push({
    verts: [verts[1], verts[2], verts[6], verts[5]],
    normal: [rnx / rnLen, rny / rnLen, rnz / rnLen]
  })

  for (const face of faces) {
    const base = v.length / 6
    for (const vert of face.verts) {
      v.push(vert[0], vert[1], vert[2], face.normal[0], face.normal[1], face.normal[2])
    }
    ix.push(base, base + 1, base + 2, base, base + 2, base + 3)
  }

  return { v, ix }
}

function makePyramid(base: number, h: number, sides: number, center: boolean) {
  const v: number[] = [], ix: number[] = []
  const r = base / 2
  const zOff = center ? -h / 2 : 0

  // Generate base polygon vertices
  const baseVerts: [number, number][] = []
  for (let i = 0; i < sides; i++) {
    const angle = (2 * Math.PI * i) / sides
    baseVerts.push([r * Math.cos(angle), r * Math.sin(angle)])
  }

  const apex: [number, number, number] = [0, 0, h + zOff]

  // Bottom cap (normal pointing down, z = zOff)
  const bottomBase = v.length / 6
  for (let i = 0; i < sides; i++) {
    v.push(baseVerts[i][0], baseVerts[i][1], zOff, 0, 0, -1)
  }
  // Fan triangulation for bottom (winding order for downward normal)
  for (let i = 1; i < sides - 1; i++) {
    ix.push(bottomBase, bottomBase + i + 1, bottomBase + i)
  }

  // Side faces: N triangles
  for (let i = 0; i < sides; i++) {
    const i2 = (i + 1) % sides
    const p0: [number, number, number] = [baseVerts[i][0], baseVerts[i][1], zOff]
    const p1: [number, number, number] = [baseVerts[i2][0], baseVerts[i2][1], zOff]

    // Compute face normal
    const e1x = p1[0] - p0[0], e1y = p1[1] - p0[1], e1z = p1[2] - p0[2]
    const e2x = apex[0] - p0[0], e2y = apex[1] - p0[1], e2z = apex[2] - p0[2]
    let fnx = e1y * e2z - e1z * e2y
    let fny = e1z * e2x - e1x * e2z
    let fnz = e1x * e2y - e1y * e2x
    const fnLen = Math.sqrt(fnx * fnx + fny * fny + fnz * fnz) || 1
    fnx /= fnLen; fny /= fnLen; fnz /= fnLen

    const sBase = v.length / 6
    v.push(p0[0], p0[1], p0[2], fnx, fny, fnz)
    v.push(p1[0], p1[1], p1[2], fnx, fny, fnz)
    v.push(apex[0], apex[1], apex[2], fnx, fny, fnz)
    ix.push(sBase, sBase + 1, sBase + 2)
  }

  return { v, ix }
}

function makeDonut(r1: number, r2: number, angle: number, fn: number) {
  if (angle >= 360) return makeTorus(r1, r2, fn)

  const v: number[] = [], ix: number[] = []
  const ringSegs = Math.max(4, Math.floor(fn * angle / 360))
  const tubeSegs = Math.max(8, Math.floor(fn * r2 / r1))
  const angleRad = (angle * Math.PI) / 180

  // Generate torus surface vertices for partial sweep
  for (let i = 0; i <= ringSegs; i++) {
    const u = (angleRad * i) / ringSegs
    const cu = Math.cos(u), su = Math.sin(u)
    for (let j = 0; j <= tubeSegs; j++) {
      const vv = (2 * Math.PI * j) / tubeSegs
      const cv = Math.cos(vv), sv = Math.sin(vv)
      const x = (r1 + r2 * cv) * cu
      const y = r2 * sv
      const z = (r1 + r2 * cv) * su
      const nx = cv * cu
      const ny = sv
      const nz = cv * su
      v.push(x, y, z, nx, ny, nz)
    }
  }
  // Index the torus surface
  for (let i = 0; i < ringSegs; i++) {
    for (let j = 0; j < tubeSegs; j++) {
      const a = i * (tubeSegs + 1) + j
      const b = a + tubeSegs + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }

  // End cap at angle=0 (ring index 0)
  // The cap center is at (r1, 0, 0) and the disc lies in the YZ plane locally
  // Normal for this cap points in the -Z direction of the sweep (tangent direction at start)
  // At u=0, tangent direction is (0, 0, 1) so cap normal is (0, 0, -1)
  const cap0Center = v.length / 6
  v.push(r1, 0, 0, 0, 0, -1)  // center of cap 0
  for (let j = 0; j <= tubeSegs; j++) {
    const vv = (2 * Math.PI * j) / tubeSegs
    const cv = Math.cos(vv), sv = Math.sin(vv)
    const x = r1 + r2 * cv
    const y = r2 * sv
    v.push(x, y, 0, 0, 0, -1)
  }
  for (let j = 0; j < tubeSegs; j++) {
    ix.push(cap0Center, cap0Center + 1 + j + 1, cap0Center + 1 + j)
  }

  // End cap at angle=angle
  const cu2 = Math.cos(angleRad), su2 = Math.sin(angleRad)
  // Cap center at (r1*cos(angle), 0, r1*sin(angle))
  // Tangent direction at end: (-sin(angle), 0, cos(angle)), so cap outward normal is same direction
  const capNx = -su2, capNy = 0, capNz = cu2  // outward-pointing tangent
  const cap1Center = v.length / 6
  v.push(r1 * cu2, 0, r1 * su2, capNx, capNy, capNz)
  for (let j = 0; j <= tubeSegs; j++) {
    const vv = (2 * Math.PI * j) / tubeSegs
    const cv = Math.cos(vv), sv = Math.sin(vv)
    const x = (r1 + r2 * cv) * cu2
    const y = r2 * sv
    const z = (r1 + r2 * cv) * su2
    v.push(x, y, z, capNx, capNy, capNz)
  }
  for (let j = 0; j < tubeSegs; j++) {
    ix.push(cap1Center, cap1Center + 1 + j, cap1Center + 1 + j + 1)
  }

  return { v, ix }
}

function makeLattice(type: string, cell: number, r: number, size: [number, number, number], fn: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const [sx, sy, sz] = size
  const nx = Math.max(1, Math.floor(sx / cell))
  const ny = Math.max(1, Math.floor(sy / cell))
  const nz = Math.max(1, Math.floor(sz / cell))

  // Helper to merge a cylinder mesh with offset and optional rotation
  const addCyl = (cyl: { v: number[]; ix: number[] }, offsetX: number, offsetY: number, offsetZ: number, axis: 'x' | 'y' | 'z') => {
    const base = v.length / 6
    const cv = cyl.v
    const nv = cv.length / 6
    for (let i = 0; i < nv; i++) {
      let px = cv[i * 6], py = cv[i * 6 + 1], pz = cv[i * 6 + 2]
      let nnx = cv[i * 6 + 3], nny = cv[i * 6 + 4], nnz = cv[i * 6 + 5]
      // Rotate from Z-axis to target axis
      if (axis === 'x') {
        // Rotate 90 deg around Y: (x,y,z) -> (z,y,-x)
        const tmp = px; px = pz; pz = -tmp
        const tmpn = nnx; nnx = nnz; nnz = -tmpn
      } else if (axis === 'y') {
        // Rotate -90 deg around X: (x,y,z) -> (x,-z,y)
        const tmp = py; py = -pz; pz = tmp
        const tmpn = nny; nny = -nnz; nnz = tmpn
      }
      v.push(px + offsetX, py + offsetY, pz + offsetZ, nnx, nny, nnz)
    }
    for (const idx of cyl.ix) {
      ix.push(base + idx)
    }
  }

  void type // currently only cubic lattice

  // X-axis struts
  for (let iy = 0; iy <= ny; iy++) {
    for (let iz = 0; iz <= nz; iz++) {
      const cyl = makeCylinder(sx, r, r, false, fn)
      addCyl(cyl, 0, iy * cell, iz * cell, 'x')
    }
  }
  // Y-axis struts
  for (let ix2 = 0; ix2 <= nx; ix2++) {
    for (let iz = 0; iz <= nz; iz++) {
      const cyl = makeCylinder(sy, r, r, false, fn)
      addCyl(cyl, ix2 * cell, 0, iz * cell, 'y')
    }
  }
  // Z-axis struts
  for (let ix2 = 0; ix2 <= nx; ix2++) {
    for (let iy = 0; iy <= ny; iy++) {
      const cyl = makeCylinder(sz, r, r, false, fn)
      addCyl(cyl, ix2 * cell, iy * cell, 0, 'z')
    }
  }

  return { v, ix }
}

function makeSlot(length: number, width: number, h: number, center: boolean): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const r = width / 2
  const halfLen = (length - width) / 2  // half of the straight section
  const fn = 16

  // Build 2D stadium profile
  const profile: number[][] = []
  // Right semicircle (center at x = halfLen)
  for (let i = 0; i <= fn / 2; i++) {
    const angle = -Math.PI / 2 + (Math.PI * i) / (fn / 2)
    profile.push([halfLen + r * Math.cos(angle), r * Math.sin(angle)])
  }
  // Left semicircle (center at x = -halfLen)
  for (let i = 0; i <= fn / 2; i++) {
    const angle = Math.PI / 2 + (Math.PI * i) / (fn / 2)
    profile.push([-halfLen + r * Math.cos(angle), r * Math.sin(angle)])
  }

  const np = profile.length
  const ox = center ? 0 : length / 2
  const oy = center ? 0 : width / 2
  const z0 = center ? -h / 2 : 0
  const z1 = center ? h / 2 : h

  // Shift profile to handle centering
  const shifted = profile.map(p => [p[0] + ox, p[1] + oy])

  // Bottom cap
  const baseBot = v.length / 6
  for (let i = 0; i < np; i++) {
    v.push(shifted[i][0], shifted[i][1], z0, 0, 0, -1)
  }
  const botTris = earClip(shifted)
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(baseBot + botTris[i], baseBot + botTris[i + 2], baseBot + botTris[i + 1])
  }

  // Top cap
  const baseTop = v.length / 6
  for (let i = 0; i < np; i++) {
    v.push(shifted[i][0], shifted[i][1], z1, 0, 0, 1)
  }
  for (let i = 0; i < botTris.length; i += 3) {
    ix.push(baseTop + botTris[i], baseTop + botTris[i + 1], baseTop + botTris[i + 2])
  }

  // Side walls
  for (let i = 0; i < np; i++) {
    const i2 = (i + 1) % np
    const x0 = shifted[i][0], y0 = shifted[i][1]
    const x1 = shifted[i2][0], y1 = shifted[i2][1]
    const ex = x1 - x0, ey = y1 - y0
    const len = Math.sqrt(ex * ex + ey * ey) || 1
    const nnx = ey / len, nny = -ex / len
    const base = v.length / 6
    v.push(x0, y0, z0, nnx, nny, 0)
    v.push(x1, y1, z0, nnx, nny, 0)
    v.push(x1, y1, z1, nnx, nny, 0)
    v.push(x0, y0, z1, nnx, nny, 0)
    ix.push(base, base + 1, base + 2, base, base + 2, base + 3)
  }

  return { v, ix }
}

function makeCross(size: [number, number, number], arm: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const [sx, sy, sz] = size

  // Bar 1: along X, centered
  const bar1 = makeCube(sx, arm, sz, true)
  // Bar 2: along Y, centered
  const bar2 = makeCube(arm, sy, sz, true)

  // Merge bar1
  v.push(...bar1.v)
  ix.push(...bar1.ix)

  // Merge bar2 with offset
  const base = v.length / 6
  v.push(...bar2.v)
  for (const idx of bar2.ix) {
    ix.push(base + idx)
  }

  return { v, ix }
}

function makeMaze(rows: number, cols: number, cell: number, wall: number, h: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []

  // Simple seeded random
  let seed = rows * 1000 + cols * 100 + cell * 10 + wall
  const rand = () => {
    seed = (seed * 1103515245 + 12345) & 0x7fffffff
    return seed / 0x7fffffff
  }

  // Initialize grid
  const visited: boolean[][] = []
  // walls: [right, bottom] for each cell
  const wallsR: boolean[][] = []
  const wallsB: boolean[][] = []
  for (let r = 0; r < rows; r++) {
    visited.push(new Array(cols).fill(false))
    wallsR.push(new Array(cols).fill(true))
    wallsB.push(new Array(cols).fill(true))
  }

  // Recursive backtracking maze generation
  const stack: [number, number][] = []
  const start: [number, number] = [0, 0]
  visited[start[0]][start[1]] = true
  stack.push(start)

  while (stack.length > 0) {
    const [cr, cc] = stack[stack.length - 1]
    // Find unvisited neighbors
    const neighbors: [number, number, string][] = []
    if (cr > 0 && !visited[cr - 1][cc]) neighbors.push([cr - 1, cc, 'up'])
    if (cr < rows - 1 && !visited[cr + 1][cc]) neighbors.push([cr + 1, cc, 'down'])
    if (cc > 0 && !visited[cr][cc - 1]) neighbors.push([cr, cc - 1, 'left'])
    if (cc < cols - 1 && !visited[cr][cc + 1]) neighbors.push([cr, cc + 1, 'right'])

    if (neighbors.length === 0) {
      stack.pop()
    } else {
      const choice = Math.floor(rand() * neighbors.length) % neighbors.length
      const [nr, nc, dir] = neighbors[choice]
      // Remove wall between current and chosen
      if (dir === 'right') wallsR[cr][cc] = false
      if (dir === 'left') wallsR[cr][nc] = false
      if (dir === 'down') wallsB[cr][cc] = false
      if (dir === 'up') wallsB[nr][cc] = false
      visited[nr][nc] = true
      stack.push([nr, nc])
    }
  }

  const addCube = (cx: number, cy: number, cz: number, csx: number, csy: number, csz: number) => {
    const cube = makeCube(csx, csy, csz, false)
    const base = v.length / 6
    const nv = cube.v.length / 6
    for (let i = 0; i < nv; i++) {
      v.push(cube.v[i * 6] + cx, cube.v[i * 6 + 1] + cy, cube.v[i * 6 + 2] + cz,
             cube.v[i * 6 + 3], cube.v[i * 6 + 4], cube.v[i * 6 + 5])
    }
    for (const idx of cube.ix) {
      ix.push(base + idx)
    }
  }

  // Floor
  const totalW = cols * cell + wall
  const totalH = rows * cell + wall
  addCube(0, 0, 0, totalW, totalH, wall)

  // Border walls
  // Bottom border (y=0)
  addCube(0, 0, 0, totalW, wall, h)
  // Top border (y=totalH-wall)
  addCube(0, totalH - wall, 0, totalW, wall, h)
  // Left border (x=0)
  addCube(0, 0, 0, wall, totalH, h)
  // Right border (x=totalW-wall)
  addCube(totalW - wall, 0, 0, wall, totalH, h)

  // Internal walls
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      const cx = wall + c * cell
      const cy = wall + r * cell
      // Right wall
      if (wallsR[r][c] && c < cols - 1) {
        addCube(cx + cell - wall, cy, 0, wall, cell, h)
      }
      // Bottom wall (which is top in our Y direction)
      if (wallsB[r][c] && r < rows - 1) {
        addCube(cx, cy + cell - wall, 0, cell, wall, h)
      }
    }
  }

  return { v, ix }
}

function makeFibonacciSphere(count: number, r: number, fn: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const goldenAngle = Math.PI * (3 - Math.sqrt(5))
  const smallR = Math.max(0.3, r / Math.sqrt(count) * 0.5)

  for (let i = 0; i < count; i++) {
    const y = 1 - (2 * i) / (count - 1)  // y goes from 1 to -1
    const radiusAtY = Math.sqrt(1 - y * y)
    const theta = goldenAngle * i

    const px = r * radiusAtY * Math.cos(theta)
    const py = r * y
    const pz = r * radiusAtY * Math.sin(theta)

    // Add a small sphere at this position
    const sphere = makeSphere(smallR, Math.max(4, Math.floor(fn / 4)))
    const base = v.length / 6
    const nv = sphere.v.length / 6
    for (let j = 0; j < nv; j++) {
      v.push(sphere.v[j * 6] + px, sphere.v[j * 6 + 1] + py, sphere.v[j * 6 + 2] + pz,
             sphere.v[j * 6 + 3], sphere.v[j * 6 + 4], sphere.v[j * 6 + 5])
    }
    for (const idx of sphere.ix) {
      ix.push(base + idx)
    }
  }

  return { v, ix }
}

function makeEllipsoid(rx: number, ry: number, rz: number, fn: number): { v: number[]; ix: number[] } {
  const sphere = makeSphere(1, fn)
  const v: number[] = []
  const nv = sphere.v.length / 6
  for (let i = 0; i < nv; i++) {
    const px = sphere.v[i * 6] * rx
    const py = sphere.v[i * 6 + 1] * ry
    const pz = sphere.v[i * 6 + 2] * rz
    // Recompute normal for ellipsoid: gradient of (x/rx)^2 + (y/ry)^2 + (z/rz)^2
    let nnx = px / (rx * rx)
    let nny = py / (ry * ry)
    let nnz = pz / (rz * rz)
    const nlen = Math.sqrt(nnx * nnx + nny * nny + nnz * nnz) || 1
    nnx /= nlen; nny /= nlen; nnz /= nlen
    v.push(px, py, pz, nnx, nny, nnz)
  }
  return { v, ix: sphere.ix }
}

function makeHemisphere(r: number, fn: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const halfSeg = Math.floor(fn / 2)

  // Upper hemisphere: phi from 0 (top) to PI/2 (equator)
  for (let ri = 0; ri <= halfSeg; ri++) {
    const phi = (Math.PI / 2) * ri / halfSeg
    const sp = Math.sin(phi), cp = Math.cos(phi)
    for (let si = 0; si <= fn; si++) {
      const th = 2 * Math.PI * si / fn
      const nx = sp * Math.cos(th), ny = cp, nz = sp * Math.sin(th)
      v.push(r * nx, r * ny, r * nz, nx, ny, nz)
    }
  }
  for (let ri = 0; ri < halfSeg; ri++) {
    for (let si = 0; si < fn; si++) {
      const a = ri * (fn + 1) + si, b = a + fn + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }

  // Flat circular cap at y=0 (equator)
  const capCenter = v.length / 6
  v.push(0, 0, 0, 0, -1, 0)  // center, normal pointing down
  for (let si = 0; si <= fn; si++) {
    const th = 2 * Math.PI * si / fn
    v.push(r * Math.cos(th), 0, r * Math.sin(th), 0, -1, 0)
  }
  for (let si = 0; si < fn; si++) {
    ix.push(capCenter, capCenter + 1 + si + 1, capCenter + 1 + si)
  }

  return { v, ix }
}

function makeOgive(r: number, h: number, fn: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  // Ogive radius of curvature: R = (r^2 + h^2) / (2*r)
  const R = (r * r + h * h) / (2 * r)
  const stacks = Math.max(4, Math.floor(fn / 2))

  // Generate profile: z goes from 0 (base) to h (tip)
  for (let ri = 0; ri <= stacks; ri++) {
    const z = h * ri / stacks
    // Ogive profile: x(z) = sqrt(R^2 - (z - h)^2) - (R - r)
    const inner = R * R - (z - h) * (z - h)
    const xr = inner > 0 ? Math.sqrt(inner) - (R - r) : 0
    const radius = Math.max(0, xr)
    for (let si = 0; si <= fn; si++) {
      const th = 2 * Math.PI * si / fn
      const nx0 = Math.cos(th), nz0 = Math.sin(th)
      const px = radius * nx0, py = z, pz = radius * nz0
      // Approximate normal: tangent along profile gives slope
      const zUp = h * Math.min(ri + 1, stacks) / stacks
      const innerUp = R * R - (zUp - h) * (zUp - h)
      const xrUp = innerUp > 0 ? Math.sqrt(innerUp) - (R - r) : 0
      const zDn = h * Math.max(ri - 1, 0) / stacks
      const innerDn = R * R - (zDn - h) * (zDn - h)
      const xrDn = innerDn > 0 ? Math.sqrt(innerDn) - (R - r) : 0
      const dr = (xrUp - xrDn) / (zUp - zDn || 1)
      // Normal perpendicular to surface: (1, -dr, 0) rotated around Y
      let nnx = nx0, nny = -dr, nnz = nz0
      const nlen = Math.sqrt(nnx * nnx + nny * nny + nnz * nnz) || 1
      nnx /= nlen; nny /= nlen; nnz /= nlen
      v.push(px, py, pz, nnx, nny, nnz)
    }
  }
  for (let ri = 0; ri < stacks; ri++) {
    for (let si = 0; si < fn; si++) {
      const a = ri * (fn + 1) + si, b = a + fn + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }

  // Bottom cap at z=0
  const capCenter = v.length / 6
  v.push(0, 0, 0, 0, -1, 0)
  for (let si = 0; si <= fn; si++) {
    const th = 2 * Math.PI * si / fn
    v.push(r * Math.cos(th), 0, r * Math.sin(th), 0, -1, 0)
  }
  for (let si = 0; si < fn; si++) {
    ix.push(capCenter, capCenter + 1 + si + 1, capCenter + 1 + si)
  }

  return { v, ix }
}

function makeTeardrop(r: number, fn: number): { v: number[]; ix: number[] } {
  const v: number[] = [], ix: number[] = []
  const halfSeg = Math.max(4, Math.floor(fn / 2))

  // Lower hemisphere: phi from PI/2 (equator, y=0) to PI (bottom, y=-r)
  for (let ri = 0; ri <= halfSeg; ri++) {
    const phi = Math.PI / 2 + (Math.PI / 2) * ri / halfSeg
    const sp = Math.sin(phi), cp = Math.cos(phi)
    for (let si = 0; si <= fn; si++) {
      const th = 2 * Math.PI * si / fn
      const nx = sp * Math.cos(th), ny = cp, nz = sp * Math.sin(th)
      v.push(r * nx, r * ny, r * nz, nx, ny, nz)
    }
  }
  for (let ri = 0; ri < halfSeg; ri++) {
    for (let si = 0; si < fn; si++) {
      const a = ri * (fn + 1) + si, b = a + fn + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }

  // 45-degree cone from equator (y=0) to tip (y=r)
  // At equator radius = r, at tip radius = 0, height = r (45 degrees)
  const coneStacks = halfSeg
  const baseOffset = v.length / 6
  for (let ri = 0; ri <= coneStacks; ri++) {
    const t = ri / coneStacks
    const y = r * t
    const radius = r * (1 - t)
    for (let si = 0; si <= fn; si++) {
      const th = 2 * Math.PI * si / fn
      const cx = Math.cos(th), cz = Math.sin(th)
      const px = radius * cx, py = y, pz = radius * cz
      // Cone normal: 45-degree slope, normalized
      const s45 = Math.SQRT1_2
      const nnx = s45 * cx, nny = s45, nnz = s45 * cz
      v.push(px, py, pz, nnx, nny, nnz)
    }
  }
  for (let ri = 0; ri < coneStacks; ri++) {
    for (let si = 0; si < fn; si++) {
      const a = baseOffset + ri * (fn + 1) + si, b = a + fn + 1
      ix.push(a, b, a + 1, a + 1, b, b + 1)
    }
  }

  // Stitch hemisphere equator (row 0) to cone base (row 0)
  // They share the same y=0, radius=r ring, so just connect them
  for (let si = 0; si < fn; si++) {
    const hTop = si  // hemisphere row 0 (equator)
    const cBot = baseOffset + si  // cone row 0 (equator)
    ix.push(hTop, cBot, hTop + 1)
    ix.push(hTop + 1, cBot, cBot + 1)
  }

  return { v, ix }
}

/* ── Main evaluator ───────────────────────────────── */

function evalNodes(nodes: ASTNode[], tf: Mat4, col: [number,number,number,number]|null, vars: Record<string, number> = {}, modules: Map<string, ModuleDef> = new Map(), echos: string[] = [], callerChildren?: ASTNode[], annotations: Annotation[] = []): MeshData[] {
  const out: MeshData[] = []
  let lastIfResult = false
  for (let ni = 0; ni < nodes.length; ni++) {
    const n = nodes[ni]
    try {

    // Handle variable assignments
    if (n.name === '__assign') {
      const varName = n.args.__varName
      const varValue = n.args.__varValue
      if (typeof varName === 'string') {
        const resolved = resolveArg(varValue, vars)
        // Store the variable regardless of type (number, vector/array,
        // string, boolean) so it can be looked up later by resolveArg /
        // evalExprNode. Only skip undefined/null results.
        if (resolved !== undefined && resolved !== null) {
          vars = { ...vars, [varName]: resolved }
        }
      }
      continue
    }

    // Handle module definitions
    if (n.name === 'module') {
      const modName = n.args.__name as string
      if (modName) {
        modules.set(modName, {
          params: n.params || [],
          children: n.children,
        })
      }
      continue
    }

    // Handle use/include
    if (n.name === 'use' || n.name === 'include') {
      const fileName = n.args.__fileName as string
      if (fileName && _resolveFile) {
        const fileSrc = _resolveFile(fileName)
        if (fileSrc) {
          try {
            const tokens = tokenize(fileSrc)
            const parser = new Parser(tokens)
            const fileAst = parser.parseAll()
            if (n.name === 'include') {
              // include: evaluate everything (modules + geometry)
              out.push(...evalNodes(fileAst, tf, col, vars, modules, echos, callerChildren, annotations))
            } else {
              // use: only import module definitions (no geometry)
              for (const fn of fileAst) {
                if (fn.name === 'module') {
                  const modName = fn.args.__name as string
                  if (modName) {
                    modules.set(modName, {
                      params: fn.params || [],
                      children: fn.children,
                    })
                  }
                }
              }
            }
          } catch (e: any) {
            echos.push('ERROR: ' + (e.message || String(e)))
          }
        } else {
          echos.push('ERROR: File not found: ' + fileName)
        }
      } else if (fileName && !_resolveFile) {
        echos.push('WARNING: use/include not supported in single-file mode: ' + fileName)
      }
      continue
    }

    if (n.name === 'else') {
      // Only evaluate else children if the previous if was false
      if (!lastIfResult) {
        out.push(...evalNodes(n.children, tf, col, vars, modules, echos, callerChildren, annotations))
      }
      continue
    }
    if (n.name === 'if') {
      const ra = resolveArgs(n.args, vars)
      const cond = arg(ra, '_0', 0, 0)
      lastIfResult = evalCondition(cond)
      if (lastIfResult) {
        out.push(...evalNodes(n.children, tf, col, vars, modules, echos, callerChildren, annotations))
      }
      continue
    }
    lastIfResult = false
    if (_profiling && _profileDepth === 0) {
      _profileDepth++
      const t0 = performance.now()
      out.push(...evalNode(n, tf, col, vars, modules, echos, callerChildren, annotations))
      const dt = performance.now() - t0
      _profileDepth--
      const beforePos = _source.substring(0, n.pos)
      const lineNum = beforePos.split('\n').length
      _profileEntries.push({ name: n.name, line: lineNum, timeMs: dt })
    } else {
      out.push(...evalNode(n, tf, col, vars, modules, echos, callerChildren, annotations))
    }

    } catch (_skipErr) {
      // Error recovery: skip this node and continue with the rest
      continue
    }
  }
  return out
}

function evalNode(node: ASTNode, tf: Mat4, col: [number,number,number,number]|null, vars: Record<string, number> = {}, modules: Map<string, ModuleDef> = new Map(), echos: string[] = [], callerChildren?: ASTNode[], annotations: Annotation[] = []): MeshData[] {
  const { name: nm, args: rawArgs, children: ch } = node
  const a = resolveArgs(rawArgs, vars)

  switch (nm) {
    case 'echo': {
      // Collect echo output, do not generate mesh
      const parts: string[] = []
      for (const [k, v] of Object.entries(a)) {
        if (k.startsWith('_')) {
          parts.push(typeof v === 'string' ? `"${v}"` : String(v))
        } else {
          parts.push(`${k} = ${typeof v === 'string' ? `"${v}"` : String(v)}`)
        }
      }
      echos.push('ECHO: ' + parts.join(', '))
      // echo can have children (pass-through)
      if (ch.length > 0) return evalNodes(ch, tf, col, vars, modules, echos, callerChildren, annotations)
      return []
    }
    case 'annotation': {
      const text = arg(a, 'text', 0, '') as string
      let pos = arg(a, 'pos', 1, [0, 0, 0])
      if (!Array.isArray(pos)) pos = [0, 0, 0]
      const wx = (pos[0] ?? 0) as number
      const wy = (pos[1] ?? 0) as number
      const wz = (pos[2] ?? 0) as number
      // Apply transform to position
      const tx = tf[0]*wx + tf[1]*wy + tf[2]*wz + tf[3]
      const ty = tf[4]*wx + tf[5]*wy + tf[6]*wz + tf[7]
      const tz = tf[8]*wx + tf[9]*wy + tf[10]*wz + tf[11]
      annotations.push({ text: String(text), position: [tx, ty, tz] })
      if (ch.length > 0) return evalNodes(ch, tf, col, vars, modules, echos, callerChildren, annotations)
      return []
    }
    case 'children': {
      // Evaluate caller's children if available
      if (callerChildren && callerChildren.length > 0) {
        return evalNodes(callerChildren, tf, col, vars, modules, echos, undefined, annotations)
      }
      return []
    }
    case 'cube': {
      const size = arg(a,'size',0,1)
      const center = arg(a,'center',1,false) === true
      let sx: number, sy: number, sz: number
      if (Array.isArray(size)) { sx = size[0]??1; sy = size[1]??1; sz = size[2]??1 }
      else { sx = sy = sz = typeof size === 'number' ? size : 1 }
      const { v, ix } = makeCube(sx, sy, sz, center)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'sphere': {
      let r = arg(a,'r',0,undefined)
      const d = arg(a,'d',-1,undefined)
      if (r == null) r = d != null ? d / 2 : 1
      if (typeof r !== 'number') r = 1
      const fn = Math.min(MAX_FN, Math.max(8, arg(a,'$fn',-1,24)))
      const { v, ix } = makeSphere(r, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'cylinder': {
      const h = arg(a,'h',0,1)
      let r1 = arg(a,'r1',-1,undefined), r2 = arg(a,'r2',-1,undefined)
      const r = arg(a,'r',1,undefined), d = arg(a,'d',-1,undefined)
      const d1 = arg(a,'d1',-1,undefined), d2 = arg(a,'d2',-1,undefined)
      if (d1 != null) r1 = d1/2; if (d2 != null) r2 = d2/2
      if (r1 == null && r2 == null) { const br = d != null ? d/2 : r != null ? r : 1; r1 = br; r2 = br }
      if (r1 == null) r1 = r2; if (r2 == null) r2 = r1
      const center = arg(a,'center',-1,false) === true
      const fn = Math.min(MAX_FN, Math.max(8, arg(a,'$fn',-1,24)))
      const { v, ix } = makeCylinder(h, r1!, r2!, center, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'polygon': {
      const pts = arg(a, 'points', 0, [])
      if (!Array.isArray(pts)) return []
      const { v, ix } = makePolygon(pts)
      if (!v.length) return []
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'circle': {
      // Generate a 2D circle as a polygon
      let r = arg(a, 'r', 0, undefined)
      const d = arg(a, 'd', -1, undefined)
      if (r == null) r = d != null ? d / 2 : 1
      if (typeof r !== 'number') r = 1
      const fn = Math.min(MAX_FN, Math.max(8, arg(a, '$fn', -1, 24)))
      const pts: number[][] = []
      for (let i = 0; i < fn; i++) {
        const angle = (2 * Math.PI * i) / fn
        pts.push([r * Math.cos(angle), r * Math.sin(angle)])
      }
      const { v, ix } = makePolygon(pts)
      if (!v.length) return []
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'square': {
      // Generate a 2D square as a polygon
      const size = arg(a, 'size', 0, 1)
      const center = arg(a, 'center', 1, false) === true
      let sx: number, sy: number
      if (Array.isArray(size)) { sx = size[0] ?? 1; sy = size[1] ?? 1 }
      else { sx = sy = typeof size === 'number' ? size : 1 }
      const x0 = center ? -sx / 2 : 0, x1 = center ? sx / 2 : sx
      const y0 = center ? -sy / 2 : 0, y1 = center ? sy / 2 : sy
      const pts = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
      const { v, ix } = makePolygon(pts)
      if (!v.length) return []
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'text': {
      const txt = arg(a, 'text', 0, arg(a, '_0', 0, ''))
      const textStr = typeof txt === 'string' ? txt : String(txt)
      const size = typeof arg(a, 'size', 1, 10) === 'number' ? arg(a, 'size', 1, 10) as number : 10
      const spacing = typeof arg(a, 'spacing', -1, 1) === 'number' ? arg(a, 'spacing', -1, 1) as number : 1
      const { v, ix } = makeText(textStr, size, spacing)
      if (!v.length) return []
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'for': {
      // for(i = [0:5]) or for(i = [0:2:10]) or for(i = [1,3,7])
      // Multiple variables are nested (Cartesian product):
      //   for(x=[0:2], y=[0:2]) -> 3*3 = 9 iterations.
      const out: MeshData[] = []
      // Collect every named loop variable and its values, preserving order.
      const loopVars: { name: string; values: any[] }[] = []
      for (const [varName, rawVal] of Object.entries(a)) {
        if (varName.startsWith('_')) continue // skip positional
        const val = rawVal
        let iterValues: any[] = []
        if (val && typeof val === 'object' && val.__range) {
          iterValues = expandRange(val)
        } else if (Array.isArray(val)) {
          iterValues = val
        } else if (val !== undefined && val !== null) {
          iterValues = [val]
        }
        loopVars.push({ name: varName, values: iterValues })
      }
      if (loopVars.length === 0) {
        return evalNodes(ch, tf, col, vars, modules, echos, callerChildren, annotations)
      }
      // Bound the Cartesian product so nested ranges can't blow up:
      // for(x=[0:9999], y=[0:9999]) would otherwise schedule 1e8 iterations.
      let totalIterations = 1
      for (const lv of loopVars) totalIterations *= lv.values.length
      if (totalIterations > 100000) {
        throw new Error('for loop too large (>100000 iterations)')
      }
      // Recursively walk the Cartesian product of all loop variables.
      const walk = (idx: number, accVars: Record<string, any>) => {
        if (idx >= loopVars.length) {
          out.push(...evalNodes(ch, tf, col, accVars, modules, echos, callerChildren, annotations))
          return
        }
        const { name, values } = loopVars[idx]
        for (const v of values) {
          walk(idx + 1, { ...accVars, [name]: v })
        }
      }
      walk(0, vars)
      return out
    }
    case 'let': {
      const newVars = { ...vars }
      for (const [k, v] of Object.entries(a)) {
        if (k.startsWith('_')) continue
        if (typeof v === 'number') newVars[k] = v
      }
      return evalNodes(ch, tf, col, newVars, modules, echos, callerChildren, annotations)
    }
    case 'if': {
      const cond = arg(a, '_0', 0, 0)
      if (evalCondition(cond)) {
        return evalNodes(ch, tf, col, vars, modules, echos, callerChildren, annotations)
      }
      return []
    }
    case 'else': {
      return evalNodes(ch, tf, col, vars, modules, echos, callerChildren, annotations)
    }
    case 'translate': {
      const raw = arg(a,'v',0,[0,0,0])
      const vec: Vec3 = Array.isArray(raw) ? [raw[0]??0, raw[1]??0, raw[2]??0] : [0,0,0]
      return evalNodes(ch, translate(tf, vec), col, vars, modules, echos, callerChildren, annotations)
    }
    case 'rotate': {
      const av = arg(a,'a',0,0), vv = arg(a,'v',1,undefined)
      let nt = tf
      if (Array.isArray(av)) {
        const [rx,ry,rz] = [(av[0]??0)*Math.PI/180, (av[1]??0)*Math.PI/180, (av[2]??0)*Math.PI/180]
        if (rz) nt = rotateZ(nt, rz); if (ry) nt = rotateY(nt, ry); if (rx) nt = rotateX(nt, rx)
      } else if (typeof av === 'number' && Array.isArray(vv)) {
        nt = axisAngle(nt, [vv[0]??0, vv[1]??0, vv[2]??1], av * Math.PI / 180)
      } else if (typeof av === 'number') {
        nt = rotateZ(nt, av * Math.PI / 180)
      }
      return evalNodes(ch, nt, col, vars, modules, echos, callerChildren, annotations)
    }
    case 'scale': {
      const raw = arg(a,'v',0,[1,1,1])
      const vec: Vec3 = Array.isArray(raw) ? [raw[0]??1,raw[1]??1,raw[2]??1] : [raw,raw,raw]
      return evalNodes(ch, scale(tf, vec), col, vars, modules, echos, callerChildren, annotations)
    }
    case 'resize': {
      const newSize = arg(a, 'newsize', 0, arg(a, '_0', 0, [0,0,0]))
      const autoVal = arg(a, 'auto', 1, false)
      void autoVal

      // Evaluate children first to get their meshes
      const childMeshes = evalNodes(ch, identity(), col, vars, modules, echos, callerChildren, annotations)
      if (childMeshes.length === 0) return []

      // Compute bounding box of all children
      let minX = Infinity, minY = Infinity, minZ = Infinity
      let maxX = -Infinity, maxY = -Infinity, maxZ = -Infinity
      for (const mesh of childMeshes) {
        const verts = mesh.vertices
        const m = mesh.transform
        const nv = verts.length / 6
        for (let i = 0; i < nv; i++) {
          const x = verts[i*6], y = verts[i*6+1], z = verts[i*6+2]
          const tx = m[0]*x + m[1]*y + m[2]*z + m[3]
          const ty = m[4]*x + m[5]*y + m[6]*z + m[7]
          const tz = m[8]*x + m[9]*y + m[10]*z + m[11]
          if (tx < minX) minX = tx; if (tx > maxX) maxX = tx
          if (ty < minY) minY = ty; if (ty > maxY) maxY = ty
          if (tz < minZ) minZ = tz; if (tz > maxZ) maxZ = tz
        }
      }

      const curX = maxX - minX || 1
      const curY = maxY - minY || 1
      const curZ = maxZ - minZ || 1

      const ns = Array.isArray(newSize) ? newSize : [0,0,0]
      const nx = typeof ns[0] === 'number' ? ns[0] : 0
      const ny = typeof ns[1] === 'number' ? ns[1] : 0
      const nz = typeof ns[2] === 'number' ? ns[2] : 0

      // Compute scale factors
      let sx = nx > 0 ? nx / curX : 0
      let sy = ny > 0 ? ny / curY : 0
      let sz = nz > 0 ? nz / curZ : 0

      // Handle auto-scaling for zero-valued axes (maintain aspect ratio)
      const specified = [sx, sy, sz].filter(s => s > 0)
      if (specified.length > 0) {
        const refScale = specified[0]
        if (sx === 0) sx = refScale
        if (sy === 0) sy = refScale
        if (sz === 0) sz = refScale
      } else {
        sx = sy = sz = 1
      }

      return evalNodes(ch, scale(tf, [sx, sy, sz]), col, vars, modules, echos, callerChildren, annotations)
    }
    case 'mirror': {
      const raw = arg(a,'v',0,[1,0,0])
      if (Array.isArray(raw)) {
        const sv: Vec3 = [raw[0]?-1:1, raw[1]?-1:1, raw[2]?-1:1]
        return evalNodes(ch, scale(tf, sv), col, vars, modules, echos, callerChildren, annotations)
      }
      return evalNodes(ch, tf, col, vars, modules, echos, callerChildren, annotations)
    }
    case 'color': {
      const c = arg(a,'c',0,arg(a,'_0',0,[0.5,0.5,0.5]))
      let nc: [number,number,number,number]
      if (Array.isArray(c)) nc = [c[0]??0.5, c[1]??0.5, c[2]??0.5, c[3]??1]
      else if (typeof c === 'string') nc = cssColor(c)
      else nc = [0.5,0.5,0.5,1]
      return evalNodes(ch, tf, nc, vars, modules, echos, callerChildren, annotations)
    }
    case 'union':
      return evalNodes(ch, tf, col, vars, modules, echos, callerChildren, annotations)
    case 'difference': {
      const out: MeshData[] = []
      if (ch.length > 0) out.push(...evalNode(ch[0], tf, col, vars, modules, echos, callerChildren, annotations))
      for (let i = 1; i < ch.length; i++)
        out.push(...evalNode(ch[i], tf, [0.9, 0.15, 0.15, 0.35], vars, modules, echos, callerChildren, annotations))
      return out
    }
    case 'intersection':
      return evalNodes(ch, tf, col ? [col[0],col[1],col[2],0.55] : null, vars, modules, echos, callerChildren, annotations)
    case 'multmatrix': {
      const m = arg(a,'m',0,undefined)
      if (Array.isArray(m) && m.length >= 4) {
        const mat = new Float32Array(16)
        for (let r = 0; r < 4; r++) for (let c = 0; c < 4; c++) mat[r*4+c] = m[r]?.[c] ?? (r===c?1:0)
        return evalNodes(ch, multiply(mat, tf), col, vars, modules, echos, callerChildren, annotations)
      }
      return evalNodes(ch, tf, col, vars, modules, echos, callerChildren, annotations)
    }
    case 'linear_extrude': {
      const height = typeof arg(a, 'height', 0, 10) === 'number' ? arg(a, 'height', 0, 10) as number : 10
      const twist = typeof arg(a, 'twist', -1, 0) === 'number' ? arg(a, 'twist', -1, 0) as number : 0
      const slices = typeof arg(a, 'slices', -1, 1) === 'number' ? arg(a, 'slices', -1, 1) as number : 1

      // Evaluate children to get flat meshes
      const childMeshes = evalNodes(ch, identity(), null, vars, modules, echos, callerChildren, annotations)

      if (childMeshes.length === 0) return []

      const out: MeshData[] = []
      for (const childMesh of childMeshes) {
        const { pts, flat } = extractFlatProfile(childMesh)
        if (flat && pts.length >= 3) {
          const { v, ix } = extrudeMesh(pts, height, twist, slices)
          if (v.length > 0) {
            out.push({
              vertices: new Float32Array(v),
              indices: new Uint32Array(ix),
              color: col ?? childMesh.color,
              transform: tf,
            })
          }
        } else {
          // Non-flat: just pass through with transform
          out.push({ ...childMesh, transform: tf })
        }
      }
      return out
    }
    case 'rotate_extrude': {
      const fn = Math.min(MAX_FN, Math.max(3, typeof arg(a, '$fn', -1, 36) === 'number' ? arg(a, '$fn', -1, 36) as number : 36))
      const angle = typeof arg(a, 'angle', -1, 360) === 'number' ? arg(a, 'angle', -1, 360) as number : 360

      // Evaluate children to get flat meshes
      const childMeshes = evalNodes(ch, identity(), null, vars, modules, echos, callerChildren, annotations)

      if (childMeshes.length === 0) return []

      const out: MeshData[] = []
      for (const childMesh of childMeshes) {
        const { pts, flat } = extractFlatProfile(childMesh)
        if (flat && pts.length >= 2) {
          // For rotate_extrude, the profile is in XZ plane (X = radius, Y = Z)
          // The children are in XY plane, so we interpret X as radius and Y as Z height
          const xzProfile: [number, number][] = pts.map(p => [Math.abs(p[0]), p[1]])

          // Sort by angle around centroid for proper ordering
          // Actually sort by Y (which becomes Z) for a proper profile
          xzProfile.sort((a2, b2) => {
            const angleDiff = Math.atan2(a2[1], a2[0]) - Math.atan2(b2[1], b2[0])
            return angleDiff
          })

          const { v, ix } = rotateExtrudeMesh(xzProfile, fn, angle)
          if (v.length > 0) {
            out.push({
              vertices: new Float32Array(v),
              indices: new Uint32Array(ix),
              color: col ?? childMesh.color,
              transform: tf,
            })
          }
        } else {
          out.push({ ...childMesh, transform: tf })
        }
      }
      return out
    }
    case 'hull': {
      // Compute convex hull of all children vertices
      const childMeshes = evalNodes(ch, identity(), null, vars, modules, echos, callerChildren, annotations)

      if (childMeshes.length === 0) return []

      // Collect all vertices from all children
      const allPts: Vec3[] = []
      for (const mesh of childMeshes) {
        const verts = mesh.vertices
        const nv = verts.length / 6
        // Apply mesh transform to get world-space vertices
        const m = mesh.transform
        for (let i = 0; i < nv; i++) {
          const x = verts[i * 6], y = verts[i * 6 + 1], z = verts[i * 6 + 2]
          // Transform point by mesh's transform matrix (row-major)
          const tx = m[0]*x + m[1]*y + m[2]*z + m[3]
          const ty = m[4]*x + m[5]*y + m[6]*z + m[7]
          const tz = m[8]*x + m[9]*y + m[10]*z + m[11]
          allPts.push([tx, ty, tz])
        }
      }

      if (allPts.length === 0) return []

      const { v, ix } = convexHull3D(allPts)
      if (v.length === 0) return []

      return [{
        vertices: new Float32Array(v),
        indices: new Uint32Array(ix),
        color: col ?? nextC(),
        transform: tf,
      }]
    }
    case 'polyhedron': {
      const pts = arg(a, 'points', 0, [])
      const fcs = arg(a, 'faces', 1, arg(a, 'triangles', -1, []))
      if (!Array.isArray(pts) || !Array.isArray(fcs)) return []
      const { v, ix } = makePolyhedron(pts, fcs)
      if (!v.length) return []
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'offset': {
      const r = typeof arg(a, 'r', 0, 0) === 'number' ? arg(a, 'r', 0, 0) as number : 0
      const delta = typeof arg(a, 'delta', -1, 0) === 'number' ? arg(a, 'delta', -1, 0) as number : 0
      const amount = r !== 0 ? r : delta

      const childMeshes = evalNodes(ch, identity(), null, vars, modules, echos, callerChildren, annotations)
      if (childMeshes.length === 0) return []

      const out: MeshData[] = []
      for (const childMesh of childMeshes) {
        const verts = childMesh.vertices
        const nv = verts.length / 6

        // Compute centroid
        let cx = 0, cy = 0
        for (let i = 0; i < nv; i++) {
          cx += verts[i * 6]
          cy += verts[i * 6 + 1]
        }
        cx /= nv; cy /= nv

        // Offset each vertex away from centroid
        const newVerts = new Float32Array(verts)
        for (let i = 0; i < nv; i++) {
          const x = verts[i * 6] - cx
          const y = verts[i * 6 + 1] - cy
          const dist = Math.sqrt(x * x + y * y)
          if (dist > 1e-10) {
            newVerts[i * 6] = verts[i * 6] + (x / dist) * amount
            newVerts[i * 6 + 1] = verts[i * 6 + 1] + (y / dist) * amount
          }
        }

        out.push({
          vertices: newVerts,
          indices: new Uint32Array(childMesh.indices),
          color: col ?? childMesh.color,
          transform: tf,
        })
      }
      return out
    }
    case 'assert': {
      const cond = arg(a, '_0', 0, true)
      const message = arg(a, '_1', 1, arg(a, 'message', -1, 'Assertion failed'))
      if (!evalCondition(cond)) {
        const msg = typeof message === 'string' ? message : 'Assertion failed'
        echos.push('ASSERT: ' + msg)
      }
      // assert can have children (pass-through)
      if (ch.length > 0) return evalNodes(ch, tf, col, vars, modules, echos, callerChildren, annotations)
      return []
    }
    case 'surface': {
      const data = arg(a, 'data', 0, undefined)
      const center = arg(a, 'center', -1, false) === true
      const file = arg(a, 'file', -1, undefined)
      if (file && typeof file === 'string') {
        echos.push('WARNING: surface(file=) not supported in web viewer, use surface(data=[[...],...]) instead')
        return []
      }
      if (!Array.isArray(data) || data.length < 2) return []
      // Validate 2D array
      for (const row of data) {
        if (!Array.isArray(row)) return []
      }
      const { v, ix } = makeSurface(data)
      if (!v.length) return []
      let surfTf = tf
      if (center) {
        const rows = data.length
        const cols = data[0].length
        surfTf = translate(tf, [-(cols - 1) / 2, -(rows - 1) / 2, 0])
      }
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: surfTf }]
    }
    case 'import': {
      const file = arg(a, 'file', 0, '')
      if (typeof file === 'string' && file) {
        echos.push('WARNING: import("' + file + '") not supported in web viewer')
      }
      return evalNodes(ch, tf, col, vars, modules, echos, callerChildren, annotations)
    }
    case 'projection': {
      const cut = arg(a, 'cut', 0, false) === true

      const childMeshes = evalNodes(ch, identity(), null, vars, modules, echos, callerChildren, annotations)
      if (childMeshes.length === 0) return []

      const out: MeshData[] = []
      for (const childMesh of childMeshes) {
        const verts = childMesh.vertices
        const nv = verts.length / 6
        const m = childMesh.transform

        const newVerts = new Float32Array(verts.length)
        let hasVerts = false

        for (let i = 0; i < nv; i++) {
          const x = verts[i * 6], y = verts[i * 6 + 1], z = verts[i * 6 + 2]
          // Apply transform to get world-space Z
          const tz = m[8]*x + m[9]*y + m[10]*z + m[11]

          if (cut && Math.abs(tz) > 0.1) {
            // In cut mode, only keep vertices near Z=0
            newVerts[i * 6] = verts[i * 6]
            newVerts[i * 6 + 1] = verts[i * 6 + 1]
            newVerts[i * 6 + 2] = 0.01
            newVerts[i * 6 + 3] = 0
            newVerts[i * 6 + 4] = 0
            newVerts[i * 6 + 5] = 1
          } else {
            newVerts[i * 6] = verts[i * 6]
            newVerts[i * 6 + 1] = verts[i * 6 + 1]
            newVerts[i * 6 + 2] = 0.01
            newVerts[i * 6 + 3] = 0
            newVerts[i * 6 + 4] = 0
            newVerts[i * 6 + 5] = 1
            hasVerts = true
          }
        }

        if (!cut || hasVerts) {
          out.push({
            vertices: newVerts,
            indices: new Uint32Array(childMesh.indices),
            color: col ?? childMesh.color,
            transform: tf,
          })
        }
      }
      return out.length > 0 ? out : evalNodes(ch, tf, col, vars, modules, echos, callerChildren, annotations)
    }
    case 'torus': {
      const r1 = typeof arg(a, 'r1', 0, 10) === 'number' ? arg(a, 'r1', 0, 10) as number : 10
      const r2 = typeof arg(a, 'r2', 1, 3) === 'number' ? arg(a, 'r2', 1, 3) as number : 3
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 32) === 'number' ? arg(a, '$fn', -1, 32) as number : 32))
      const { v, ix } = makeTorus(r1, r2, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'helix': {
      const r = typeof arg(a, 'r', 0, 10) === 'number' ? arg(a, 'r', 0, 10) as number : 10
      const pitch = typeof arg(a, 'pitch', 1, 5) === 'number' ? arg(a, 'pitch', 1, 5) as number : 5
      const turns = typeof arg(a, 'turns', 2, 3) === 'number' ? arg(a, 'turns', 2, 3) as number : 3
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 32) === 'number' ? arg(a, '$fn', -1, 32) as number : 32))
      const { v, ix } = makeHelix(r, pitch, turns, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'bezier': {
      const pts = arg(a, 'points', 0, [[0,0,0],[10,10,0],[20,10,0],[30,0,0]])
      const thickness = typeof arg(a, 'thickness', 1, 1) === 'number' ? arg(a, 'thickness', 1, 1) as number : 1
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 32) === 'number' ? arg(a, '$fn', -1, 32) as number : 32))
      const points = Array.isArray(pts) ? pts.map((p: any) => Array.isArray(p) ? [p[0]??0, p[1]??0, p[2]??0] : [0,0,0]) : [[0,0,0],[10,10,0],[20,10,0],[30,0,0]]
      if (points.length < 2) return []
      const { v, ix } = makeBezier(points, thickness, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'sweep': {
      const pathRaw = arg(a, 'path', 0, [[0,0,0],[10,0,5],[20,0,0]])
      const radius = typeof arg(a, 'r', 1, 1) === 'number' ? arg(a, 'r', 1, 1) as number : 1
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 16) === 'number' ? arg(a, '$fn', -1, 16) as number : 16))
      const path = Array.isArray(pathRaw) ? pathRaw.map((p: any) => Array.isArray(p) ? [p[0]??0, p[1]??0, p[2]??0] : [0,0,0]) : [[0,0,0],[10,0,5],[20,0,0]]
      if (path.length < 2) return []
      const { v, ix } = makeSweep(path, radius, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'grid': {
      const xCount = typeof arg(a, 'x_count', 0, 3) === 'number' ? Math.max(1, Math.floor(arg(a, 'x_count', 0, 3) as number)) : 3
      const yCount = typeof arg(a, 'y_count', 1, 3) === 'number' ? Math.max(1, Math.floor(arg(a, 'y_count', 1, 3) as number)) : 3
      const xSpacing = typeof arg(a, 'x_spacing', 2, 10) === 'number' ? arg(a, 'x_spacing', 2, 10) as number : 10
      const ySpacing = typeof arg(a, 'y_spacing', 3, 10) === 'number' ? arg(a, 'y_spacing', 3, 10) as number : 10
      const out: MeshData[] = []
      for (let yi = 0; yi < yCount; yi++) {
        for (let xi = 0; xi < xCount; xi++) {
          const offset: Vec3 = [xi * xSpacing, yi * ySpacing, 0]
          out.push(...evalNodes(ch, translate(tf, offset), col, vars, modules, echos, callerChildren, annotations))
        }
      }
      return out
    }
    case 'star': {
      const points = typeof arg(a, 'points', 0, 5) === 'number' ? Math.max(3, Math.floor(arg(a, 'points', 0, 5) as number)) : 5
      const r1 = typeof arg(a, 'r1', 1, 15) === 'number' ? arg(a, 'r1', 1, 15) as number : 15
      const r2 = typeof arg(a, 'r2', 2, 8) === 'number' ? arg(a, 'r2', 2, 8) as number : 8
      const h = typeof arg(a, 'h', 3, 3) === 'number' ? arg(a, 'h', 3, 3) as number : 3
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 32) === 'number' ? arg(a, '$fn', -1, 32) as number : 32))
      const { v, ix } = makeStar(points, r1, r2, h, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'thread': {
      const d = typeof arg(a, 'd', 0, 10) === 'number' ? arg(a, 'd', 0, 10) as number : 10
      const pitch = typeof arg(a, 'pitch', 1, 1.5) === 'number' ? arg(a, 'pitch', 1, 1.5) as number : 1.5
      const length = typeof arg(a, 'length', 2, 20) === 'number' ? arg(a, 'length', 2, 20) as number : 20
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 32) === 'number' ? arg(a, '$fn', -1, 32) as number : 32))
      const { v, ix } = makeThread(d, pitch, length, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'rounded_cube': {
      const sizeRaw = arg(a, 'size', 0, [10, 10, 10])
      let sx: number, sy: number, sz: number
      if (Array.isArray(sizeRaw)) {
        sx = typeof sizeRaw[0] === 'number' ? sizeRaw[0] : 10
        sy = typeof sizeRaw[1] === 'number' ? sizeRaw[1] : 10
        sz = typeof sizeRaw[2] === 'number' ? sizeRaw[2] : 10
      } else if (typeof sizeRaw === 'number') {
        sx = sy = sz = sizeRaw
      } else {
        sx = sy = sz = 10
      }
      const r = typeof arg(a, 'r', 1, 1) === 'number' ? arg(a, 'r', 1, 1) as number : 1
      const center = !!arg(a, 'center', 2, false)
      const fn = Math.min(MAX_FN, Math.max(4, typeof arg(a, '$fn', -1, 8) === 'number' ? arg(a, '$fn', -1, 8) as number : 8))
      const cr = Math.min(r, sx / 2, sy / 2, sz / 2)
      // Place 8 small spheres at the inner corners
      const innerX = sx - 2 * cr, innerY = sy - 2 * cr, innerZ = sz - 2 * cr
      const ox = center ? -sx / 2 + cr : cr
      const oy = center ? -sy / 2 + cr : cr
      const oz = center ? -sz / 2 + cr : cr
      const allPts: [number, number, number][] = []
      for (let xi = 0; xi <= 1; xi++) {
        for (let yi = 0; yi <= 1; yi++) {
          for (let zi = 0; zi <= 1; zi++) {
            const cx = ox + xi * innerX
            const cy = oy + yi * innerY
            const cz = oz + zi * innerZ
            const { v: sv } = makeSphere(cr, fn)
            const nv = sv.length / 6
            for (let i = 0; i < nv; i++) {
              allPts.push([sv[i * 6] + cx, sv[i * 6 + 1] + cy, sv[i * 6 + 2] + cz])
            }
          }
        }
      }
      if (allPts.length === 0) return []
      const { v, ix } = convexHull3D(allPts)
      if (v.length === 0) return []
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'arrow': {
      const length = typeof arg(a, 'length', 0, 20) === 'number' ? arg(a, 'length', 0, 20) as number : 20
      const shaftR = typeof arg(a, 'shaft_r', 1, 1) === 'number' ? arg(a, 'shaft_r', 1, 1) as number : 1
      const headR = typeof arg(a, 'head_r', 2, 3) === 'number' ? arg(a, 'head_r', 2, 3) as number : 3
      const headLen = typeof arg(a, 'head_length', 3, 5) === 'number' ? arg(a, 'head_length', 3, 5) as number : 5
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 16) === 'number' ? arg(a, '$fn', -1, 16) as number : 16))
      const shaftLen = Math.max(0, length - headLen)
      // Shaft: cylinder from z=0 to z=shaftLen
      const shaft = makeCylinder(shaftLen, shaftR, shaftR, false, fn)
      // Head: cone from z=shaftLen to z=length
      const head = makeCylinder(headLen, headR, 0, false, fn)
      // Offset head vertices by shaftLen on Z
      const headVerts = head.v
      const headNv = headVerts.length / 6
      for (let i = 0; i < headNv; i++) {
        headVerts[i * 6 + 2] += shaftLen  // z += shaftLen
      }
      // Merge meshes
      const sv = shaft.v, si = shaft.ix
      const hv = head.v, hi = head.ix
      const shaftVertCount = sv.length / 6
      const mergedV = [...sv, ...hv]
      const mergedIx = [...si, ...hi.map(idx => idx + shaftVertCount)]
      return [{ vertices: new Float32Array(mergedV), indices: new Uint32Array(mergedIx), color: col ?? nextC(), transform: tf }]
    }
    case 'pipe': {
      const h = typeof arg(a, 'h', 0, 10) === 'number' ? arg(a, 'h', 0, 10) as number : 10
      const r1 = typeof arg(a, 'r1', 1, 10) === 'number' ? arg(a, 'r1', 1, 10) as number : 10
      const r2 = typeof arg(a, 'r2', 2, 8) === 'number' ? arg(a, 'r2', 2, 8) as number : 8
      const center = arg(a, 'center', -1, false) === true
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 32) === 'number' ? arg(a, '$fn', -1, 32) as number : 32))
      const { v, ix } = makePipe(h, r1, r2, center, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'wedge': {
      const size = arg(a, 'size', 0, [10, 10, 10])
      let sx: number, sy: number, sz: number
      if (Array.isArray(size)) { sx = typeof size[0] === 'number' ? size[0] : 10; sy = typeof size[1] === 'number' ? size[1] : 10; sz = typeof size[2] === 'number' ? size[2] : 10 }
      else if (typeof size === 'number') { sx = sy = sz = size }
      else { sx = sy = sz = 10 }
      const { v, ix } = makeWedge(sx, sy, sz)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'prism': {
      const sides = typeof arg(a, 'sides', 0, 6) === 'number' ? Math.max(3, Math.floor(arg(a, 'sides', 0, 6) as number)) : 6
      const r = typeof arg(a, 'r', 1, 10) === 'number' ? arg(a, 'r', 1, 10) as number : 10
      const h = typeof arg(a, 'h', 2, 10) === 'number' ? arg(a, 'h', 2, 10) as number : 10
      const center = arg(a, 'center', 3, false) === true
      const { v, ix } = makePrism(sides, r, h, center)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'cone': {
      const r = typeof arg(a, 'r', 0, 10) === 'number' ? arg(a, 'r', 0, 10) as number : 10
      const h = typeof arg(a, 'h', 1, 20) === 'number' ? arg(a, 'h', 1, 20) as number : 20
      const center = arg(a, 'center', 2, false) === true
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 32) === 'number' ? arg(a, '$fn', -1, 32) as number : 32))
      const { v, ix } = makeCylinder(h, r, 0, center, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'capsule': {
      const r = typeof arg(a, 'r', 0, 5) === 'number' ? arg(a, 'r', 0, 5) as number : 5
      const h = typeof arg(a, 'h', 1, 20) === 'number' ? arg(a, 'h', 1, 20) as number : 20
      const center = arg(a, 'center', 2, false) === true
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 24) === 'number' ? arg(a, '$fn', -1, 24) as number : 24))
      const { v, ix } = makeCapsule(r, h, center, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'gear': {
      const teeth = typeof arg(a, 'teeth', 0, 12) === 'number' ? Math.max(3, Math.floor(arg(a, 'teeth', 0, 12) as number)) : 12
      const mod = typeof arg(a, 'mod', 1, 2) === 'number' ? arg(a, 'mod', 1, 2) as number : 2
      const thickness = typeof arg(a, 'thickness', 2, 5) === 'number' ? arg(a, 'thickness', 2, 5) as number : 5
      const fn = Math.min(MAX_FN, Math.max(1, typeof arg(a, '$fn', -1, 6) === 'number' ? arg(a, '$fn', -1, 6) as number : 6))
      const { v, ix } = makeGear(teeth, mod, thickness, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'radial_array': {
      const count = typeof arg(a, 'count', 0, 6) === 'number' ? Math.max(1, Math.floor(arg(a, 'count', 0, 6) as number)) : 6
      const r = typeof arg(a, 'r', 1, 20) === 'number' ? arg(a, 'r', 1, 20) as number : 20
      const out: MeshData[] = []
      for (let i = 0; i < count; i++) {
        const angle = (i * 360 / count) * Math.PI / 180
        let nt = rotateZ(tf, angle)
        nt = translate(nt, [r, 0, 0])
        out.push(...evalNodes(ch, nt, col, vars, modules, echos, callerChildren, annotations))
      }
      return out
    }
    case 'linear_array': {
      const count = typeof arg(a, 'count', 0, 5) === 'number' ? Math.max(1, Math.floor(arg(a, 'count', 0, 5) as number)) : 5
      let spacing = arg(a, 'spacing', 1, [12, 0, 0])
      if (!Array.isArray(spacing)) spacing = [typeof spacing === 'number' ? spacing : 12, 0, 0]
      const sx = typeof spacing[0] === 'number' ? spacing[0] : 0
      const sy = typeof spacing[1] === 'number' ? spacing[1] : 0
      const sz = typeof spacing[2] === 'number' ? spacing[2] : 0
      const out: MeshData[] = []
      for (let i = 0; i < count; i++) {
        const nt = translate(tf, [i * sx, i * sy, i * sz])
        out.push(...evalNodes(ch, nt, col, vars, modules, echos, callerChildren, annotations))
      }
      return out
    }
    case 'honeycomb': {
      const rows = typeof arg(a, 'rows', 0, 5) === 'number' ? Math.max(1, Math.floor(arg(a, 'rows', 0, 5) as number)) : 5
      const cols = typeof arg(a, 'cols', 1, 5) === 'number' ? Math.max(1, Math.floor(arg(a, 'cols', 1, 5) as number)) : 5
      const r = typeof arg(a, 'r', 2, 5) === 'number' ? arg(a, 'r', 2, 5) as number : 5
      const h = typeof arg(a, 'h', 3, 3) === 'number' ? arg(a, 'h', 3, 3) as number : 3
      const wall = typeof arg(a, 'wall', 4, 1) === 'number' ? arg(a, 'wall', 4, 1) as number : 1
      const { v, ix } = makeHoneycomb(rows, cols, r, h, wall)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'spring': {
      const r = typeof arg(a, 'r', 0, 10) === 'number' ? arg(a, 'r', 0, 10) as number : 10
      const wire_r = typeof arg(a, 'wire_r', 1, 1) === 'number' ? arg(a, 'wire_r', 1, 1) as number : 1
      const coils = typeof arg(a, 'coils', 2, 5) === 'number' ? arg(a, 'coils', 2, 5) as number : 5
      const pitch = typeof arg(a, 'pitch', 3, 3) === 'number' ? arg(a, 'pitch', 3, 3) as number : 3
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 16) === 'number' ? arg(a, '$fn', -1, 16) as number : 16))
      // Delegate to makeHelix with wire_r as the tube radius factor
      const helixResult = makeHelix(r, pitch, coils, fn)
      // Scale tube radius: makeHelix uses pitch * 0.15 as tube radius
      // We want wire_r instead, so scale all vertex positions relative to the helix center
      // Simpler: just use makeHelix directly; the wire radius is pitch * 0.15
      // For better control, let's re-generate with the correct tube radius
      const vArr: number[] = [], ixArr: number[] = []
      const tubeR = wire_r
      const safeCoils = Math.min(Math.max(coils, 0), 200) // clamp coils to prevent DoS
      const totalHeight = pitch * safeCoils
      const ringSegs = Math.min(20000, Math.max(16, fn * safeCoils))
      const tubeSegs = Math.max(6, Math.floor(fn / 4))
      for (let i = 0; i <= ringSegs; i++) {
        const t = i / ringSegs
        const angle = 2 * Math.PI * safeCoils * t
        const ca = Math.cos(angle), sa = Math.sin(angle)
        const cx2 = r * ca, cy2 = totalHeight * t, cz = r * sa
        const tx = -r * sa * 2 * Math.PI * safeCoils
        const ty = totalHeight
        const tz = r * ca * 2 * Math.PI * safeCoils
        const tlen = Math.sqrt(tx * tx + ty * ty + tz * tz) || 1
        const ttx = tx / tlen, tty = ty / tlen, ttz = tz / tlen
        let upx = 0, upy = 1, upz = 0
        if (Math.abs(tty) > 0.9) { upx = 1; upy = 0; upz = 0 }
        let bx = tty * upz - ttz * upy
        let by = ttz * upx - ttx * upz
        let bz = ttx * upy - tty * upx
        const blen = Math.sqrt(bx * bx + by * by + bz * bz) || 1
        bx /= blen; by /= blen; bz /= blen
        let nx = by * ttz - bz * tty
        let ny = bz * ttx - bx * ttz
        let nz = bx * tty - by * ttx
        const nlen = Math.sqrt(nx * nx + ny * ny + nz * nz) || 1
        nx /= nlen; ny /= nlen; nz /= nlen
        for (let j = 0; j <= tubeSegs; j++) {
          const phi = (2 * Math.PI * j) / tubeSegs
          const cp = Math.cos(phi), sp = Math.sin(phi)
          const px = cx2 + tubeR * (cp * nx + sp * bx)
          const py = cy2 + tubeR * (cp * ny + sp * by)
          const pz = cz + tubeR * (cp * nz + sp * bz)
          const snx = cp * nx + sp * bx
          const sny = cp * ny + sp * by
          const snz = cp * nz + sp * bz
          vArr.push(px, py, pz, snx, sny, snz)
        }
      }
      for (let i = 0; i < ringSegs; i++) {
        for (let j = 0; j < tubeSegs; j++) {
          const a2 = i * (tubeSegs + 1) + j
          const b = a2 + tubeSegs + 1
          ixArr.push(a2, b, a2 + 1, a2 + 1, b, b + 1)
        }
      }
      void helixResult // we built our own
      return [{ vertices: new Float32Array(vArr), indices: new Uint32Array(ixArr), color: col ?? nextC(), transform: tf }]
    }
    case 'knurl': {
      const d = typeof arg(a, 'd', 0, 10) === 'number' ? arg(a, 'd', 0, 10) as number : 10
      const h = typeof arg(a, 'h', 1, 10) === 'number' ? arg(a, 'h', 1, 10) as number : 10
      const pitch = typeof arg(a, 'pitch', 2, 1.5) === 'number' ? arg(a, 'pitch', 2, 1.5) as number : 1.5
      const depth = typeof arg(a, 'depth', 3, 0.5) === 'number' ? arg(a, 'depth', 3, 0.5) as number : 0.5
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 32) === 'number' ? arg(a, '$fn', -1, 32) as number : 32))
      const { v, ix } = makeKnurl(d, h, pitch, depth, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'chamfer_cube': {
      const sizeRaw = arg(a, 'size', 0, [10, 10, 10])
      let sx: number, sy: number, sz: number
      if (Array.isArray(sizeRaw)) {
        sx = typeof sizeRaw[0] === 'number' ? sizeRaw[0] : 10
        sy = typeof sizeRaw[1] === 'number' ? sizeRaw[1] : 10
        sz = typeof sizeRaw[2] === 'number' ? sizeRaw[2] : 10
      } else if (typeof sizeRaw === 'number') {
        sx = sy = sz = sizeRaw
      } else {
        sx = sy = sz = 10
      }
      const chamfer = typeof arg(a, 'chamfer', 1, 1) === 'number' ? arg(a, 'chamfer', 1, 1) as number : 1
      const center = !!arg(a, 'center', 2, false)
      const { v, ix } = makeChamferCube(sx, sy, sz, chamfer, center)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'loft': {
      const profilesRaw = arg(a, 'profiles', 0, [])
      const heightsRaw = arg(a, 'heights', 1, [])
      const fn = Math.min(MAX_FN, Math.max(3, typeof arg(a, '$fn', -1, 16) === 'number' ? arg(a, '$fn', -1, 16) as number : 16))
      if (!Array.isArray(profilesRaw) || profilesRaw.length < 2) return []
      const profiles: number[][][] = profilesRaw.map((p: any) => {
        if (!Array.isArray(p)) return []
        return p.map((pt: any) => Array.isArray(pt) ? [typeof pt[0] === 'number' ? pt[0] : 0, typeof pt[1] === 'number' ? pt[1] : 0] : [0, 0])
      })
      let heights: number[] = []
      if (Array.isArray(heightsRaw) && heightsRaw.length >= 2) {
        heights = heightsRaw.map((h: any) => typeof h === 'number' ? h : 0)
      } else {
        // Auto-generate heights: evenly spaced
        for (let i = 0; i < profiles.length; i++) {
          heights.push(i * 10)
        }
      }
      // Ensure heights array matches profiles length
      while (heights.length < profiles.length) heights.push(heights[heights.length - 1] + 10)
      const { v, ix } = makeLoft(profiles, heights, fn)
      if (!v.length) return []
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'trapezoid': {
      const top = arg(a, 'top', 0, 10) as number
      const bottom = arg(a, 'bottom', 1, 20) as number
      const h = arg(a, 'h', 2, 15) as number
      const depth = arg(a, 'depth', 3, 5) as number
      const { v, ix } = makeTrapezoid(top, bottom, h, depth)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'pyramid': {
      const base = arg(a, 'base', 0, 20) as number
      const h = arg(a, 'h', 1, 15) as number
      const sides = Math.max(3, Math.floor(arg(a, 'sides', 2, 4) as number))
      const center = arg(a, 'center', 3, false) === true
      const { v, ix } = makePyramid(base, h, sides, center)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'donut': {
      const r1 = arg(a, 'r1', 0, 15) as number
      const r2 = arg(a, 'r2', 1, 5) as number
      const angle = arg(a, 'angle', 2, 360) as number
      const fn = Math.min(MAX_FN, Math.max(8, arg(a, '$fn', -1, 32) as number))
      const { v, ix } = makeDonut(r1, r2, angle, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'spiral_extrude': {
      const r = typeof arg(a, 'r', 0, 10) === 'number' ? arg(a, 'r', 0, 10) as number : 10
      const pitch = typeof arg(a, 'pitch', 1, 5) === 'number' ? arg(a, 'pitch', 1, 5) as number : 5
      const turnsRaw = typeof arg(a, 'turns', 2, 3) === 'number' ? arg(a, 'turns', 2, 3) as number : 3
      const turns = Math.min(Math.max(turnsRaw, 0), 200) // clamp turns to prevent DoS
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 32) === 'number' ? arg(a, '$fn', -1, 32) as number : 32))

      const childMeshes = evalNodes(ch, identity(), null, vars, modules, echos, callerChildren, annotations)
      if (childMeshes.length === 0) return []

      const out: MeshData[] = []
      for (const childMesh of childMeshes) {
        const { pts, flat } = extractFlatProfile(childMesh)
        if (!flat || pts.length < 3) { out.push({ ...childMesh, transform: tf }); continue }

        // Build helical path
        const totalSteps = Math.min(20000, Math.max(16, Math.floor(fn * turns)))
        const path: number[][] = []
        for (let i = 0; i <= totalSteps; i++) {
          const angle = (2 * Math.PI * turns * i) / totalSteps
          const x = r * Math.cos(angle)
          const y = r * Math.sin(angle)
          const z = pitch * angle / (2 * Math.PI)
          path.push([x, y, z])
        }

        // Compute tangents
        const tangents: number[][] = []
        for (let i = 0; i < path.length; i++) {
          let tx: number, ty: number, tz: number
          if (i < path.length - 1) {
            tx = path[i+1][0] - path[i][0]
            ty = path[i+1][1] - path[i][1]
            tz = path[i+1][2] - path[i][2]
          } else {
            tx = tangents[i-1][0]; ty = tangents[i-1][1]; tz = tangents[i-1][2]
          }
          const tlen = Math.sqrt(tx*tx + ty*ty + tz*tz) || 1
          tangents.push([tx/tlen, ty/tlen, tz/tlen])
        }

        // Parallel transport frames
        const normals: number[][] = []
        const binormals: number[][] = []
        const t0 = tangents[0]
        let upx = 0, upy = 1, upz = 0
        if (Math.abs(t0[1]) > 0.9) { upx = 1; upy = 0; upz = 0 }
        let bx = t0[1]*upz - t0[2]*upy, by = t0[2]*upx - t0[0]*upz, bz = t0[0]*upy - t0[1]*upx
        let blen = Math.sqrt(bx*bx + by*by + bz*bz) || 1
        bx /= blen; by /= blen; bz /= blen
        let nx = by*t0[2] - bz*t0[1], ny = bz*t0[0] - bx*t0[2], nz = bx*t0[1] - by*t0[0]
        let nlen = Math.sqrt(nx*nx + ny*ny + nz*nz) || 1
        nx /= nlen; ny /= nlen; nz /= nlen
        normals.push([nx, ny, nz])
        binormals.push([bx, by, bz])

        for (let i = 1; i < path.length; i++) {
          const tPrev = tangents[i-1], tCur = tangents[i]
          const ax = tPrev[1]*tCur[2] - tPrev[2]*tCur[1]
          const ay = tPrev[2]*tCur[0] - tPrev[0]*tCur[2]
          const az = tPrev[0]*tCur[1] - tPrev[1]*tCur[0]
          const alen = Math.sqrt(ax*ax + ay*ay + az*az)
          if (alen < 1e-10) {
            normals.push([...normals[i-1]])
            binormals.push([...binormals[i-1]])
          } else {
            const dot = tPrev[0]*tCur[0] + tPrev[1]*tCur[1] + tPrev[2]*tCur[2]
            const angle = Math.acos(Math.max(-1, Math.min(1, dot)))
            const ux = ax/alen, uy = ay/alen, uz = az/alen
            const cosA = Math.cos(angle), sinA = Math.sin(angle)
            const pn = normals[i-1]
            const dotUN = ux*pn[0] + uy*pn[1] + uz*pn[2]
            const crossX = uy*pn[2] - uz*pn[1]
            const crossY = uz*pn[0] - ux*pn[2]
            const crossZ = ux*pn[1] - uy*pn[0]
            let rnx = pn[0]*cosA + crossX*sinA + ux*dotUN*(1-cosA)
            let rny = pn[1]*cosA + crossY*sinA + uy*dotUN*(1-cosA)
            let rnz = pn[2]*cosA + crossZ*sinA + uz*dotUN*(1-cosA)
            const rnlen = Math.sqrt(rnx*rnx + rny*rny + rnz*rnz) || 1
            rnx /= rnlen; rny /= rnlen; rnz /= rnlen
            normals.push([rnx, rny, rnz])
            const rbx = tCur[1]*rnz - tCur[2]*rny
            const rby = tCur[2]*rnx - tCur[0]*rnz
            const rbz = tCur[0]*rny - tCur[1]*rnx
            const rblen = Math.sqrt(rbx*rbx + rby*rby + rbz*rbz) || 1
            binormals.push([rbx/rblen, rby/rblen, rbz/rblen])
          }
        }

        // Center the profile around its centroid
        let cx = 0, cy = 0
        for (const p of pts) { cx += p[0]; cy += p[1] }
        cx /= pts.length; cy /= pts.length
        const centeredPts = pts.map(p => [p[0] - cx, p[1] - cy] as [number, number])

        // Generate vertices
        const v: number[] = [], ix: number[] = []
        const profileLen = centeredPts.length
        for (let i = 0; i < path.length; i++) {
          const cur = path[i]
          const n = normals[i], b = binormals[i]
          for (let j = 0; j < profileLen; j++) {
            const px = cur[0] + centeredPts[j][0]*n[0] + centeredPts[j][1]*b[0]
            const py = cur[1] + centeredPts[j][0]*n[1] + centeredPts[j][1]*b[1]
            const pz = cur[2] + centeredPts[j][0]*n[2] + centeredPts[j][1]*b[2]
            // Normal: direction from path center to surface point
            const dx = px - cur[0], dy = py - cur[1], dz = pz - cur[2]
            const dl = Math.sqrt(dx*dx + dy*dy + dz*dz) || 1
            v.push(px, py, pz, dx/dl, dy/dl, dz/dl)
          }
        }

        // Connect rings
        for (let i = 0; i < path.length - 1; i++) {
          for (let j = 0; j < profileLen; j++) {
            const j2 = (j + 1) % profileLen
            const a0 = i * profileLen + j
            const a1 = i * profileLen + j2
            const b0 = (i+1) * profileLen + j
            const b1 = (i+1) * profileLen + j2
            ix.push(a0, b0, a1, a1, b0, b1)
          }
        }

        if (v.length > 0) {
          out.push({ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? childMesh.color, transform: tf })
        }
      }
      return out
    }
    case 'lattice': {
      const type = typeof arg(a, 'type', 0, 'cubic') === 'string' ? arg(a, 'type', 0, 'cubic') as string : 'cubic'
      const cell = typeof arg(a, 'cell', 1, 5) === 'number' ? arg(a, 'cell', 1, 5) as number : 5
      const r = typeof arg(a, 'r', 2, 0.5) === 'number' ? arg(a, 'r', 2, 0.5) as number : 0.5
      const sizeRaw = arg(a, 'size', 3, [30, 30, 30])
      let lsx: number, lsy: number, lsz: number
      if (Array.isArray(sizeRaw)) {
        lsx = typeof sizeRaw[0] === 'number' ? sizeRaw[0] : 30
        lsy = typeof sizeRaw[1] === 'number' ? sizeRaw[1] : 30
        lsz = typeof sizeRaw[2] === 'number' ? sizeRaw[2] : 30
      } else if (typeof sizeRaw === 'number') {
        lsx = lsy = lsz = sizeRaw
      } else {
        lsx = lsy = lsz = 30
      }
      const fn = Math.min(MAX_FN, Math.max(4, typeof arg(a, '$fn', -1, 6) === 'number' ? arg(a, '$fn', -1, 6) as number : 6))
      const { v, ix } = makeLattice(type, cell, r, [lsx, lsy, lsz], fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'slot': {
      const length = typeof arg(a, 'length', 0, 20) === 'number' ? arg(a, 'length', 0, 20) as number : 20
      const width = typeof arg(a, 'width', 1, 5) === 'number' ? arg(a, 'width', 1, 5) as number : 5
      const h = typeof arg(a, 'h', 2, 3) === 'number' ? arg(a, 'h', 2, 3) as number : 3
      const center = arg(a, 'center', 3, false) === true
      const { v, ix } = makeSlot(length, width, h, center)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'cross': {
      const sizeRaw = arg(a, 'size', 0, [20, 20, 10])
      let csx: number, csy: number, csz: number
      if (Array.isArray(sizeRaw)) {
        csx = typeof sizeRaw[0] === 'number' ? sizeRaw[0] : 20
        csy = typeof sizeRaw[1] === 'number' ? sizeRaw[1] : 20
        csz = typeof sizeRaw[2] === 'number' ? sizeRaw[2] : 10
      } else if (typeof sizeRaw === 'number') {
        csx = csy = csz = sizeRaw
      } else {
        csx = csy = csz = 20
      }
      const arm = typeof arg(a, 'arm', 1, 5) === 'number' ? arg(a, 'arm', 1, 5) as number : 5
      const { v, ix } = makeCross([csx, csy, csz], arm)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'maze': {
      const rows = typeof arg(a, 'rows', 0, 5) === 'number' ? Math.max(1, Math.floor(arg(a, 'rows', 0, 5) as number)) : 5
      const cols = typeof arg(a, 'cols', 1, 5) === 'number' ? Math.max(1, Math.floor(arg(a, 'cols', 1, 5) as number)) : 5
      const cell = typeof arg(a, 'cell', 2, 5) === 'number' ? arg(a, 'cell', 2, 5) as number : 5
      const wall = typeof arg(a, 'wall', 3, 1) === 'number' ? arg(a, 'wall', 3, 1) as number : 1
      const h = typeof arg(a, 'h', 4, 3) === 'number' ? arg(a, 'h', 4, 3) as number : 3
      const { v, ix } = makeMaze(rows, cols, cell, wall, h)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'fibonacci_sphere': {
      const count = typeof arg(a, 'count', 0, 50) === 'number' ? Math.min(5000, Math.max(2, Math.floor(arg(a, 'count', 0, 50) as number))) : 50
      const r = typeof arg(a, 'r', 1, 10) === 'number' ? arg(a, 'r', 1, 10) as number : 10
      const fn = Math.min(MAX_FN, Math.max(4, typeof arg(a, '$fn', -1, 8) === 'number' ? arg(a, '$fn', -1, 8) as number : 8))
      const { v, ix } = makeFibonacciSphere(count, r, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'iso_triangle': {
      const size = typeof arg(a, 'size', 0, 10) === 'number' ? arg(a, 'size', 0, 10) as number : 10
      const h = typeof arg(a, 'h', 1, 5) === 'number' ? arg(a, 'h', 1, 5) as number : 5
      const center = arg(a, 'center', 2, false) === true
      // Equilateral triangle prism = prism with 3 sides
      // For equilateral triangle with side length = size, circumradius = size / sqrt(3)
      const r = size / Math.sqrt(3)
      const { v, ix } = makePrism(3, r, h, center)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'ellipsoid': {
      const rx = typeof arg(a, 'rx', 0, 10) === 'number' ? arg(a, 'rx', 0, 10) as number : 10
      const ry = typeof arg(a, 'ry', 1, 8) === 'number' ? arg(a, 'ry', 1, 8) as number : 8
      const rz = typeof arg(a, 'rz', 2, 5) === 'number' ? arg(a, 'rz', 2, 5) as number : 5
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 24) === 'number' ? arg(a, '$fn', -1, 24) as number : 24))
      const { v, ix } = makeEllipsoid(rx, ry, rz, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'hemisphere': {
      let r = arg(a, 'r', 0, undefined)
      const d = arg(a, 'd', -1, undefined)
      if (r == null) r = d != null ? d / 2 : 10
      if (typeof r !== 'number') r = 10
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 24) === 'number' ? arg(a, '$fn', -1, 24) as number : 24))
      const { v, ix } = makeHemisphere(r, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'fillet': {
      const r = arg(a, 'r', 0, 2)
      echos.push('NOTE: fillet(r=' + r + ') is visual-only, applied as pass-through')
      return evalNodes(ch, tf, col, vars, modules, echos, callerChildren, annotations)
    }
    case 'chamfer': {
      const r = arg(a, 'r', 0, 1)
      echos.push('NOTE: chamfer(r=' + r + ') is visual-only, applied as pass-through')
      return evalNodes(ch, tf, col, vars, modules, echos, callerChildren, annotations)
    }
    case 'ogive': {
      const r = typeof arg(a, 'r', 0, 10) === 'number' ? arg(a, 'r', 0, 10) as number : 10
      const h = typeof arg(a, 'h', 1, 20) === 'number' ? arg(a, 'h', 1, 20) as number : 20
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 32) === 'number' ? arg(a, '$fn', -1, 32) as number : 32))
      const { v, ix } = makeOgive(r, h, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'teardrop': {
      const r = typeof arg(a, 'r', 0, 5) === 'number' ? arg(a, 'r', 0, 5) as number : 5
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 24) === 'number' ? arg(a, '$fn', -1, 24) as number : 24))
      const { v, ix } = makeTeardrop(r, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'ring': {
      const r1 = typeof arg(a, 'r1', 0, 15) === 'number' ? arg(a, 'r1', 0, 15) as number : 15
      const r2 = typeof arg(a, 'r2', 1, 12) === 'number' ? arg(a, 'r2', 1, 12) as number : 12
      const h = typeof arg(a, 'h', 2, 5) === 'number' ? arg(a, 'h', 2, 5) as number : 5
      const center = arg(a, 'center', -1, false) === true
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 32) === 'number' ? arg(a, '$fn', -1, 32) as number : 32))
      const { v, ix } = makePipe(h, r1, r2, center, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'tube': {
      const r = typeof arg(a, 'r', 0, 10) === 'number' ? arg(a, 'r', 0, 10) as number : 10
      const wall = typeof arg(a, 'wall', 1, 1) === 'number' ? arg(a, 'wall', 1, 1) as number : 1
      const h = typeof arg(a, 'h', 2, 20) === 'number' ? arg(a, 'h', 2, 20) as number : 20
      const center = arg(a, 'center', -1, false) === true
      const fn = Math.min(MAX_FN, Math.max(8, typeof arg(a, '$fn', -1, 32) === 'number' ? arg(a, '$fn', -1, 32) as number : 32))
      const innerR = Math.max(0, r - wall)
      const { v, ix } = makePipe(h, r, innerR, center, fn)
      return [{ vertices: new Float32Array(v), indices: new Uint32Array(ix), color: col ?? nextC(), transform: tf }]
    }
    case 'mirror_copy': {
      const raw = arg(a, 'v', 0, [1, 0, 0])
      const original = evalNodes(ch, tf, col, vars, modules, echos, callerChildren, annotations)
      if (Array.isArray(raw)) {
        const sv: Vec3 = [raw[0] ? -1 : 1, raw[1] ? -1 : 1, raw[2] ? -1 : 1]
        const mirrored = evalNodes(ch, scale(tf, sv), col, vars, modules, echos, callerChildren, annotations)
        return [...original, ...mirrored]
      }
      return original
    }
    case 'distribute': {
      const count = typeof arg(a, 'count', 0, 5) === 'number' ? Math.max(1, Math.floor(arg(a, 'count', 0, 5) as number)) : 5
      const spacing = typeof arg(a, 'spacing', 1, 15) === 'number' ? arg(a, 'spacing', 1, 15) as number : 15
      const out: MeshData[] = []
      for (let i = 0; i < count; i++) {
        const nt = translate(tf, [i * spacing, 0, 0])
        out.push(...evalNodes(ch, nt, col, vars, modules, echos, callerChildren, annotations))
      }
      return out
    }
    case 'minkowski': case 'render': case 'group':
      return evalNodes(ch, tf, col, vars, modules, echos, callerChildren, annotations)
    case 'module': case 'function': case '__assign':
      return []
    default: {
      // Check if it's a user-defined module call
      if (modules.has(nm)) {
        const mod = modules.get(nm)!
        const modVars: Record<string, number> = { ...vars }

        // Map arguments to module parameters
        for (let i = 0; i < mod.params.length; i++) {
          const param = mod.params[i]
          // Check named arg first, then positional
          const val = a[param.name] ?? a[`_${i}`]
          if (val !== undefined) {
            if (typeof val === 'number') modVars[param.name] = val
          } else if (param.defaultVal !== undefined) {
            const resolved = resolveArg(param.defaultVal, modVars)
            if (typeof resolved === 'number') modVars[param.name] = resolved
          }
        }

        // Pass the caller's children (the node's children) as callerChildren for children() support
        return evalNodes(mod.children, tf, col, modVars, modules, echos, ch.length > 0 ? ch : callerChildren, annotations)
      }

      // Unknown: evaluate children as pass-through
      return evalNodes(ch, tf, col, vars, modules, echos, callerChildren, annotations)
    }
  }
}

function axisAngle(m: Mat4, axis: Vec3, angle: number): Mat4 {
  const l = Math.sqrt(axis[0]**2 + axis[1]**2 + axis[2]**2)
  if (l < 1e-10) return m
  const x = axis[0]/l, y = axis[1]/l, z = axis[2]/l
  const c = Math.cos(angle), s = Math.sin(angle), t = 1-c
  const r = identity()
  r[0] = t*x*x+c;   r[1] = t*x*y-s*z; r[2] = t*x*z+s*y
  r[4] = t*x*y+s*z;  r[5] = t*y*y+c;   r[6] = t*y*z-s*x
  r[8] = t*x*z-s*y;  r[9] = t*y*z+s*x; r[10] = t*z*z+c
  return multiply(r, m)
}

const CSS_COLORS: Record<string,[number,number,number,number]> = {
  red:[1,0,0,1], green:[0,.5,0,1], blue:[0,0,1,1], yellow:[1,1,0,1],
  cyan:[0,1,1,1], magenta:[1,0,1,1], white:[1,1,1,1], black:[0,0,0,1],
  orange:[1,.65,0,1], gray:[.5,.5,.5,1], grey:[.5,.5,.5,1],
  pink:[1,.75,.8,1], purple:[.5,0,.5,1], brown:[.65,.16,.16,1],
  lime:[0,1,0,1], navy:[0,0,.5,1], teal:[0,.5,.5,1], maroon:[.5,0,0,1],
  olive:[.5,.5,0,1], silver:[.75,.75,.75,1], aqua:[0,1,1,1],
}
function cssColor(name: string): [number,number,number,number] {
  return CSS_COLORS[name.toLowerCase()] ?? [.5,.5,.5,1]
}

/* ── Public API ───────────────────────────────────── */

export interface Annotation {
  text: string
  position: [number, number, number]
}

export interface ProfileEntry {
  name: string
  line: number
  timeMs: number
}

export interface ParseResult {
  meshes: MeshData[]
  ast: ASTNode[]
  echos: string[]
  errors: string[]
  annotations: Annotation[]
  profileEntries?: ProfileEntry[]
}

export function parseOpenSCADWithAST(source: string, resolveFile?: (name: string) => string | null): ParseResult {
  cIdx = 0
  _resolveFile = resolveFile || null
  _profiling = true
  _profileEntries = []
  _profileDepth = 0
  _source = source
  const echos: string[] = []
  const errors: string[] = []
  const annotations: Annotation[] = []
  let ast: ASTNode[] = []
  let meshes: MeshData[] = []
  try {
    const tokens = tokenize(source)
    const parser = new Parser(tokens)
    ast = parser.parseAll()
  } catch (e: any) {
    errors.push(e.message || String(e))
  }
  try {
    meshes = evalNodes(ast, identity(), null, {}, new Map(), echos, undefined, annotations)
  } catch (e: any) {
    errors.push(e.message || String(e))
  }
  return { meshes, ast, echos, errors, annotations, profileEntries: _profileEntries }
}

export function parseOpenSCAD(source: string): MeshData[] {
  cIdx = 0
  _resolveFile = null
  const tokens = tokenize(source)
  const parser = new Parser(tokens)
  const ast = parser.parseAll()
  return evalNodes(ast, identity(), null, {}, new Map(), [], undefined, [])
}
