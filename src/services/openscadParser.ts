/**
 * OpenSCAD subset parser + mesh generator.
 * Supports: cube, sphere, cylinder, translate, rotate, scale, color,
 *           mirror, multmatrix, union, difference, intersection
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

/* ── AST ──────────────────────────────────────────── */

interface ASTNode {
  type: 'call'
  name: string
  args: Record<string, any>
  children: ASTNode[]
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
    const name = this.expect(TT.Ident).v
    const args: Record<string, any> = {}

    if (this.peek().t === TT.Eq && name !== 'module' && name !== 'function') {
      this.adv()
      this.skipExpr()
      this.match(TT.Semi)
      return { type: 'call', name: '__assign', args: {}, children: [] }
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
    return { type: 'call', name, args, children }
  }

  private parseArgs(args: Record<string, any>) {
    let pi = 0
    while (this.peek().t !== TT.RParen && this.peek().t !== TT.Eof) {
      if (this.peek().t === TT.Ident && this.tok[this.pos + 1]?.t === TT.Eq) {
        const key = this.adv().v; this.adv()
        args[key] = this.parseValue()
      } else {
        args[`_${pi++}`] = this.parseValue()
      }
      this.match(TT.Comma)
    }
  }

  private parseValue(): any {
    if (this.peek().t === TT.Minus) {
      this.adv(); const v = this.parseValue()
      return typeof v === 'number' ? -v : v
    }
    if (this.peek().t === TT.Num) return parseFloat(this.adv().v)
    if (this.peek().t === TT.Str) return this.adv().v
    if (this.peek().t === TT.Ident) {
      const v = this.peek().v
      if (v === 'true') { this.adv(); return true }
      if (v === 'false') { this.adv(); return false }
      if (v === 'undef') { this.adv(); return undefined }
      this.adv()
      if (this.peek().t === TT.LParen) {
        this.adv(); let depth = 1
        while (depth > 0 && this.peek().t !== TT.Eof) {
          if (this.peek().t === TT.LParen) depth++
          if (this.peek().t === TT.RParen) depth--
          if (depth > 0) this.adv()
        }
        this.match(TT.RParen)
      }
      return v
    }
    if (this.peek().t === TT.LBracket) return this.parseVec()
    this.adv(); return 0
  }

  private parseVec(): any[] {
    this.expect(TT.LBracket)
    const vals: any[] = []
    while (this.peek().t !== TT.RBracket && this.peek().t !== TT.Eof) {
      vals.push(this.parseValue()); this.match(TT.Comma)
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

function evalNodes(nodes: ASTNode[], tf: Mat4, col: [number,number,number,number]|null): MeshData[] {
  const out: MeshData[] = []
  for (const n of nodes) out.push(...evalNode(n, tf, col))
  return out
}

function evalNode(node: ASTNode, tf: Mat4, col: [number,number,number,number]|null): MeshData[] {
  const { name: nm, args: a, children: ch } = node

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
    case 'translate': {
      const raw = arg(a,'v',0,[0,0,0])
      const vec: Vec3 = Array.isArray(raw) ? [raw[0]??0, raw[1]??0, raw[2]??0] : [0,0,0]
      return evalNodes(ch, translate(tf, vec), col)
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
      return evalNodes(ch, nt, col)
    }
    case 'scale': {
      const raw = arg(a,'v',0,[1,1,1])
      const vec: Vec3 = Array.isArray(raw) ? [raw[0]??1,raw[1]??1,raw[2]??1] : [raw,raw,raw]
      return evalNodes(ch, scale(tf, vec), col)
    }
    case 'mirror': {
      const raw = arg(a,'v',0,[1,0,0])
      if (Array.isArray(raw)) {
        const sv: Vec3 = [raw[0]?-1:1, raw[1]?-1:1, raw[2]?-1:1]
        return evalNodes(ch, scale(tf, sv), col)
      }
      return evalNodes(ch, tf, col)
    }
    case 'color': {
      const c = arg(a,'c',0,arg(a,'_0',0,[0.5,0.5,0.5]))
      let nc: [number,number,number,number]
      if (Array.isArray(c)) nc = [c[0]??0.5, c[1]??0.5, c[2]??0.5, c[3]??1]
      else if (typeof c === 'string') nc = cssColor(c)
      else nc = [0.5,0.5,0.5,1]
      return evalNodes(ch, tf, nc)
    }
    case 'union':
      return evalNodes(ch, tf, col)
    case 'difference': {
      const out: MeshData[] = []
      if (ch.length > 0) out.push(...evalNode(ch[0], tf, col))
      for (let i = 1; i < ch.length; i++)
        out.push(...evalNode(ch[i], tf, [0.9, 0.15, 0.15, 0.35]))
      return out
    }
    case 'intersection':
      return evalNodes(ch, tf, col ? [col[0],col[1],col[2],0.55] : null)
    case 'multmatrix': {
      const m = arg(a,'m',0,undefined)
      if (Array.isArray(m) && m.length >= 4) {
        const mat = new Float32Array(16)
        for (let r = 0; r < 4; r++) for (let c = 0; c < 4; c++) mat[r*4+c] = m[r]?.[c] ?? (r===c?1:0)
        return evalNodes(ch, multiply(mat, tf), col)
      }
      return evalNodes(ch, tf, col)
    }
    case 'hull': case 'minkowski': case 'linear_extrude': case 'rotate_extrude':
    case 'projection': case 'import': case 'render': case 'group':
      return evalNodes(ch, tf, col)
    case 'module': case 'function': case '__assign':
      return []
    default:
      return evalNodes(ch, tf, col)
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

export function parseOpenSCAD(source: string): MeshData[] {
  cIdx = 0
  const tokens = tokenize(source)
  const parser = new Parser(tokens)
  const ast = parser.parseAll()
  return evalNodes(ast, identity(), null)
}
