/**
 * # Svet Component Lifecycle & App Factory (`svet-component.ts`)
 *
 * Component mounting, reactive subtree updates, unmounting, and `createAppAPI`
 * powered by Svet's fine-grained `effect()` and `effectScope()`.
 */
import { effect, untrack } from './svet'
import {
  camelize,
  createRefUnwrapProxy,
  hyphenate,
  normalizeVNode,
  setTemplateRef,
  splitPropsAndAttrs,
  syncReactiveRecord,
  toHandlerKey,
} from './svet-props'
import {
  effectScope,
  enterComponentSetup,
  exitComponentSetup,
  isRef,
  queuePostFlushCb,
  shallowReactive,
} from './svet-reactivity'
import {
  Comment,
  Fragment,
  Text,
  createCommentVNode,
  createVNode,
  mergeProps,
  setCurrentInstance,
  setCurrentRenderingInstance,
  withScopeId,
  type SvetComponentInstance,
  type SvetVNode,
} from './svet-vnode'

export interface SvetApp<HostElement = any> {
  readonly _component: any
  readonly _props: Record<string, any> | null
  _container: HostElement | null
  mount(rootContainer: HostElement | string): any
  unmount(): void
  use(plugin: any, ...options: any[]): this
  provide(key: any, value: any): this
  component(name: string, component?: any): any
  config: Record<string, any>
}

export interface ComponentRendererCallbacks<HostNode = any, HostElement = any> {
  patch(
    n1: SvetVNode | null,
    n2: SvetVNode,
    container: HostElement,
    anchor: HostNode | null,
    namespace?: string,
  ): void
  unmount(vnode: SvetVNode): void
  getFirstHostNode(vnode: SvetVNode | null): HostNode | null
  getNextHostNode(vnode: SvetVNode): HostNode | null
  hostParentNode(node: HostNode): HostElement | null
  querySelector?(selector: string): HostElement | null
  render(vnode: SvetVNode | null, container: HostElement): void
}

let uidCounter = 0

function emitComponentEvent(instance: SvetComponentInstance, event: string, ...args: any[]): void {
  const rawProps = instance.vnode.props || {}
  let handler =
    rawProps[toHandlerKey(event)] ||
    rawProps[toHandlerKey(camelize(event))] ||
    rawProps[toHandlerKey(hyphenate(event))]
  if (!handler && event.startsWith('update:')) {
    const propName = event.slice(7)
    handler =
      rawProps[`onUpdate:${propName}`] ||
      rawProps[`onUpdate:${camelize(propName)}`] ||
      rawProps[`onUpdate:${hyphenate(propName)}`]
  }
  if (typeof handler === 'function') {
    handler(...args)
  } else if (Array.isArray(handler)) {
    for (const fn of handler) {
      if (typeof fn === 'function') fn(...args)
    }
  }
}

