import { computed, createRenderer, defineComponent, ref, shallowRef } from 'vue'
import { beforeEach, expect, it, vi } from 'vitest'
import { useWorkspaceFileActions } from '../src/composables/useWorkspaceFileActions'
import type { MeshData } from '../src/core/mesh'

const control = vi.hoisted(() => ({
  defer: false,
  run: vi.fn(),
  cancel: vi.fn(),
  dispose: vi.fn(),
  jobs: [] as any[],
  events: [] as { phase: string; completed: number; total: number }[],
  resolve: (value?: unknown) => {},
}))
const download = vi.hoisted(() => vi.fn())

vi.mock('../src/services/solidPreviewWorker', () => ({
  createSolidPreviewWorker: () => ({ run: control.run, cancel: control.cancel, dispose: control.dispose }),
}))
vi.mock('../src/appHelpers', async importOriginal => ({
  ...await importOriginal<typeof import('../src/appHelpers')>(),
  downloadBlob: download,
}))

class Node {
  parent: Node | null = null
  children: Node[] = []
  text = ''
  constructor(public tag: string) {}
}
const renderer = createRenderer<Node, Node>({
  createElement: tag => new Node(tag),
  createText: text => Object.assign(new Node('#text'), { text }),
  createComment: () => new Node('#comment'),
  setText: (node, text) => { node.text = text },
  setElementText: (node, text) => { node.text = text },
  parentNode: node => node.parent,
  nextSibling: () => null,
  patchProp: () => {},
  insert: (node, parent) => { node.parent = parent; parent.children.push(node) },
  remove: node => {
    if (node.parent) node.parent.children = node.parent.children.filter(child => child !== node)
  },
  setScopeId: () => {},
  insertStaticContent: () => { throw new Error('Unexpected static content') },
})

const source = [
  'v 0 0 0', 'v 1 0 0', 'v 0 1 0', 'v 0 0 1',
  'f 1 3 2', 'f 1 2 4', 'f 2 3 4', 'f 1 4 3',
].join('\n')
const bytes = new TextEncoder().encode(source)
const file = { name: 'solid.obj', arrayBuffer: async () => bytes.buffer } as unknown as File
const lateResult = {
  data: new Uint8Array([1]), mimeType: 'application/x-ply', extension: 'ply',
  fileName: 'solid.ply', format: 'ply',
  source: {
    fileName: 'solid.obj', format: 'obj', byteLength: bytes.byteLength,
    triangleCount: 4, vertexCount: 4, sourceVertexCount: 4, degenerateTriangles: 0,
  },
}

const sceneMesh = {
  vertices: new Float32Array([
    0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1,
    0, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1,
  ]),
  indices: new Uint32Array([0, 2, 1, 0, 1, 3, 1, 2, 3, 0, 3, 2]),
  transform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
} as MeshData

function mount() {
  const notices: string[] = []
  let actions!: ReturnType<typeof useWorkspaceFileActions>
  const Host = defineComponent({
    setup() {
      actions = useWorkspaceFileActions({
        code: ref(''), fileName: ref('model.scad'), error: ref(''),
        canExport: computed(() => true), sceneMeshes: computed(() => [sceneMesh]),
        buildCounters: shallowRef({ builds: 0, superseded: 0, workerStarts: 0, hardRestarts: 0 }),
        performanceSamples: shallowRef([]), markFitNextRender: () => {},
        showNotice: message => notices.push(message), translate: key => key === 'converted' ? 'Converted' : key,
      })
      return () => null
    },
  })
  const app = renderer.createApp(Host)
  app.mount(new Node('root'))
  return { actions, notices, unmount: () => app.unmount() }
}

beforeEach(async () => {
  const { convertMeshFile, exportMesh } = await import('../src/services/meshConvert')
  control.defer = false
  control.jobs = []
  control.events = []
  control.resolve = () => {}
  control.run.mockReset()
  control.cancel.mockReset()
  control.dispose.mockReset()
  download.mockReset()
  control.run.mockImplementation((job, options) => {
    control.jobs.push(job)
    const report = (progress: { phase: string; completed: number; total: number }) => {
      control.events.push(progress)
      options?.onProgress?.(progress)
    }
    if (control.defer) {
      report(job.kind === 'meshExport'
        ? { phase: 'inspect', completed: 0, total: 2 }
        : { phase: 'decode', completed: 0, total: 7 })
      return new Promise(resolve => { control.resolve = resolve })
    }
    return job.kind === 'meshExport'
      ? exportMesh(job.mesh, job.target, job.options, report)
      : convertMeshFile(job.fileName, job.input, job.target, job.options, report)
  })
})

