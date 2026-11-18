/**
 * # Svet VNode & Component Definition Layer (`svet-vnode.ts`)
 *
 * Provides the VNode data model, template helpers (`createElementVNode`, `renderList`,
 * `renderSlot`, `normalizeClass`, `normalizeStyle`, `mergeProps`), and component
 * lifecycle hooks for Svet.
 */
import { isReactive, shallowRef, trackDeep } from './svet-reactivity'

export const Fragment = Symbol.for('svet.fragment')
export const Text = Symbol.for('svet.text')
export const Comment = Symbol.for('svet.comment')
export const Static = Symbol.for('svet.static')
export const Teleport = Symbol.for('svet.teleport')
export const Transition = Symbol.for('svet.transition')

export interface DirectiveBinding {
  dir: Directive
  value: unknown
  oldValue?: unknown
  arg?: string | undefined
  modifiers: Record<string, boolean>
}

export interface Directive {
  created?: (el: unknown, binding: DirectiveBinding, vnode: SvetVNode) => void
  beforeMount?: (el: unknown, binding: DirectiveBinding, vnode: SvetVNode) => void
  mounted?: (el: unknown, binding: DirectiveBinding, vnode: SvetVNode) => void
  beforeUpdate?: (el: unknown, binding: DirectiveBinding, vnode: SvetVNode, prevVNode: SvetVNode) => void
  updated?: (el: unknown, binding: DirectiveBinding, vnode: SvetVNode, prevVNode: SvetVNode) => void
  beforeUnmount?: (el: unknown, binding: DirectiveBinding, vnode: SvetVNode) => void
  unmounted?: (el: unknown, binding: DirectiveBinding, vnode: SvetVNode) => void
}

export interface SvetVNode {
  readonly __v_isVNode: true
  type: unknown
  props: Record<string, unknown> | null
  children: unknown
  key: unknown
  ref: unknown
  el: unknown
  anchor: unknown
  target: unknown
  component: SvetComponentInstance | null
  dirs: DirectiveBinding[] | null
  scopeId: string | null
  patchFlag?: number | undefined
  dynamicProps?: string[] | null | undefined
}

/** Render function stored on a component instance. */
export type InstanceRender = (ctx: unknown, cache: unknown[], ...args: unknown[]) => unknown

/** Context passed to a component's `setup` or functional render. */
export interface ComponentSetupContext {
  attrs: Record<string, unknown>
  slots: Record<string, unknown>
  emit: (event: string, ...args: unknown[]) => void
  expose: (exposed?: unknown) => void
}

/** Component definition as the runtime sees it once mounted: an options object or a functional component. */
export type InstanceComponentType =
  | {
      props?: unknown
      emits?: unknown
      inheritAttrs?: boolean
      __scopeId?: string
      setup?: (props: Record<string, unknown>, ctx: ComponentSetupContext) => unknown
      render?: InstanceRender
    }
  | (((props: Record<string, unknown>, ctx: ComponentSetupContext) => unknown) & {
      props?: unknown
      emits?: unknown
      inheritAttrs?: boolean
      render?: InstanceRender
    })

export interface SvetComponentInstance {
  uid: number
  vnode: SvetVNode
  type: InstanceComponentType
  props: Record<string, unknown>
  attrs: Record<string, unknown>
  slots: Record<string, unknown>
  setupState: Record<PropertyKey, unknown>
  exposed: Record<string, unknown> | null
  render: InstanceRender | null
  renderCache: unknown[]
  ctx: Record<string, unknown>
  subTree: SvetVNode | null
  isMounted: boolean
  isUnmounted: boolean
  bm: Array<() => void>
  m: Array<() => void>
  bu: Array<() => void>
  u: Array<() => void>
  bum: Array<() => void>
  um: Array<() => void>
  stopEffect: (() => void) | null
  update: (() => void) | null
  emit: (event: string, ...args: unknown[]) => void
}

export type VNode = SvetVNode
export type VNodeChild = SvetVNode | string | number | boolean | null | undefined | VNodeChild[]
export type Slots = Readonly<Record<string, ((...args: unknown[]) => VNode[]) | undefined>>
export type ComponentPublicInstance<P = object, B = object> = {
  $props: P
  $attrs: Record<string, unknown>
  $slots: Slots
  $emit: (event: string, ...args: unknown[]) => void
} & P &
  B
export type FunctionalComponent<P = object, E = unknown> = {
  (props: P, ctx: { attrs: unknown; slots: unknown; emit: E }): unknown
  props?: unknown
  emits?: unknown
  inheritAttrs?: boolean
  displayName?: string
}

