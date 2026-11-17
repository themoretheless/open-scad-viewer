/**
 * Hardware picking entry point for the renderer canvas, split out so the
 * client-to-canvas mapping and lazy picker lifetime can be tested with a
 * fake device.
 */
import { GpuPicker, type GpuPickerMesh, type GpuPickerSceneParams } from './gpuPicker'

export interface CanvasRect {
  left: number
  top: number
  width: number
  height: number
}

/** Canvas-local CSS pixel for a client point, or null when it falls outside the canvas. */
export function canvasPickPoint(rect: CanvasRect, clientX: number, clientY: number): { x: number; y: number } | null {
  const x = clientX - rect.left
  const y = clientY - rect.top
  if (x < 0 || y < 0 || x >= rect.width || y >= rect.height) return null
  return { x, y }
}

export type CanvasPickScene = Omit<GpuPickerSceneParams, 'viewportWidth' | 'viewportHeight'>

/** Owns a lazily created `GpuPicker` and maps client coordinates to it. */
export class CanvasPicker {
  private picker: GpuPicker | null = null

  constructor(private readonly createPicker: () => GpuPicker = () => new GpuPicker()) {}

  async pickAt(
    dev: GPUDevice,
    rect: CanvasRect,
    clientX: number,
    clientY: number,
    meshes: readonly GpuPickerMesh[],
    scene: () => CanvasPickScene,
  ): Promise<number | null> {
    const point = canvasPickPoint(rect, clientX, clientY)
    if (!point) return null
    this.picker ??= this.createPicker()
    return this.picker.pick(
      dev,
      meshes,
      { ...scene(), viewportWidth: rect.width, viewportHeight: rect.height },
      point.x,
      point.y,
    )
  }

  destroy(): void {
    this.picker?.destroy()
    this.picker = null
  }
}
