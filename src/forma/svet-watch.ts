/**
 * # Svet Watch, Scheduler & Model Helpers (`svet-watch.ts`)
 *
 * Pure Svet implementation of `watch`, `watchEffect`, `nextTick`, `useModel`,
 * `mergeModels`, `useId`, and component pre/post flush queues.
 */
import {
  signal,
  effect as svetEffect,
  untrack,
} from './svet'
import {
  DEEP_SIG_SYM,
  canObserve,
  isReactive,
  isRef,
  registerScopeCleanup,
  type Ref,
} from './svet-reactivity'

const preFlushQueue: Array<() => void> = []
const postFlushQueue: Array<() => void> = []
let isFlushPending = false
const resolvedPromise = Promise.resolve()
let setupDepth = 0
const preSetupQueue: Array<() => void> = []

export function enterComponentSetup(): void {
  setupDepth++
}

export function exitComponentSetup(): void {
  setupDepth = Math.max(0, setupDepth - 1)
  if (setupDepth === 0 && preSetupQueue.length > 0) {
    const queue = preSetupQueue.splice(0)
    for (let i = 0; i < queue.length; i++) {
      try {
        queue[i]()
      } catch {
        // ignore pre-setup watcher errors
      }
    }
  }
}

export function queuePreFlushCb(cb: () => void): void {
  if (!preFlushQueue.includes(cb)) preFlushQueue.push(cb)
  if (!isFlushPending) {
    isFlushPending = true
    resolvedPromise.then(flushPostQueue)
  }
}

export function queuePostFlushCb(cb: () => void): void {
  if (!postFlushQueue.includes(cb)) postFlushQueue.push(cb)
  if (!isFlushPending) {
    isFlushPending = true
    resolvedPromise.then(flushPostQueue)
  }
}

export function flushPreQueue(): void {
  let safety = 0
  while (preFlushQueue.length > 0 && safety++ < 1000) {
    const queue = preFlushQueue.splice(0)
    for (const cb of queue) {
      try { cb() } catch { /* ignore */ }
    }
  }
}

export function flushPostQueue(): void {
  isFlushPending = false
  flushPreQueue()
  let safety = 0
  while ((postFlushQueue.length > 0 || preFlushQueue.length > 0) && safety++ < 1000) {
    flushPreQueue()
    const queue = postFlushQueue.splice(0)
    for (const cb of queue) {
      try { cb() } catch { /* ignore */ }
    }
  }
}

export function nextTick<T = void>(fn?: () => T): Promise<T> {
  return resolvedPromise.then(() => {
    flushPostQueue()
    return fn ? fn() : (undefined as unknown as T)
  })
}

function traverse(value: unknown, seen = new Set<unknown>()): unknown {
  if (!value || typeof value !== 'object' || seen.has(value)) return value
  seen.add(value)
  if (isRef(value)) {
    traverse(value.value, seen)
    return value
  }
  if (isReactive(value)) {
    void (value as any)[DEEP_SIG_SYM]
  }
  if (Array.isArray(value)) {
    for (let i = 0; i < value.length; i++) traverse(value[i], seen)
  } else if (canObserve(value)) {
    for (const k of Object.keys(value)) traverse((value as any)[k], seen)
  }
  return value
}

export interface WatchOptions {
  immediate?: boolean
  deep?: boolean
  flush?: 'pre' | 'post' | 'sync'
  once?: boolean
}

export type WatchStopHandle = () => void

export function watchEffect(
  effectFn: (onCleanupFn: (cb: () => void) => void) => void,
  options?: { flush?: 'pre' | 'post' | 'sync' },
): WatchStopHandle {
  const flush = options?.flush ?? 'pre'
  let cleanup: (() => void) | undefined
  let stopped = false
  let pending = false
  const registerCleanup = (cb: () => void) => { cleanup = cb }

  const handle = svetEffect(() => {
    if (stopped) return
    if (cleanup) {
      try { cleanup() } finally { cleanup = undefined }
    }
    effectFn(registerCleanup)
  }, {
    sync: flush === 'sync',
    scheduler: flush === 'sync' ? undefined : () => {
      if (stopped || pending) return
      pending = true
      const queueFn = flush === 'post' ? queuePostFlushCb : queuePreFlushCb
      queueFn(() => {
        pending = false
        if (!stopped) handle.run()
      })
    },
  })

  const stop = () => {
    if (stopped) return
    stopped = true
    if (cleanup) {
      try { cleanup() } finally { cleanup = undefined }
    }
    handle.dispose()
  }
  registerScopeCleanup(stop)
  return stop
}

