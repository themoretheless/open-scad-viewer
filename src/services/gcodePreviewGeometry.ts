import type { GcodePreviewResult } from './geometry/polygon'
import type { GcodeSceneMesh } from './gcodePreviewProtocol'
import { GCODE_MOVE_ROW_WIDTH, gcodePreviewMoveRows } from './gcodePreviewTransport'

export interface GcodeMeshBounds { min: [number, number, number]; max: [number, number, number] }
/** Exact bounds in scene coordinates, without materializing a second mesh. */
export function gcodeMeshBounds(mesh: GcodeSceneMesh): GcodeMeshBounds {
  const bounds: GcodeMeshBounds = { min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity] }
  const transform = mesh.transform, vertices = mesh.vertices
  if (transform.length !== 16 || !transform.every(Number.isFinite) || vertices.length % 6 || !mesh.indices.length) throw new Error('Invalid scene mesh.')
  for (const index of mesh.indices) {
    if (index * 6 + 2 >= vertices.length) throw new Error('Mesh contains an invalid vertex index.')
    for (let axis = 0; axis < 3; axis++) {
      const offset = axis * 4
      const value = transform[offset] * vertices[index * 6] + transform[offset + 1] * vertices[index * 6 + 1]
        + transform[offset + 2] * vertices[index * 6 + 2] + transform[offset + 3]
      if (!Number.isFinite(value)) throw new Error('Mesh contains non-finite positions.')
      bounds.min[axis] = Math.min(bounds.min[axis], value)
      bounds.max[axis] = Math.max(bounds.max[axis], value)
    }
  }
  return bounds
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
