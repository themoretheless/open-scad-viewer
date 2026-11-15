import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { MeshLodLevel } from '../src/core/mesh'
import type { NativeGeometryArtifact } from '../src/core/nativeGeometry'
import type { DisplayLevelPayload, DisplayLevelSummaryEntry } from '../src/kernel/displayLevelService'
import { DisplayLevelService } from '../src/kernel/displayLevelService'
import { createDisplayLevelWorkerHandler, type DisplayLevelResponse } from '../src/kernel/displayLevelWorkerRuntime'
import {
  BoundingBoxLayer,
  buildBoundingBoxLines,
  BOX_VERTICES_PER_MESH,
  rayBoxHit,
} from '../src/render/boundingBoxDisplay'
import { isDeferredLevel, levelByteSize } from '../src/render/deferredLevels'
import {
  DISPLAY_BUDGET_PRESETS,
  geometryKey,
  planDisplayCaps,
  ProgressiveLevelStore,
  type IdleScheduler,
} from '../src/render/displayScheduler'
import { MeshDisplayController } from '../src/render/meshDisplayController'
import { createMeshLodState, updateMeshLods } from '../src/render/meshLod'
import type { NativePickingCache } from '../src/render/nativePickingCache'
import { OnDemandLevelSource, type DisplayLevelProvider } from '../src/render/onDemandLevels'
import { findHitCandidatesForRay } from '../src/render/pickingController'
import { RecordingRenderer } from '../src/render/recordingRenderer'
import { buildSceneAabbIndex } from '../src/render/sceneAabbIndex'
import type { GMesh } from '../src/render/types'
import type { ViewerLodCamera } from '../src/render/viewerLod'
import * as viewerLod from '../src/render/viewerLod'
import { displayQualityStorageKey, readDisplayQuality, useDisplayQuality } from '../src/composables/useDisplayQuality'
import { buildMeshBvh } from '../src/kernel/meshBvh'
import { GpuResidencyCache } from '../src/render/gpuResidencyCache'

interface FakeBuffer {
  size: number
  destroyed: boolean
  destroy(): void
}

function device() {
  const created: FakeBuffer[] = []
  const writes: number[] = []
  return {
    created,
    writes,
    dev: {
      limits: { maxBufferSize: 1 << 30 },
      createBuffer: ({ size }: { size: number }) => {
        const buffer: FakeBuffer = {
          size,
          destroyed: false,
          destroy() {
            this.destroyed = true
          },
        }
        created.push(buffer)
        return buffer
      },
      queue: {
        writeBuffer: (_buffer: unknown, _offset: number, data: ArrayBufferView) => writes.push(data.byteLength),
      },
    } as unknown as GPUDevice,
  }
}

const level = (triangles: number, errorUpper: number): MeshLodLevel => ({
  vertices: new Float32Array(triangles * 18),
  indices: new Uint32Array(triangles * 3),
  edgeIndices: new Uint32Array(2),
  faceIds: new Uint32Array(triangles),
  provenance: [],
  bvh: {} as never,
  triangleCount: triangles,
  errorUpper,
})

const snapshot = (json = '{"geometry":{}}'): NativeGeometryArtifact =>
  ({
    version: 1,
    nodeId: 'n',
    kind: 'brep',
    revision: 'r',
    geometryJson: json,
    documentJson: '{}',
    documentRevision: 'd',
  }) as NativeGeometryArtifact

function mesh(
  opts: {
    levels?: MeshLodLevel[]
    assetId?: string
    center?: [number, number, number]
    color?: [number, number, number, number]
    native?: NativeGeometryArtifact
  } = {},
): GMesh {
  const center = opts.center ?? [0, 0, 0]
  const m = {
    assetId: opts.assetId,
    nativeGeometry: opts.native,
    vb: { destroy() {} },
    ib: { destroy() {} },
    ic: 3,
    edgeIB: null,
    edgeIC: 0,
    vertices: new Float32Array(18),
    indices: new Uint32Array(3),
    edgeIndices: new Uint32Array(0),
    faceIds: new Uint32Array(1),
    bvh: {},
    provenance: [{ triangleStart: 0, triangleEnd: 1, backside: false, source: null }],
    color: opts.color ?? [1, 0, 0, 1],
    visible: true,
    transform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
    inverseTransform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
    worldBounds: {
      center,
      radius: Math.sqrt(3),
      min: [center[0] - 1, center[1] - 1, center[2] - 1],
      max: [center[0] + 1, center[1] + 1, center[2] + 1],
    },
  } as unknown as GMesh
  if (opts.levels) m.lod = createMeshLodState(m, opts.levels)
  return m
}