export type Component<P = unknown> =
  | {
      __name?: string
      name?: string
      props?: unknown
      emits?: unknown
      inheritAttrs?: boolean
      setup?: (
        props: P,
        ctx: { attrs: unknown; slots: unknown; emit: unknown; expose: (exposed?: unknown) => void },
      ) => unknown
      render?: (ctx: unknown, cache: unknown[]) => unknown
      ssrRender?: (...args: unknown[]) => unknown
      [key: string]: unknown
    }
  | ((props: P, ctx: unknown) => unknown)

export type DefineComponent<
  PropsOrPropOptions = object,
  RawBindings = object,
  D = unknown,
> = Component<PropsOrPropOptions> & {
  new (...args: unknown[]): { $props: PropsOrPropOptions } & RawBindings & D
}

let currentInstance: SvetComponentInstance | null = null
let currentRenderingInstance: SvetComponentInstance | null = null

export function setCurrentRenderingInstance(instance: SvetComponentInstance | null): SvetComponentInstance | null {
  const prev = currentRenderingInstance
  currentRenderingInstance = instance
  return prev
}

export function getCurrentInstance(): SvetComponentInstance | null {
  return currentInstance
}

export function setCurrentInstance(instance: SvetComponentInstance | null): SvetComponentInstance | null {
  const prev = currentInstance
  currentInstance = instance
  return prev
}

export function onMounted(hook: () => void, target: SvetComponentInstance | null = currentInstance): void {
  if (target) target.m.push(hook)
}

export function onBeforeUpdate(hook: () => void, target: SvetComponentInstance | null = currentInstance): void {
  if (target) target.bu.push(hook)
}

export function onBeforeUnmount(hook: () => void, target: SvetComponentInstance | null = currentInstance): void {
  if (target) target.bum.push(hook)
}

export function onUnmounted(hook: () => void, target: SvetComponentInstance | null = currentInstance): void {
  if (target) target.um.push(hook)
}

export function defineComponent<T extends Component>(options: T): T {
  const filename = (options as { __file?: unknown })?.__file || (options as { __name?: unknown })?.__name
  if (filename && typeof filename === 'string') {
    registerCompiledSfc(filename, options)
  }
  return options
}

const asyncResolvedCache = new Map<string, Component>()
const asyncPromiseCache = new Map<string, Promise<Component>>()

export function registerCompiledSfc(filename: string, comp: Component): void {
  const base = filename.split('/').pop()
  if (base) {
    asyncResolvedCache.set(base, comp)
    const stem = base.replace(/\.svt$/, '')
    if (stem) asyncResolvedCache.set(stem, comp)
  }
}

