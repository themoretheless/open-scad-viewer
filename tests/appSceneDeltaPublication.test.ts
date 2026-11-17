import { describe, expect, it, vi } from 'vitest'
import type { MeshData } from '../src/core/mesh'
import { reconcileCarriedSelection, SceneIntentGenerations } from '../src/app/sceneDeltaPublication'
import { RecordingRenderer } from '../src/render/recordingRenderer'
import { createRendererOperations } from '../src/composables/useBuildAndRendererLifecycle/rendererOperations'
import type { RendererOperationsContext } from '../src/composables/useBuildAndRendererLifecycle/rendererOperationsContext'
import { BuildPerformanceHistory } from '../src/app/buildPerformance'

vi.mock('../src/kernel/meshSurfaceGroups', () => ({ withSelectionSurfaces: (mesh: MeshData) => mesh }))

const mesh = (entityId: string, assetId: string, size = 1): MeshData =>
  ({
    entityId,
    geometryAssetId: assetId,
    vertices: new Float32Array(18 * size),
    indices: new Uint32Array([0, 1, 2]),
    faceIds: new Uint32Array([0]),
    transform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
  }) as unknown as MeshData

describe('SceneIntentGenerations', () => {
  it('drops intents older than the newest accepted generation', () => {
    const gate = new SceneIntentGenerations()
    expect(gate.current).toBeNull()
    expect(gate.accept(2)).toBe(true)
    expect(gate.accept(2)).toBe(true)
    expect(gate.accept(1)).toBe(false)
    expect(gate.isCurrent(2)).toBe(true)
    expect(gate.accept(Number.NaN)).toBe(false)
    expect(gate.accept(3)).toBe(true)
    expect(gate.current).toBe(3)
  })
})

describe('reconcileCarriedSelection', () => {
  const target = (selectedIndex: number | null, isIsolated = false) => ({
    selectedIndex,
    isIsolated,
    selectMesh: vi.fn(),
    toggleIsolateSelection: vi.fn(() => true),
    restoreNativeFaceSelection: vi.fn(() => false),
  })

  it('keeps a selection the renderer already carried', () => {
    const t = target(1, true)
    expect(reconcileCarriedSelection(t, { nextSelectedIndex: 1, nextIsolated: true }, null)).toBe('carried')
    expect(t.selectMesh).not.toHaveBeenCalled()
    expect(t.toggleIsolateSelection).not.toHaveBeenCalled()
  })

  it('applies the plan when the renderer dropped or moved the selection', () => {
    const t = target(null)
    expect(reconcileCarriedSelection(t, { nextSelectedIndex: 0, nextIsolated: true }, null)).toBe('restored')
    expect(t.selectMesh).toHaveBeenCalledWith(0)
    expect(t.toggleIsolateSelection).toHaveBeenCalledTimes(1)
  })

  it('clears a carried selection the plan rejects', () => {
    const t = target(2)
    expect(reconcileCarriedSelection(t, { nextSelectedIndex: null, nextIsolated: false }, null)).toBe('cleared')
    expect(t.selectMesh).toHaveBeenCalledWith(null)
  })
})

async function appPublication() {
  const renderer = new RecordingRenderer()
  await renderer.init({ clientHeight: 100 } as HTMLCanvasElement)
  const ref = <T>(value: T) => ({ value })
  const sceneMeshes = ref<MeshData[]>([]),
    meshVisibility = ref<boolean[]>([]),
    selectedMesh = ref<number | null>(null),
    isolated = ref(false)
  const sceneController = {
    publish: (state: {
      meshes: MeshData[]
      visibility: boolean[]
      selectedIndex: number | null
      isolated: boolean
    }) => {
      sceneMeshes.value = state.meshes
      meshVisibility.value = state.visibility
      selectedMesh.value = state.selectedIndex
      isolated.value = state.isolated
    },
    applyRendererHover: () => {},
    applyRendererSelection: (index: number | null) => {
      selectedMesh.value = index
    },
    setMeasurement: () => {},
  }
  const options = {
    code: ref('cube(1);'),
    renderDuration: ref(0),
    error: ref(''),
    editorDiagnostic: ref(null),
    translate: (key: string) => key,
    mainSelectedIndices: ref([]),
    renderedSource: ref(''),
    sceneMeshes,
    meshVisibility,
    selectedMesh,
    isolated,
    measurement: ref(null),
    measureActive: ref(false),
    sceneController,
    meshCount: ref(0),
    triangleCount: ref(0),
    volume: ref(0),
    surfaceArea: ref(0),
    warnings: ref([]),
    renderedQuality: ref(null),
    autoRender: ref(false),
    syncSourceHighlightFromEditor: () => {},
    syncSectionRange: () => {},
  }
  const context = {
    options,
    renderer,
    buildGeneration: 1,
    nextPerformanceFrameToken: 1,
    mainEditSelection: null,
    fitNextRender: false,
    clearSweepViewportPreview: () => {},
    pendingPerformanceFrame: null,
    performanceHistory: new BuildPerformanceHistory(),
    performanceSamples: ref([]),
    autoBuildScheduler: { observe: () => {}, promote: () => {} },
  } as unknown as RendererOperationsContext
  const operations = createRendererOperations(context)
  const publish = (documentRevision: number, meshes: MeshData[]) =>
    operations.handleGeometryResponse({
      status: 'succeeded',
      documentRevision,
      quality: 'full',
      reduced: false,
      durationMs: 1,
      meshes,
      volume: 0,
      surfaceArea: 0,
      warnings: [],
      timings: {},
    } as never)
  return { renderer, context, publish, selectedMesh }
}

describe('App scene-delta publication', () => {
  it('re-uploads only the edited object and keeps selection and camera', async () => {
    const { renderer, context, publish, selectedMesh } = await appPublication()
    const a = mesh('a', 'asset-a'),
      b = mesh('b', 'asset-b'),
      c = mesh('c', 'asset-c')
    publish(1, [a, b, c])
    expect(renderer.sceneUploadMetrics).toMatchObject({ geometryBuffersCreated: 6, reusedEntities: 0 })
    renderer.selectMesh(0)
    selectedMesh.value = 0
    renderer.applyKeyboardCameraStep({ kind: 'orbit', dx: 24, dy: 0 })
    const camera = renderer.getCameraState()
    const selects = renderer.snapshot().calls.selectMesh ?? 0

    context.buildGeneration = 2
    publish(2, [a, b, mesh('c', 'asset-c2', 2)])
    expect(renderer.sceneUploadMetrics).toEqual({
      geometryUploadBytes: 18 * 2 * 4 + 12,
      geometryBuffersCreated: 2,
      reusedEntities: 2,
    })
    expect(renderer.selectedIndex).toBe(0)
    expect(selectedMesh.value).toBe(0)
    expect(renderer.getCameraState()).toEqual(camera)
    // The carried selection is not re-applied, so the face hit survives.
    expect(renderer.snapshot().calls.selectMesh ?? 0).toBe(selects)
  })

  it('ignores a publication from a stale generation', async () => {
    const { renderer, context, publish } = await appPublication()
    publish(1, [mesh('a', 'asset-a')])
    context.buildGeneration = 3
    publish(3, [mesh('a', 'asset-a'), mesh('b', 'asset-b')])
    expect(renderer.snapshot().meshCount).toBe(2)
    // The revision gate in the coordinator moved back; the generation gate still drops it.
    context.buildGeneration = 2
    publish(2, [mesh('x', 'asset-x')])
    expect(renderer.snapshot().meshCount).toBe(2)
    expect(context.options.sceneMeshes.value).toHaveLength(2)
  })
})