const camera: ViewerLodCamera = {
  eye: [0, 0, 10],
  distance: 10,
  eyeDistance: 10,
  fovY: Math.PI / 2,
  projection: 'perspective',
  viewportHeightPx: 1000,
}

function manualScheduler() {
  const tasks: (() => void)[] = []
  const schedule: IdleScheduler = task => tasks.push(task)
  return {
    schedule,
    flush() {
      while (tasks.length) tasks.shift()!()
    },
  }
}

const picking = () => ({ release: vi.fn(), query: vi.fn(() => null) }) as unknown as NativePickingCache

/** Provider whose requests resolve only when the test says so. */
function fakeProvider(summary: DisplayLevelSummaryEntry[] | null) {
  const summaries: { geometry: string; resolve: (value: DisplayLevelSummaryEntry[] | null) => void }[] = []
  const levels: { index: number; signal: AbortSignal; resolve: (value: DisplayLevelPayload) => void }[] = []
  const provider: DisplayLevelProvider = {
    summary: (geometry, _json, _signal) => new Promise(resolve => summaries.push({ geometry, resolve })),
    level: (_geometry, _json, index, signal) =>
      new Promise((resolve, reject) => {
        signal.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')))
        levels.push({ index, signal, resolve })
      }),
    forget: vi.fn(),
    dispose: vi.fn(),
  }
  return {
    provider,
    summaries,
    levels,
    answerSummaries: () => summaries.splice(0).forEach(request => request.resolve(summary)),
  }
}

const payload = (triangles: number): DisplayLevelPayload => {
  const vertices = new Float32Array(triangles * 18)
  const indices = new Uint32Array(triangles * 3).map((_, i) => i)
  return {
    vertices,
    indices,
    edgeIndices: new Uint32Array(2),
    faceIds: new Uint32Array(triangles),
    bvh: buildMeshBvh(vertices, indices),
  }
}

const flushPromises = () => new Promise(resolve => setTimeout(resolve, 0))
const summary: DisplayLevelSummaryEntry[] = [
  { triangleCount: 4, vertexCount: 12, errorUpper: 0.4 },
  { triangleCount: 16, vertexCount: 48, errorUpper: 0.1 },
]

beforeEach(() => vi.stubGlobal('GPUBufferUsage', { VERTEX: 1, INDEX: 2, COPY_DST: 4 }))
afterEach(() => {
  vi.unstubAllGlobals()
  vi.restoreAllMocks()
})

