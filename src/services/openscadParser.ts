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
  ln: Math.log,
  log: Math.log,
  exp: Math.exp,
  sign: Math.sign,
  rands: (minv: number, maxv: number, count: number) => {
    // Returns a single value for simplicity (OpenSCAD returns vector)
    void count
    return minv + Math.random() * (maxv - minv)
  },
  len: (x: number) => {
    // For arrays this would be different; for numbers return 0
    void x
    return 0
  },
  norm: (x: number) => Math.abs(x),
  cross: () => 0, // placeholder
  lookup: () => 0, // placeholder
  str: () => 0, // placeholder
  chr: () => 0, // placeholder
  concat: () => 0, // placeholder
}

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
        else if (op === '/') left = right !== 0 ? left / right : 0
        else left = right !== 0 ? left % right : 0
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
        if (name in MATH_FUNCS) {
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
    const vals: any[] = []
    while (this.peek().t !== TT.RBracket && this.peek().t !== TT.Eof) {
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

function pointInTriangle(px: number, py: number, ax: number, ay: number, bx: number, by: number, cx: number, cy: number): boolean {
  const d1 = (px - bx) * (ay - by) - (ax - bx) * (py - by)
  const d2 = (px - cx) * (by - cy) - (bx - cx) * (py - cy)
  const d3 = (px - ax) * (cy - ay) - (cx - ax) * (py - ay)
  const hasNeg = (d1 < 0) || (d2 < 0) || (d3 < 0)
  const hasPos = (d1 > 0) || (d2 > 0) || (d3 > 0)
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

  let attempts = 0
  while (remaining.length > 2 && attempts < remaining.length * 2) {
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
      const isConvex = ccw ? cross > 0 : cross < 0
      if (!isConvex) continue

      // Check no other point is inside this triangle
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
      attempts = 0
      break
    }
    if (!found) attempts++
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

/* ── Expression evaluator ─────────────────────────── */

function evalComparison(op: string, left: any, right: any): boolean {
  const l = typeof left === 'number' ? left : 0
  const r = typeof right === 'number' ? right : 0
  switch (op) {
    case '<': return l < r
    case '>': return l > r
    case '<=': return l <= r
    case '>=': return l >= r
    case '==': return l === r
    case '!=': return l !== r
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

      const l = typeof left === 'number' ? left : 0
      const r = typeof right === 'number' ? right : 0

      switch (op) {
        case '+': return l + r
        case '-': return l - r
        case '*': return l * r
        case '/': return r !== 0 ? l / r : 0
        case '%': return r !== 0 ? l % r : 0
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
function nextC(): [number,number,number,number] { return PALETTE[(cIdx++) % PALETTE.length] }

function arg(a: Record<string,any>, name: string, pos: number, def: any): any {
  return a[name] ?? a[`_${pos}`] ?? def
}

/** Resolve a parsed arg value: evaluate expressions, substitute variables */
function resolveArg(val: any, vars: Record<string, number>): any {
  if (isExpr(val)) return evalExprNode(val, vars)
  if (typeof val === 'string' && val in vars) return vars[val]
  if (Array.isArray(val)) return val.map(v => resolveArg(v, vars))
  if (val && typeof val === 'object' && val.__range) {
    return { __range: true, start: resolveArg(val.start, vars), step: resolveArg(val.step, vars), end: resolveArg(val.end, vars) }
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
  if (step === 0) return []
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

/* ── Main evaluator ───────────────────────────────── */

function evalNodes(nodes: ASTNode[], tf: Mat4, col: [number,number,number,number]|null, vars: Record<string, number> = {}, modules: Map<string, ModuleDef> = new Map()): MeshData[] {
  const out: MeshData[] = []
  let lastIfResult = false
  for (let ni = 0; ni < nodes.length; ni++) {
    const n = nodes[ni]

    // Handle variable assignments
    if (n.name === '__assign') {
      const varName = n.args.__varName
      const varValue = n.args.__varValue
      if (typeof varName === 'string') {
        const resolved = resolveArg(varValue, vars)
        if (typeof resolved === 'number') {
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

    if (n.name === 'else') {
      // Only evaluate else children if the previous if was false
      if (!lastIfResult) {
        out.push(...evalNodes(n.children, tf, col, vars, modules))
      }
      continue
    }
    if (n.name === 'if') {
      const ra = resolveArgs(n.args, vars)
      const cond = arg(ra, '_0', 0, 0)
      lastIfResult = evalCondition(cond)
      if (lastIfResult) {
        out.push(...evalNodes(n.children, tf, col, vars, modules))
      }
      continue
    }
    lastIfResult = false
    out.push(...evalNode(n, tf, col, vars, modules))
  }
  return out
}

function evalNode(node: ASTNode, tf: Mat4, col: [number,number,number,number]|null, vars: Record<string, number> = {}, modules: Map<string, ModuleDef> = new Map()): MeshData[] {
  const { name: nm, args: rawArgs, children: ch } = node
  const a = resolveArgs(rawArgs, vars)

  switch (nm) {
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
      const fn = Math.max(8, arg(a,'$fn',-1,24))
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
      const fn = Math.max(8, arg(a,'$fn',-1,24))
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
      const fn = Math.max(8, arg(a, '$fn', -1, 24))
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
    case 'for': {
      // for(i = [0:5]) or for(i = [0:2:10]) or for(i = [1,3,7])
      const out: MeshData[] = []
      for (const [varName, rawVal] of Object.entries(a)) {
        if (varName.startsWith('_')) continue // skip positional
        const val = rawVal
        let iterValues: number[] = []
        if (val && typeof val === 'object' && val.__range) {
          iterValues = expandRange(val)
        } else if (Array.isArray(val)) {
          iterValues = val.filter((v): v is number => typeof v === 'number')
        } else if (typeof val === 'number') {
          iterValues = [val]
        }
        for (const iterVal of iterValues) {
          const newVars = { ...vars, [varName]: iterVal }
          out.push(...evalNodes(ch, tf, col, newVars, modules))
        }
        return out // only support one loop variable per for()
      }
      return evalNodes(ch, tf, col, vars, modules)
    }
    case 'let': {
      const newVars = { ...vars }
      for (const [k, v] of Object.entries(a)) {
        if (k.startsWith('_')) continue
        if (typeof v === 'number') newVars[k] = v
        else if (typeof v === 'string' && v in vars) newVars[k] = vars[v]
      }
      return evalNodes(ch, tf, col, newVars, modules)
    }
    case 'if': {
      const cond = arg(a, '_0', 0, 0)
      if (evalCondition(cond)) {
        return evalNodes(ch, tf, col, vars, modules)
      }
      return []
    }
    case 'else': {
      return evalNodes(ch, tf, col, vars, modules)
    }
    case 'translate': {
      const raw = arg(a,'v',0,[0,0,0])
      const vec: Vec3 = Array.isArray(raw) ? [raw[0]??0, raw[1]??0, raw[2]??0] : [0,0,0]
      return evalNodes(ch, translate(tf, vec), col, vars, modules)
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
      return evalNodes(ch, nt, col, vars, modules)
    }
    case 'scale': {
      const raw = arg(a,'v',0,[1,1,1])
      const vec: Vec3 = Array.isArray(raw) ? [raw[0]??1,raw[1]??1,raw[2]??1] : [raw,raw,raw]
      return evalNodes(ch, scale(tf, vec), col, vars, modules)
    }
    case 'mirror': {
      const raw = arg(a,'v',0,[1,0,0])
      if (Array.isArray(raw)) {
        const sv: Vec3 = [raw[0]?-1:1, raw[1]?-1:1, raw[2]?-1:1]
        return evalNodes(ch, scale(tf, sv), col, vars, modules)
      }
      return evalNodes(ch, tf, col, vars, modules)
    }
    case 'color': {
      const c = arg(a,'c',0,arg(a,'_0',0,[0.5,0.5,0.5]))
      let nc: [number,number,number,number]
      if (Array.isArray(c)) nc = [c[0]??0.5, c[1]??0.5, c[2]??0.5, c[3]??1]
      else if (typeof c === 'string') nc = cssColor(c)
      else nc = [0.5,0.5,0.5,1]
      return evalNodes(ch, tf, nc, vars, modules)
    }
    case 'union':
      return evalNodes(ch, tf, col, vars, modules)
    case 'difference': {
      const out: MeshData[] = []
      if (ch.length > 0) out.push(...evalNode(ch[0], tf, col, vars, modules))
      for (let i = 1; i < ch.length; i++)
        out.push(...evalNode(ch[i], tf, [0.9, 0.15, 0.15, 0.35], vars, modules))
      return out
    }
    case 'intersection':
      return evalNodes(ch, tf, col ? [col[0],col[1],col[2],0.55] : null, vars, modules)
    case 'multmatrix': {
      const m = arg(a,'m',0,undefined)
      if (Array.isArray(m) && m.length >= 4) {
        const mat = new Float32Array(16)
        for (let r = 0; r < 4; r++) for (let c = 0; c < 4; c++) mat[r*4+c] = m[r]?.[c] ?? (r===c?1:0)
        return evalNodes(ch, multiply(mat, tf), col, vars, modules)
      }
      return evalNodes(ch, tf, col, vars, modules)
    }
    case 'linear_extrude': {
      const height = typeof arg(a, 'height', 0, 10) === 'number' ? arg(a, 'height', 0, 10) as number : 10
      const twist = typeof arg(a, 'twist', -1, 0) === 'number' ? arg(a, 'twist', -1, 0) as number : 0
      const slices = typeof arg(a, 'slices', -1, 1) === 'number' ? arg(a, 'slices', -1, 1) as number : 1

      // Evaluate children to get flat meshes
      const childMeshes = evalNodes(ch, identity(), null, vars, modules)

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
      const fn = Math.max(3, typeof arg(a, '$fn', -1, 36) === 'number' ? arg(a, '$fn', -1, 36) as number : 36)
      const angle = typeof arg(a, 'angle', -1, 360) === 'number' ? arg(a, 'angle', -1, 360) as number : 360

      // Evaluate children to get flat meshes
      const childMeshes = evalNodes(ch, identity(), null, vars, modules)

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
      const childMeshes = evalNodes(ch, identity(), null, vars, modules)

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
    case 'minkowski': case 'projection': case 'import': case 'render': case 'group':
      return evalNodes(ch, tf, col, vars, modules)
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

        return evalNodes(mod.children, tf, col, modVars, modules)
      }

      // Unknown: evaluate children as pass-through
      return evalNodes(ch, tf, col, vars, modules)
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

export interface ParseResult {
  meshes: MeshData[]
  ast: ASTNode[]
}

export function parseOpenSCADWithAST(source: string): ParseResult {
  cIdx = 0
  const tokens = tokenize(source)
  const parser = new Parser(tokens)
  const ast = parser.parseAll()
  const meshes = evalNodes(ast, identity(), null)
  return { meshes, ast }
}

export function parseOpenSCAD(source: string): MeshData[] {
  cIdx = 0
  const tokens = tokenize(source)
  const parser = new Parser(tokens)
  const ast = parser.parseAll()
  return evalNodes(ast, identity(), null)
}
