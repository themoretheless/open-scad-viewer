/**
 * # Svet Fine-Grained Reactive Signals & Compiled Forma AST Engine (`svet.ts`)
 */
export interface ReadonlySignal<T> { (): T; readonly id: number; readonly __svet_sig: true; peek(): T; dispose(): void }
export interface Signal<T> extends ReadonlySignal<T> { set(value: T | ((prev: T) => T)): void; update(updater: (prev: T) => T): void }
export interface EffectHandle { readonly id: number; run(): void; dispose(): void }
export interface OwnerScope { disposables: Array<() => void>; children: Set<OwnerScope> | null; parent: OwnerScope | null; disposed: boolean }

const STATE_CLEAN = 0, STATE_CHECK = 1, STATE_DIRTY = 2

interface ObserverNode {
  readonly kind: 'computed' | 'effect'; readonly id: number; state: number
  deps: Set<SignalNode<any> | ComputedNode<any>>; cleanups: Array<() => void>; disposed: boolean; run(): void
}
interface SignalNode<T> {
  readonly kind: 'signal'; readonly id: number; value: T; version: number; subs: Set<ObserverNode>; disposed: boolean
}
interface ComputedNode<T> extends ObserverNode {
  readonly kind: 'computed'; value: T; version: number; initialized: boolean; computing: boolean
  fn: () => T; equals: (a: T, b: T) => boolean; subs: Set<ObserverNode>
}
interface EffectNode extends ObserverNode {
  readonly kind: 'effect'; fn: (onCleanupFn: (cb: () => void) => void) => void | (() => void)
  owner: OwnerScope | null; queued: boolean; running: boolean; sync: boolean; scheduler?: () => void
}

let nextNodeId = 1
const freeIds: number[] = []
let activeObserver: ObserverNode | null = null
let trackingEnabled = true
let currentOwner: OwnerScope | null = null
let batchDepth = 0
const pendingEffects: EffectNode[] = []
let flushing = false

const allocNodeId = (): number => freeIds.pop() ?? nextNodeId++
const releaseNodeId = (id: number): void => { freeIds.push(id) }

function clearObserverDeps(observer: ObserverNode): void {
  for (const dep of observer.deps) dep.subs.delete(observer)
  observer.deps.clear()
}

function runObserverCleanups(observer: ObserverNode): void {
  if (observer.cleanups.length === 0) return
  const list = observer.cleanups.splice(0)
  for (let i = list.length - 1; i >= 0; i--) {
    try { list[i]() } catch { /* continue */ }
  }
}

function markSubscribers(subs: Set<ObserverNode>, state: number): void {
  const list = Array.from(subs)
  for (let i = 0; i < list.length; i++) {
    const sub = list[i]
    if (sub.disposed) continue
    if (sub === activeObserver) continue
    const prevState = sub.state
    if (prevState < state) {
      sub.state = state
    }
    if (sub.kind === 'computed') {
      markSubscribers((sub as ComputedNode<any>).subs, STATE_CHECK)
    } else {
      const eff = sub as EffectNode
      if (!eff.queued) {
        eff.queued = true
        pendingEffects.push(eff)
      }
    }
  }
}

function ensureComputedUpToDate(node: ComputedNode<any>): void {
  if (node.disposed) return
  if (!node.initialized || node.state === STATE_DIRTY) {
    recomputeNode(node)
    return
  }
  if (node.state === STATE_CHECK) {
    for (const dep of node.deps) {
      if (dep.kind === 'computed') {
        ensureComputedUpToDate(dep)
        if ((node.state as number) === STATE_DIRTY) {
          recomputeNode(node)
          return
        }
      }
    }
    node.state = STATE_CLEAN
  }
}

function isEffectDirty(eff: EffectNode): boolean {
  if (eff.state === STATE_DIRTY) return true
  if (eff.state === STATE_CHECK) {
    for (const dep of eff.deps) {
      if (dep.kind === 'computed') {
        ensureComputedUpToDate(dep)
        if ((eff.state as number) === STATE_DIRTY) return true
      }
    }
    eff.state = STATE_CLEAN
  }
  return false
}

function recomputeNode<T>(node: ComputedNode<T>): void {
  if (node.computing) return
  node.computing = true
  const prevObserver = activeObserver
  const prevTracking = trackingEnabled
  clearObserverDeps(node)
  runObserverCleanups(node)
  activeObserver = node
  trackingEnabled = true
  try {
    const nextVal = node.fn()
    if (!node.initialized || !node.equals(node.value, nextVal)) {
      node.value = nextVal
      node.version++
      if (node.initialized) {
        markSubscribers(node.subs, STATE_DIRTY)
      }
      node.initialized = true
    }
    node.state = STATE_CLEAN
  } finally {
    activeObserver = prevObserver
    trackingEnabled = prevTracking
    node.computing = false
  }
}