describe('bounding-box draw path', () => {
  it('batches the 12 edges of every boxed, drawable mesh into one line list', () => {
    const a = mesh({ color: [0.2, 0.4, 0.6, 0.5] })
    const b = mesh({ center: [5, 0, 0] })
    const hidden = mesh()
    a.displayBox = b.displayBox = hidden.displayBox = true
    hidden.visible = false
    const lines = buildBoundingBoxLines([a, b, hidden, mesh()], () => true)
    expect(lines.length).toBe(2 * BOX_VERTICES_PER_MESH * 7)
    expect(lines.slice(0, 7)).toEqual([-1, -1, -1, 0.2, 0.4, 0.6, 1])
    expect(buildBoundingBoxLines([a, b], index => index === 1).length).toBe(BOX_VERTICES_PER_MESH * 7)
  })

  it('rewrites the batched buffer only when the box set changes', () => {
    const { dev, writes, created } = device()
    const layer = new BoundingBoxLayer()
    const a = mesh()
    a.displayBox = true
    layer.update(dev, [a], () => true)
    layer.update(dev, [a], () => true)
    expect(writes).toHaveLength(1)
    expect(layer.boxCount).toBe(1)
    a.displayBox = false
    layer.update(dev, [a], () => true)
    expect(layer.boxCount).toBe(0)
    layer.release()
    expect(created.every(buffer => buffer.destroyed)).toBe(true)
  })

  it('intersects rays with the world box', () => {
    const bounds = { min: [-1, -1, -1] as [number, number, number], max: [1, 1, 1] as [number, number, number] }
    expect(rayBoxHit({ origin: [0, 0, 5], direction: [0, 0, -1] }, bounds)).toEqual({
      t: 4,
      point: [0, 0, 1],
      normal: [0, 0, 1],
    })
    expect(rayBoxHit({ origin: [3, 0, 5], direction: [0, 0, -1] }, bounds)).toBeNull()
    expect(rayBoxHit({ origin: [0, 0, 5], direction: [0, 0, 1] }, bounds)).toBeNull()
  })

  it('picks a boxed mesh by its box without querying triangles', () => {
    const boxed = mesh({ center: [0, 0, 0] })
    const behind = mesh({ center: [0, 0, -5] })
    boxed.displayBox = true
    const nativePicking = picking()
    const meshes = [boxed, behind]
    const hits = findHitCandidatesForRay(
      {
        ray: { origin: [0, 0, 10], direction: [0, 0, -1] },
        bounds: null,
        meshes,
        selectionMode: 'face',
        sectionEnabled: false,
        sectionNormal: [0, 0, 1],
        sectionOffset: 0,
        sceneAabbIndex: buildSceneAabbIndex(meshes.map((m, id) => ({ id, bounds: m.worldBounds }))),
        nativePicking,
        isMeshVisible: () => true,
      },
      4,
    )
    expect(hits[0].value).toMatchObject({
      meshIndex: 0,
      triangleIndex: -1,
      faceId: null,
      point: [0, 0, 1],
      normal: [0, 0, 1],
    })
    // Only the unboxed mesh behind reaches the triangle BVH.
    expect(nativePicking.query).toHaveBeenCalledTimes(1)
  })

  it('marks meshes on the bounding-box representation for the frame encoder', () => {
    vi.spyOn(viewerLod, 'selectViewerLod').mockImplementation(levels => ({
      level: levels.length - 1,
      projectedError: 0,
      withinPixelBudget: true,
      reason: 'initial',
    }))
    const { dev } = device()
    const near = mesh({ levels: [level(4, 0.4), level(16, 0.1)], center: [0, 0, 5] })
    const far = mesh({ levels: [level(4, 0.4), level(16, 0.1)], center: [0, 0, -5000] })
    updateMeshLods(dev, [near, far], camera, picking(), undefined, { budget: DISPLAY_BUDGET_PRESETS.medium })
    expect(near.displayBox).toBe(false)
    expect(far.displayBox).toBe(true)
    expect(far.lod?.current).toBe(0)
  })
})

describe('display budget presets', () => {
  it('orders presets by memory and detail', () => {
    const { low, medium, high } = DISPLAY_BUDGET_PRESETS
    expect(low.budgetBytes).toBeLessThan(medium.budgetBytes)
    expect(medium.budgetBytes).toBeLessThan(high.budgetBytes)
    expect(low.targetErrorPixels).toBeGreaterThan(high.targetErrorPixels)
  })

  it('a low budget boxes a mesh a high budget refines', () => {
    const m = mesh({ levels: [level(4, 0.4), level(16, 0.1)], center: [0, 0, -60] })
    expect(planDisplayCaps([m], camera, DISPLAY_BUDGET_PRESETS.low).get(m)?.representation).toBe('bounding-box')
    expect(planDisplayCaps([m], camera, DISPLAY_BUDGET_PRESETS.high).get(m)?.representation).toBe('exact')
  })

  it('persists the preset per viewer and applies it to the renderer', () => {
    const store = new Map<string, string>()
    vi.stubGlobal('localStorage', {
      getItem: (key: string) => store.get(key) ?? null,
      setItem: (key: string, value: string) => store.set(key, value),
    })
    const renderer = new RecordingRenderer()
    const { displayQuality, setDisplayQuality } = useDisplayQuality({ viewer: 'main', getRenderer: () => renderer })
    expect(displayQuality.value).toBe('medium')
    setDisplayQuality('low')
    expect(store.get(displayQualityStorageKey('main'))).toBe('low')
    expect(renderer.getDisplayBudget()).toBe(DISPLAY_BUDGET_PRESETS.low)
    expect(readDisplayQuality('main')).toBe('low')
    expect(readDisplayQuality('other')).toBe('medium')
    store.set(displayQualityStorageKey('other'), 'bogus')
    expect(readDisplayQuality('other')).toBe('medium')
  })
})

