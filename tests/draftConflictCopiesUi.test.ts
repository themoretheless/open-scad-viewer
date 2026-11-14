import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'
import { createRenderer, h, nextTick, shallowReactive } from 'svet'
import { listSolidDraftConflicts, writeSolidDraftConflict } from '../src/kernel/solidDraftStore'

const downloads = vi.hoisted(() => ({ calls: [] as Array<[string, string, string | undefined]> }))
vi.mock('../src/features/directModeler/helpers', () => ({
  download: (text: string, name: string, mimeType?: string) => downloads.calls.push([text, name, mimeType]),
}))
const { default: DraftConflictCopies } = await import('../src/features/directModeler/DraftConflictCopies.svt')

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

const key = 'scad-solid-modeler-v1'
const unmounts: Array<() => void> = []
beforeEach(() => {
  vi.stubGlobal('indexedDB', new IDBFactory())
  vi.stubGlobal('document', doc)
  downloads.calls = []
})
afterEach(() => {
  unmounts.splice(0).forEach(unmount => unmount())
  body.children = []
  doc.activeElement = body
  vi.unstubAllGlobals()
})

const settle = async () => {
  for (let i = 0; i < 20; i++) {
    await nextTick()
    await new Promise(resolve => setTimeout(resolve, 0))
  }
}
const all = (node: El = body): El[] => [node, ...node.children.flatMap(child => all(child))]
const text = (node: El): string => node.text + node.children.map(child => text(child)).join('')
const rows = () => all().filter(node => 'data-conflict-copy' in node.props)
const button = (row: El, name: string) => {
  const node = all(row).find(candidate => candidate.tag === 'button' && text(candidate) === name)
  expect(node, `button ${name}`).toBeDefined()
  return node!
}

async function mount(restore: (text: string) => Promise<boolean>) {
  const state = shallowReactive({ open: true })
  const onClose = vi.fn(() => {
    state.open = false
  })
  const host = new El('div')
  body.children.push(host)
  host.parent = body
  const app = renderer.createApp({
    setup: () => () => h(DraftConflictCopies, { open: state.open, draftKey: key, ru: false, restore, onClose }),
  })
  app.mount(host)
  unmounts.push(() => app.unmount())
  await settle()
  return { state, onClose }
}

it('lists conflict copies newest first with time and size', async () => {
  await writeSolidDraftConflict(key, '{"older":1}', Date.UTC(2026, 0, 1, 10))
  await writeSolidDraftConflict(key, '{"newer":22}', Date.UTC(2026, 0, 2, 10))
  await writeSolidDraftConflict('scad-main-modeler-v1', '{"other":1}', Date.UTC(2026, 0, 3, 10))
  await mount(vi.fn())
  const dialog = all().find(node => node.props.role === 'dialog')
  expect(dialog?.props['aria-modal']).toBe('true')
  const listed = rows().map(row => text(row))
  expect(listed).toHaveLength(2)
  expect(listed[0]).toContain('12 B')
  expect(listed[1]).toContain('11 B')
  expect(listed[0]).toContain(new Date(Date.UTC(2026, 0, 2, 10)).toLocaleString('en-US'))
})

it('restores a copy through the parent save path and closes', async () => {
  await writeSolidDraftConflict(key, '{"copy":1}', 1000)
  const restore = vi.fn(async () => true)
  const { onClose } = await mount(restore)
  await button(rows()[0], 'Restore').props.onClick({})
  await settle()
  expect(restore).toHaveBeenCalledWith('{"copy":1}')
  expect(onClose).toHaveBeenCalled()
  // Restoring does not consume the copy; it stays until deleted or expired.
  expect((await listSolidDraftConflicts(key)).map(c => c.text)).toEqual(['{"copy":1}'])
})

it('keeps the dialog open when the restore is refused', async () => {
  await writeSolidDraftConflict(key, '{"copy":1}', 1000)
  const { onClose } = await mount(async () => false)
  await button(rows()[0], 'Restore').props.onClick({})
  await settle()
  expect(onClose).not.toHaveBeenCalled()
  expect(rows()).toHaveLength(1)
})

it('downloads a copy as JSON', async () => {
  await writeSolidDraftConflict(key, '{"copy":1}', Date.UTC(2026, 0, 1))
  await mount(vi.fn())
  button(rows()[0], 'Download JSON').props.onClick({})
  expect(downloads.calls).toEqual([
    ['{"copy":1}', 'solid-draft-conflict-2026-01-01T00-00-00-000Z.json', 'application/json'],
  ])
})

it('deletes a copy from storage and from the list', async () => {
  await writeSolidDraftConflict(key, '{"a":1}', 1000)
  await writeSolidDraftConflict(key, '{"b":1}', 2000)
  await mount(vi.fn())
  await button(rows()[0], 'Delete').props.onClick({})
  await settle()
  expect(rows()).toHaveLength(1)
  expect((await listSolidDraftConflicts(key)).map(c => c.text)).toEqual(['{"a":1}'])
})

it('shows an empty state without copies', async () => {
  await mount(vi.fn())
  expect(text(body)).toContain('No conflict copies.')
})
