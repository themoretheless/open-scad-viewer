/**
 * # Svet Custom & Browser DOM Renderer (`svet-renderer.ts`)
 *
 * Implements `createRenderer(options)` for elements, fragments, teleports,
 * transitions, and keyed children patching, delegating component instances
 * to `svet-component.ts` and reconciliation helpers to `svet-props.ts`.
 */
import { createComponentOperations, type SvetApp } from './svet-component'
import {
  invokeDirectiveHook,
  isSameVNodeType,
  normalizeChildrenArray,
  patchElementProps,
  patchKeyedVNodeChildren,
  resolveTeleportTarget,
  resolveTransitionChild,
  setTemplateRef,
} from './svet-props'
import {
  Comment,
  Fragment,
  Static,
  Teleport,
  Text,
  Transition,
  isVNode,
  type SvetVNode,
} from './svet-vnode'

export type { SvetApp }

export interface RendererOptions<HostNode = any, HostElement = any> {
  createElement(type: string, namespace?: string, isCustomizedBuiltIn?: string, vnodeProps?: any): HostElement
  createText(text: string): HostNode
  createComment(text: string): HostNode
  setText(node: HostNode, text: string): void
  setElementText(node: HostElement, text: string): void
  insert(el: HostNode, parent: HostElement, anchor?: HostNode | null): void
  remove(el: HostNode): void
  parentNode(node: HostNode): HostElement | null
  nextSibling(node: HostNode): HostNode | null
  patchProp(el: HostElement, key: string, prevValue: any, nextValue: any, namespace?: string, parentComponent?: any): void
  setScopeId?(el: HostElement, id: string): void
  insertStaticContent?(content: string, parent: HostElement, anchor: HostNode | null, namespace?: string): [HostNode, HostNode]
  querySelector?(selector: string): HostElement | null
}