describe('levels produced on demand', () => {
  it('requests one summary per geometry and shares placeholder levels between instances', async () => {
    const fake = fakeProvider(summary)
    const onSummary = vi.fn()
    const source = new OnDemandLevelSource(fake.provider, geometryKey, onSummary)
    const a = mesh({ assetId: 'asset:a', native: snapshot() })
    const b = mesh({ assetId: 'asset:a', native: snapshot() })
    expect(source.attach([a, b])).toEqual([])
    expect(fake.summaries).toHaveLength(1)
    fake.answerSummaries()
    await flushPromises()
    expect(onSummary).toHaveBeenCalledTimes(1)
    expect(source.attach([a, b])).toEqual([a, b])
    expect(a.lod!.levels).toBe(b.lod!.levels)
    expect(isDeferredLevel(a.lod!.levels[1])).toBe(true)
    expect(levelByteSize(a.lod!.levels[1])).toBe(48 * 24 + 16 * 12)
    // Meshes with published levels or without a B-rep snapshot are left alone.
    expect(OnDemandLevelSource.eligible(mesh({ assetId: 'asset:x' }))).toBe(false)
  })

  it('produces only the targeted level, uploads it and cancels a superseded request', async () => {
    const fake = fakeProvider(summary)
    const source = new OnDemandLevelSource(fake.provider, geometryKey)
    const m = mesh({ assetId: 'asset:a', native: snapshot() })
    source.attach([m])
    fake.answerSummaries()
    await flushPromises()
    source.attach([m])
    const { dev, created } = device()
    const idle = manualScheduler()
    const store = new ProgressiveLevelStore(1e6, undefined, idle.schedule, source)

    expect(store.resolve(dev, m, 1)).toBeNull()
    expect(store.state(m, 1)).toBe('pending')
    expect(fake.levels.map(request => request.index)).toEqual([1])
    // The scheduler now wants the coarse level: the fine request is stale.
    store.resolve(dev, m, 0)
    expect(fake.levels[0].signal.aborted).toBe(true)
    await flushPromises()
    expect(store.state(m, 1)).toBe('missing')
    fake.levels[1].resolve(payload(4))
    await flushPromises()
    expect(isDeferredLevel(m.lod!.levels[0])).toBe(false)
    expect(m.lod!.levels[0].provenance).toEqual([{ triangleStart: 0, triangleEnd: 4, backside: false, source: null }])
    idle.flush()
    expect(store.state(m, 0)).toBe('ready')
    expect(created).toHaveLength(2)
    expect(isDeferredLevel(m.lod!.levels[1])).toBe(true)
  })

  it('never binds a placeholder through the synchronous path', async () => {
    vi.spyOn(viewerLod, 'selectViewerLod').mockImplementation(levels => ({
      level: levels.length - 1,
      projectedError: 0,
      withinPixelBudget: true,
      reason: 'initial',
    }))
    const fake = fakeProvider(summary)
    const source = new OnDemandLevelSource(fake.provider, geometryKey)
    const m = mesh({ assetId: 'asset:a', native: snapshot() })
    source.attach([m])
    fake.answerSummaries()
    await flushPromises()
    source.attach([m])
    const { dev, created } = device()
    expect(updateMeshLods(dev, [m], camera, picking())).toEqual([])
    expect(m.lod?.current).toBeNull()
    expect(created).toHaveLength(0)
  })

  it('drives summaries, selection and boxes through the display controller', async () => {
    vi.spyOn(viewerLod, 'selectViewerLod').mockImplementation(levels => ({
      level: levels.length - 1,
      projectedError: 0,
      withinPixelBudget: true,
      reason: 'initial',
    }))
    const fake = fakeProvider(summary)
    const requestRender = vi.fn()
    const controller = new MeshDisplayController(requestRender, () => fake.provider)
    const { dev } = device()
    const near = mesh({ assetId: 'asset:near', native: snapshot('{"geometry":{"n":1}}'), center: [0, 0, 5] })
    const far = mesh({ assetId: 'asset:far', native: snapshot('{"geometry":{"f":1}}'), center: [0, 0, -5000] })
    const residency = new GpuResidencyCache(1e6)
    const frame = (key: string) => ({
      dev,
      meshes: [near, far],
      camera,
      selectionKey: key,
      sceneEpoch: 1,
      residency,
      nativePicking: picking(),
      drawable: () => true,
    })
    expect(controller.update(frame('a'))).toEqual([])
    expect(fake.summaries).toHaveLength(2)
    fake.answerSummaries()
    await flushPromises()
    expect(requestRender).toHaveBeenCalled()
    controller.update(frame('a'))
    expect(near.lod).toBeDefined()
    expect(far.displayBox).toBe(true)
    expect(controller.boxes.boxCount).toBe(1)
    expect(fake.levels.map(request => request.index)).toEqual([1, 0])
    expect(controller.setBudget(DISPLAY_BUDGET_PRESETS.medium)).toBe(false)
    expect(controller.setBudget(DISPLAY_BUDGET_PRESETS.high)).toBe(true)
    controller.dispose()
    expect(fake.provider.dispose).toHaveBeenCalled()
  })
})

