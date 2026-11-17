import { readFileSync } from 'node:fs'
import { createRenderer, nextTick, ref, h } from 'svet'
import { it, expect, vi, type Mock } from 'vitest'
import type { NurbsSurface } from '../src/kernel/nurbsSurface'
import type { MainSolidJob } from '../src/kernel/mainSolidProtocol'
import type { MainSolidPort } from '../src/kernel/mainSolidWorkerClient'
type PreviewTask = { resolve: (value: unknown) => void; reject: (reason: unknown) => void; job: unknown }
type PreviewWorker = { cancel: Mock; dispose: Mock; run: Mock }
type HostHandler = (event?: unknown) => unknown
type HostNode = {
  type: string
  children: HostNode[]
  props: Record<string, unknown> & { onClick: HostHandler; onToggle: HostHandler }
  parent: HostNode | null
  text: string
}
type PreviewDisplay = { witness: unknown; lines: unknown[] } | null
type TestPort = MainSolidPort & { postMessage: Mock; terminate: Mock }
const state = vi.hoisted(() => ({ tasks: [] as PreviewTask[], worker: null as PreviewWorker | null }))
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
vi.mock('../src/components/CadQuantityInput.svt', () => ({
  default: {
    props: ['modelValue'],
    render() {
      return h('input')
    },
  },
}))
vi.mock('../src/kernel/nurbsSurface', () => ({
  extractNurbsSurfaceIsocurvePoints: () => [
    [0, 0, 0],
    [1, 1, 1],
  ],
  createNurbsSurfaceEvaluator: () => Object.assign(() => ({ point: [1, 2, 3] }), { dispose: vi.fn() }),
}))
import Panel from '../src/components/SourceGraphG2Preview.svt'
const actual = JSON.parse(readFileSync('tests/fixtures/source-graph-g2-preview-v1.json', 'utf8')).direct
it('opens preview, cancels stale work, retries and clears diagnostics on source edits and close', async () => {
  const node = (type: string): HostNode => ({
    type,
    children: [],
    props: {} as HostNode['props'],
    parent: null,
    text: '',
  })
  const renderer = createRenderer<HostNode, HostNode>({
    createElement: node,
    createText: t => ({ ...node('text'), text: t }),
    createComment: node,
    setText: (n, t) => (n.text = t),
    setElementText: (n, t) => (n.text = t),
    parentNode: n => n.parent,
    nextSibling: () => null,
    insert(n, p) {
      n.parent = p
      p.children.push(n)
    },
    remove(n) {
      const p = n.parent
      if (p) p.children.splice(p.children.indexOf(n), 1)
    },
    patchProp: (n, k, _p, v) => (n.props[k] = v),
  })
  const surfaces = ref([actual.patch, structuredClone(actual.patch)]),
    active = ref(true),
    displays: PreviewDisplay[] = [],
    root = node('root')
  const app = renderer.createApp({
    render: () =>
      h(Panel, {
        surfaces: surfaces.value as [NurbsSurface, NurbsSurface],
        active: active.value,
        ru: false,
        onDisplay: (d: PreviewDisplay) => displays.push(d),
      }),
  })
  app.mount(root)
  const all = (n: HostNode): HostNode[] => [n, ...n.children.flatMap(all)]
  const button = (text: string) => all(root).find(n => n.type === 'button' && n.text === text)!
  const details = () => all(root).find(n => n.type === 'details')!
  const flush = async () => {
    await nextTick()
    await nextTick()
  }
  try {
    details().props.onToggle({ target: { open: true } })
    await flush()
    void button('Preview').props.onClick()
    await flush()
    expect(state.tasks).toHaveLength(1)
    button('Cancel · Esc').props.onClick()
    state.tasks[0].resolve(structuredClone(actual))
    await flush()
    expect(displays.at(-1)).toBe(null)
    void button('Preview').props.onClick()
    await flush()
    state.tasks[1].reject(Error('unsupported source'))
    await flush()
    expect(all(root).some(n => n.type === 'output' && n.text.includes('Check the surface order and width'))).toBe(true)
    void button('Retry').props.onClick()
    await flush()
    state.tasks[2].resolve(structuredClone(actual))
    await flush()
    expect(displays.at(-1)!.witness).toEqual([1, 2, 3])
    expect(displays.at(-1)!.lines).toHaveLength(10)
    expect(all(root).some(n => n.type === 'output' && n.text.includes('not maintained over the whole surface'))).toBe(
      true,
    )
    surfaces.value[0].controlPoints[0][0][0] += 1
    await flush()
    expect(displays.at(-1)).toBe(null)
    void button('Preview').props.onClick()
    await flush()
    active.value = false
    await flush()
    state.tasks[3].resolve(structuredClone(actual))
    await flush()
    expect(displays.at(-1)).toBe(null)
    details().props.onToggle({ target: { open: false } })
    await flush()
    expect(state.worker!.cancel).toHaveBeenCalled()
  } finally {
    app.unmount()
    expect(state.worker!.dispose).toHaveBeenCalledOnce()
  }
})

it('discards a captured late worker reply after cancellation and accepts Retry', async () => {
  const { MainSolidWorkerClient } = await import('../src/kernel/mainSolidWorkerClient')
  const ports: TestPort[] = []
  const client = new MainSolidWorkerClient(() => {
    const p: TestPort = {
      onmessage: null,
      onerror: null,
      onmessageerror: null,
      terminate: vi.fn(),
      postMessage: vi.fn(),
    }
    ports.push(p)
    return p
  })
  const job: Extract<MainSolidJob, { kind: 'sourceGraphG2Preview' }> = {
    kind: 'sourceGraphG2Preview',
    sources: [actual.patch, actual.patch],
    width: {
      degree: 1,
      knots: [0, 0, 1, 1],
      controlPoints: [
        [0.25, 0, 0],
        [0.25, 0, 0],
      ],
      weights: [1, 1],
    },
    options: { radiusMm: 1, radiusToleranceMm: 0.001 },
  }
  try {
    const first = client.run(job),
      rejected = expect(first).rejects.toMatchObject({ name: 'AbortError' }),
      late = ports[0].onmessage!
    client.cancel()
    await rejected
    const retry = client.run(job)
    let settled = false
    void retry.then(() => (settled = true))
    late({ data: { version: 1, id: 1, kind: job.kind, ok: true, result: structuredClone(actual) } } as MessageEvent)
    await Promise.resolve()
    expect(settled).toBe(false)
    const request = ports[1].postMessage.mock.lastCall![0]
    ports[1].onmessage!({
      data: { version: 1, id: request.id, kind: job.kind, ok: true, result: structuredClone(actual) },
    } as MessageEvent)
    await expect(retry).resolves.toEqual(actual)
  } finally {
    client.dispose()
  }
})
