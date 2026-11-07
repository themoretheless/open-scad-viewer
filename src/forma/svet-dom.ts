/**
 * # Svet Browser DOM & Server String Runtime (`svet-dom.ts`)
 *
 * Implements the default browser DOM host renderer (`createApp`, `createSSRApp`)
 * and deterministic HTML string serializer (`renderToString`) for Svet.
 */
import { hyphenate, isReservedPropKey } from './svet-props'
import { createRenderer, type RendererOptions, type SvetApp } from './svet-renderer'
import { Comment, Fragment, Static, Teleport, Text, Transition, isVNode, type SvetVNode } from './svet-vnode'

const SVG_NS = 'http://www.w3.org/2000/svg'
const MATH_NS = 'http://www.w3.org/1998/Math/MathML'
const XLINK_NS = 'http://www.w3.org/1999/xlink'

const BOOLEAN_ATTRS = new Set([
  'itemscope', 'allowfullscreen', 'formnovalidate', 'ismap', 'nomodule',
  'novalidate', 'readonly', 'async', 'autofocus', 'autoplay', 'controls',
  'default', 'defer', 'disabled', 'hidden', 'inert', 'loop', 'open',
  'required', 'reversed', 'scoped', 'seamless', 'checked', 'muted',
  'multiple', 'selected',
])

const VOID_TAGS = new Set([
  'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input',
  'link', 'meta', 'param', 'source', 'track', 'wbr',
])

function patchStyle(el: HTMLElement | SVGElement, prev: any, next: any): void {
  const style = el.style
  if (!style) return
  if (typeof next === 'string') {
    style.cssText = next
    return
  }
  if (next && typeof next === 'object') {
    if (prev && typeof prev === 'object') {
      for (const key of Object.keys(prev)) {
        if (!(key in next)) {
          if (key.startsWith('--')) style.setProperty(key, '')
          else (style as any)[key] = ''
        }
      }
    }
    for (const key of Object.keys(next)) {
      const val = next[key] ?? ''
      if (key.startsWith('--')) style.setProperty(key, String(val))
      else (style as any)[key] = val
    }
  } else {
    el.removeAttribute?.('style')
  }
}

function patchEvent(el: any, rawName: string, _prevValue: any, nextValue: any): void {
  const invokers = el._vei || (el._vei = {})
  let name = rawName.slice(2)
  let options: AddEventListenerOptions | undefined
  if (name.endsWith('Capture') && name.length > 7) {
    name = name.slice(0, -7)
    options = { ...(options || {}), capture: true }
  }
  if (name.endsWith('Passive') && name.length > 7) {
    name = name.slice(0, -7)
    options = { ...(options || {}), passive: true }
  }
  if (name.endsWith('Once') && name.length > 4) {
    name = name.slice(0, -4)
    options = { ...(options || {}), once: true }
  }
  const eventName = name.charAt(0).toLowerCase() + name.slice(1)
  const existingInvoker = invokers[rawName]
  if (nextValue && existingInvoker) {
    existingInvoker.value = nextValue
  } else if (nextValue) {
    const invoker = ((e: Event) => {
      const val = invoker.value
      if (Array.isArray(val)) {
        for (let i = 0; i < val.length; i++) {
          if (typeof val[i] === 'function') val[i](e)
        }
      } else if (typeof val === 'function') {
        val(e)
      }
    }) as ((e: Event) => void) & { value: any }
    invoker.value = nextValue
    invokers[rawName] = invoker
    el.addEventListener?.(eventName, invoker, options)
  } else if (existingInvoker) {
    el.removeEventListener?.(eventName, existingInvoker, options)
    invokers[rawName] = undefined
  }
}

function shouldSetAsProp(el: any, key: string, value: any, isSVG: boolean): boolean {
  if (isSVG) {
    if (key === 'innerHTML' || key === 'textContent') return true
    return false
  }
  if (key === 'spellcheck' || key === 'draggable' || key === 'translate' || key === 'form') return false
  if (key === 'list' && el.tagName === 'INPUT') return false
  if (key === 'type' && el.tagName === 'TEXTAREA') return false
  if ((key === 'width' || key === 'height') && (el.tagName === 'IMG' || el.tagName === 'VIDEO' || el.tagName === 'CANVAS' || el.tagName === 'SOURCE')) {
    return false
  }
  if (key.includes('-') || key.startsWith('aria-') || key.startsWith('data-')) return false
  return key in el
}