export function watch(
  source: any,
  cb: (value: any, oldValue: any, onCleanupFn: (fn: () => void) => void) => any,
  options: WatchOptions = {},
): WatchStopHandle {
  const { immediate = false, deep = false, flush = 'pre', once = false } = options
  const isMulti = Array.isArray(source)

  let getter: () => any
  let forceTrigger = deep
  if (isRef(source)) {
    getter = () => source.value
  } else if (isReactive(source)) {
    getter = () => source
    forceTrigger = true
  } else if (Array.isArray(source)) {
    getter = () => source.map(s => {
      if (isRef(s)) return s.value
      if (isReactive(s)) return traverse(s)
      if (typeof s === 'function') return s()
      return s
    })
  } else if (typeof source === 'function') {
    getter = source
  } else {
    getter = () => undefined
  }

  if (deep) {
    const baseGetter = getter
    getter = () => {
      const val = baseGetter()
      traverse(val)
      return val
    }
  }

  let cleanup: (() => void) | undefined
  const onCleanupRegister = (fn: () => void) => { cleanup = fn }
  let oldValue: any = isMulti ? [] : undefined
  let initialized = false
  let stopped = false

  const sampleGetter = () => {
    const val = getter()
    if (deep && val && typeof val === 'object') {
      if (Array.isArray(val)) return [...val]
      return val
    }
    return val
  }

  let latestValue: any
  let hasPendingRun = false

  const executeCallback = (newValue: any, prevValue: any) => {
    if (stopped) return
    if (cleanup) {
      try { cleanup() } finally { cleanup = undefined }
    }
    untrack(() => cb(newValue, prevValue, onCleanupRegister))
    if (once) stop()
  }

  const handle = svetEffect(() => {
    const newValue = sampleGetter()
    if (!initialized) {
      initialized = true
      oldValue = isMulti ? [...newValue] : newValue
      if (immediate) {
        if (flush === 'post') {
          queuePostFlushCb(() => executeCallback(newValue, isMulti ? [] : undefined))
        } else {
          executeCallback(newValue, isMulti ? [] : undefined)
        }
      }
      return
    }
    const changed = forceTrigger || (isMulti
      ? newValue.some((v: any, i: number) => !Object.is(v, oldValue[i]))
      : !Object.is(newValue, oldValue))
    if (!changed) return
    const prevValue = oldValue
    oldValue = isMulti ? [...newValue] : newValue
    latestValue = newValue

    if (flush === 'post') {
      if (!hasPendingRun) {
        hasPendingRun = true
        queuePostFlushCb(() => {
          hasPendingRun = false
          executeCallback(latestValue, prevValue)
        })
      }
    } else if (flush === 'pre' && setupDepth > 0) {
      if (!hasPendingRun) {
        hasPendingRun = true
        queuePreFlushCb(() => {
          hasPendingRun = false
          executeCallback(latestValue, prevValue)
        })
      }
    } else {
      executeCallback(newValue, prevValue)
    }
  }, { sync: flush === 'sync' || flush === 'pre' })

  const stop = () => {
    if (stopped) return
    stopped = true
    if (cleanup) {
      try { cleanup() } finally { cleanup = undefined }
    }
    handle.dispose()
  }
  registerScopeCleanup(stop)
  return stop
}

let nextUniqueId = 0
export function useId(): string {
  return `svet-id-${++nextUniqueId}`
}

export function mergeModels(a: any, b: any): any {
  if (!a || !b) return a || b
  if (Array.isArray(a) && Array.isArray(b)) return Array.from(new Set([...a, ...b]))
  return { ...a, ...b }
}

export function useModel<T = any>(props: Record<string, any>, name: string, options?: { defaultValue?: T }): Ref<T> {
  const camelName = name.replace(/-([a-z])/g, (_, c: string) => c.toUpperCase())
  const readProp = () => (camelName in props ? props[camelName] : props[name])
  const localSig = untrack(() => {
    const initial = readProp()
    return signal<T>(initial !== undefined ? initial : (options?.defaultValue as T))
  })
  let lastPropValue = untrack(readProp)

  watch(
    readProp,
    val => {
      lastPropValue = val
      localSig.set(val as T)
    },
    { flush: 'sync' },
  )

  return {
    __v_isRef: true,
    get value(): T {
      const propVal = readProp()
      if (!Object.is(propVal, lastPropValue)) {
        lastPropValue = propVal
        untrack(() => localSig.set(propVal))
      }
      return localSig()
    },
    set value(v: T) {
      lastPropValue = v
      localSig.set(v)
      const emit = props.__svet_emit as ((event: string, ...args: any[]) => void) | undefined
      if (emit) {
        emit(`update:${name}`, v)
      } else {
        const handler = props[`onUpdate:${name}`] ?? props[`onUpdate:${camelName}`]
        if (typeof handler === 'function') handler(v)
        else if (Array.isArray(handler)) handler.forEach(fn => typeof fn === 'function' && fn(v))
      }
    },
  }
}
