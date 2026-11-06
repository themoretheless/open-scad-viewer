/**
 * # Svet Renderer Props, Directives & VNode Normalization (`svet-props.ts`)
 *
 * VNode normalization, template ref assignment, directive hook invocation,
 * and component props/attrs/emits splitting for the Svet renderer.
 */
import { untrack } from './svet'
import { isRef } from './svet-reactivity'
import {
  Fragment,
  createCommentVNode,
  createTextVNode,
  createVNode,
  isVNode,
  type SvetComponentInstance,
  type SvetVNode,
} from './svet-vnode'

export function camelize(str: string): string {
  return str.replace(/-(\w)/g, (_, c) => (c ? c.toUpperCase() : ''))
}

export function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1)
}

export function toHandlerKey(str: string): string {
  return str ? `on${capitalize(str)}` : ''
}

export function cloneVNode(vnode: SvetVNode): SvetVNode {
  return {
    ...vnode,
    props: vnode.props ? { ...vnode.props } : null,
    children: Array.isArray(vnode.children) ? vnode.children.slice() : vnode.children,
  }
}

export function normalizeVNode(child: any): SvetVNode {
  if (child == null || typeof child === 'boolean') {
    return createCommentVNode()
  }
  if (Array.isArray(child)) {
    return createVNode(Fragment, null, child.slice())
  }
  if (isVNode(child)) {
    return child.el === null ? child : cloneVNode(child)
  }
  return createTextVNode(String(child))
}

export function normalizeChildrenArray(children: any): SvetVNode[] {
  if (children == null) return []
  const rawList = Array.isArray(children) ? children : [children]
  const out: SvetVNode[] = []
  for (let i = 0; i < rawList.length; i++) {
    out.push(normalizeVNode(rawList[i]))
  }
  return out
}

export function isSameVNodeType(n1: SvetVNode, n2: SvetVNode): boolean {
  return n1.type === n2.type && n1.key === n2.key
}

export function setTemplateRef(rawRef: any, value: any): void {
  if (!rawRef) return
  const refTarget = rawRef.ref !== undefined ? rawRef.ref : rawRef
  const owner = rawRef.owner as SvetComponentInstance | null | undefined
  if (typeof refTarget === 'function') {
    refTarget(value, owner?.setupState || {})
  } else if (isRef(refTarget)) {
    refTarget.value = value
  } else if (typeof refTarget === 'string' && owner?.setupState) {
    const targetRef = owner.setupState[refTarget]
    if (isRef(targetRef)) {
      targetRef.value = value
    } else if (refTarget in owner.setupState) {
      owner.setupState[refTarget] = value
    }
  }
}

export function invokeDirectiveHook(
  vnode: SvetVNode,
  prevVNode: SvetVNode | null,
  name: 'created' | 'beforeMount' | 'mounted' | 'beforeUpdate' | 'updated' | 'beforeUnmount' | 'unmounted',
): void {
  const bindings = vnode.dirs
  if (!bindings) return
  for (let i = 0; i < bindings.length; i++) {
    const binding = bindings[i]
    if (prevVNode?.dirs?.[i]) {
      binding.oldValue = prevVNode.dirs[i].value
    }
    const hook = binding.dir[name]
    if (hook) {
      untrack(() => hook(vnode.el, binding, vnode, prevVNode!))
    }
  }
}

export function normalizePropsOptions(rawOptions: any): {
  declaredKeys: Set<string>
  defaults: Record<string, any>
  booleanKeys: Set<string>
} {
  const declaredKeys = new Set<string>()
  const defaults: Record<string, any> = {}
  const booleanKeys = new Set<string>()

  if (!rawOptions) return { declaredKeys, defaults, booleanKeys }
  if (Array.isArray(rawOptions)) {
    for (const k of rawOptions) {
      if (typeof k === 'string') declaredKeys.add(camelize(k))
    }
    return { declaredKeys, defaults, booleanKeys }
  }
  if (typeof rawOptions === 'object') {
    for (const key of Object.keys(rawOptions)) {
      const camelKey = camelize(key)
      declaredKeys.add(camelKey)
      const opt = rawOptions[key]
      if (opt && typeof opt === 'object' && !Array.isArray(opt)) {
        if ('default' in opt) defaults[camelKey] = opt.default
        const types = Array.isArray(opt.type) ? opt.type : [opt.type]
        if (types.includes(Boolean)) booleanKeys.add(camelKey)
      } else if (opt === Boolean || (Array.isArray(opt) && opt.includes(Boolean))) {
        booleanKeys.add(camelKey)
      }
    }
  }
  return { declaredKeys, defaults, booleanKeys }
}