function getAsyncCacheKey(loaderStr: string): string {
  const match = loaderStr.match(/\/([^/?'"`()]+?)(?:\.(?:svt|ts|js))?(?:\?[^'"`()]*)?['"`)]/)
  return match ? match[1]! : loaderStr
}

export function defineAsyncComponent<T extends Component = Component>(
  source:
    (() => Promise<T | { default: T }>) | { loader: () => Promise<T | { default: T }>; loadingComponent?: Component },
): Component {
  const loader = typeof source === 'function' ? source : source.loader
  const loadingComponent = typeof source === 'function' ? undefined : source.loadingComponent
  const cacheKey = getAsyncCacheKey(loader.toString())
  let resolvedComp: Component | null = asyncResolvedCache.get(cacheKey) ?? null
  let loadPromise: Promise<Component> | null = asyncPromiseCache.get(cacheKey) ?? null

  return defineComponent({
    name: 'SvetAsyncWrapper',
    __svet_isAsyncWrapper: true,
    inheritAttrs: false,
    setup(_props, { attrs, slots, expose }) {
      if (!resolvedComp) {
        resolvedComp = asyncResolvedCache.get(cacheKey) ?? null
      }
      const loaded = shallowRef<Component | null>(resolvedComp)
      const innerRef = shallowRef<unknown>(null)
      expose({
        __svet_asyncInnerRef: innerRef,
      })
      const inst = currentInstance
      if (!resolvedComp) {
        if (!loadPromise) {
          loadPromise = Promise.resolve(loader())
            .then(mod => {
              const comp = (mod && (mod as { default?: T }).default) || (mod as Component)
              resolvedComp = comp
              if (comp) asyncResolvedCache.set(cacheKey, comp)
              return comp
            })
            .catch(() => null as unknown as Component)
          asyncPromiseCache.set(cacheKey, loadPromise)
        }
        void loadPromise.then(comp => {
          if (comp && (!inst || !inst.isUnmounted)) {
            loaded.value = comp
          }
        })
      }
      return () => {
        const comp = loaded.value
        if (!comp)
          return loadingComponent
            ? createVNode(loadingComponent, { ...(attrs as Record<string, unknown>) })
            : createCommentVNode('async')
        return createVNode(comp, { ...(attrs as Record<string, unknown>), ref: innerRef }, slots)
      }
    },
  })
}

let currentScopeId: string | null = null
export function withScopeId(id: string): <T extends (...args: unknown[]) => unknown>(fn: T) => T {
  return <T extends (...args: unknown[]) => unknown>(fn: T): T => {
    return ((...args: unknown[]) => {
      const prev = currentScopeId
      currentScopeId = id
      try {
        return fn(...args)
      } finally {
        currentScopeId = prev
      }
    }) as unknown as T
  }
}

export function isVNode(value: unknown): value is SvetVNode {
  return Boolean(value && (value as { __v_isVNode?: unknown }).__v_isVNode === true)
}

export function openBlock(_disableTracking = false): void {
  // Svet uses fine-grained signals and fast VNode diffing; block tracking is a no-op
}

export function createVNode(
  type: unknown,
  props?: Record<string, unknown> | null,
  children?: unknown,
  patchFlag?: number,
  dynamicProps?: string[] | null,
): SvetVNode {
  if (!type) type = Comment
  if (props && isReactive(props)) trackDeep(props)
  const normalizedProps = props ? { ...props } : null
  let key: unknown = null
  let refProp: unknown = null
  if (normalizedProps) {
    if ('key' in normalizedProps) {
      key = normalizedProps.key
    }
    if ('ref' in normalizedProps) {
      refProp = {
        ref: normalizedProps.ref,
        ref_key: normalizedProps.ref_key,
        ref_for: Boolean(normalizedProps.ref_for),
        owner: currentRenderingInstance,
      }
      delete normalizedProps.ref
      delete normalizedProps.ref_key
      delete normalizedProps.ref_for
    }
    if ('class' in normalizedProps) {
      normalizedProps.class = normalizeClass(normalizedProps.class)
    }
    if ('style' in normalizedProps) {
      normalizedProps.style = normalizeStyle(normalizedProps.style)
    }
  }

  return {
    __v_isVNode: true,
    type,
    props: normalizedProps,
    children:
      children === undefined || children === null
        ? null
        : typeof children === 'string' || typeof children === 'number'
          ? String(children)
          : children,
    key,
    ref: refProp,
    el: null,
    anchor: null,
    target: null,
    component: null,
    dirs: null,
    scopeId: currentScopeId,
    patchFlag,
    dynamicProps,
  }
}

export const createElementVNode = createVNode
export const createElementBlock = createVNode
export const createBlock = createVNode

export function createTextVNode(text = '', _patchFlag?: number): SvetVNode {
  return createVNode(Text, null, String(text))
}

export function createCommentVNode(text = '', _asBlock?: boolean): SvetVNode {
  return createVNode(Comment, null, String(text))
}

export function createStaticVNode(content: string, numberOfNodes = 1): SvetVNode {
  const vnode = createVNode(Static, null, content)
  ;(vnode as SvetVNode & { staticCount?: number }).staticCount = numberOfNodes
  return vnode
}

export function h(type: unknown, ...rest: unknown[]): SvetVNode {
  const l = rest.length + 1
  const propsOrChildren = rest[0]
  let children = rest[1]
  if (l === 2) {
    if (typeof propsOrChildren === 'object' && !Array.isArray(propsOrChildren)) {
      if (isVNode(propsOrChildren)) return createVNode(type, null, [propsOrChildren])
      return createVNode(type, propsOrChildren as Record<string, unknown> | null)
    }
    return createVNode(type, null, propsOrChildren)
  }
  if (l > 3) {
    children = rest.slice(1)
  } else if (l === 3 && isVNode(children)) {
    children = [children]
  }
  return createVNode(type, propsOrChildren as Record<string, unknown> | null | undefined, children)
}

export function withCtx(
  fn: (...args: unknown[]) => unknown,
  ctx = currentRenderingInstance,
): (...args: unknown[]) => unknown {
  if (!ctx) return fn
  const scopeId = (ctx.type as { __scopeId?: unknown } | null)?.__scopeId as string | null | undefined
  return (...args: unknown[]) => {
    const prev = setCurrentRenderingInstance(ctx)
    const prevScope = currentScopeId
    currentScopeId = scopeId ?? null
    try {
      return fn(...args)
    } finally {
      currentScopeId = prevScope
      setCurrentRenderingInstance(prev)
    }
  }
}

export function renderList(
  source: unknown,
  renderItem: (value: unknown, keyOrIndex: unknown, index?: number) => unknown,
): unknown[] {
  if (Array.isArray(source) || typeof source === 'string') {
    const items = new Array(source.length)
    for (let i = 0, l = source.length; i < l; i++) {
      items[i] = renderItem(source[i], i)
    }
    return items
  }
  if (typeof source === 'number') {
    const items = new Array(source)
    for (let i = 0; i < source; i++) {
      items[i] = renderItem(i + 1, i)
    }
    return items
  }
  if (source && typeof source === 'object') {
    if ((source as Partial<Iterable<unknown>>)[Symbol.iterator]) {
      return Array.from(source as Iterable<unknown>, (item, i) => renderItem(item, i))
    }
    const keys = Object.keys(source)
    const items = new Array(keys.length)
    for (let i = 0, l = keys.length; i < l; i++) {
      const k = keys[i]!
      items[i] = renderItem((source as Record<string, unknown>)[k], k, i)
    }
    return items
  }
  return []
}

export function renderSlot(
  slots: Record<string, unknown>,
  name: string,
  props: Record<string, unknown> = {},
  fallback?: () => unknown[],
): SvetVNode {
  const slot = slots?.[name]
  if (typeof slot === 'function') {
    const validProps: Record<string, unknown> = {}
    for (const k of Object.keys(props)) {
      if (k !== 'key') validProps[k] = props[k]
    }
    return createVNode(Fragment, { key: props.key ?? `_${name}` }, slot(validProps))
  }
  return createVNode(Fragment, { key: props.key ?? `_${name}` }, fallback ? fallback() : [])
}

export function toDisplayString(val: unknown): string {
  if (val == null) return ''
  if (Array.isArray(val) || (typeof val === 'object' && (val as object).toString === Object.prototype.toString)) {
    return JSON.stringify(val, null, 2)
  }
  return String(val)
}

export function normalizeClass(value: unknown): string {
  let res = ''
  if (typeof value === 'string') {
    res = value
  } else if (Array.isArray(value)) {
    for (let i = 0; i < value.length; i++) {
      const normalized = normalizeClass(value[i])
      if (normalized) res += `${normalized} `
    }
  } else if (value && typeof value === 'object') {
    for (const name of Object.keys(value as Record<string, unknown>)) {
      if ((value as Record<string, unknown>)[name]) res += `${name} `
    }
  }
  return res.trim()
}

export function normalizeStyle(value: unknown): Record<string, unknown> | string | undefined {
  if (!value) return undefined
  if (Array.isArray(value)) {
    const res: Record<string, unknown> = {}
    for (let i = 0; i < value.length; i++) {
      const item = value[i]
      const normalized = typeof item === 'string' ? parseStringStyle(item) : normalizeStyle(item)
      if (normalized && typeof normalized === 'object') Object.assign(res, normalized)
    }
    return res
  }
  if (typeof value === 'string' || typeof value === 'object') return value as Record<string, unknown> | string
  return undefined
}

function parseStringStyle(cssText: string): Record<string, unknown> {
  const ret: Record<string, unknown> = {}
  for (const item of cssText.split(';')) {
    if (item) {
      const idx = item.indexOf(':')
      if (idx > 0) {
        ret[item.slice(0, idx).trim()] = item.slice(idx + 1).trim()
      }
    }
  }
  return ret
}

export function mergeProps(...args: Array<Record<string, unknown> | null | undefined>): Record<string, unknown> {
  const ret: Record<string, unknown> = {}
  for (let i = 0; i < args.length; i++) {
    const toMerge = args[i]
    if (!toMerge) continue
    for (const key of Object.keys(toMerge)) {
      if (key === 'class') {
        if (ret.class !== toMerge.class) {
          ret.class = normalizeClass([ret.class, toMerge.class])
        }
      } else if (key === 'style') {
        ret.style = normalizeStyle([ret.style, toMerge.style])
      } else if (/^on[A-Z]/.test(key)) {
        const existing = ret[key]
        const incoming = toMerge[key]
        if (incoming && existing !== incoming && !(Array.isArray(existing) && existing.includes(incoming))) {
          ret[key] = existing ? ([] as unknown[]).concat(existing, incoming) : incoming
        }
      } else if (key !== '') {
        ret[key] = toMerge[key]
      }
    }
  }
  return ret
}

export {
  vModelCheckbox,
  vModelRadio,
  vModelSelect,
  vModelText,
  vShow,
  withDirectives,
  withKeys,
  withModifiers,
} from './svet-directives'
