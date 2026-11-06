/**
 * # Svet Reactivity & Signal-Backed State Engine (`svet-reactivity.ts`)
 *
 * Pure Svet implementation of reactive state primitives (`ref`, `shallowRef`,
 * `computed`, `reactive`, `shallowReactive`, `readonly`, `effectScope`),
 * powered by the Svet glitch-free reactive graph in `./svet.ts`.
 */
import {
  signal,
  computed as svetComputed,
  batch,
  untrack,
  createRoot,
  onCleanup,
  type Signal,
} from './svet'

const RAW_SYM = Symbol('svet.raw')
const IS_REACTIVE_SYM = Symbol('svet.reactive')
const IS_SHALLOW_SYM = Symbol('svet.shallow')
const SKIP_SYM = Symbol('svet.skip')
export const DEEP_SIG_SYM = Symbol('svet.deepSig')

const reactiveMap = new WeakMap<object, any>()
const shallowReactiveMap = new WeakMap<object, any>()
const propSignalsMap = new WeakMap<object, Map<PropertyKey, Signal<number>>>()
const deepSignalMap = new WeakMap<object, Signal<number>>()
const parentTargetMap = new WeakMap<object, Set<object>>()

function getDeepSignal(target: object): Signal<number> {
  let sig = deepSignalMap.get(target)
  if (!sig) {
    sig = untrack(() => signal(0))
    deepSignalMap.set(target, sig)
  }
  return sig
}

function getPropSignal(target: object, key: PropertyKey): Signal<number> {
  let map = propSignalsMap.get(target)
  if (!map) {
    map = new Map()
    propSignalsMap.set(target, map)
  }
  let sig = map.get(key)
  if (!sig) {
    sig = untrack(() => signal(0))
    map.set(key, sig)
  }
  return sig
}

function notifyDeep(target: object, visited = new Set<object>()): void {
  if (visited.has(target)) return
  visited.add(target)
  const deepSig = deepSignalMap.get(target)
  if (deepSig) deepSig.set(deepSig.peek() + 1)
  const parents = parentTargetMap.get(target)
  if (parents) {
    for (const p of parents) notifyDeep(p, visited)
  }
}

export function trackDeep(value: unknown): void {
  const raw = toRaw(value)
  if (raw && typeof raw === 'object') {
    getDeepSignal(raw as object)()
  }
}

function linkParentChild(parent: object, child: unknown): void {
  const rawChild = toRaw(child)
  if (rawChild && typeof rawChild === 'object') {
    let parents = parentTargetMap.get(rawChild as object)
    if (!parents) {
      parents = new Set()
      parentTargetMap.set(rawChild as object, parents)
    }
    parents.add(parent)
  }
}

export function canObserve(value: unknown): value is object {
  if (!value || typeof value !== 'object') return false
  if ((value as any)[SKIP_SYM] || (value as any).__v_skip) return false
  if (Array.isArray(value)) return true
  if (ArrayBuffer.isView(value) || value instanceof ArrayBuffer) return false
  const proto = Object.getPrototypeOf(value)
  return proto === null || proto === Object.prototype
}

export function toRaw<T>(observed: T): T {
  const raw = observed && (observed as any)[RAW_SYM]
  return raw ? toRaw(raw) : observed
}

export function markRaw<T extends object>(value: T): T {
  if (Object.isExtensible(value)) {
    Object.defineProperty(value, SKIP_SYM, { configurable: true, enumerable: false, value: true })
  }
  return value
}

export function isReactive(value: unknown): boolean {
  return Boolean(value && (value as any)[IS_REACTIVE_SYM])
}