const domRendererOptions: RendererOptions<any, any> = {
  createElement(tag, namespace) {
    if (namespace === 'svg') return document.createElementNS(SVG_NS, tag)
    if (namespace === 'mathml') return document.createElementNS(MATH_NS, tag)
    return document.createElement(tag)
  },
  createText(text) {
    return document.createTextNode(text)
  },
  createComment(text) {
    return document.createComment(text)
  },
  setText(node, text) {
    node.nodeValue = text
  },
  setElementText(el, text) {
    el.textContent = text
  },
  insert(child, parent, anchor) {
    parent.insertBefore(child, anchor || null)
  },
  remove(child) {
    const parent = child.parentNode
    if (parent) parent.removeChild(child)
  },
  parentNode(node) {
    return node.parentNode
  },
  nextSibling(node) {
    return node.nextSibling
  },
  querySelector(selector) {
    return typeof document !== 'undefined' ? document.querySelector(selector) : null
  },
  setScopeId(el, id) {
    el.setAttribute(id, '')
  },
  insertStaticContent(content, parent, anchor, namespace) {
    const temp = namespace === 'svg' ? document.createElementNS(SVG_NS, 'svg') : document.createElement('div')
    temp.innerHTML = content
    const first = temp.firstChild!
    const last = temp.lastChild!
    while (temp.firstChild) {
      parent.insertBefore(temp.firstChild, anchor)
    }
    return [first, last]
  },
  patchProp(el, key, prevValue, nextValue, namespace) {
    const isSVG = namespace === 'svg'
    if (key === 'class') {
      const cls = nextValue || ''
      if (isSVG) el.setAttribute('class', cls)
      else el.className = cls
    } else if (key === 'style') {
      patchStyle(el, prevValue, nextValue)
    } else if (/^on[A-Z]/.test(key)) {
      if (!key.startsWith('onUpdate:')) {
        patchEvent(el, key, prevValue, nextValue)
      }
    } else if (key === 'innerHTML') {
      el.innerHTML = nextValue == null ? '' : String(nextValue)
    } else if (key === 'textContent') {
      el.textContent = nextValue == null ? '' : String(nextValue)
    } else if (key === 'value' && el.tagName !== 'PROGRESS' && !el.tagName?.includes('-')) {
      el._value = nextValue
      const newValue = nextValue == null ? '' : nextValue
      if (el.value !== newValue || el.tagName === 'OPTION') {
        el.value = newValue
      }
      if (nextValue == null) el.removeAttribute('value')
    } else if (shouldSetAsProp(el, key, nextValue, isSVG)) {
      if (nextValue === '' && typeof el[key] === 'boolean') {
        el[key] = true
      } else if (nextValue == null && typeof el[key] === 'string') {
        el[key] = ''
        el.removeAttribute(key)
      } else {
        try {
          el[key] = nextValue
        } catch { /* ignore read-only DOM prop */ }
      }
    } else {
      if (isSVG && key.startsWith('xlink:')) {
        if (nextValue == null) el.removeAttributeNS(XLINK_NS, key.slice(6))
        else el.setAttributeNS(XLINK_NS, key, String(nextValue))
        return
      }
      const isBoolean = BOOLEAN_ATTRS.has(key)
      if (nextValue == null || (isBoolean && (nextValue === false || nextValue === 'false'))) {
        el.removeAttribute(key)
      } else {
        el.setAttribute(key, isBoolean ? '' : String(nextValue))
      }
    }
  },
}

let defaultRenderer: ReturnType<typeof createRenderer> | null = null
function ensureRenderer() {
  return defaultRenderer || (defaultRenderer = createRenderer(domRendererOptions))
}

export function createApp(rootComponent: any, rootProps: Record<string, any> | null = null): SvetApp {
  return ensureRenderer().createApp(rootComponent, rootProps)
}

export function createSSRApp(rootComponent: any, rootProps: Record<string, any> | null = null): SvetApp {
  return createApp(rootComponent, rootProps)
}

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

class SSRNode {
  parent: SSRNode | null = null
  children: SSRNode[] = []
  props: Record<string, any> = {}
  style: Record<string, any> = {}
  text = ''
  value: any = undefined
  checked: boolean | undefined = undefined
  selected: boolean | undefined = undefined
  scopeIds: string[] = []
  constructor(public tag: string) {}
  get tagName() { return this.tag.toUpperCase() }
  get options() { return this.children.filter(c => c.tag === 'option') }
  addEventListener() {}
  removeEventListener() {}
  setAttribute(k: string, v: any) { this.props[k] = v }
  removeAttribute(k: string) { delete this.props[k] }
}

