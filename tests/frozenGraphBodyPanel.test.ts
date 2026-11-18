import { createRenderer, nextTick, ref, h } from 'svet'
import { it, expect, vi, type Mock } from 'vitest'
type Task = { resolve: (value: unknown) => void; reject: (reason: unknown) => void; job: unknown }
type FakeWorker = { cancel: Mock; dispose: Mock; run: Mock }
type FakeNode = {
  type: string
  tagName: string
  children: FakeNode[]
  props: Record<string, unknown>
  parent: FakeNode | null
  text: string
  _value?: unknown
  value?: unknown
  readonly options: FakeNode[]
  addEventListener(): void
  removeEventListener(): void
  getRootNode(): object
  setAttribute(): void
  removeAttribute(): void
}
const state = vi.hoisted(() => ({ tasks: [] as Task[], worker: null as FakeWorker | null }))
vi.mock('../src/kernel/solidPreviewWorker', () => ({
  createSolidPreviewWorker: () => {
    state.tasks = []
    state.worker = {
      cancel: vi.fn(),
      dispose: vi.fn(),
      run: vi.fn((job: unknown) => new Promise((resolve, reject) => state.tasks.push({ resolve, reject, job }))),
    }
    return state.worker
  },
}))
import Panel from '../src/components/FrozenGraphBodyPanel.svt'
it('loads a display-only body, selects an edge and invalidates late data on cancel', async () => {
  const node = (type: string): FakeNode => ({
    type,
    tagName: type.toUpperCase(),
    children: [],
    props: {},
    parent: null,
    text: '',
    get options() {
      return this.children.filter((n: FakeNode) => n.type === 'option')
    },
    addEventListener() {},
    removeEventListener() {},
    getRootNode() {
      return {}
    },
    setAttribute() {},
    removeAttribute() {},
  })
  const renderer = createRenderer<FakeNode, FakeNode>({
    createElement: node,
    createText: t => ({ ...node('text'), text: t }),
    createComment: node,
    setText: (n, t) => (n.text = t),
    setElementText: (n, t) => (n.text = t),
    parentNode: n => n.parent,
    nextSibling: n => {
      const a = n.parent?.children ?? []
      return a[a.indexOf(n) + 1] ?? null
    },
    insert(n, p, a) {
      if (n.parent) n.parent.children.splice(n.parent.children.indexOf(n), 1)
      n.parent = p
      const i = a ? p.children.indexOf(a) : -1
      if (i < 0) p.children.push(n)
      else p.children.splice(i, 0, n)
    },
    remove(n) {
      const p = n.parent
      if (p) p.children.splice(p.children.indexOf(n), 1)
      n.parent = null
    },
    patchProp: (n, k, _p, v) => {
      n.props[k] = v
      if (k === 'value') {
        n._value = v
        n.value = v
      }
    },
  })
  const root = node('root'),
    active = ref(true),
    selected = ref<number | null>(null),
    displays: unknown[] = []
  const app = renderer.createApp({
    render: () =>
      h(Panel, {
        active: active.value,
        ru: false,
        selectedEdge: selected.value,
        'onUpdate:selectedEdge': (v: number | null) => (selected.value = v),
        onDisplay: (v: unknown) => displays.push(v),
      }),
  })
  app.mount(root)
  const all = (n: FakeNode): FakeNode[] => [n, ...n.children.flatMap(all)],
    button = (s: string) => all(root).find(n => n.type === 'button' && n.text === s)
  const call = (n: FakeNode | undefined, key: string, ...args: unknown[]): unknown => {
    const handler = n?.props[key]
    if (typeof handler !== 'function') throw new TypeError(`missing ${key} handler`)
    return handler(...args)
  }
  const flush = async () => {
    for (let i = 0; i < 20; i++) await Promise.resolve()
    await nextTick()
    await nextTick()
  }
  const response = {
    report: { faces: 9, edges: 27, curvatureLimitPerMm: 10 },
    artifact: { geometry: { identityLinks: { edges: [{ edge: 0, sourceId: 'original-edge' }] } } },
    display: {
      edges: [
        {
          edge: 0,
          root: 1,
          points: [
            [0, 0, 0],
            [1, 1, 1],
          ],
        },
      ],
    },
  }
  try {
    call(
      all(root).find(n => n.type === 'details'),
      'onToggle',
      { target: { open: true } },
    )
    await flush()
    const input = all(root).find(n => n.type === 'input' && n.props.type === 'file')
    const file = {
      name: 'test.json',
      size: 2,
      stream: () =>
        new ReadableStream({
          start(c) {
            c.enqueue(new TextEncoder().encode('{}'))
            c.close()
          },
        }),
    }
    void call(input, 'onChange', { target: { files: [file], value: 'test.json' } })
    await flush()
    expect(state.tasks).toHaveLength(1)
    call(button('Cancel · Esc'), 'onClick')
    state.tasks[0].resolve(response)
    await flush()
    expect(displays.at(-1)).toBe(null)
    void call(button('Retry · Enter'), 'onClick')
    await flush()
    state.tasks[1].resolve(response)
    await flush()
    expect(displays.at(-1)).toBe(response)
    call(
      all(root).find(n => n.type === 'select'),
      'onUpdate:modelValue',
      0,
    )
    await flush()
    expect(selected.value).toBe(0)
    active.value = false
    await flush()
    expect(displays.at(-1)).toBe(null)
    expect(selected.value).toBe(null)
  } finally {
    app.unmount()
    expect(state.worker?.dispose).toHaveBeenCalledOnce()
  }
})
