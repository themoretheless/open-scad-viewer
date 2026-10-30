import {createViewportOperations} from './useViewportLayerAndPolygons/viewportOperations'
import {
  computed,
  nextTick,
  onUnmounted,
  ref,
  watch,
  watchEffect,
  type ComponentPublicInstance,
  type ComputedRef,
  type Ref,
  type ShallowRef,
} from 'vue'
import type { DirectBody, DirectDocument, Point2 } from '../../services/directModeling'
import { createDirectProjector, directExtrusionTool, directFaceShade, projectDirectPoint } from '../../services/directModelingTools'
import type { Vec3 } from '../../services/directSketchGeometry'
import type { PolygonMesh } from '../../services/geometry/polygon'
import { SolidDisplayCache, type DisplayMesh } from '../../services/solidDisplayCache'
import { SolidDisplayQueue } from '../../services/solidDisplayQueue'
import { isSolidGpuSupported, smoothTriangleList, SolidGpuLayer, type SolidGpuBody } from '../../services/solidGpuView'
import { solidMaterialRgb } from '../../services/solidMaterial'
import type { SolidNurbsSurface } from '../../services/solidNurbs'
import { createSolidPreviewWorker } from '../../services/solidPreviewWorker'
import type { SolidSurfaceDisplayQueue } from '../../services/solidSurfaceDisplayQueue'
import { TransparentBsp } from '../../services/transparentBsp'
import { prepareVrPolygons } from '../../services/vrScene'
import { COARSE_DISPLAY, type Pane } from './helpers'

export interface UseViewportLayerAndPolygonsOptions {
  props: { open: boolean }
  emitBackend: (gpu: boolean) => void
  kernelReady: Ref<boolean>
  document: ShallowRef<DirectDocument>
  camera: Ref<{ yaw: number; pitch: number }>
  views: Ref<Record<Pane, number>>
  centers: Ref<Record<Pane, Point2>>
  cameraDragging: Ref<boolean>
  heightDragging: Ref<boolean>
  previewingTransform: Ref<boolean>
  selectedIds: ComputedRef<string[]>
  selectedBody: ComputedRef<DirectBody | undefined>
  selectedFaceTriangles: ComputedRef<Set<number>>
  pickMode: Ref<'body' | 'face' | 'edge' | 'vertex'>
  hovered: Ref<string>
  extrusionMode: Ref<'new' | 'union' | 'difference'>
  previewBody: ShallowRef<ReturnType<typeof directExtrusionTool> | null>
  previewReplacesTarget: ComputedRef<boolean>
  renderBodies: ComputedRef<DirectBody[]>
  surfaceDisplayQueue: SolidSurfaceDisplayQueue
  surfaceDisplayVersion: Ref<number>
  objectInView: (id: string) => boolean
  getAdvancedPreview: () => {
    document: DirectDocument | null
    bodyIds: Set<string>
    surfaceIds: string[]
  }
  getSubtract: () => { a: string[]; b: string[]; active: 'a' | 'b' } | null
  setNotice: (message: string) => void
  getNotice: () => string
  label: (ru: string, en: string) => string
}

