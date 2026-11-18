import { afterEach, describe, expect, it, vi } from 'vitest'
import { createRenderer, h, nextTick, shallowReactive } from 'svet'
import Modal from '../src/components/Modal.svt'
import SidePanel from '../src/components/SidePanel.svt'
import { nextTrappedIndex } from '../src/composables/useModalDialog'

const NATIVE_TAB_STOPS = new Set(['a', 'button', 'input', 'select', 'textarea', 'summary'])

/** Minimal element model: enough DOM for focus, containment and tab-stop queries. */
class El {
  parent: El | null = null
  children: El[] = []
  props: Record<string, unknown> & { [key: string]: any } = {} // eslint-disable-line @typescript-eslint/no-explicit-any -- host props hold handlers and attributes
  text = ''
  style: Record<string, string> = {}
  constructor(public tag: string) {}
  get tabIndex() {
    const value = this.props.tabindex
    if (value !== undefined && value !== null) return Number(value)
    return NATIVE_TAB_STOPS.has(this.tag) ? 0 : -1
  }
  get isConnected(): boolean {
    let node: El | null = this.parent
    while (node?.parent) node = node.parent
    return node === doc.body || this === doc.body
  }
  focus() {
    doc.activeElement = this
  }
  contains(node: El | null) {
    for (let current = node; current; current = current.parent) if (current === this) return true
    return false
  }
  matches(selector: string) {
    return selector === ':disabled' && Boolean(this.props.disabled)
  }
  getClientRects() {
    return this.props.hidden ? [] : [{}]
  }
  querySelectorAll() {
    const all = (node: El): El[] => node.children.flatMap(child => [child, ...all(child)])
    return all(this).filter(node => NATIVE_TAB_STOPS.has(node.tag) || 'tabindex' in node.props)
  }
}

const body = Object.assign(new El('body'), { style: { overflow: '' } })
const doc = { body, activeElement: body as El | null }
const renderer = createRenderer<El, El>({
  createElement: tag => new El(tag),
  createText: text => Object.assign(new El('#text'), { text }),
  createComment: () => new El('#comment'),
  setText: (node, text) => {
    node.text = text
  },
  setElementText: (node, text) => {
    node.children = []
    node.text = text
  },
  parentNode: node => node.parent,
  nextSibling: node => node.parent?.children[node.parent.children.indexOf(node) + 1] ?? null,
  patchProp: (node, key, _old, value) => {
    node.props[key] = value
  },
  insert: (node, parent, anchor) => {
    node.parent = parent
    const index = anchor ? parent.children.indexOf(anchor) : -1
    if (index < 0) parent.children.push(node)
    else parent.children.splice(index, 0, node)
  },
  remove: node => {
    // Like a browser, removing the focused subtree moves focus to the body.
    if (node.contains(doc.activeElement)) doc.activeElement = body
    if (node.parent) node.parent.children = node.parent.children.filter(child => child !== node)
    node.parent = null
  },
  querySelector: selector => (selector === 'body' ? body : null),
  setScopeId: () => {},
  insertStaticContent: () => {
    throw new Error('Unexpected static content')
  },
})

const mounts: (() => void)[] = []
afterEach(() => {
  mounts.splice(0).forEach(unmount => unmount())
  body.children = []
  doc.activeElement = body
  body.style = { overflow: '' }
  vi.unstubAllGlobals()
})

const settle = async () => {
  for (let i = 0; i < 4; i++) await nextTick()
}
const find = (id: string, root: El = body): El | undefined => {
  if (root.props.id === id) return root
  for (const child of root.children) {
    const match = find(id, child)
    if (match) return match
  }
}
const dialogNode = (root: El = body): El | undefined =>
  root.props.role === 'dialog' ? root : root.children.map(child => dialogNode(child)).find(Boolean)
const key = (name: string, shiftKey = false) => ({
  key: name,
  shiftKey,
  preventDefault: vi.fn(),
  stopPropagation: vi.fn(),
})

async function mountSurface(component: unknown, extra: Record<string, unknown> = {}) {
  vi.stubGlobal('document', doc)
  const state = shallowReactive({ open: false })
  const onClose = vi.fn()
  const host = new El('div')
  body.children.push(host)
  host.parent = body
  const app = renderer.createApp({
    setup: () => () => [
      h('button', { id: 'opener' }, 'Open'),
      h(
        component as Parameters<typeof h>[0],
        { open: state.open, labelledby: 'title', describedby: 'note', onClose, ...extra },
        {
          default: () => [
            h('h2', { id: 'title' }, 'Title'),
            h('p', { id: 'note' }, 'Note'),
            h('button', { id: 'first' }, 'First'),
            h('button', { id: 'skipped-disabled', disabled: true }, 'Disabled'),
            h('button', { id: 'skipped-negative', tabindex: '-1' }, 'Not a tab stop'),
            h('input', { id: 'last' }),
          ],
        },
      ),
    ],
  })
  app.mount(host)
  mounts.push(() => app.unmount())
  await settle()
  find('opener')!.focus()
  return { state, onClose }
}

