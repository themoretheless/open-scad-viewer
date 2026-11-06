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
  value: any
  oldValue?: any
  arg?: string
  modifiers: Record<string, boolean>
}

export interface Directive {
  created?: (el: any, binding: DirectiveBinding, vnode: SvetVNode) => void
  beforeMount?: (el: any, binding: DirectiveBinding, vnode: SvetVNode) => void
  mounted?: (el: any, binding: DirectiveBinding, vnode: SvetVNode) => void
  beforeUpdate?: (el: any, binding: DirectiveBinding, vnode: SvetVNode, prevVNode: SvetVNode) => void
  updated?: (el: any, binding: DirectiveBinding, vnode: SvetVNode, prevVNode: SvetVNode) => void
  beforeUnmount?: (el: any, binding: DirectiveBinding, vnode: SvetVNode) => void
  unmounted?: (el: any, binding: DirectiveBinding, vnode: SvetVNode) => void
}

export interface SvetVNode {
  readonly __v_isVNode: true
  type: any
  props: Record<string, any> | null
  children: any
  key: any
  ref: any
  el: any
  anchor: any
  target: any
  component: SvetComponentInstance | null
  dirs: DirectiveBinding[] | null
  scopeId: string | null
  patchFlag?: number
  dynamicProps?: string[] | null
}

export interface SvetComponentInstance {
  uid: number
  vnode: SvetVNode
  type: any
  props: Record<string, any>
  attrs: Record<string, any>
  slots: Record<string, any>
  setupState: any
  exposed: Record<string, any> | null
  render: ((ctx: any, cache: any[], ...args: any[]) => SvetVNode | null) | null
  renderCache: any[]
  ctx: Record<string, any>
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
  emit: (event: string, ...args: any[]) => void
}

export type VNode = SvetVNode
export type VNodeChild = SvetVNode | string | number | boolean | null | undefined | VNodeChild[]
export type Slots = Readonly<Record<string, ((...args: any[]) => VNode[]) | undefined>>
export type ComponentPublicInstance<P = {}, B = {}> = { $props: P; $attrs: Record<string, any>; $slots: Slots; $emit: (event: string, ...args: any[]) => void } & P & B
export type FunctionalComponent<P = {}, E = any> = {
  (props: P, ctx: { attrs: any; slots: any; emit: E }): any
  props?: any
  emits?: any
  inheritAttrs?: boolean
  displayName?: string
}

export type Component<P = any> =
  | {
      __name?: string
      name?: string
      props?: any
      emits?: any
      inheritAttrs?: boolean
      setup?: (props: P, ctx: { attrs: any; slots: any; emit: any; expose: (exposed?: any) => void }) => any
      render?: (ctx: any, cache: any[]) => any
      ssrRender?: (...args: any[]) => any
      [key: string]: any
    }
  | ((props: P, ctx: any) => any)

export type DefineComponent<PropsOrPropOptions = {}, RawBindings = {}, D = any> = Component<PropsOrPropOptions> & {
  new (...args: any[]): { $props: PropsOrPropOptions } & RawBindings & D
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
  const filename = (options as any)?.__file || (options as any)?.__name
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
    const stem = base.replace(/\.svet$/, '')
    if (stem) asyncResolvedCache.set(stem, comp)
  }
}

