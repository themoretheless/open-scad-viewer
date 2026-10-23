import type { GcodePreviewResult } from './geometry/polygon'
import type { GcodeSceneMesh } from './gcodePreviewProtocol'
import { GCODE_MOVE_ROW_WIDTH, gcodePreviewMoveRows } from './gcodePreviewTransport'

export interface GcodeMeshBounds { min: [number, number, number]; max: [number, number, number] }
/** Exact bounds in scene coordinates, without materializing a second mesh. */
export function gcodeMeshBounds(mesh: GcodeSceneMesh): GcodeMeshBounds {
  const transform = mesh.transform, vertices = mesh.vertices, indices = mesh.indices
  if (transform.length !== 16 || vertices.length % 6 || !indices.length) throw new Error('Invalid scene mesh.')
  for (let i = 0; i < 16; i++) {
    if (!Number.isFinite(transform[i])) throw new Error('Invalid scene mesh.')
  }
  const vertexCount = vertices.length / 6
  const visited = new Uint8Array(vertexCount)
  const m0 = transform[0], m1 = transform[1], m2 = transform[2], m3 = transform[3]
  const m4 = transform[4], m5 = transform[5], m6 = transform[6], m7 = transform[7]
  const m8 = transform[8], m9 = transform[9], m10 = transform[10], m11 = transform[11]
  let minX = Infinity, minY = Infinity, minZ = Infinity
  let maxX = -Infinity, maxY = -Infinity, maxZ = -Infinity
  for (let i = 0; i < indices.length; i++) {
    const index = indices[i]
    if (index >= vertexCount) throw new Error('Mesh contains an invalid vertex index.')
    if (visited[index]) continue
    visited[index] = 1
    const vo = index * 6
    const vx = vertices[vo], vy = vertices[vo + 1], vz = vertices[vo + 2]
    const wx = m0 * vx + m1 * vy + m2 * vz + m3
    const wy = m4 * vx + m5 * vy + m6 * vz + m7
    const wz = m8 * vx + m9 * vy + m10 * vz + m11
    if (!Number.isFinite(wx) || !Number.isFinite(wy) || !Number.isFinite(wz)) {
      throw new Error('Mesh contains non-finite positions.')
    }
    if (wx < minX) minX = wx
    if (wy < minY) minY = wy
    if (wz < minZ) minZ = wz
    if (wx > maxX) maxX = wx
    if (wy > maxY) maxY = wy
    if (wz > maxZ) maxZ = wz
  }
  return { min: [minX, minY, minZ], max: [maxX, maxY, maxZ] }
}

/** Binary search avoids rescanning all moves each time the layer slider changes. */
export function gcodeLayerRange(preview: GcodePreviewResult, layerIndex: number): { start: number; end: number; z: number | null } {
  // Read packed rows directly: no per-move objects on the slider hot path.
  const rows = gcodePreviewMoveRows(preview), moveCount = rows.length / GCODE_MOVE_ROW_WIDTH
  const lowerBound = (layer: number) => {
    let left = 0, right = moveCount
    while (left < right) {
      const middle = Math.floor((left + right) / 2)
      if (rows[middle * GCODE_MOVE_ROW_WIDTH + 5] < layer) left = middle + 1
      else right = middle
    }
    return left
  }
  const start = lowerBound(layerIndex), end = lowerBound(layerIndex + 1)
  return { start, end, z: start < end ? rows[start * GCODE_MOVE_ROW_WIDTH + 2] : null }
}

export function drawGcodeLayer(canvas: HTMLCanvasElement, preview: GcodePreviewResult, layerIndex: number, showTravel: boolean) {
  const context = canvas.getContext('2d'), bounds = preview.bounds
  if (!context) return
  const size = 600, padding = 34
  canvas.width = size; canvas.height = size
  context.fillStyle = '#101922'; context.fillRect(0, 0, size, size)
  if (!bounds) return
  const spanX = Math.max(bounds.max[0] - bounds.min[0], 0.1), spanY = Math.max(bounds.max[1] - bounds.min[1], 0.1)
  const scale = Math.min((size - padding * 2) / spanX, (size - padding * 2) / spanY)
  const offsetX = (size - spanX * scale) / 2, offsetY = (size - spanY * scale) / 2
  const x = (value: number) => offsetX + (value - bounds.min[0]) * scale
  const y = (value: number) => size - offsetY - (value - bounds.min[1]) * scale
  const range = gcodeLayerRange(preview, layerIndex)
  const rows = gcodePreviewMoveRows(preview)
  for (const extrusion of [false, true]) {
    if (!extrusion && !showTravel) continue
    context.beginPath()
    for (let index = Math.max(1, range.start); index < range.end; index++) {
      const current = index * GCODE_MOVE_ROW_WIDTH, previous = current - GCODE_MOVE_ROW_WIDTH
      if ((rows[current + 6] === 1) !== extrusion) continue
      context.moveTo(x(rows[previous]), y(rows[previous + 1])); context.lineTo(x(rows[current]), y(rows[current + 1]))
    }
    context.strokeStyle = extrusion ? '#5bcbff' : '#e8ad61'
    context.lineWidth = extrusion ? 2 : 1.2
    context.setLineDash(extrusion ? [] : [5, 5]); context.stroke()
  }
  context.setLineDash([])
  context.fillStyle = '#becbd7'; context.font = '19px sans-serif'
  context.fillText(`X ${bounds.min[0].toFixed(2)} … ${bounds.max[0].toFixed(2)} mm`, padding, size - 10)
  context.save(); context.translate(20, size - padding); context.rotate(-Math.PI / 2)
  context.fillText(`Y ${bounds.min[1].toFixed(2)} … ${bounds.max[1].toFixed(2)} mm`, 0, 0); context.restore()
}

export { runGpuFilterGcodeSegments, referenceFilterGcodeSegments, type GcodeFilterOptions, type GcodeSegmentFilterResult } from './gpuGcodePreview'

