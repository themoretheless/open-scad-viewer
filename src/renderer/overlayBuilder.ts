import type { Vec3 } from '../services/math3d'
import {
  buildFaceOverlayGeometry,
  buildFaceTriangleIndex,
  buildSourceOverlayGeometry,
  MAX_SOURCE_OVERLAY_TRIANGLES,
  pointOverlayPosition,
} from '../services/meshSelectionOverlay'
import type { PickHit, SelectionMode } from '../services/rendererContracts'
import { MAX_OVERLAY_BUFFER_BYTES, type GMesh, type OverlaySlot } from './types'

export function releaseOverlaySlot(slot: OverlaySlot): void {
  slot.buffer?.destroy()
  slot.buffer = null
  slot.capacity = 0
  slot.count = 0
}

/**
 * Uploads overlay vertices into a persistent buffer, growing its capacity
 * with headroom only when the current allocation is too small. Per-frame
 * hover updates therefore reuse the same GPU buffer via queue.writeBuffer
 * instead of a destroy/create pair.
 */
export function writeOverlaySlot(
  dev: GPUDevice | null,
  slot: OverlaySlot,
  values: number[],
): void {
  slot.count = 0
  if (!dev || !values.length) return
  const data = new Float32Array(values)
  const maxBytes = Math.min(MAX_OVERLAY_BUFFER_BYTES, dev.limits.maxBufferSize)
  if (data.byteLength > maxBytes) return
  if (!slot.buffer || slot.capacity < data.byteLength) {
    const capacity = Math.min(maxBytes, Math.max(4096, data.byteLength * 2))
    let replacement: GPUBuffer | null = null
    try {
      replacement = dev.createBuffer({ size: capacity, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
      dev.queue.writeBuffer(replacement, 0, data)
    } catch {
      replacement?.destroy()
      return
    }
    const previous = slot.buffer
    slot.buffer = replacement
    slot.capacity = capacity
    slot.count = values.length / 7
    previous?.destroy()
    return
  }
  try {
    dev.queue.writeBuffer(slot.buffer, 0, data)
    slot.count = values.length / 7
  } catch {
    slot.count = 0
  }
}

export function appendColoredPositions(
  target: number[],
  positions: Float32Array,
  color: readonly number[],
): void {
  for (let offset = 0; offset + 2 < positions.length; offset += 3) {
    target.push(positions[offset], positions[offset + 1], positions[offset + 2], ...color)
  }
}

export function pointForOverlay(meshes: readonly GMesh[], hit: PickHit): Vec3 | null {
  const mesh = meshes[hit.meshIndex]
  if (!mesh) return null
  return pointOverlayPosition(
    mesh.vertices,
    mesh.indices,
    mesh.transform,
    hit.triangleIndex,
    hit.barycentric,
  ) ?? (hit.point.every(Number.isFinite) ? [...hit.point] : null)
}

export function appendFaceOverlayForMesh(
  mesh: GMesh,
  hit: PickHit,
  faceValues: number[],
  lineValues: number[],
  fillColor: readonly number[],
  lineColor: readonly number[],
): void {
  if (mesh.faceIndex === undefined) {
    // Built once per mesh on the first face-mode hover; every later overlay
    // rebuild then costs O(face) instead of scanning the whole mesh.
    mesh.faceIndex = buildFaceTriangleIndex(mesh.faceIds, Math.floor(mesh.indices.length / 3))
  }
  const geometry = buildFaceOverlayGeometry(
    mesh.vertices,
    mesh.indices,
    mesh.faceIds,
    mesh.transform,
    hit.triangleIndex,
    hit.faceId,
    undefined,
    mesh.faceIndex,
  )
  appendColoredPositions(faceValues, geometry.triangles, fillColor)
  appendColoredPositions(lineValues, geometry.boundaryLines, lineColor)
}

export function appendPointOverlayAt(
  point: Vec3,
  radius: number | undefined,
  lineValues: number[],
  color: readonly number[],
): void {
  const markerSize = Math.max(0.002, (radius ?? 10) * 0.022)
  const positions = new Float32Array([
    point[0] - markerSize, point[1], point[2], point[0] + markerSize, point[1], point[2],
    point[0], point[1] - markerSize, point[2], point[0], point[1] + markerSize, point[2],
    point[0], point[1], point[2] - markerSize, point[0], point[1], point[2] + markerSize,
  ])
  appendColoredPositions(lineValues, positions, color)
}

export function rebuildMeasurementBufferData(
  dev: GPUDevice | null,
  measurementPoints: readonly Vec3[],
  boundsRadius: number | undefined,
): { buffer: GPUBuffer | null; count: number } {
  if (!dev || !measurementPoints.length) return { buffer: null, count: 0 }
  const data: number[] = []
  const markerSize = Math.max(0.01, (boundsRadius ?? 10) * 0.018)
  const colors = [[1, 0.55, 0.08, 1], [0.1, 0.82, 1, 1]]
  const line = (a: Vec3, b: Vec3, color: number[]) => data.push(...a, ...color, ...b, ...color)
  measurementPoints.forEach((point, index) => {
    const color = colors[index] ?? colors[0]
    line([point[0] - markerSize, point[1], point[2]], [point[0] + markerSize, point[1], point[2]], color)
    line([point[0], point[1] - markerSize, point[2]], [point[0], point[1] + markerSize, point[2]], color)
    line([point[0], point[1], point[2] - markerSize], [point[0], point[1], point[2] + markerSize], color)
  })
  if (measurementPoints.length === 2) line(measurementPoints[0], measurementPoints[1], [1, 0.83, 0.26, 1])
  const values = new Float32Array(data)
  const buffer = dev.createBuffer({ size: values.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
  dev.queue.writeBuffer(buffer, 0, values)
  return { buffer, count: values.length / 7 }
}

export function rebuildSelectionOverlaySlots(options: {
  readonly dev: GPUDevice | null
  readonly selectionMode: SelectionMode
  readonly meshes: readonly GMesh[]
  readonly boundsRadius: number | undefined
  readonly selectedHit: PickHit | null
  readonly hoveredHit: PickHit | null
  readonly sameOverlayTarget: (a: PickHit | null, b: PickHit | null) => boolean
  readonly isMeshVisible: (index: number) => boolean
  readonly selectionFaceSlot: OverlaySlot
  readonly selectionLineSlot: OverlaySlot
  readonly deepSelectionLineSlot: OverlaySlot
}): void {
  options.selectionFaceSlot.count = 0
  options.selectionLineSlot.count = 0
  options.deepSelectionLineSlot.count = 0
  if (!options.dev || options.selectionMode === 'object') return

  const faceValues: number[] = []
  const lineValues: number[] = []
  const deepLineValues: number[] = []
  const add = (hit: PickHit, preselected: boolean) => {
    const targetLines = !preselected && (hit.cycleIndex ?? 0) > 0
      ? deepLineValues
      : lineValues
    if (options.selectionMode === 'face') {
      const mesh = options.meshes[hit.meshIndex]
      if (!mesh || !options.isMeshVisible(hit.meshIndex)) return
      appendFaceOverlayForMesh(
        mesh,
        hit,
        faceValues,
        targetLines,
        preselected ? [0.05, 0.82, 1, 0.27] : [1, 0.47, 0.04, 0.38],
        preselected ? [0.08, 0.88, 1, 1] : [1, 0.58, 0.08, 1],
      )
    } else {
      if (!options.isMeshVisible(hit.meshIndex)) return
      const point = pointForOverlay(options.meshes, hit)
      if (!point) return
      appendPointOverlayAt(
        point,
        options.boundsRadius,
        targetLines,
        preselected ? [0.08, 0.88, 1, 1] : [1, 0.58, 0.08, 1],
      )
    }
  }

  if (options.selectedHit) add(options.selectedHit, false)
  if (options.hoveredHit && !options.sameOverlayTarget(options.selectedHit, options.hoveredHit)) {
    add(options.hoveredHit, true)
  }

  writeOverlaySlot(options.dev, options.selectionFaceSlot, faceValues)
  writeOverlaySlot(options.dev, options.selectionLineSlot, lineValues)
  writeOverlaySlot(options.dev, options.deepSelectionLineSlot, deepLineValues)
}

export function rebuildSourceHighlightOverlaySlots(options: {
  readonly dev: GPUDevice | null
  readonly sourceHighlightId: number | null
  readonly meshes: readonly GMesh[]
  readonly isMeshVisible: (index: number) => boolean
  readonly sourceFaceSlot: OverlaySlot
  readonly sourceLineSlot: OverlaySlot
}): void {
  options.sourceFaceSlot.count = 0
  options.sourceLineSlot.count = 0
  if (!options.dev || options.sourceHighlightId === null) return

  const faceValues: number[] = []
  const lineValues: number[] = []
  let remainingTriangles = MAX_SOURCE_OVERLAY_TRIANGLES
  for (let index = 0; index < options.meshes.length && remainingTriangles > 0; index++) {
    if (!options.isMeshVisible(index)) continue
    const mesh = options.meshes[index]
    const geometry = buildSourceOverlayGeometry(
      mesh.vertices,
      mesh.indices,
      mesh.provenance,
      mesh.transform,
      options.sourceHighlightId,
      remainingTriangles,
    )
    appendColoredPositions(faceValues, geometry.triangles, [0.68, 0.24, 1, 0.32])
    appendColoredPositions(lineValues, geometry.boundaryLines, [0.82, 0.42, 1, 1])
    remainingTriangles -= geometry.triangleCount
    if (geometry.truncated) break
  }

  writeOverlaySlot(options.dev, options.sourceFaceSlot, faceValues)
  writeOverlaySlot(options.dev, options.sourceLineSlot, lineValues)
}