export function createComponentOperations<HostNode = any, HostElement = HostNode>(
  cb: ComponentRendererCallbacks<HostNode, HostElement>,
) {
  function mountComponent(
    initialVNode: SvetVNode,
    container: HostElement,
    anchor: HostNode | null,
    namespace?: string,
  ): void {
    const type = initialVNode.type
    let instance!: SvetComponentInstance

    const emitFn = (event: string, ...args: any[]) => emitComponentEvent(instance, event, ...args)
    const { propsValues, attrsValues } = splitPropsAndAttrs(type, initialVNode.props, emitFn)

    const slots: Record<string, any> =
      initialVNode.children && typeof initialVNode.children === 'object' && !Array.isArray(initialVNode.children)
        ? initialVNode.children
        : initialVNode.children != null
          ? { default: () => (Array.isArray(initialVNode.children) ? initialVNode.children : [initialVNode.children]) }
          : {}

    instance = {
      uid: ++uidCounter,
      vnode: initialVNode,
      type,
      props: shallowReactive(propsValues),
      attrs: shallowReactive(attrsValues),
      slots: shallowReactive(slots),
      setupState: {},
      exposed: null,
      render: null,
      renderCache: [],
      ctx: {},
      subTree: null,
      isMounted: false,
      isUnmounted: false,
      bm: [],
      m: [],
      bu: [],
      u: [],
      bum: [],
      um: [],
      stopEffect: null,
      update: null,
      emit: emitFn,
    }
    initialVNode.component = instance

    instance.ctx = new Proxy(instance, {
      get(target, key) {
        if (key === '$attrs') return target.attrs
        if (key === '$slots') return target.slots
        if (key === '$emit') return target.emit
        if (key === '$props') return target.props
        if (target.setupState && key in target.setupState) {
          const val = target.setupState[key]
          return isRef(val) ? val.value : val
        }
        if (key in target.props) return target.props[key as string]
        return undefined
      },
      set(target, key, value) {
        if (target.setupState && key in target.setupState) {
          const cur = target.setupState[key]
          if (isRef(cur)) cur.value = value
          else target.setupState[key] = value
          return true
        }
        return false
      },
    })

    const exposeProxy = (exp?: any) => {
      instance.exposed = exp && typeof exp === 'object' ? createRefUnwrapProxy(exp) : (exp || {})
    }

    const scope = effectScope(true)
    scope.run(() => {
      const prevInst = setCurrentInstance(instance)
      enterComponentSetup()
      try {
        if (typeof type === 'function') {
          instance.render = () => type(instance.props, {
            attrs: instance.attrs,
            slots: instance.slots,
            emit: instance.emit,
            expose: exposeProxy,
          })
        } else if (type.setup) {
          const setupResult = type.setup(instance.props, {
            attrs: instance.attrs,
            slots: instance.slots,
            emit: instance.emit,
            expose: exposeProxy,
          })
          if (typeof setupResult === 'function') {
            instance.render = setupResult
          } else if (setupResult && typeof setupResult === 'object') {
            instance.setupState = createRefUnwrapProxy(setupResult)
          }
        }
        if (!instance.render && type.render) {
          instance.render = type.render
        }
      } finally {
        setCurrentInstance(prevInst)
        exitComponentSetup()
      }

      const renderComponentRoot = (): SvetVNode => {
        if (!instance.render) return createCommentVNode('empty')
        const prevRendering = setCurrentRenderingInstance(instance)
        const compScopeId = (type as any).__scopeId as string | undefined
        const invokeRender = () =>
          instance.render!.call(
            instance.ctx,
            instance.ctx,
            instance.renderCache,
            instance.props,
            instance.setupState,
            instance.ctx,
            instance.ctx,
          )
        let rawSubTree: any
        try {
          rawSubTree = compScopeId ? withScopeId(compScopeId)(invokeRender)() : invokeRender()
        } finally {
          setCurrentRenderingInstance(prevRendering)
        }
        const subTree = normalizeVNode(rawSubTree)
        const inheritAttrs = type.inheritAttrs !== false
        if (inheritAttrs && subTree.type !== Fragment && subTree.type !== Comment && subTree.type !== Text) {
          const attrKeys = Object.keys(instance.attrs)
          if (attrKeys.length > 0) {
            subTree.props = mergeProps(instance.attrs, subTree.props)
          }
        }
        if (initialVNode.scopeId && !subTree.scopeId) {
          subTree.scopeId = initialVNode.scopeId
        }
        if (compScopeId && subTree.scopeId !== compScopeId) {
          subTree.scopeId = compScopeId
        }
        return subTree
      }

      let updateQueued = false
      const runComponentUpdate = () => {
        updateQueued = false
        if (!instance.isUnmounted) handle.run()
      }
      const handle = effect(() => {
        updateQueued = false
        if (instance.isUnmounted) return
        if (!instance.isMounted) {
          for (const fn of instance.bm) fn()
          const subTree = (instance.subTree = renderComponentRoot())
          untrack(() => cb.patch(null, subTree, container, anchor, namespace))
          initialVNode.el = subTree.el
          initialVNode.anchor = subTree.anchor
          instance.isMounted = true
          for (const fn of instance.m) {
            queuePostFlushCb(fn)
          }
        } else {
          for (const fn of instance.bu) fn()
          const prevTree = instance.subTree!
          const nextTree = (instance.subTree = renderComponentRoot())
          const firstHost = cb.getFirstHostNode(prevTree)
          const currentContainer = (firstHost ? cb.hostParentNode(firstHost) : null) ?? container
          const currentAnchor = cb.getNextHostNode(prevTree) ?? anchor
          untrack(() => cb.patch(prevTree, nextTree, currentContainer, currentAnchor, namespace))
          instance.vnode.el = nextTree.el
          instance.vnode.anchor = nextTree.anchor
          if (instance.vnode.ref != null) {
            setTemplateRef(instance.vnode.ref, instance.exposed ?? instance.ctx)
          }
          for (const fn of instance.u) {
            queuePostFlushCb(fn)
          }
        }
      }, {
        sync: false,
        scheduler: () => {
          if (instance.isUnmounted || updateQueued) return
          updateQueued = true
          queuePostFlushCb(runComponentUpdate)
        },
      })
      instance.update = () => handle.run()

      instance.stopEffect = () => {
        handle.dispose()
        scope.stop()
      }
    })
  }

  function updateComponent(n1: SvetVNode, n2: SvetVNode): void {
    const instance = (n2.component = n1.component!)
    instance.vnode = n2

    const { propsValues, attrsValues } = splitPropsAndAttrs(instance.type, n2.props, instance.emit)

    const patchFlag = n2.patchFlag ?? 0
    const dynamicSlots = (patchFlag & 1024) !== 0
    let changed = dynamicSlots
    untrack(() => {
      if (syncReactiveRecord(instance.props, propsValues)) changed = true
      if (syncReactiveRecord(instance.attrs, attrsValues)) changed = true
      if (n2.children && typeof n2.children === 'object' && !Array.isArray(n2.children)) {
        if (syncReactiveRecord(instance.slots, n2.children as Record<string, any>, dynamicSlots)) {
          changed = true
        }
      }
    })
    if (changed && instance.update) {
      instance.update()
    }
    n2.el = instance.vnode.el
    n2.anchor = instance.vnode.anchor
  }

  function unmountComponent(instance: SvetComponentInstance): void {
    if (instance.isUnmounted) return
    for (const fn of instance.bum) {
      try { fn() } catch { /* ignore */ }
    }
    if (instance.stopEffect) {
      instance.stopEffect()
      instance.stopEffect = null
    }
    if (instance.subTree) {
      cb.unmount(instance.subTree)
    }
    instance.isUnmounted = true
    for (const fn of instance.um) {
      try { fn() } catch { /* ignore */ }
    }
  }

  function createAppAPI(rootComponent: any, rootProps: Record<string, any> | null = null): SvetApp<HostElement> {
    let isMounted = false
    const app: SvetApp<HostElement> = {
      _component: rootComponent,
      _props: rootProps,
      _container: null,
      config: { globalProperties: {} },
      use(plugin: any, ...pluginArgs: any[]) {
        if (plugin && typeof plugin.install === 'function') {
          plugin.install(app, ...pluginArgs)
        } else if (typeof plugin === 'function') {
          plugin(app, ...pluginArgs)
        }
        return app
      },
      provide() {
        return app
      },
      component() {
        return app
      },
      mount(rootContainer: HostElement | string) {
        if (isMounted) return
        const container =
          typeof rootContainer === 'string'
            ? (cb.querySelector
                ? cb.querySelector(rootContainer)
                : (typeof document !== 'undefined' ? (document.querySelector(rootContainer) as unknown as HostElement) : null))
            : rootContainer
        if (!container) return
        const vnode = createVNode(rootComponent, rootProps)
        app._container = container
        cb.render(vnode, container)
        isMounted = true
        return vnode.component?.exposed ?? vnode.component?.ctx
      },
      unmount() {
        if (isMounted && app._container) {
          cb.render(null, app._container)
          isMounted = false
        }
      },
    }
    return app
  }

  return {
    mountComponent,
    updateComponent,
    unmountComponent,
    createAppAPI,
  }
}
