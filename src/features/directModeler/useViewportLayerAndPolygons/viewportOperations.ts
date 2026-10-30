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
import type { DirectBody, DirectDocument, Point2 } from '../../../services/directModeling'
import { createDirectProjector, directExtrusionTool, directFaceShade, projectDirectPoint } from '../../../services/directModelingTools'
import type { Vec3 } from '../../../services/directSketchGeometry'
import type { PolygonMesh } from '../../../services/geometry/polygon'
import { SolidDisplayCache, type DisplayMesh } from '../../../services/solidDisplayCache'
import { SolidDisplayQueue } from '../../../services/solidDisplayQueue'
import { isSolidGpuSupported, smoothTriangleList, SolidGpuLayer, type SolidGpuBody } from '../../../services/solidGpuView'
import { solidMaterialRgb } from '../../../services/solidMaterial'
import type { SolidNurbsSurface } from '../../../services/solidNurbs'
import { createSolidPreviewWorker } from '../../../services/solidPreviewWorker'
import type { SolidSurfaceDisplayQueue } from '../../../services/solidSurfaceDisplayQueue'
import { TransparentBsp } from '../../../services/transparentBsp'
import { prepareVrPolygons } from '../../../services/vrScene'
import { COARSE_DISPLAY, type Pane } from '.././helpers'
import type {UseViewportLayerAndPolygonsOptions} from '../useViewportLayerAndPolygons'
import type {ViewportOperationsContext} from './viewportOperationsContext'
export function createViewportOperations(context: ViewportOperationsContext) {
function hasDragNodes(): boolean { return context.dragNodes.length > 0 }
function setDragNodes(nodes: SVGElement[]) { context.dragNodes = nodes }
function fpsTick(now: number) {
    if (context.fpsPrevious) context.fpsIntervals.push(now - context.fpsPrevious)
    context.fpsPrevious = now
    const elapsed = now - context.fpsSince
    if (elapsed >= 500 && context.fpsIntervals.length) {
      context.fps.value = Math.round((context.fpsIntervals.length * 1000) / elapsed)
      const sorted = [...context.fpsIntervals].sort((a, b) => a - b)
      context.frameMs.value = Math.round(sorted[sorted.length >> 1] * 10) / 10
      context.drawMs.value = Math.round((context.gpuLayer?.drawMs ?? 0) * 10) / 10
      context.fpsIntervals = []
      context.fpsSince = now
    }
    context.fpsHandle = requestAnimationFrame(fpsTick)
  }
function applyDragPreviewTransform(offset: string, ids: string[], worldDelta: [number, number, number]) {
    for (const node of context.dragNodes) node.setAttribute('transform', offset)
    context.dragOffsetIds = ids
    context.gpuLayer?.setDragOffset(context.dragOffsetIds, worldDelta)
  }
function clearDragPreview() {
    for (const node of context.dragNodes) node.removeAttribute?.('transform')
    context.dragNodes = []
    if (context.dragOffsetIds.length) {
      context.gpuLayer?.setDragOffset(context.dragOffsetIds, [0, 0, 0])
      context.dragOffsetIds = []
    }
  }
function settleAfterDrag() {
    // The committed document normally rebuilds the GPU buffer, but never rely on that: a
    // commit that changes nothing leaves the buffer, and with it a stale offset, in place.
    if (context.dragOffsetIds.length) {
      context.gpuLayer?.setDragOffset(context.dragOffsetIds, [0, 0, 0])
      context.dragOffsetIds = []
    }
    if (typeof setTimeout !== 'function') {
      clearDragPreview()
      return
    }
    context.settling.value = true
    clearTimeout(context.settleHandle)
    context.settleHandle = setTimeout(() => {
      context.settling.value = false
      clearDragPreview()
    }, 140) as unknown as number
  }
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
function displayMeshFor(b: ReturnType<typeof directExtrusionTool>): DisplayMesh {
    void context.displayVersion.value
    const transforming =
      context.options.previewingTransform.value &&
      (context.options.selectedIds.value.includes(b.id) || (!!b.instance && context.options.selectedIds.value.includes(b.instance.sourceId)))
    if (
      !transforming &&
      !(context.options.heightDragging.value && b === context.options.previewBody.value) &&
      context.smoothDisplay.value &&
      b.brep &&
      context.options.kernelReady.value
    ) {
      const cached = context.displayQueue.get(b)
      if (cached) return cached
    }
    return { mesh: b.mesh, map: null, normals: triangleNormals(b.mesh) }
  }
function shadeFromNormal(n: number[], projectPoint?: (p: number[]) => [number, number, number]): number {
    const light = projectPoint ? projectPoint(n) : projectDirectPoint([n[0], n[1], n[2]], context.options.camera.value)
    return Math.max(24, Math.min(78, 48 + 20 * light[2] - 15 * light[1] + 8 * light[0]))
  }
function stopGpuLayer() {
    context.gpuGeneration++
    context.gpuResize?.disconnect()
    context.gpuResize = null
    context.gpuLayer?.destroy()
    context.gpuLayer = null
    context.gpuActive.value = false
    context.gpuInitializing.value = false
  }
function retryGpuLayer() {
    if (context.gpuInitializing.value || !context.gpuCanvas || !context.options.props.open) return
    stopGpuLayer()
    const generation = context.gpuGeneration
    const canvas = context.gpuCanvas
    context.gpuInitializing.value = true
    const unavailable = () => {
      if (generation !== context.gpuGeneration) return
      context.gpuInitializing.value = false
      context.gpuActive.value = false
      context.gpuRetryVisible.value = true
      context.gpuResize?.disconnect()
      context.gpuResize = null
      context.gpuFailureNotice.value = context.options.label(
        'WebGPU отключён. Работа продолжена на CPU; можно сохранить проект.',
        'WebGPU stopped. Work continues on CPU; you can save the project.',
      )
      context.options.setNotice(context.gpuFailureNotice.value)
    }
    const layer = new SolidGpuLayer(canvas, unavailable)
    context.gpuLayer = layer
    void layer.init().then(ok => {
      if (generation !== context.gpuGeneration) {
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
        context.gpuResize = new ResizeObserver(() => layer.resize(wrap.clientWidth, wrap.clientHeight, window.devicePixelRatio || 1))
        context.gpuResize.observe(wrap)
        layer.resize(wrap.clientWidth, wrap.clientHeight, window.devicePixelRatio || 1)
      }
      context.gpuInitializing.value = false
      context.gpuRetryVisible.value = false
      context.gpuActive.value = true
      if (context.options.getNotice() === context.gpuFailureNotice.value) context.options.setNotice('')
    })
  }
function mountGpuCanvas(el: Element | ComponentPublicInstance | null) {
    if (typeof HTMLCanvasElement === 'undefined' || !(el instanceof HTMLCanvasElement) || context.gpuInitStarted || !isSolidGpuSupported()) return
    context.gpuInitStarted = true
    context.gpuCanvas = el
    retryGpuLayer()
  }
function refreshDisplayPreparation(advDoc: DirectDocument | null | undefined) {
    context.cancelDisplayPreparation()
    if (!context.options.props.open || !context.options.kernelReady.value) return
    const generation = context.displayGeneration
    const bodies = [
      ...context.options.renderBodies.value,
      ...(context.options.previewBody.value ? [context.options.previewBody.value] : []),
      ...(advDoc?.bodies ?? []),
    ].filter(
      b =>
        (context.smoothDisplay.value || (b.brep?.shells.some(s => s.closed) && b.brep.shells.some(s => !s.closed))) &&
        !(
          context.options.previewingTransform.value &&
          (context.options.selectedIds.value.includes(b.id) || (!!b.instance && context.options.selectedIds.value.includes(b.instance.sourceId)))
        ),
    )
    context.displayPending.value = true
    void context.displayQueue
      .prepare(bodies)
      .then(changed => {
        if (generation === context.displayGeneration && changed) context.displayVersion.value++
      })
      .finally(() => {
        if (generation === context.displayGeneration) context.displayPending.value = false
      })
  }
function vrSnapshot() {
    return prepareVrPolygons(
      context.options.renderBodies.value.map(body => ({
        ...displayMeshFor(body).mesh,
        color: body.material ? solidMaterialRgb(body.material.color) : undefined,
      })),
    )
  }
function meshPolygons(
    b: ReturnType<typeof directExtrusionTool>,
    preview = 0,
    projectPoint = createDirectProjector(context.options.camera.value),
    world = false,
  ) {
    const display: DisplayMesh =
      context.options.cameraDragging.value || context.gpuActive.value
        ? { ...COARSE_DISPLAY, mesh: b.mesh, normals: world ? triangleNormals(b.mesh) : [] }
        : displayMeshFor(b)
    const closed = display.closed ?? (b.brep?.shells.some(s => !s.closed) ? context.displayQueue.get(b)?.workClosed : null)
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
            : directFaceShade(mesh, i, context.options.camera.value, projectPoint),
        depth: p0[2] + p1[2] + p2[2],
      })
    }
    return out
  }