function createReactiveObject<T extends object>(target: T, shallow: boolean): T {
  if (!canObserve(target)) return target
  if ((target as any)[IS_REACTIVE_SYM] && Boolean((target as any)[IS_SHALLOW_SYM]) === shallow) {
    return target
  }
  const rawTarget = toRaw(target)
  const proxyMap = shallow ? shallowReactiveMap : reactiveMap
  const existing = proxyMap.get(rawTarget)
  if (existing) return existing

  if (!shallow) {
    if (Array.isArray(rawTarget)) {
      for (let i = 0; i < rawTarget.length; i++) linkParentChild(rawTarget, rawTarget[i])
    } else {
      for (const k of Object.keys(rawTarget)) linkParentChild(rawTarget, (rawTarget as any)[k])
    }
  }

  const proxy = new Proxy(rawTarget, {
    get(obj, key, receiver) {
      if (key === RAW_SYM) return obj
      if (key === IS_REACTIVE_SYM) return true
      if (key === IS_SHALLOW_SYM) return shallow
      if (key === DEEP_SIG_SYM) return getDeepSignal(obj)()
      const res = Reflect.get(obj, key, receiver)
      if (typeof key === 'symbol') return res
      getPropSignal(obj, key)()
      if (isRef(res)) {
        return Array.isArray(obj) && Number.isInteger(Number(key)) ? res : res.value
      }
      if (!shallow && canObserve(res)) {
        linkParentChild(obj, res)
        return reactive(res)
      }
      return res
    },
    set(obj, key, value, receiver) {
      const oldValue = (obj as any)[key]
      const rawValue = shallow ? value : toRaw(value)
      const hadKey = Array.isArray(obj)
        ? Number(key) < obj.length
        : Object.prototype.hasOwnProperty.call(obj, key)
      if (!shallow && isRef(oldValue) && !isRef(value)) {
        oldValue.value = value
        return true
      }
      const result = Reflect.set(obj, key, rawValue, receiver)
      if (obj === toRaw(receiver)) {
        if (!hadKey || !Object.is(oldValue, rawValue)) {
          if (!shallow) linkParentChild(obj, rawValue)
          batch(() => {
            const sig = getPropSignal(obj, key)
            sig.set(sig.peek() + 1)
            if (Array.isArray(obj) && key !== 'length') {
              const lenSig = getPropSignal(obj, 'length')
              lenSig.set(lenSig.peek() + 1)
            }
            notifyDeep(obj)
          })
        }
      }
      return result
    },
    deleteProperty(obj, key) {
      const hadKey = Object.prototype.hasOwnProperty.call(obj, key)
      const result = Reflect.deleteProperty(obj, key)
      if (result && hadKey) {
        batch(() => {
          const sig = getPropSignal(obj, key)
          sig.set(sig.peek() + 1)
          notifyDeep(obj)
        })
      }
      return result
    },
    has(obj, key) {
      if (typeof key !== 'symbol') getPropSignal(obj, key)()
      return Reflect.has(obj, key)
    },
    ownKeys(obj) {
      getDeepSignal(obj)()
      return Reflect.ownKeys(obj)
    },
  })

  proxyMap.set(rawTarget, proxy)
  return proxy
}

export function reactive<T extends object>(target: T): T {
  return createReactiveObject(target, false)
}

export function shallowReactive<T extends object>(target: T): T {
  return createReactiveObject(target, true)
}

export function readonly<T extends object>(target: T): Readonly<T> {
  return target
}

export interface Ref<T = any, S = T> {
  get value(): T
  set value(_: S)
  readonly __v_isRef: true
}

export type ShallowRef<T = any, S = T> = Ref<T, S>
export type ComputedRef<T = any> = Readonly<Ref<T>>
export type WritableComputedRef<T, S = T> = Ref<T, S>

export interface WritableComputedOptions<T> {
  get: () => T
  set: (v: T) => void
}

export type PropType<T> = { new (...args: any[]): T & {} } | { (): T }

class RefImpl<T> implements Ref<T> {
  public readonly __v_isRef = true as const
  private _rawValue: T
  private _value: T
  private _sig: Signal<number>
  private readonly _shallow: boolean

  constructor(value: T, shallow: boolean) {
    this._shallow = shallow
    this._rawValue = shallow ? value : toRaw(value)
    this._value = !shallow && canObserve(value) ? reactive(value) : value
    this._sig = untrack(() => signal(0))
  }

  get value(): T {
    this._sig()
    if (!this._shallow && canObserve(this._rawValue)) {
      getDeepSignal(this._rawValue as object)()
    }
    return this._value
  }

  set value(newVal: T) {
    const useDirect = this._shallow || !canObserve(newVal)
    const nextRaw = useDirect ? newVal : toRaw(newVal)
    if (!Object.is(nextRaw, this._rawValue)) {
      this._rawValue = nextRaw
      this._value = useDirect ? newVal : reactive(nextRaw as unknown as object) as unknown as T
      this._sig.set(this._sig.peek() + 1)
    }
  }
}