export function createRenderer<HostNode = any, HostElement extends HostNode = HostNode>(
  options: RendererOptions<HostNode, HostElement>,
) {
  const {
    createElement: hostCreateElement,
    createText: hostCreateText,
    createComment: hostCreateComment,
    setText: hostSetText,
    setElementText: hostSetElementText,
    insert: hostInsert,
    remove: hostRemove,
    parentNode: hostParentNode,
    nextSibling: hostNextSibling,
    patchProp: hostPatchProp,
    setScopeId: hostSetScopeId,
    insertStaticContent: hostInsertStaticContent,
  } = options

  const { mountComponent, updateComponent, unmountComponent, createAppAPI } =
    createComponentOperations<HostNode, HostElement>({
      patch,
      unmount,
      getFirstHostNode,
      getNextHostNode,
      hostParentNode,
      querySelector: options.querySelector,
      render,
    })

  function patch(
    n1: SvetVNode | null,
    n2: SvetVNode,
    container: HostElement,
    anchor: HostNode | null = null,
    namespace?: string,
  ): void {
    if (n1 === n2) return
    if (n1 && !isSameVNodeType(n1, n2)) {
      anchor = getNextHostNode(n1)
      unmount(n1)
      n1 = null
    }

    const { type } = n2
    switch (type) {
      case Text:
        processText(n1, n2, container, anchor)
        break
      case Comment:
        processComment(n1, n2, container, anchor)
        break
      case Static:
        processStatic(n1, n2, container, anchor, namespace)
        break
      case Fragment:
        processFragment(n1, n2, container, anchor, namespace)
        break
      case Teleport:
        processTeleport(n1, n2, container, anchor, namespace)
        break
      case Transition:
        processTransition(n1, n2, container, anchor, namespace)
        break
      default:
        if (typeof type === 'string') {
          processElement(n1, n2, container, anchor, namespace)
        } else if (typeof type === 'object' || typeof type === 'function') {
          if (n1 == null) {
            mountComponent(n2, container, anchor, namespace)
          } else {
            updateComponent(n1, n2)
          }
        }
    }

    if (n2.ref != null) {
      let refValue = n2.component ? (n2.component.exposed ?? n2.component.ctx) : n2.el
      if (refValue && typeof refValue === 'object' && '__svet_asyncInnerRef' in refValue) {
        refValue = refValue.__svet_asyncInnerRef
      }
      setTemplateRef(n2.ref, refValue)
    } else if (n1 && n1.ref != null) {
      setTemplateRef(n1.ref, null)
    }
  }

  function processText(n1: SvetVNode | null, n2: SvetVNode, container: HostElement, anchor: HostNode | null): void {
    if (n1 == null) {
      n2.el = hostCreateText(n2.children as string)
      hostInsert(n2.el, container, anchor)
    } else {
      const el = (n2.el = n1.el!)
      if (n2.children !== n1.children) {
        hostSetText(el, n2.children as string)
      }
    }
  }

  function processComment(n1: SvetVNode | null, n2: SvetVNode, container: HostElement, anchor: HostNode | null): void {
    if (n1 == null) {
      n2.el = hostCreateComment((n2.children as string) || '')
      hostInsert(n2.el, container, anchor)
    } else {
      n2.el = n1.el
    }
  }

  function processStatic(
    n1: SvetVNode | null,
    n2: SvetVNode,
    container: HostElement,
    anchor: HostNode | null,
    namespace?: string,
  ): void {
    if (n1 == null) {
      if (hostInsertStaticContent) {
        const [first, last] = hostInsertStaticContent(String(n2.children ?? ''), container, anchor, namespace)
        n2.el = first
        n2.anchor = last
      } else {
        n2.el = hostCreateText(String(n2.children ?? ''))
        hostInsert(n2.el, container, anchor)
      }
    } else {
      n2.el = n1.el
      n2.anchor = n1.anchor
    }
  }

  function processFragment(
    n1: SvetVNode | null,
    n2: SvetVNode,
    container: HostElement,
    anchor: HostNode | null,
    namespace?: string,
  ): void {
    const fragmentStartAnchor = (n2.el = n1 ? n1.el : hostCreateText(''))
    const fragmentEndAnchor = (n2.anchor = n1 ? n1.anchor : hostCreateText(''))
    if (n1 == null) {
      hostInsert(fragmentStartAnchor, container, anchor)
      hostInsert(fragmentEndAnchor, container, anchor)
      mountChildren(n2, container, fragmentEndAnchor, namespace)
    } else {
      patchChildren(n1, n2, container, fragmentEndAnchor, namespace)
    }
  }

  function processTeleport(
    n1: SvetVNode | null,
    n2: SvetVNode,
    container: HostElement,
    anchor: HostNode | null,
    namespace?: string,
  ): void {
    const disabled = Boolean(n2.props?.disabled)
    if (n1 == null) {
      const start = (n2.el = hostCreateComment('teleport start'))
      const end = (n2.anchor = hostCreateComment('teleport end'))
      hostInsert(start, container, anchor)
      hostInsert(end, container, anchor)
      const target = (n2.target = resolveTeleportTarget(n2.props?.to, container, options.querySelector))
      const targetContainer = disabled ? container : target
      const targetAnchor = disabled ? end : null
      mountChildren(n2, targetContainer, targetAnchor, namespace)
    } else {
      n2.el = n1.el
      n2.anchor = n1.anchor
      const target = (n2.target = n1.target || resolveTeleportTarget(n2.props?.to, container, options.querySelector))
      const targetContainer = disabled ? container : target
      const targetAnchor = disabled ? n2.anchor : null
      patchChildren(n1, n2, targetContainer, targetAnchor, namespace)
    }
  }

  function processTransition(
    n1: SvetVNode | null,
    n2: SvetVNode,
    container: HostElement,
    anchor: HostNode | null,
    namespace?: string,
  ): void {
    const child2 = resolveTransitionChild(n2)
    ;(n2 as any)._transitionChild = child2
    if (n1 == null) {
      patch(null, child2, container, anchor, namespace)
    } else {
      const child1 = (n1 as any)._transitionChild as SvetVNode | null
      patch(child1, child2, container, anchor, namespace)
    }
    n2.el = child2.el
    n2.anchor = child2.anchor
  }

  function processElement(
    n1: SvetVNode | null,
    n2: SvetVNode,
    container: HostElement,
    anchor: HostNode | null,
    namespace?: string,
  ): void {
    const tag = n2.type as string
    const elNamespace = tag === 'svg' ? 'svg' : tag === 'math' ? 'mathml' : namespace
    if (n1 == null) {
      mountElement(n2, container, anchor, elNamespace)
    } else {
      patchElement(n1, n2, elNamespace)
    }
  }

  function mountElement(
    vnode: SvetVNode,
    container: HostElement,
    anchor: HostNode | null,
    namespace?: string,
  ): void {
    const el = (vnode.el = hostCreateElement(vnode.type as string, namespace, undefined, vnode.props))
    if (vnode.scopeId && hostSetScopeId) {
      hostSetScopeId(el, vnode.scopeId)
    }

    if (typeof vnode.children === 'string' || typeof vnode.children === 'number') {
      hostSetElementText(el, String(vnode.children))
    } else if (Array.isArray(vnode.children)) {
      mountChildren(vnode, el, null, vnode.type === 'foreignObject' ? undefined : namespace)
    }

    if (vnode.dirs) invokeDirectiveHook(vnode, null, 'created')

    if (vnode.props) {
      for (const key of Object.keys(vnode.props)) {
        if (key === 'key' || key === 'ref' || key === 'ref_for' || key === 'ref_key') continue
        hostPatchProp(el, key, null, vnode.props[key], namespace)
      }
    }

    if (vnode.dirs) invokeDirectiveHook(vnode, null, 'beforeMount')
    hostInsert(el, container, anchor)
    if (vnode.dirs) invokeDirectiveHook(vnode, null, 'mounted')
  }

  function patchElement(n1: SvetVNode, n2: SvetVNode, namespace?: string): void {
    const el = (n2.el = n1.el!)
    const patchFlag = n2.patchFlag ?? 0

    if (n2.dirs) invokeDirectiveHook(n2, n1, 'beforeUpdate')

    patchElementProps(el, n1, n2, namespace, hostPatchProp)

    const childNs = n2.type === 'foreignObject' ? undefined : namespace
    if (patchFlag === 1 && (typeof n2.children === 'string' || typeof n2.children === 'number')) {
      if (n1.children !== n2.children) {
        hostSetElementText(el, String(n2.children))
      }
    } else if (patchFlag !== -1) {
      patchChildren(n1, n2, el, null, childNs)
    }

    if (n2.dirs) invokeDirectiveHook(n2, n1, 'updated')
  }

  function mountChildren(
    vnode: SvetVNode,
    container: HostElement,
    anchor: HostNode | null,
    namespace?: string,
  ): void {
    const list = normalizeChildrenArray(vnode.children)
    vnode.children = list
    for (let i = 0; i < list.length; i++) {
      patch(null, list[i], container, anchor, namespace)
    }
  }

  function patchChildren(
    n1: SvetVNode,
    n2: SvetVNode,
    container: HostElement,
    anchor: HostNode | null,
    namespace?: string,
  ): void {
    const c1 = n1.children
    const c2 = n2.children

    if (typeof c2 === 'string' || typeof c2 === 'number') {
      if (Array.isArray(c1)) {
        unmountChildren(c1)
      }
      if (c2 !== c1) {
        hostSetElementText(container, String(c2))
      }
      return
    }

    if (Array.isArray(c2)) {
      const nextList = normalizeChildrenArray(c2)
      n2.children = nextList
      if (Array.isArray(c1)) {
        patchKeyedVNodeChildren(
          c1 as SvetVNode[],
          nextList,
          container,
          anchor,
          namespace,
          patch,
          unmount,
          moveVNode,
          getFirstHostNode,
        )
      } else {
        if (typeof c1 === 'string' || typeof c1 === 'number') {
          hostSetElementText(container, '')
        }
        for (let i = 0; i < nextList.length; i++) {
          patch(null, nextList[i], container, anchor, namespace)
        }
      }
    } else {
      if (Array.isArray(c1)) {
        unmountChildren(c1)
      } else if (typeof c1 === 'string' || typeof c1 === 'number') {
        hostSetElementText(container, '')
      }
    }
  }

  function moveVNode(vnode: SvetVNode, container: HostElement, anchor: HostNode | null): void {
    const { type } = vnode
    if (type === Fragment) {
      hostInsert(vnode.el, container, anchor)
      const children = vnode.children as SvetVNode[]
      if (Array.isArray(children)) {
        for (let i = 0; i < children.length; i++) {
          moveVNode(children[i], container, anchor)
        }
      }
      hostInsert(vnode.anchor, container, anchor)
      return
    }
    if (type === Teleport) {
      hostInsert(vnode.el, container, anchor)
      hostInsert(vnode.anchor, container, anchor)
      return
    }
    if (type === Transition) {
      const child = (vnode as any)._transitionChild as SvetVNode | undefined
      if (child) moveVNode(child, container, anchor)
      return
    }
    if (vnode.component) {
      if (vnode.component.subTree) moveVNode(vnode.component.subTree, container, anchor)
      return
    }
    if (vnode.el) {
      hostInsert(vnode.el, container, anchor)
    }
  }

  function getFirstHostNode(vnode: SvetVNode | null): HostNode | null {
    if (!vnode) return null
    if (vnode.component) return getFirstHostNode(vnode.component.subTree)
    if (vnode.type === Transition) return getFirstHostNode((vnode as any)._transitionChild)
    return vnode.el
  }

  function getNextHostNode(vnode: SvetVNode): HostNode | null {
    if (vnode.component) {
      return vnode.component.subTree ? getNextHostNode(vnode.component.subTree) : null
    }
    if (vnode.type === Fragment || vnode.type === Teleport || vnode.type === Static) {
      return vnode.anchor ? hostNextSibling(vnode.anchor) : null
    }
    if (vnode.type === Transition) {
      const child = (vnode as any)._transitionChild
      return child ? getNextHostNode(child) : null
    }
    return vnode.el ? hostNextSibling(vnode.el) : null
  }

  function unmount(vnode: SvetVNode): void {
    if (vnode.ref != null) {
      setTemplateRef(vnode.ref, null)
    }

    if (vnode.component) {
      unmountComponent(vnode.component)
      return
    }

    const { type } = vnode
    if (type === Fragment || type === Teleport) {
      if (Array.isArray(vnode.children)) unmountChildren(vnode.children)
      if (vnode.el) hostRemove(vnode.el)
      if (vnode.anchor) hostRemove(vnode.anchor)
      return
    }
    if (type === Transition) {
      const child = (vnode as any)._transitionChild
      if (child) unmount(child)
      return
    }

    if (vnode.dirs) invokeDirectiveHook(vnode, null, 'beforeUnmount')
    if (Array.isArray(vnode.children)) {
      unmountChildren(vnode.children)
    }
    if (vnode.el) {
      hostRemove(vnode.el)
    }
    if (vnode.dirs) invokeDirectiveHook(vnode, null, 'unmounted')
  }

  function unmountChildren(children: SvetVNode[]): void {
    for (let i = 0; i < children.length; i++) {
      if (isVNode(children[i])) unmount(children[i])
    }
  }

  function render(vnode: SvetVNode | null, container: HostElement): void {
    const prevVNode = (container as any)._vnode as SvetVNode | null | undefined
    if (vnode == null) {
      if (prevVNode) {
        unmount(prevVNode)
      }
    } else {
      patch(prevVNode || null, vnode, container, null)
    }
    ;(container as any)._vnode = vnode
  }

  return {
    render,
    createApp: createAppAPI,
  }
}