export function normalizeEmitsOptions(rawEmits: any): Set<string> {
  const declared = new Set<string>()
  if (!rawEmits) return declared
  if (Array.isArray(rawEmits)) {
    for (const e of rawEmits) declared.add(e)
  } else if (typeof rawEmits === 'object') {
    for (const e of Object.keys(rawEmits)) declared.add(e)
  }
  return declared
}

export function splitPropsAndAttrs(
  type: any,
  rawProps: Record<string, any> | null,
  emitFn: (event: string, ...args: any[]) => void,
): { propsValues: Record<string, any>; attrsValues: Record<string, any> } {
  const { declaredKeys, defaults, booleanKeys } = normalizePropsOptions(type.props)
  const declaredEmits = normalizeEmitsOptions(type.emits)
  const hasDeclaredProps = Boolean(type.props)

  const propsValues: Record<string, any> = {}
  const attrsValues: Record<string, any> = {}

  for (const key of declaredKeys) {
    if (key in defaults) {
      const def = defaults[key]
      propsValues[key] = typeof def === 'function' ? def() : def
    } else if (booleanKeys.has(key)) {
      propsValues[key] = false
    } else {
      propsValues[key] = undefined
    }
  }

  if (rawProps) {
    for (const rawKey of Object.keys(rawProps)) {
      if (rawKey === 'key' || rawKey === 'ref' || rawKey === 'ref_for' || rawKey === 'ref_key') continue
      const val = rawProps[rawKey]
      const camelKey = camelize(rawKey)
      if (hasDeclaredProps) {
        if (declaredKeys.has(camelKey)) {
          if (booleanKeys.has(camelKey) && (val === '' || val === camelKey || val === rawKey)) {
            propsValues[camelKey] = true
          } else if (val === undefined && camelKey in defaults) {
            const def = defaults[camelKey]
            propsValues[camelKey] = typeof def === 'function' ? def() : def
          } else {
            propsValues[camelKey] = val
          }
        } else {
          const isUpdateListener =
            rawKey.startsWith('onUpdate:') &&
            declaredKeys.has(camelize(rawKey.slice(9)))
          if (isUpdateListener) {
            propsValues[rawKey] = val
            const camelUpdate = `onUpdate:${camelize(rawKey.slice(9))}`
            if (camelUpdate !== rawKey) propsValues[camelUpdate] = val
          }
          const isDeclaredEmit =
            isUpdateListener ||
            (rawKey.startsWith('on') &&
              (declaredEmits.has(rawKey.slice(2, 3).toLowerCase() + rawKey.slice(3)) ||
                declaredEmits.has(camelize(rawKey.slice(2, 3).toLowerCase() + rawKey.slice(3)))))
          if (!isDeclaredEmit) {
            attrsValues[rawKey] = val
          }
        }
      } else {
        propsValues[camelKey] = val
        attrsValues[rawKey] = val
      }
    }
  }

  Object.defineProperty(propsValues, '__svet_emit', {
    value: emitFn,
    enumerable: false,
    configurable: true,
  })
  return { propsValues, attrsValues }
}

export function resolveTransitionChild(vnode: SvetVNode): SvetVNode {
  const raw = vnode.children
  if (raw && typeof raw === 'object' && typeof raw.default === 'function') {
    const res = raw.default()
    if (Array.isArray(res)) {
      return res.length === 1 ? normalizeVNode(res[0]) : createVNode(Fragment, null, res)
    }
    return normalizeVNode(res)
  }
  if (Array.isArray(raw)) {
    return raw.length === 1 ? normalizeVNode(raw[0]) : createVNode(Fragment, null, raw)
  }
  return normalizeVNode(raw)
}

export function resolveTeleportTarget<HostElement = any>(
  to: any,
  fallback: HostElement,
  querySelector?: (selector: string) => HostElement | null,
): HostElement {
  if (typeof to === 'string') {
    if (querySelector) {
      return querySelector(to) ?? fallback
    }
    if (typeof document !== 'undefined' && typeof document.querySelector === 'function') {
      const found = document.querySelector(to)
      if (found) return found as unknown as HostElement
    }
  } else if (to && typeof to === 'object') {
    return to as HostElement
  }
  return fallback
}