function getAsyncCacheKey(loaderStr: string): string {
  const match = loaderStr.match(/\/([^/?'"`()]+?)(?:\.(?:svet|ts|js))?(?:\?[^'"`()]*)?['"`)]/)
  return match ? match[1] : loaderStr
}

export function defineAsyncComponent<T extends Component = Component>(
  source: (() => Promise<T | { default: T }>) | { loader: () => Promise<T | { default: T }> },
): Component {
  const loader = typeof source === 'function' ? source : source.loader
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
      const innerRef = shallowRef<any>(null)
      expose({
        __svet_asyncInnerRef: innerRef,
      })
      const inst = currentInstance
      if (!resolvedComp) {
        if (!loadPromise) {
          loadPromise = Promise.resolve(loader())
            .then(mod => {
              const comp = (mod && (mod as any).default) || (mod as Component)
              resolvedComp = comp
              if (comp) asyncResolvedCache.set(cacheKey, comp)
              return comp
            })
            .catch(() => null as any)
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
        if (!comp) return createCommentVNode('async')
        return createVNode(comp, { ...attrs, ref: innerRef }, slots)
      }
    },
  })
}

let currentScopeId: string | null = null
export function withScopeId(id: string): <T extends (...args: any[]) => any>(fn: T) => T {
  return (<T extends (...args: any[]) => any>(fn: T): T => {
    return ((...args: any[]) => {
      const prev = currentScopeId
      currentScopeId = id
      try {
        return fn(...args)
      } finally {
        currentScopeId = prev
      }
    }) as unknown as T
  })
}

export function isVNode(value: unknown): value is SvetVNode {
  return Boolean(value && (value as any).__v_isVNode === true)
}

export function openBlock(_disableTracking = false): void {
  // Svet uses fine-grained signals and fast VNode diffing; block tracking is a no-op
}

export function createVNode(
  type: any,
  props?: Record<string, any> | null,
  children?: any,
  patchFlag?: number,
  dynamicProps?: string[] | null,
): SvetVNode {
  if (!type) type = Comment
  if (props && isReactive(props)) trackDeep(props)
  const normalizedProps = props ? { ...props } : null
  let key: any = null
  let refProp: any = null
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
  ;(vnode as any).staticCount = numberOfNodes
  return vnode
}

export function h(type: any, propsOrChildren?: any, children?: any): SvetVNode {
  const l = arguments.length
  if (l === 2) {
    if (typeof propsOrChildren === 'object' && !Array.isArray(propsOrChildren)) {
      if (isVNode(propsOrChildren)) return createVNode(type, null, [propsOrChildren])
      return createVNode(type, propsOrChildren)
    }
    return createVNode(type, null, propsOrChildren)
  }
  if (l > 3) {
    children = Array.prototype.slice.call(arguments, 2)
  } else if (l === 3 && isVNode(children)) {
    children = [children]
  }
  return createVNode(type, propsOrChildren, children)
}

export function withCtx(fn: (...args: any[]) => any, ctx = currentRenderingInstance): (...args: any[]) => any {
  if (!ctx) return fn
  const scopeId = (ctx.type as any)?.__scopeId as string | null | undefined
  return (...args: any[]) => {
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
  source: any,
  renderItem: (value: any, keyOrIndex: any, index?: number) => any,
): any[] {
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
    if (source[Symbol.iterator as any]) {
      return Array.from(source as Iterable<any>, (item, i) => renderItem(item, i))
    }
    const keys = Object.keys(source)
    const items = new Array(keys.length)
    for (let i = 0, l = keys.length; i < l; i++) {
      const k = keys[i]
      items[i] = renderItem(source[k], k, i)
    }
    return items
  }
  return []
}

export function renderSlot(
  slots: Record<string, any>,
  name: string,
  props: Record<string, any> = {},
  fallback?: () => any[],
): SvetVNode {
  const slot = slots?.[name]
  if (typeof slot === 'function') {
    const validProps: Record<string, any> = {}
    for (const k of Object.keys(props)) {
      if (k !== 'key') validProps[k] = props[k]
    }
    return createVNode(Fragment, { key: props.key ?? `_${name}` }, slot(validProps))
  }
  return createVNode(Fragment, { key: props.key ?? `_${name}` }, fallback ? fallback() : [])
}

export function toDisplayString(val: unknown): string {
  if (val == null) return ''
  if (Array.isArray(val) || (typeof val === 'object' && (val as any).toString === Object.prototype.toString)) {
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

export function normalizeStyle(value: unknown): Record<string, any> | string | undefined {
  if (!value) return undefined
  if (Array.isArray(value)) {
    const res: Record<string, any> = {}
    for (let i = 0; i < value.length; i++) {
      const item = value[i]
      const normalized = typeof item === 'string' ? parseStringStyle(item) : normalizeStyle(item)
      if (normalized && typeof normalized === 'object') Object.assign(res, normalized)
    }
    return res
  }
  if (typeof value === 'string' || typeof value === 'object') return value as any
  return undefined
}

function parseStringStyle(cssText: string): Record<string, any> {
  const ret: Record<string, any> = {}
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

export function mergeProps(...args: Array<Record<string, any> | null | undefined>): Record<string, any> {
  const ret: Record<string, any> = {}
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
          ret[key] = existing ? [].concat(existing as any, incoming as any) : incoming
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