it('converts through the worker and publishes staged progress before downloading', async () => {
  const { actions, notices, unmount } = mount()
  try {
    await actions.convertFile(file)
    expect(control.jobs).toHaveLength(1)
    expect(control.jobs[0]).toMatchObject({ kind: 'meshConvert', fileName: 'solid.obj', target: 'stl_binary' })
    expect(control.events).toEqual([
      { phase: 'decode', completed: 0, total: 7 },
      { phase: 'finalize', completed: 1, total: 7 },
      { phase: 'check', completed: 2, total: 7 },
      { phase: 'verify', completed: 4, total: 7 },
      { phase: 'inspect', completed: 5, total: 7 },
      { phase: 'export', completed: 6, total: 7 },
    ])
    expect(download).toHaveBeenCalledWith(expect.any(Blob), 'solid.stl')
    expect(notices).toEqual(['Converted: OBJ → STL_BINARY'])
    expect(actions.conversionPending.value).toBe(false)
    expect(actions.conversionProgress.value).toBeNull()
  } finally { unmount() }
})

it('exports the scene and legacy STL/OBJ actions through the same worker job', async () => {
  const { actions, notices, unmount } = mount()
  try {
    await actions.exportAdditionalMesh()
    await actions.exportObj()
    await actions.exportStl()
    expect(control.jobs.map(job => [job.kind, job.target])).toEqual([
      ['meshExport', 'stl_binary'], ['meshExport', 'obj'], ['meshExport', 'stl_binary'],
    ])
    expect(control.jobs.every(job => job.mesh.length === 1 && job.mesh[0] === sceneMesh)).toBe(true)
    expect(control.events.slice(0, 2)).toEqual([
      { phase: 'inspect', completed: 0, total: 2 },
      { phase: 'export', completed: 1, total: 2 },
    ])
    expect(notices).toEqual(['exported', 'exported', 'exported'])
    expect(download.mock.calls.map(call => call[1])).toEqual(['model.stl', 'model.obj', 'model.stl'])
    expect(actions.conversionOperation.value).toBe('export')
    expect(actions.conversionPending.value).toBe(false)
  } finally { unmount() }
})

it('cancels scene export and ignores a late artifact', async () => {
  control.defer = true
  const { actions, notices, unmount } = mount()
  const exporting = actions.exportObj()
  await vi.waitFor(() => expect(control.jobs).toHaveLength(1))
  expect(actions.conversionOperation.value).toBe('export')
  expect(actions.conversionProgress.value).toEqual({ phase: 'inspect', completed: 0, total: 2 })
  await actions.exportAdditionalMesh()
  expect(control.jobs).toHaveLength(1)
  actions.cancelConversion()
  control.resolve({ data: new Uint8Array([1]), mimeType: 'model/obj', extension: 'obj' })
  await exporting
  expect(download).not.toHaveBeenCalled()
  expect(notices).toEqual([])
  unmount()
})

it('cancels conversion and ignores a late result', async () => {
  control.defer = true
  const { actions, notices, unmount } = mount()
  const converting = actions.convertFile(file)
  await vi.waitFor(() => expect(control.jobs).toHaveLength(1))
  expect(actions.conversionPending.value).toBe(true)
  expect(actions.conversionProgress.value).toEqual({ phase: 'decode', completed: 0, total: 7 })
  await actions.convertFile(file)
  expect(control.jobs).toHaveLength(1)
  actions.cancelConversion()
  expect(control.cancel).toHaveBeenCalledOnce()
  expect(actions.conversionPending.value).toBe(false)
  control.resolve(lateResult)
  await converting
  expect(download).not.toHaveBeenCalled()
  expect(notices).toEqual([])
  unmount()
})

it('disposes the worker on unmount and drops a late result', async () => {
  control.defer = true
  const { actions, notices, unmount } = mount()
  const converting = actions.convertFile(file)
  await vi.waitFor(() => expect(control.jobs).toHaveLength(1))
  unmount()
  expect(control.dispose).toHaveBeenCalledOnce()
  control.resolve(lateResult)
  await converting
  expect(download).not.toHaveBeenCalled()
  expect(notices).toEqual([])
})