export function patchElementProps<HostElement = any>(
  el: HostElement,
  n1: SvetVNode,
  n2: SvetVNode,
  namespace: string | undefined,
  hostPatchProp: (el: HostElement, key: string, prevValue: any, nextValue: any, namespace?: string) => void,
): void {
  const oldProps = n1.props || {}
  const newProps = n2.props || {}
  if (oldProps === newProps) return
  const patchFlag = n2.patchFlag ?? 0
  const dynamicProps = n2.dynamicProps

  if (dynamicProps && (patchFlag & 16) === 0) {
    for (let i = 0; i < dynamicProps.length; i++) {
      const key = dynamicProps[i]
      const prev = oldProps[key]
      const next = newProps[key]
      if (next !== prev || key === 'value' || key === 'checked' || key === 'selected') {
        hostPatchProp(el, key, prev, next, namespace)
      }
    }
    if (patchFlag & 2 && oldProps.class !== newProps.class) {
      hostPatchProp(el, 'class', oldProps.class, newProps.class, namespace)
    }
    if (patchFlag & 4 && oldProps.style !== newProps.style) {
      hostPatchProp(el, 'style', oldProps.style, newProps.style, namespace)
    }
    return
  }
  if (patchFlag === 2) {
    if (oldProps.class !== newProps.class) {
      hostPatchProp(el, 'class', oldProps.class, newProps.class, namespace)
    }
    return
  }
  if (patchFlag === 4) {
    if (oldProps.style !== newProps.style) {
      hostPatchProp(el, 'style', oldProps.style, newProps.style, namespace)
    }
    return
  }
  if (patchFlag === 1 || patchFlag === -1) return

  for (const key of Object.keys(newProps)) {
    if (key === 'key' || key === 'ref' || key === 'ref_for' || key === 'ref_key') continue
    const next = newProps[key]
    const prev = oldProps[key]
    if (next !== prev || key === 'value' || key === 'checked' || key === 'selected') {
      hostPatchProp(el, key, prev, next, namespace)
    }
  }
  for (const key of Object.keys(oldProps)) {
    if (key === 'key' || key === 'ref' || key === 'ref_for' || key === 'ref_key') continue
    if (!(key in newProps)) {
      hostPatchProp(el, key, oldProps[key], null, namespace)
    }
  }
}

export function patchKeyedVNodeChildren<HostNode = any, HostElement = HostNode>(
  c1: SvetVNode[],
  c2: SvetVNode[],
  container: HostElement,
  parentAnchor: HostNode | null,
  namespace: string | undefined,
  patch: (n1: SvetVNode | null, n2: SvetVNode, container: HostElement, anchor: HostNode | null, namespace?: string) => void,
  unmount: (vnode: SvetVNode) => void,
  moveVNode: (vnode: SvetVNode, container: HostElement, anchor: HostNode | null) => void,
  getFirstHostNode: (vnode: SvetVNode | null) => HostNode | null,
): void {
  let i = 0
  const l2 = c2.length
  let e1 = c1.length - 1
  let e2 = l2 - 1

  while (i <= e1 && i <= e2) {
    if (isSameVNodeType(c1[i], c2[i])) {
      patch(c1[i], c2[i], container, parentAnchor, namespace)
    } else break
    i++
  }

  while (i <= e1 && i <= e2) {
    if (isSameVNodeType(c1[e1], c2[e2])) {
      patch(c1[e1], c2[e2], container, parentAnchor, namespace)
    } else break
    e1--
    e2--
  }

  if (i > e1) {
    if (i <= e2) {
      const nextPos = e2 + 1
      const anchor = nextPos < l2 ? getFirstHostNode(c2[nextPos]) : parentAnchor
      while (i <= e2) {
        patch(null, c2[i], container, anchor, namespace)
        i++
      }
    }
  } else if (i > e2) {
    while (i <= e1) {
      unmount(c1[i])
      i++
    }
  } else {
    const s1 = i
    const s2 = i
    const keyToNewIndexMap = new Map<any, number>()
    for (i = s2; i <= e2; i++) {
      if (c2[i].key != null) keyToNewIndexMap.set(c2[i].key, i)
    }

    let patched = 0
    const toBePatched = e2 - s2 + 1
    const newIndexToOldIndexMap = new Int32Array(toBePatched)

    for (i = s1; i <= e1; i++) {
      const prevChild = c1[i]
      if (patched >= toBePatched) {
        unmount(prevChild)
        continue
      }
      let newIndex: number | undefined
      if (prevChild.key != null) {
        newIndex = keyToNewIndexMap.get(prevChild.key)
      } else {
        for (let j = s2; j <= e2; j++) {
          if (newIndexToOldIndexMap[j - s2] === 0 && isSameVNodeType(prevChild, c2[j])) {
            newIndex = j
            break
          }
        }
      }
      if (newIndex === undefined) {
        unmount(prevChild)
      } else {
        newIndexToOldIndexMap[newIndex - s2] = i + 1
        patch(prevChild, c2[newIndex], container, null, namespace)
        patched++
      }
    }

    for (i = toBePatched - 1; i >= 0; i--) {
      const nextIndex = s2 + i
      const nextChild = c2[nextIndex]
      const anchor = nextIndex + 1 < l2 ? getFirstHostNode(c2[nextIndex + 1]) : parentAnchor
      if (newIndexToOldIndexMap[i] === 0) {
        patch(null, nextChild, container, anchor, namespace)
      } else {
        moveVNode(nextChild, container, anchor)
      }
    }
  }
}
