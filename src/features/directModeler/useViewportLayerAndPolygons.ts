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

  function fpsTick(now: number) {
    if (fpsPrevious) fpsIntervals.push(now - fpsPrevious)
    fpsPrevious = now
    const elapsed = now - fpsSince
    if (elapsed >= 500 && fpsIntervals.length) {
      fps.value = Math.round((fpsIntervals.length * 1000) / elapsed)
      const sorted = [...fpsIntervals].sort((a, b) => a - b)
      frameMs.value = Math.round(sorted[sorted.length >> 1] * 10) / 10
      drawMs.value = Math.round((gpuLayer?.drawMs ?? 0) * 10) / 10
      fpsIntervals = []
      fpsSince = now
    }
    fpsHandle = requestAnimationFrame(fpsTick)
  }

  function hasDragNodes(): boolean {
    return dragNodes.length > 0
  }

  function setDragNodes(nodes: SVGElement[]) {
    dragNodes = nodes
  }

  function applyDragPreviewTransform(offset: string, ids: string[], worldDelta: [number, number, number]) {
    for (const node of dragNodes) node.setAttribute('transform', offset)
    dragOffsetIds = ids
    gpuLayer?.setDragOffset(dragOffsetIds, worldDelta)
  }

  function clearDragPreview() {
    for (const node of dragNodes) node.removeAttribute?.('transform')
    dragNodes = []
    if (dragOffsetIds.length) {
      gpuLayer?.setDragOffset(dragOffsetIds, [0, 0, 0])
      dragOffsetIds = []
    }
  }

  function settleAfterDrag() {
    // The committed document normally rebuilds the GPU buffer, but never rely on that: a
    // commit that changes nothing leaves the buffer, and with it a stale offset, in place.
    if (dragOffsetIds.length) {
      gpuLayer?.setDragOffset(dragOffsetIds, [0, 0, 0])
      dragOffsetIds = []
    }
    if (typeof setTimeout !== 'function') {
      clearDragPreview()
      return
    }
    settling.value = true
    clearTimeout(settleHandle)
    settleHandle = setTimeout(() => {
      settling.value = false
      clearDragPreview()
    }, 140) as unknown as number
  }

  const smoothDisplay = ref(true)
  // Exact geometry keys survive history clones and include retained B-rep changes.
  const displayCache = new SolidDisplayCache()

  function triangleNormals(mesh: PolygonMesh) {
    const indices = mesh.indices
    const pos = mesh.positions
    const count = indices.length / 3
    const normals = new Array<number[]>(count)
    for (let t = 0, o = 0; t < count; t++, o += 3) {
      const i0 = indices[o] * 3
      const i1 = indices[o + 1] * 3
      const i2 = indices[o + 2] * 3
      const ax = pos[i0]
      const ay = pos[i0 + 1]
      const az = pos[i0 + 2]
      const ux = pos[i1] - ax
      const uy = pos[i1 + 1] - ay
      const uz = pos[i1 + 2] - az
      const wx = pos[i2] - ax
      const wy = pos[i2 + 1] - ay
      const wz = pos[i2 + 2] - az
      const nx = uy * wz - uz * wy
      const ny = uz * wx - ux * wz
      const nz = ux * wy - uy * wx
      const len = (nx * nx + ny * ny + nz * nz) ** 0.5 || 1
      normals[t] = [nx / len, ny / len, nz / len]
    }
    return normals
  }

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

  function displayMeshFor(b: ReturnType<typeof directExtrusionTool>): DisplayMesh {
    void displayVersion.value
    const transforming =
      options.previewingTransform.value &&
      (options.selectedIds.value.includes(b.id) || (!!b.instance && options.selectedIds.value.includes(b.instance.sourceId)))
    if (
      !transforming &&
      !(options.heightDragging.value && b === options.previewBody.value) &&
      smoothDisplay.value &&
      b.brep &&
      options.kernelReady.value
    ) {
      const cached = displayQueue.get(b)
      if (cached) return cached
    }
    return { mesh: b.mesh, map: null, normals: triangleNormals(b.mesh) }
  }

  function shadeFromNormal(n: number[], projectPoint?: (p: number[]) => [number, number, number]): number {
    const light = projectPoint ? projectPoint(n) : projectDirectPoint([n[0], n[1], n[2]], options.camera.value)
    return Math.max(24, Math.min(78, 48 + 20 * light[2] - 15 * light[1] + 8 * light[0]))
  }

  watch(gpuActive, value => options.emitBackend(value), { immediate: true })

  function stopGpuLayer() {
    gpuGeneration++
    gpuResize?.disconnect()
    gpuResize = null
    gpuLayer?.destroy()
    gpuLayer = null
    gpuActive.value = false
    gpuInitializing.value = false
  }

  function retryGpuLayer() {
    if (gpuInitializing.value || !gpuCanvas || !options.props.open) return
    stopGpuLayer()
    const generation = gpuGeneration
    const canvas = gpuCanvas
    gpuInitializing.value = true
    const unavailable = () => {
      if (generation !== gpuGeneration) return
      gpuInitializing.value = false
      gpuActive.value = false
      gpuRetryVisible.value = true
      gpuResize?.disconnect()
      gpuResize = null
      gpuFailureNotice.value = options.label(
        'WebGPU отключён. Работа продолжена на CPU; можно сохранить проект.',
        'WebGPU stopped. Work continues on CPU; you can save the project.',
      )
      options.setNotice(gpuFailureNotice.value)
    }
    const layer = new SolidGpuLayer(canvas, unavailable)
    gpuLayer = layer
    void layer.init().then(ok => {
      if (generation !== gpuGeneration) {
        layer.destroy()
        return
      }
      if (!ok || !layer.ready) {
        layer.destroy()
        unavailable()
        return
      }
      const wrap = canvas.parentElement
      if (wrap) {
        gpuResize = new ResizeObserver(() => layer.resize(wrap.clientWidth, wrap.clientHeight, window.devicePixelRatio || 1))
        gpuResize.observe(wrap)
        layer.resize(wrap.clientWidth, wrap.clientHeight, window.devicePixelRatio || 1)
      }
      gpuInitializing.value = false
      gpuRetryVisible.value = false
      gpuActive.value = true
      if (options.getNotice() === gpuFailureNotice.value) options.setNotice('')
    })
  }

  function mountGpuCanvas(el: Element | ComponentPublicInstance | null) {
    if (typeof HTMLCanvasElement === 'undefined' || !(el instanceof HTMLCanvasElement) || gpuInitStarted || !isSolidGpuSupported()) return
    gpuInitStarted = true
    gpuCanvas = el
    retryGpuLayer()
  }

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

  function refreshDisplayPreparation(advDoc: DirectDocument | null | undefined) {
    cancelDisplayPreparation()
    if (!options.props.open || !options.kernelReady.value) return
    const generation = displayGeneration
    const bodies = [
      ...options.renderBodies.value,
      ...(options.previewBody.value ? [options.previewBody.value] : []),
      ...(advDoc?.bodies ?? []),
    ].filter(
      b =>
        (smoothDisplay.value || (b.brep?.shells.some(s => s.closed) && b.brep.shells.some(s => !s.closed))) &&
        !(
          options.previewingTransform.value &&
          (options.selectedIds.value.includes(b.id) || (!!b.instance && options.selectedIds.value.includes(b.instance.sourceId)))
        ),
    )
    displayPending.value = true
    void displayQueue
      .prepare(bodies)
      .then(changed => {
        if (generation === displayGeneration && changed) displayVersion.value++
      })
      .finally(() => {
        if (generation === displayGeneration) displayPending.value = false
      })
  }

  function vrSnapshot() {
    return prepareVrPolygons(
      options.renderBodies.value.map(body => ({
        ...displayMeshFor(body).mesh,
        color: body.material ? solidMaterialRgb(body.material.color) : undefined,
      })),
    )
  }

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

  function meshPolygons(
    b: ReturnType<typeof directExtrusionTool>,
    preview = 0,
    projectPoint = createDirectProjector(options.camera.value),
    world = false,
  ) {
    const display: DisplayMesh =
      options.cameraDragging.value || gpuActive.value
        ? { ...COARSE_DISPLAY, mesh: b.mesh, normals: world ? triangleNormals(b.mesh) : [] }
        : displayMeshFor(b)
    const closed = display.closed ?? (b.brep?.shells.some(s => !s.closed) ? displayQueue.get(b)?.workClosed : null)
    const allClosed = !!b.brep?.shells.length && b.brep.shells.every(s => s.closed)
    const mesh = display.mesh
    const pos = mesh.positions
    const vertCount = pos.length / 3
    const points = new Array<[number, number, number]>(vertCount)
    for (let v = 0, o = 0; v < vertCount; v++, o += 3) points[v] = projectPoint([pos[o], pos[o + 1], pos[o + 2]])
    const indices = mesh.indices
    const triCount = indices.length / 3
    const out: Array<{
      preview: number
      surface: boolean
      normal: number[]
      closed: boolean
      vertices: [number, number, number][]
      id: string
      material: typeof b.material
      triangle: number
      key: string
      points: string
      shade: number
      depth: number
    }> = []
    const checkBackface = !world && !preview && (b.material?.opacity ?? 1) === 1
    for (let i = 0, o = 0; i < triCount; i++, o += 3) {
      const p0 = points[indices[o]]
      const p1 = points[indices[o + 1]]
      const p2 = points[indices[o + 2]]
      const isClosed = closed?.[i] ?? allClosed
      if (checkBackface && isClosed && (p1[0] - p0[0]) * (p2[1] - p0[1]) - (p1[1] - p0[1]) * (p2[0] - p0[0]) <= 0) continue
      const triangle = display.map ? display.map[i] : i
      const face = [p0, p1, p2]
      out.push({
        preview,
        surface: false,
        normal: display.normals[i],
        closed: isClosed,
        vertices: face,
        id: b.id,
        material: b.material,
        triangle,
        key: b.id + ':' + i,
        points: `${p0[0]},${p0[1]} ${p1[0]},${p1[1]} ${p2[0]},${p2[1]}`,
        shade: world
          ? 0
          : display.map
            ? shadeFromNormal(display.normals[i], projectPoint)
            : directFaceShade(mesh, i, options.camera.value, projectPoint),
        depth: p0[2] + p1[2] + p2[2],
      })
    }
    return out
  }

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

  function polygonColor(p: (typeof polygons.value)[number]) {
    const subtract = options.getSubtract()
    return p.preview
      ? p.preview === 2
        ? options.extrusionMode.value === 'difference'
          ? '#ff647c'
          : '#75e4b8'
        : p.id === 'preview-split'
          ? '#ffc977'
          : '#77eac5'
      : p.surface
        ? options.selectedIds.value.includes(p.id)
          ? '#8061bd'
          : '#315f72'
        : p.material &&
            !options.selectedIds.value.includes(p.id) &&
            options.hovered.value !== p.id &&
            !subtract?.a.includes(p.id) &&
            !subtract?.b.includes(p.id)
          ? `rgb(${solidMaterialRgb(p.material.color)
              .map(c => (c * 255 * p.shade) / 78)
              .join(' ')})`
          : `hsl(${
              options.selectedBody.value?.id === p.id &&
              options.selectedFaceTriangles.value.has(p.triangle) &&
              options.pickMode.value === 'face'
                ? 40
                : subtract?.a.includes(p.id)
                  ? 45
                  : subtract?.b.includes(p.id)
                    ? 350
                    : options.selectedIds.value.includes(p.id)
                      ? 266
                      : options.hovered.value === p.id
                        ? 190
                        : 220
            } 45% ${p.shade}%)`
  }

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

  function surfacePolygons(item: SolidNurbsSurface) {
    void options.surfaceDisplayVersion.value
    const mesh = options.surfaceDisplayQueue.get(item)
    return mesh ? meshPolygons({ id: item.id, name: item.name, mesh }) : []
  }

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
