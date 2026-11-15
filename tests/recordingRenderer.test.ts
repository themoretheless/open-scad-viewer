import { describe, expect, it } from 'vitest'
import type { MeshData } from '../src/core/mesh'
import { operationCounts, resetOperationCounts } from '../src/dev/operationCounters'
import { renderSeamFromSearch } from '../src/dev/testSeam'
import { RecordingRenderer } from '../src/render/recordingRenderer'

const triangle = { vertices: new Float32Array(18), indices: new Uint32Array([0, 1, 2]) } as unknown as MeshData

describe('recording render seam', () => {
  it('is selected only by the explicit test flag', () => {
    expect(renderSeamFromSearch('?renderSeam=recording')).toBe('recording')
    expect(renderSeamFromSearch('')).toBe('webgpu')
    expect(renderSeamFromSearch('?renderSeam=other')).toBe('webgpu')
  })

  it('initializes without a GPU and records scene publication', async () => {
    resetOperationCounts()
    const renderer = new RecordingRenderer()
    const statuses: string[] = []
    renderer.onStatusChange = event => statuses.push(event.status)
    expect(await renderer.init({ clientHeight: 100 } as HTMLCanvasElement)).toBe(true)
    expect(statuses).toEqual(['initializing', 'ready'])
    renderer.setMeshes([triangle, triangle], { frameToken: 7 })
    expect(renderer.setMeshVisibilityBatch([true, false])).toBe(true)
    renderer.selectMesh(1)
    const state = renderer.snapshot()
    expect(state).toMatchObject({ meshCount: 2, triangleCount: 2, visibleCount: 1, selectedIndex: 1, sceneUploads: 1 })
    expect(operationCounts().viewportSceneUploads).toBe(1)
    renderer.destroy()
    expect(renderer.currentStatus.status).toBe('destroyed')
  })

  it('applies keyboard camera steps with Previous View history', async () => {
    const renderer = new RecordingRenderer()
    await renderer.init({ clientHeight: 100 } as HTMLCanvasElement)
    const start = renderer.getCameraState()
    expect(renderer.applyKeyboardCameraStep({ kind: 'orbit', dx: 24, dy: 0 })).toBe(true)
    expect(renderer.getCameraState().yaw).not.toBe(start.yaw)
    expect(renderer.applyKeyboardCameraStep({ kind: 'zoom', delta: -120 })).toBe(true)
    expect(renderer.getCameraState().distance).toBeLessThan(start.distance)
    expect(renderer.canGoToPreviousView).toBe(true)
    expect(renderer.previousView()).not.toBeNull()
  })
})