function flushEffects(): void {
  if (flushing || batchDepth > 0) return
  flushing = true
  try {
    let iterations = 0
    while (pendingEffects.length > 0) {
      if (++iterations > 1000) {
        pendingEffects.length = 0
        throw new Error('Svet: maximum reactive effect cascade depth exceeded')
      }
      let idx = -1
      for (let i = 0; i < pendingEffects.length; i++) {
        const cand = pendingEffects[i]
        if (cand.running) continue
        if (cand.sync) {
          if (idx < 0 || !pendingEffects[idx].sync || cand.id < pendingEffects[idx].id) {
            idx = i
          }
        } else if (idx < 0 || (!pendingEffects[idx].sync && cand.id < pendingEffects[idx].id)) {
          idx = i
        }
      }
      if (idx < 0) break
      const eff = pendingEffects.splice(idx, 1)[0]
      eff.queued = false
      if (eff.disposed || eff.state === STATE_CLEAN) continue
      if (isEffectDirty(eff)) {
        if (eff.scheduler) eff.scheduler()
        else eff.run()
      }
    }
  } finally {
    flushing = false
  }
}

export function signal<T>(initialValue: T, equals: (a: T, b: T) => boolean = Object.is): Signal<T> {
  const node: SignalNode<T> = { kind: 'signal', id: allocNodeId(), value: initialValue, version: 0, subs: new Set(), disposed: false }
  const read = (() => {
    if (!node.disposed && trackingEnabled && activeObserver) {
      node.subs.add(activeObserver)
      activeObserver.deps.add(node)
    }
    return node.value
  }) as Signal<T>

  Object.defineProperty(read, 'id', { value: node.id, enumerable: true })
  Object.defineProperty(read, '__svet_sig', { value: true })
  read.peek = () => node.value
  read.set = (nextValue: T | ((prev: T) => T)) => {
    if (node.disposed) return
    const resolved = typeof nextValue === 'function' ? (nextValue as (prev: T) => T)(node.value) : nextValue
    if (equals(node.value, resolved)) return
    node.value = resolved
    node.version++
    if (node.subs.size > 0) {
      markSubscribers(node.subs, STATE_DIRTY)
      if (batchDepth === 0) flushEffects()
    }
  }
  read.update = (updater: (prev: T) => T) => read.set(updater(node.value))
  read.dispose = () => {
    if (node.disposed) return
    node.disposed = true
    node.subs.clear()
    releaseNodeId(node.id)
  }
  return read
}

export function computed<T>(fn: () => T, equals: (a: T, b: T) => boolean = Object.is): ReadonlySignal<T> {
  const node: ComputedNode<T> = {
    kind: 'computed', id: allocNodeId(), state: STATE_DIRTY, deps: new Set(), cleanups: [], disposed: false,
    value: undefined as unknown as T, version: 0, initialized: false, computing: false, fn, equals, subs: new Set(),
    run: () => recomputeNode(node),
  }
  const read = (() => {
    if (!node.disposed) {
      ensureComputedUpToDate(node)
      if (trackingEnabled && activeObserver) {
        node.subs.add(activeObserver)
        activeObserver.deps.add(node)
      }
    }
    return node.value
  }) as ReadonlySignal<T>

  Object.defineProperty(read, 'id', { value: node.id, enumerable: true })
  Object.defineProperty(read, '__svet_sig', { value: true })
  read.peek = () => {
    if (!node.disposed) ensureComputedUpToDate(node)
    return node.value
  }
  read.dispose = () => {
    if (node.disposed) return
    node.disposed = true
    clearObserverDeps(node)
    runObserverCleanups(node)
    node.subs.clear()
    releaseNodeId(node.id)
  }
  return read
}