describe.each([
  ['Modal', Modal],
  ['SidePanel', SidePanel],
])('%s', (_name, component) => {
  it('renders an aria-modal dialog labelled by its title and moves focus in', async () => {
    const { state } = await mountSurface(component)
    expect(dialogNode()).toBeUndefined()
    state.open = true
    await settle()
    const dialog = dialogNode()!
    expect(dialog.props['aria-modal']).toBe('true')
    expect(dialog.props['aria-labelledby']).toBe('title')
    expect(dialog.props['aria-describedby']).toBe('note')
    expect(
      find('title')!.text +
        find('title')!
          .children.map(c => c.text)
          .join(''),
    ).toBe('Title')
    expect(doc.activeElement).toBe(find('first'))
    expect(body.style.overflow).toBe('hidden')
  })

  it('cycles Tab and Shift+Tab inside the dialog, skipping disabled and tabindex=-1 controls', async () => {
    const { state } = await mountSurface(component)
    state.open = true
    await settle()
    const dialog = dialogNode()!
    const tab = key('Tab')
    dialog.props.onKeydown(tab)
    expect(tab.preventDefault).toHaveBeenCalled()
    expect(doc.activeElement).toBe(find('last'))
    dialog.props.onKeydown(key('Tab'))
    expect(doc.activeElement).toBe(find('first'))
    dialog.props.onKeydown(key('Tab', true))
    expect(doc.activeElement).toBe(find('last'))
    find('opener')!.focus()
    dialog.props.onKeydown(key('Tab'))
    expect(doc.activeElement).toBe(find('first'))
  })

  it('closes on Escape and restores focus and scroll when closed', async () => {
    const { state, onClose } = await mountSurface(component)
    state.open = true
    await settle()
    const escape = key('Escape')
    dialogNode()!.props.onKeydown(escape)
    expect(onClose).toHaveBeenCalledTimes(1)
    expect(escape.stopPropagation).toHaveBeenCalled()
    state.open = false
    await settle()
    expect(dialogNode()).toBeUndefined()
    expect(doc.activeElement).toBe(find('opener'))
    expect(body.style.overflow).toBe('')
  })

  it('honors initialFocus and restoreFocus=false', async () => {
    const { state } = await mountSurface(component, { initialFocus: () => find('last'), restoreFocus: false })
    state.open = true
    await settle()
    expect(doc.activeElement).toBe(find('last'))
    state.open = false
    await settle()
    expect(doc.activeElement).toBe(body)
  })

  it('forwards other keys and closes from the backdrop', async () => {
    const onKeydown = vi.fn()
    const { state, onClose } = await mountSurface(component, { onKeydown })
    state.open = true
    await settle()
    const dialog = dialogNode()!
    dialog.props.onKeydown(key('Enter'))
    expect(onKeydown).toHaveBeenCalledWith(expect.objectContaining({ key: 'Enter' }))
    const backdrop = dialog.parent!
    backdrop.props.onPointerdown({ target: backdrop, currentTarget: backdrop })
    expect(onClose).toHaveBeenCalledTimes(1)
  })
})

it('docks the side panel to the requested edge', async () => {
  const { state } = await mountSurface(SidePanel, { side: 'left' })
  state.open = true
  await settle()
  expect(JSON.stringify(dialogNode()!.props.class)).toContain('side-panel-left')
})

it('computes wrapped tab stops', () => {
  expect(nextTrappedIndex(-1, 3, false)).toBe(0)
  expect(nextTrappedIndex(2, 3, false)).toBe(0)
  expect(nextTrappedIndex(0, 3, true)).toBe(2)
  expect(nextTrappedIndex(-1, 3, true)).toBe(2)
  expect(nextTrappedIndex(1, 3, false)).toBe(2)
})

it('keyboard shortcut help uses the shared modal contract', async () => {
  vi.stubGlobal('document', doc)
  const { default: KeyboardShortcuts } = await import('../src/components/KeyboardShortcuts.svt')
  const onClose = vi.fn()
  const host = new El('div')
  body.children.push(host)
  host.parent = body
  const app = renderer.createApp(KeyboardShortcuts, { open: true, groups: [], locale: 'en', onClose })
  app.mount(host)
  mounts.push(() => app.unmount())
  await settle()
  const dialog = dialogNode()!
  expect(dialog.props['aria-modal']).toBe('true')
  expect(find(dialog.props['aria-labelledby'])!.tag).toBe('h2')
  expect(doc.activeElement?.props['aria-label']).toBe('Close')
  dialog.props.onKeydown(key('?'))
  dialog.props.onKeydown(key('Escape'))
  expect(onClose).toHaveBeenCalledTimes(2)
})
