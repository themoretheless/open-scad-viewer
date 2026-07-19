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
import { MAX_FN } from '../parser/limits'
import {
  makeText, makeCube, makeSphere, makeCylinder, makePipe, makeWedge, makeTorus, makeHelix,
  makeBezier, makeSweep, makeStar, makePrism, makeCapsule, makeGear, makeThread, pointInTriangle,
  earClip, makePolygon, makeHoneycomb, makeKnurl, makeChamferCube, makeLoft, makeSurface, extractFlatProfile,
  extrudeMesh, rotateExtrudeMesh, computeHullFaceNormal, convexHull3D, makePolyhedron, makeTrapezoid, makePyramid, makeDonut,
  makeLattice, makeSlot, makeCross, makeMaze, makeFibonacciSphere, makeEllipsoid, makeHemisphere, makeOgive,
  makeTeardrop,
} from '../parser/geometry'

/* ── Limits ───────────────────────────────────────── */


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
      const teeth = typeof arg(a, 'teeth', 0, 12) === 'number' ? Math.min(500, Math.max(3, Math.floor(arg(a, 'teeth', 0, 12) as number))) : 12
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