export function effect(
  fn: (onCleanupFn: (cb: () => void) => void) => void | (() => void),
  options?: { sync?: boolean; scheduler?: () => void },
): EffectHandle {
  const node: EffectNode = {
    kind: 'effect', id: allocNodeId(), state: STATE_DIRTY, deps: new Set(), cleanups: [], disposed: false,
    fn, owner: currentOwner, queued: false, running: false, sync: Boolean(options?.sync),
    scheduler: options?.scheduler,
    run() {
      if (node.disposed || node.running) return
      node.running = true
      clearObserverDeps(node)
      runObserverCleanups(node)
      const prevObserver = activeObserver
      const prevTracking = trackingEnabled
      activeObserver = node
      trackingEnabled = true
      node.state = STATE_CLEAN
      try {
        const ret = node.fn(cb => node.cleanups.push(cb))
        if (typeof ret === 'function') node.cleanups.push(ret)
      } finally {
        activeObserver = prevObserver
        trackingEnabled = prevTracking
        node.running = false
        if (batchDepth === 0 && pendingEffects.length > 0 && !flushing) {
          flushEffects()
        }
      }
    },
  }
  const handle: EffectHandle = {
    id: node.id,
    run() {
      node.run()
    },
    dispose() {
      if (node.disposed) return
      node.disposed = true
      clearObserverDeps(node)
      runObserverCleanups(node)
      releaseNodeId(node.id)
    },
  }
  if (currentOwner && !currentOwner.disposed) currentOwner.disposables.push(() => handle.dispose())
  node.run()
  return handle
}

export function batch<T>(fn: () => T): T {
  batchDepth++
  try { return fn() } finally { if (--batchDepth === 0) flushEffects() }
}

export function untrack<T>(fn: () => T): T {
  const prev = trackingEnabled
  trackingEnabled = false
  try { return fn() } finally { trackingEnabled = prev }
}

function disposeScope(scope: OwnerScope): void {
  if (scope.disposed) return
  scope.disposed = true
  if (scope.children) {
    for (const child of scope.children) disposeScope(child)
    scope.children.clear()
  }
  for (let i = scope.disposables.length - 1; i >= 0; i--) {
    try { scope.disposables[i]() } catch { /* continue */ }
  }
  scope.disposables.length = 0
  if (scope.parent?.children) {
    scope.parent.children.delete(scope)
    scope.parent = null
  }
}

export function createRoot<T>(fn: (dispose: () => void) => T): T {
  const prevOwner = currentOwner
  const scope: OwnerScope = { disposables: [], children: null, parent: prevOwner, disposed: false }
  if (prevOwner && !prevOwner.disposed) {
    if (!prevOwner.children) prevOwner.children = new Set()
    prevOwner.children.add(scope)
  }
  currentOwner = scope
  try { return fn(() => untrack(() => disposeScope(scope))) } finally { currentOwner = prevOwner }
}

export function onCleanup(cb: () => void): void {
  if (activeObserver) activeObserver.cleanups.push(cb)
  else if (currentOwner && !currentOwner.disposed) currentOwner.disposables.push(cb)
}

export type CompiledExprAst =
  | { type: 'literal'; value: unknown; static: true }
  | { type: 'token'; key: string; static: true }
  | { type: 'i18n'; key: string; static: false }
  | { type: 'state'; path: readonly string[]; static: false }
  | { type: 'ident'; name: string; path: readonly string[]; static: false }
  | { type: 'unary'; op: '!' | '-' | '+'; arg: CompiledExprAst; static: boolean }
  | { type: 'binary'; op: string; left: CompiledExprAst; right: CompiledExprAst; static: boolean }
  | { type: 'conditional'; test: CompiledExprAst; consequent: CompiledExprAst; alternate: CompiledExprAst; static: boolean }

interface ExprTok { kind: 'num' | 'str' | 'ident' | 'op' | 'punc'; val: string }

const exprAstCache = new Map<string, CompiledExprAst>()
const MAX_EXPR_CACHE = 2048