function serializeSSRNode(node: SSRNode): string {
  if (node.tag === '#text' || node.tag === '#static') {
    return node.tag === '#static' ? node.text : escapeHtml(node.text)
  }
  if (node.tag === '#comment') {
    return `<!--${node.text || ''}-->`
  }
  if (node.tag === '#root') {
    return node.children.map(serializeSSRNode).join('')
  }

  let attrs = ''
  for (const scopeId of node.scopeIds) {
    attrs += ` ${scopeId}`
  }

  const allProps = { ...node.props }
  if (node.value !== undefined && node.tag === 'input' && allProps.value === undefined) {
    allProps.value = node.value
  }
  if (node.checked === true && node.tag === 'input') {
    allProps.checked = true
  }
  if (node.selected === true && node.tag === 'option') {
    allProps.selected = true
  }

  for (const [key, val] of Object.entries(allProps)) {
    if (isReservedPropKey(key) || /^on[A-Z]/.test(key)) continue
    if (key === 'innerHTML' || key === 'textContent') continue
    if (key === 'style') {
      if (typeof val === 'string' && val) {
        attrs += ` style="${escapeHtml(val)}"`
      } else if (val && typeof val === 'object') {
        const parts: string[] = []
        for (const [sk, sv] of Object.entries(val)) {
          if (sv != null && sv !== '') {
            parts.push(`${hyphenate(sk)}:${sv}`)
          }
        }
        if (parts.length > 0) attrs += ` style="${escapeHtml(parts.join(';'))}"`
      }
      continue
    }
    if (val == null || val === false) continue
    if (BOOLEAN_ATTRS.has(key)) {
      if (val) attrs += ` ${key}`
    } else if (val === true) {
      attrs += ` ${key}="true"`
    } else {
      attrs += ` ${key}="${escapeHtml(String(val))}"`
    }
  }

  if (VOID_TAGS.has(node.tag)) {
    return `<${node.tag}${attrs}>`
  }

  let inner = ''
  if (allProps.innerHTML != null) {
    inner = String(allProps.innerHTML)
  } else if (allProps.textContent != null) {
    inner = escapeHtml(String(allProps.textContent))
  } else if (node.tag === 'textarea' && node.value !== undefined) {
    inner = escapeHtml(String(node.value))
  } else if (node.children.length > 0) {
    inner = node.children.map(serializeSSRNode).join('')
  } else if (node.text) {
    inner = escapeHtml(node.text)
  }

  return `<${node.tag}${attrs}>${inner}</${node.tag}>`
}

export async function renderToString(input: SvetApp | SvetVNode): Promise<string> {
  const ssrRenderer = createRenderer<SSRNode, SSRNode>({
    createElement: tag => new SSRNode(tag),
    createText: text => { const n = new SSRNode('#text'); n.text = text; return n },
    createComment: text => { const n = new SSRNode('#comment'); n.text = text; return n },
    setText: (n, t) => { n.text = t },
    setElementText: (n, t) => { n.children = []; n.text = t },
    parentNode: n => n.parent,
    nextSibling: n => n.parent ? (n.parent.children[n.parent.children.indexOf(n) + 1] ?? null) : null,
    patchProp: (n, k, _prev, v) => {
      n.props[k] = v
      if (k === 'value') n.value = v
      if (k === 'checked') n.checked = Boolean(v)
      if (k === 'selected') n.selected = Boolean(v)
    },
    insert: (n, p, anchor) => {
      if (n.parent) n.parent.children = n.parent.children.filter(c => c !== n)
      n.parent = p
      const idx = anchor ? p.children.indexOf(anchor) : -1
      if (idx < 0) p.children.push(n)
      else p.children.splice(idx, 0, n)
    },
    remove: n => {
      if (n.parent) n.parent.children = n.parent.children.filter(c => c !== n)
      n.parent = null
    },
    setScopeId: (n, id) => {
      if (!n.scopeIds.includes(id)) n.scopeIds.push(id)
    },
    insertStaticContent: (content, p, anchor) => {
      const n = new SSRNode('#static')
      n.text = content
      n.parent = p
      const idx = anchor ? p.children.indexOf(anchor) : -1
      if (idx < 0) p.children.push(n)
      else p.children.splice(idx, 0, n)
      return [n, n]
    },
  })

  const root = new SSRNode('#root')
  if (isVNode(input)) {
    ssrRenderer.render(input, root)
  } else {
    const app = ssrRenderer.createApp(input._component, input._props)
    app.mount(root)
    const html = serializeSSRNode(root)
    app.unmount()
    return html
  }
  const html = serializeSSRNode(root)
  ssrRenderer.render(null, root)
  return html
}

export { Comment, Fragment, Static, Teleport, Text, Transition }