describe('display level worker runtime', () => {
  const service = () =>
    new DisplayLevelService(() =>
      [4, 16].map((triangles, i) => ({
        ...payload(triangles),
        triangleCount: triangles,
        vertexCount: triangles * 3,
        errorUpper: 1 / (i + 1),
      })),
    )

  it('answers summaries and transfers copies of cached levels', () => {
    const posted: { message: DisplayLevelResponse; transfer: Transferable[] }[] = []
    const tasks: (() => void)[] = []
    const handle = createDisplayLevelWorkerHandler(
      (message, transfer) => posted.push({ message, transfer }),
      service(),
      task => tasks.push(task),
    )
    handle({ type: 'summary', id: 1, geometry: 'g', geometryJson: '{"geometry":{}}' })
    handle({ type: 'level', id: 2, geometry: 'g', geometryJson: '{"geometry":{}}', index: 1 })
    handle({ type: 'level', id: 3, geometry: 'g', geometryJson: '{"geometry":{}}', index: 1 })
    while (tasks.length) tasks.shift()!()
    expect(posted[0].message).toEqual({
      id: 1,
      ok: true,
      summary: [
        { triangleCount: 4, vertexCount: 12, errorUpper: 1 },
        { triangleCount: 16, vertexCount: 48, errorUpper: 0.5 },
      ],
    })
    expect(posted[1].transfer.length).toBeGreaterThan(0)
    const second = posted[2].message as Extract<DisplayLevelResponse, { level: unknown }>
    expect(second.level.indices.length).toBe(48)
  })

  it('drops a queued request when it is cancelled before it runs', () => {
    const posted: DisplayLevelResponse[] = []
    const tasks: (() => void)[] = []
    const handle = createDisplayLevelWorkerHandler(
      message => posted.push(message),
      service(),
      task => tasks.push(task),
    )
    handle({ type: 'level', id: 1, geometry: 'g', geometryJson: '{"geometry":{}}', index: 0 })
    handle({ type: 'level', id: 2, geometry: 'g', geometryJson: '{"geometry":{}}', index: 1 })
    handle({ type: 'cancel', id: 1 })
    while (tasks.length) tasks.shift()!()
    expect(posted.map(message => message.id)).toEqual([2])
  })

  it('reports levels it cannot produce', () => {
    const posted: DisplayLevelResponse[] = []
    const handle = createDisplayLevelWorkerHandler(
      message => posted.push(message),
      new DisplayLevelService(() => null),
      task => task(),
    )
    handle({ type: 'summary', id: 1, geometry: 'g', geometryJson: '{"geometry":{}}' })
    handle({ type: 'level', id: 2, geometry: 'g', geometryJson: '{"geometry":{}}', index: 0 })
    expect(posted[0]).toEqual({ id: 1, ok: true, summary: null })
    expect(posted[1]).toMatchObject({ id: 2, ok: false })
  })
})