export function ref<T>(value: T): Ref<T>
export function ref<T = any>(): Ref<T | undefined>
export function ref(value?: unknown): Ref<any> {
  if (isRef(value)) return value
  return new RefImpl(value, false)
}

export function shallowRef<T>(value: T): ShallowRef<T>
export function shallowRef<T = any>(): ShallowRef<T | undefined>
export function shallowRef(value?: unknown): ShallowRef<any> {
  if (isRef(value)) return value
  return new RefImpl(value, true)
}

export function isRef<T>(r: Ref<T> | unknown): r is Ref<T> {
  return Boolean(r && (r as any).__v_isRef === true)
}

export function unref<T>(r: T | Ref<T>): T {
  return isRef(r) ? r.value : r
}

export function toRef<T extends object, K extends keyof T>(object: T, key: K, defaultValue?: T[K]): Ref<T[K]>
export function toRef<T>(value: () => T): Readonly<Ref<T>>
export function toRef<T>(value: T): Ref<T>
export function toRef(source: any, key?: PropertyKey, defaultValue?: any): Ref<any> {
  if (isRef(source)) return source
  if (typeof source === 'function' && key === undefined) return computed(source)
  if (source && typeof source === 'object' && key !== undefined) {
    return {
      __v_isRef: true,
      get value() {
        const val = source[key]
        return val === undefined ? defaultValue : val
      },
      set value(v) {
        source[key] = v
      },
    }
  }
  return ref(source)
}

export function computed<T>(getter: () => T): ComputedRef<T>
export function computed<T>(options: WritableComputedOptions<T>): WritableComputedRef<T>
export function computed<T>(getterOrOptions: (() => T) | WritableComputedOptions<T>): any {
  const isGetter = typeof getterOrOptions === 'function'
  const getter = isGetter ? getterOrOptions : getterOrOptions.get
  const setter = isGetter ? undefined : getterOrOptions.set
  const c = svetComputed(getter)
  return {
    __v_isRef: true,
    get value(): T {
      return c()
    },
    set value(v: T) {
      if (setter) setter(v)
    },
  }
}

export interface EffectScope {
  readonly active: boolean
  run<T>(fn: () => T): T | undefined
  stop(): void
}

let activeEffectScope: EffectScopeImpl | null = null

class EffectScopeImpl implements EffectScope {
  active = true
  cleanups: Array<() => void> = []
  scopes: EffectScopeImpl[] = []
  parent: EffectScopeImpl | null

  constructor(detached = false) {
    this.parent = detached ? null : activeEffectScope
    if (this.parent) this.parent.scopes.push(this)
  }

  run<T>(fn: () => T): T | undefined {
    if (!this.active) return undefined
    const prev = activeEffectScope
    activeEffectScope = this
    try {
      let result: T | undefined
      createRoot(dispose => {
        this.cleanups.push(dispose)
        result = fn()
      })
      return result
    } finally {
      activeEffectScope = prev
    }
  }

  stop(): void {
    if (!this.active) return
    this.active = false
    for (const child of this.scopes) child.stop()
    this.scopes.length = 0
    for (let i = this.cleanups.length - 1; i >= 0; i--) {
      try { this.cleanups[i]() } catch { /* ignore */ }
    }
    this.cleanups.length = 0
  }
}

export function effectScope(detached = false): EffectScope {
  return new EffectScopeImpl(detached)
}

export function getCurrentScope(): EffectScope | null {
  return activeEffectScope
}

export function onScopeDispose(fn: () => void): void {
  if (activeEffectScope && activeEffectScope.active) {
    activeEffectScope.cleanups.push(fn)
  } else {
    onCleanup(fn)
  }
}

export function registerScopeCleanup(fn: () => void): void {
  if (activeEffectScope && activeEffectScope.active) {
    activeEffectScope.cleanups.push(fn)
  }
}

export {
  enterComponentSetup,
  exitComponentSetup,
  flushPostQueue,
  flushPreQueue,
  mergeModels,
  nextTick,
  queuePostFlushCb,
  queuePreFlushCb,
  useId,
  useModel,
  watch,
  watchEffect,
  type WatchOptions,
  type WatchStopHandle,
} from './svet-watch'