function polygonColor(p: (typeof context.polygons.value)[number]) {
    const subtract = context.options.getSubtract()
    return p.preview
      ? p.preview === 2
        ? context.options.extrusionMode.value === 'difference'
          ? '#ff647c'
          : '#75e4b8'
        : p.id === 'preview-split'
          ? '#ffc977'
          : '#77eac5'
      : p.surface
        ? context.options.selectedIds.value.includes(p.id)
          ? '#8061bd'
          : '#315f72'
        : p.material &&
            !context.options.selectedIds.value.includes(p.id) &&
            context.options.hovered.value !== p.id &&
            !subtract?.a.includes(p.id) &&
            !subtract?.b.includes(p.id)
          ? `rgb(${solidMaterialRgb(p.material.color)
              .map(c => (c * 255 * p.shade) / 78)
              .join(' ')})`
          : `hsl(${
              context.options.selectedBody.value?.id === p.id &&
              context.options.selectedFaceTriangles.value.has(p.triangle) &&
              context.options.pickMode.value === 'face'
                ? 40
                : subtract?.a.includes(p.id)
                  ? 45
                  : subtract?.b.includes(p.id)
                    ? 350
                    : context.options.selectedIds.value.includes(p.id)
                      ? 266
                      : context.options.hovered.value === p.id
                        ? 190
                        : 220
            } 45% ${p.shade}%)`
  }
function surfacePolygons(item: SolidNurbsSurface) {
    void context.options.surfaceDisplayVersion.value
    const mesh = context.options.surfaceDisplayQueue.get(item)
    return mesh ? meshPolygons({ id: item.id, name: item.name, mesh }) : []
  }
return {hasDragNodes, setDragNodes, fpsTick, applyDragPreviewTransform, clearDragPreview, settleAfterDrag, triangleNormals, displayMeshFor, shadeFromNormal, stopGpuLayer, retryGpuLayer, mountGpuCanvas, refreshDisplayPreparation, vrSnapshot, meshPolygons, polygonColor, surfacePolygons}
}