export function useViewportLayerAndPolygons(options: UseViewportLayerAndPolygonsOptions) {
  const {hasDragNodes, setDragNodes, fpsTick, applyDragPreviewTransform, clearDragPreview, settleAfterDrag, triangleNormals, displayMeshFor, shadeFromNormal, stopGpuLayer, retryGpuLayer, mountGpuCanvas, refreshDisplayPreparation, vrSnapshot, meshPolygons, polygonColor, surfacePolygons} = createViewportOperations({
    get fpsPrevious() { return fpsPrevious },
    set fpsPrevious(value: typeof fpsPrevious) { fpsPrevious = value },
    get fpsIntervals() { return fpsIntervals },
    set fpsIntervals(value: typeof fpsIntervals) { fpsIntervals = value },
    get fpsSince() { return fpsSince },
    set fpsSince(value: typeof fpsSince) { fpsSince = value },
    get fps() { return fps },
    get frameMs() { return frameMs },
    get drawMs() { return drawMs },
    get gpuLayer() { return gpuLayer },
    set gpuLayer(value: typeof gpuLayer) { gpuLayer = value },
    get fpsHandle() { return fpsHandle },
    set fpsHandle(value: typeof fpsHandle) { fpsHandle = value },
    get dragNodes() { return dragNodes },
    set dragNodes(value: typeof dragNodes) { dragNodes = value },
    get dragOffsetIds() { return dragOffsetIds },
    set dragOffsetIds(value: typeof dragOffsetIds) { dragOffsetIds = value },
    get settling() { return settling },
    get settleHandle() { return settleHandle },
    set settleHandle(value: typeof settleHandle) { settleHandle = value },
    get displayVersion() { return displayVersion },
    get options() { return options },
    get smoothDisplay() { return smoothDisplay },
    get displayQueue() { return displayQueue },
    get gpuGeneration() { return gpuGeneration },
    set gpuGeneration(value: typeof gpuGeneration) { gpuGeneration = value },
    get gpuResize() { return gpuResize },
    set gpuResize(value: typeof gpuResize) { gpuResize = value },
    get gpuActive() { return gpuActive },
    get gpuInitializing() { return gpuInitializing },
    get gpuCanvas() { return gpuCanvas },
    set gpuCanvas(value: typeof gpuCanvas) { gpuCanvas = value },
    get gpuRetryVisible() { return gpuRetryVisible },
    get gpuFailureNotice() { return gpuFailureNotice },
    get gpuInitStarted() { return gpuInitStarted },
    set gpuInitStarted(value: typeof gpuInitStarted) { gpuInitStarted = value },
    get cancelDisplayPreparation() { return cancelDisplayPreparation },
    get displayGeneration() { return displayGeneration },
    set displayGeneration(value: typeof displayGeneration) { displayGeneration = value },
    get displayPending() { return displayPending },
    get polygons() { return polygons },
  })

  /**
   * Frame rate of the viewport, sampled over half-second windows.
   *
   * Measured from the browser's own frame callbacks, so it reflects what the eye sees
   * rather than the cost of any one handler.
   */
  const fps = ref(0)
  /** Median frame interval of the last window. A steady 33.3 ms means the frames are
   * being paced externally; uneven, longer intervals mean the work itself is too slow. */
  const frameMs = ref(0)
  /** GPU time of the latest draw, sampled alongside the frame rate. */
  const drawMs = ref(0)
  let fpsHandle = 0
  let fpsSince = 0
  let fpsPrevious = 0
  let fpsIntervals: number[] = []

  let gpuLayer: SolidGpuLayer | null = null
  let gpuResize: ResizeObserver | null = null
  let gpuInitStarted = false
  let gpuGeneration = 0
  let gpuCanvas: HTMLCanvasElement | null = null
  const gpuActive = ref(false)
  const gpuInitializing = ref(false)
  const gpuRetryVisible = ref(false)
  const gpuFailureNotice = ref('')

  let dragNodes: SVGElement[] = []
  let dragOffsetIds: string[] = []

  /**
   * Rebuilding the thousands of hit-test polygons after a commit costs far more than a
   * frame, and doing it the instant a drag ends is felt as a stall. The dragged elements
   * already carry the exact translation that was committed, so they stay correct for both
   * display and picking; the rebuild is deferred until the pointer has settled.
   */
  const settling = ref(false)
  let settleHandle = 0



  const smoothDisplay = ref(true)
  // Exact geometry keys survive history clones and include retained B-rep changes.
  const displayCache = new SolidDisplayCache()

  const displayWorker = createSolidPreviewWorker()
  const displayQueue = new SolidDisplayQueue(displayWorker, displayCache)
  const displayVersion = ref(0)
  const displayPending = ref(false)
  let displayGeneration = 0

  function cancelDisplayPreparation() {
    displayGeneration++
    displayQueue.cancel()
    displayPending.value = false
  }
  onUnmounted(() => {
    cancelDisplayPreparation()
    displayWorker.dispose()
  })

  watch(gpuActive, value => options.emitBackend(value), { immediate: true })

  watch(
    () => options.props.open,
    open => {
      if (!open) {
        stopGpuLayer()
        gpuRetryVisible.value = false
      } else if (gpuCanvas) {
        void nextTick(retryGpuLayer)
      }
    },
  )

  onUnmounted(() => {
    stopGpuLayer()
    gpuCanvas = null
    if (fpsHandle) cancelAnimationFrame(fpsHandle)
    fpsHandle = 0
    clearTimeout(settleHandle)
  })

  watch(
    () => options.props.open,
    open => {
      // The DOM-less test renderer has no frame callbacks, and a hidden workspace need not count.
      if (typeof requestAnimationFrame !== 'function') return
      if (fpsHandle) {
        cancelAnimationFrame(fpsHandle)
        fpsHandle = 0
      }
      if (!open) {
        fps.value = 0
        frameMs.value = 0
        return
      }
      fpsIntervals = []
      fpsPrevious = 0
      fpsSince = performance.now()
      fpsHandle = requestAnimationFrame(fpsTick)
    },
    { immediate: true },
  )

  watch(
    [smoothDisplay, options.kernelReady],
    () => {
      cancelDisplayPreparation()
      displayCache.clear()
      displayVersion.value++
    },
    { flush: 'sync' },
  )

  let surfaceGpuCache = new Map<PolygonMesh, ReturnType<typeof smoothTriangleList>>()
  onUnmounted(() => surfaceGpuCache.clear())

  const gpuBodies = computed<SolidGpuBody[]>(() => {
    if (!gpuActive.value) {
      surfaceGpuCache.clear()
      return []
    }
    const adv = options.getAdvancedPreview()
    const subtract = options.getSubtract()
    const hideSelected = !!adv.document
    const items = [
      ...options.renderBodies.value.filter(b => !hideSelected || !adv.bodyIds.has(b.id)).map(b => ({ b, preview: 0 })),
      ...(adv.document?.bodies ?? []).filter(b => adv.bodyIds.has(b.id)).map(b => ({ b, preview: 1 })),
      ...(options.previewBody.value && !options.previewReplacesTarget.value ? [{ b: options.previewBody.value, preview: 2 }] : []),
    ]
    const bodies = items.flatMap(({ b, preview }) => {
      const display = displayMeshFor(b)
      const flat = (display.flat ??= smoothTriangleList(display.mesh.positions, display.mesh.indices))
      const count = display.mesh.indices.length / 3
      const bodyHue = subtract?.a.includes(b.id)
        ? 45
        : subtract?.b.includes(b.id)
          ? 350
          : options.selectedIds.value.includes(b.id)
            ? 266
            : options.hovered.value === b.id
              ? 190
              : b.material
                ? -1
                : 220
      const faceHighlight = options.selectedBody.value?.id === b.id && options.pickMode.value === 'face'
      const hues = new Float32Array(count)
      for (let i = 0; i < count; i++) {
        const working = display.map ? display.map[i] : i
        hues[i] = preview ? -1 : faceHighlight && options.selectedFaceTriangles.value.has(working) ? 40 : bodyHue
      }
      return [
        {
          id: b.id,
          positions: flat.positions,
          normals: flat.normals,
          hues,
          color: preview
            ? solidMaterialRgb(
                preview === 2
                  ? options.extrusionMode.value === 'difference'
                    ? '#ff647c'
                    : '#75e4b8'
                  : b.id === 'preview-split'
                    ? '#ffc977'
                    : '#77eac5',
              )
            : b.material
              ? solidMaterialRgb(b.material.color)
              : undefined,
          metallic: preview ? 0 : b.material?.metallic,
          roughness: b.material?.roughness,
          opacity: preview ? (preview === 2 ? 0.28 : 0.45) : b.material?.opacity,
        },
      ]
    })
    void options.surfaceDisplayVersion.value
    const retainedSurfaceMeshes = new Map<PolygonMesh, ReturnType<typeof smoothTriangleList>>()
    const surfaceItems = [
      ...(options.document.value.surfaces ?? [])
        .filter(item => options.objectInView(item.id) && !(hideSelected && adv.surfaceIds.includes(item.id)))
        .map(item => ({ item, preview: false })),
      ...(adv.document?.surfaces ?? []).filter(item => adv.surfaceIds.includes(item.id)).map(item => ({ item, preview: true })),
    ]
    for (const { item, preview } of surfaceItems) {
      const mesh = options.surfaceDisplayQueue.get(item)
      if (!mesh) continue
      const flat = surfaceGpuCache.get(mesh) ?? smoothTriangleList(mesh.positions, mesh.indices)
      retainedSurfaceMeshes.set(mesh, flat)
      bodies.push({
        id: item.id,
        positions: flat.positions,
        normals: flat.normals,
        hues: new Float32Array(mesh.indices.length / 3).fill(!preview && options.selectedIds.value.includes(item.id) ? 266 : -1),
        color: preview ? [0.467, 0.918, 0.773] : [0.192, 0.373, 0.447],
        metallic: undefined,
        roughness: undefined,
        opacity: preview ? 0.45 : 0.72,
      })
    }
    surfaceGpuCache = retainedSurfaceMeshes
    return bodies
  })

  watch(gpuBodies, bodies => gpuLayer?.setBodies(bodies), { flush: 'post' })
  watchEffect(() => {
    if (!gpuActive.value) return
    const size = options.views.value['3d']
    const center = options.centers.value['3d']
    gpuLayer?.setView({ camera: options.camera.value, viewBox: [center[0] - size / 2, center[1] - size / 2, size] })
  })
  watch(gpuActive, active => {
    if (active && gpuLayer) gpuLayer.setBodies(gpuBodies.value)
  })

  /**
   * While the GPU layer draws the scene, these thousands of polygons are transparent hit
   * targets and nothing more. Re-projecting and re-rendering them mid-orbit costs more
   * than a frame and changes nothing anyone can see, so they are held still until the
   * camera settles, then rebuilt once for picking.
   */
  const worldPointProjection = (p: number[]): [number, number, number] => [p[0], p[1], p[2] ?? 0]
  const cpuTransparency = computed(() => {
    if (gpuActive.value) return null
    const adv = options.getAdvancedPreview()
    const bodies = [
      ...options.renderBodies.value.filter(b => !adv.document || !adv.bodyIds.has(b.id)).map(b => ({ b, preview: 0 })),
      ...(adv.document?.bodies ?? []).filter(b => adv.bodyIds.has(b.id)).map(b => ({ b, preview: 1 })),
      ...(!options.previewReplacesTarget.value && options.previewBody.value ? [{ b: options.previewBody.value, preview: 2 }] : []),
    ]
    const surfaces = [
      ...(options.document.value.surfaces ?? [])
        .filter(item => options.objectInView(item.id) && (!adv.document || !adv.surfaceIds.includes(item.id)))
        .map(item => ({ item, preview: 0 })),
      ...(adv.document?.surfaces ?? []).filter(item => adv.surfaceIds.includes(item.id)).map(item => ({ item, preview: 1 })),
    ]
    if (!surfaces.length && !bodies.some(({ b, preview }) => preview || (b.material?.opacity ?? 1) < 1)) return null
    const items = [
      ...bodies.flatMap(({ b, preview }) => meshPolygons(b, preview, worldPointProjection, true)),
      ...surfaces.flatMap(({ item, preview }) => {
        void options.surfaceDisplayVersion.value
        const mesh = options.surfaceDisplayQueue.get(item)
        return mesh
          ? meshPolygons({ id: item.id, name: item.name, mesh }, preview, worldPointProjection, true).map(p => ({
              ...p,
              surface: true,
            }))
          : []
      }),
    ]
    try {
      return {
        items,
        tree: new TransparentBsp(items.map(p => ({ owner: p.key, triangle: p.vertices as unknown as [Vec3, Vec3, Vec3] }))),
      }
    } catch {
      return { items, tree: null }
    }
  })

  let restingPolygons: ReturnType<typeof meshPolygons> = []
  const polygonView = computed(() => {
    let limited = false
    if (gpuActive.value && (options.cameraDragging.value || settling.value) && restingPolygons.length) {
      return { items: restingPolygons, limited }
    }
    const projectPoint = createDirectProjector(options.camera.value)
    const transparent = cpuTransparency.value
    if (!transparent?.tree) {
      restingPolygons = options.renderBodies.value.flatMap(b => meshPolygons(b, 0, projectPoint)).sort((a, b) => a.depth - b.depth)
    }
    if (transparent) {
      limited = !transparent.tree
      {
        const source = new Map(transparent.items.map(p => [p.key, p]))
        // projectDirectPoint([0, 0, 1], camera) = [sin(yaw)*cos(pitch), cos(yaw)*cos(pitch), sin(pitch)]?
        // Actually projectPoint([0, 0, 1]) is not identical, let's compute direction from projectDirectPoint or basis:
        // Wait: projectDirectPoint([1,0,0], camera) gives [cos(yaw), -sin(yaw)*sin(pitch), sin(yaw)*cos(pitch)]!
        // So the 3rd component of projectDirectPoint([1,0,0]), projectDirectPoint([0,1,0]), projectDirectPoint([0,0,1]) is literally [sin(yaw)*cos(pitch), cos(yaw)*cos(pitch), sin(pitch)]!
        const ex = projectDirectPoint([1, 0, 0], options.camera.value)
        const ey = projectDirectPoint([0, 1, 0], options.camera.value)
        const ez = projectDirectPoint([0, 0, 1], options.camera.value)
        const direction: Vec3 = [ex[2], ey[2], ez[2]]
        let fragments
        try {
          fragments = transparent.tree?.ordered(direction)
        } catch {
          limited = true
        }
        restingPolygons = (fragments ?? transparent.items.map(p => ({ owner: p.key, triangle: p.vertices }))).flatMap((fragment, i) => {
          const original = source.get(fragment.owner)!
          const vertices = fragment.triangle.map(p => projectPoint([...p]))
          const a = vertices[0]
          const b = vertices[1]
          const c = vertices[2]
          if (
            original.closed &&
            !original.preview &&
            !original.surface &&
            (original.material?.opacity ?? 1) === 1 &&
            (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]) <= 0
          ) {
            return []
          }

          return [
            {
              ...original,
              key: original.key + ':split:' + i,
              vertices,
              depth: vertices.reduce((sum, p) => sum + p[2] / 3, 0),
              points: vertices.map(p => p.slice(0, 2).join(',')).join(' '),
              shade: shadeFromNormal(original.normal!, projectPoint),
            },
          ]
        })
        if (limited) restingPolygons.sort((a, b) => a.depth - b.depth)
      }
    }
    return { items: restingPolygons, limited }
  })

  const polygons = computed(() => polygonView.value.items)
  const cpuOrbitReady = ref(false)
  const cpuOrbitActive = computed(() => !gpuActive.value && options.cameraDragging.value && polygons.value.length > 1200)

  const cpuOrbitTriangles = computed(() =>
    cpuOrbitActive.value
      ? polygons.value.map(p => ({
          vertices: p.vertices,
          color: polygonColor(p),
          opacity: p.preview ? (p.preview === 2 ? 0.28 : 0.45) : p.surface ? 0.72 : (p.material?.opacity ?? 1),
          stroke: !p.preview && !p.surface && (p.material?.opacity ?? 1) === 1,
        }))
      : [],
  )

  let restingSurfacePolygons: ReturnType<typeof meshPolygons> = []
  const nurbsSurfacePolygons = computed(() => {
    if (gpuActive.value && options.cameraDragging.value && restingSurfacePolygons.length) return restingSurfacePolygons
    restingSurfacePolygons = !options.kernelReady.value
      ? []
      : (options.document.value.surfaces ?? [])
          .filter(item => options.objectInView(item.id))
          .flatMap(item => surfacePolygons(item).sort((a, b) => a.depth - b.depth))
    return restingSurfacePolygons
  })

  const ghostPolygons = computed(() =>
    options.previewBody.value ? meshPolygons(options.previewBody.value, 2).sort((a, b) => a.depth - b.depth) : [],
  )

  return {
    fps,
    frameMs,
    drawMs,
    settling,
    hasDragNodes,
    setDragNodes,
    applyDragPreviewTransform,
    clearDragPreview,
    settleAfterDrag,
    smoothDisplay,
    displayPending,
    cancelDisplayPreparation,
    refreshDisplayPreparation,
    vrSnapshot,
    gpuActive,
    gpuInitializing,
    gpuRetryVisible,
    gpuFailureNotice,
    stopGpuLayer,
    retryGpuLayer,
    mountGpuCanvas,
    meshPolygons,
    cpuTransparency,
    polygonView,
    polygons,
    cpuOrbitReady,
    cpuOrbitActive,
    polygonColor,
    cpuOrbitTriangles,
    surfacePolygons,
    nurbsSurfacePolygons,
    ghostPolygons,
  }
}