function tokenizeExpr(src: string): ExprTok[] {
  const tokens: ExprTok[] = []
  let i = 0
  const n = src.length
  while (i < n) {
    const ch = src[i]
    if (/\s/.test(ch)) { i++; continue }
    if (ch === '"' || ch === "'") {
      const q = ch
      let s = ''
      i++
      while (i < n && src[i] !== q) {
        if (src[i] === '\\' && i + 1 < n) i++
        s += src[i++]
      }
      i++
      tokens.push({ kind: 'str', val: s })
      continue
    }
    if (/\d/.test(ch) || (ch === '-' && /\d/.test(src[i + 1] ?? '') && (tokens.length === 0 || tokens.at(-1)!.kind === 'op' || tokens.at(-1)!.val === '('))) {
      let numStr = ch
      i++
      while (i < n && /[0-9.]/.test(src[i])) numStr += src[i++]
      let unit = ''
      while (i < n && /[a-zA-Z%]/.test(src[i])) unit += src[i++]
      tokens.push(unit ? { kind: 'str', val: numStr + unit } : { kind: 'num', val: numStr })
      continue
    }
    const three = src.slice(i, i + 3)
    if (three === '===' || three === '!==') { tokens.push({ kind: 'op', val: three }); i += 3; continue }
    const two = src.slice(i, i + 2)
    if (['==', '!=', '<=', '>=', '&&', '||', '??'].includes(two)) { tokens.push({ kind: 'op', val: two }); i += 2; continue }
    if ('+-*/%<>!?:'.includes(ch)) { tokens.push({ kind: 'op', val: ch }); i++; continue }
    if ('()[].,'.includes(ch)) { tokens.push({ kind: 'punc', val: ch }); i++; continue }
    if (/[a-zA-Z_$#]/.test(ch)) {
      let id = ''
      while (i < n && /[a-zA-Z0-9_$#-]/.test(src[i])) id += src[i++]
      tokens.push({ kind: 'ident', val: id })
      continue
    }
    i++
  }
  return tokens
}

export function compileFormaExpression(raw: string): CompiledExprAst {
  const trimmed = raw.trim()
  if (!trimmed) return { type: 'literal', value: '', static: true }
  const cached = exprAstCache.get(trimmed)
  if (cached) return cached

  const tokens = tokenizeExpr(trimmed)
  let pos = 0
  const peek = () => tokens[pos]
  const next = () => tokens[pos++]

  const parsePrimary = (): CompiledExprAst => {
    const tok = next()
    if (!tok) return { type: 'literal', value: undefined, static: true }
    if (tok.kind === 'num') return { type: 'literal', value: Number(tok.val), static: true }
    if (tok.kind === 'str') return { type: 'literal', value: tok.val, static: true }
    if (tok.kind === 'ident') {
      if (tok.val === 'true') return { type: 'literal', value: true, static: true }
      if (tok.val === 'false') return { type: 'literal', value: false, static: true }
      if (tok.val === 'null' || tok.val === 'none') return { type: 'literal', value: null, static: true }
      if (tok.val.startsWith('#')) return { type: 'literal', value: tok.val, static: true }
      const parts = [tok.val]
      while (peek()?.kind === 'punc' && peek()!.val === '.') {
        next()
        const prop = next()
        if (prop) parts.push(prop.val)
      }
      if (parts[0] === 'tokens' && parts.length > 1) return { type: 'token', key: parts.slice(1).join('.'), static: true }
      if (parts[0] === 'i18n' && parts.length > 1) return { type: 'i18n', key: parts.slice(1).join('.'), static: false }
      if (parts[0] === 'state' && parts.length > 1) return { type: 'state', path: parts.slice(1), static: false }
      return { type: 'ident', name: parts[0], path: parts.slice(1), static: false }
    }
    if (tok.kind === 'punc' && tok.val === '(') {
      const inner = parseTernary()
      if (peek()?.val === ')') next()
      return inner
    }
    return { type: 'literal', value: trimmed, static: true }
  }

  const parseUnary = (): CompiledExprAst => {
    const p = peek()
    if (p && p.kind === 'op' && (p.val === '!' || p.val === '-' || p.val === '+')) {
      next()
      const arg = parseUnary()
      return { type: 'unary', op: p.val as '!' | '-' | '+', arg, static: arg.static }
    }
    return parsePrimary()
  }

  const parseBin = (nextLevel: () => CompiledExprAst, ops: readonly string[]): CompiledExprAst => {
    let left = nextLevel()
    while (peek() && ops.includes(peek()!.val)) {
      const op = next()!.val
      const right = nextLevel()
      left = { type: 'binary', op, left, right, static: left.static && right.static }
    }
    return left
  }

  const parseMul = () => parseBin(parseUnary, ['*', '/', '%'])
  const parseAdd = () => parseBin(parseMul, ['+', '-'])
  const parseRel = () => parseBin(parseAdd, ['<', '>', '<=', '>='])
  const parseEq = () => parseBin(parseRel, ['==', '!=', '===', '!=='])
  const parseAnd = () => parseBin(parseEq, ['&&'])
  const parseOr = () => parseBin(parseAnd, ['||', '??'])

  const parseTernary = (): CompiledExprAst => {
    const test = parseOr()
    if (peek()?.val === '?') {
      next()
      const consequent = parseTernary()
      if (peek()?.val === ':') next()
      const alternate = parseTernary()
      return { type: 'conditional', test, consequent, alternate, static: test.static && consequent.static && alternate.static }
    }
    return test
  }

  const ast = parseTernary()
  if (exprAstCache.size >= MAX_EXPR_CACHE) {
    const oldest = exprAstCache.keys().next().value
    if (oldest !== undefined) exprAstCache.delete(oldest)
  }
  exprAstCache.set(trimmed, ast)
  return ast
}
