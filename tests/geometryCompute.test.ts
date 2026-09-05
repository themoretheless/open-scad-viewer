import { afterEach, describe, expect, it, vi } from 'vitest'
import { analyzeSurfaceArea, cpuSurfaceArea, webgpuSurfaceArea, type SurfaceInput } from '../src/services/geometryCompute'

const input = (): SurfaceInput => ({
  vertices: new Float32Array([0, 0, 0, 0, 0, 1, 2, 0, 0, 0, 0, 1, 0, 3, 0, 0, 0, 1]),
  indices: new Uint32Array([0, 1, 2]),
  transform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
})

describe('world surface-area compute pilot', () => {
  afterEach(() => { vi.unstubAllGlobals(); vi.useRealTimers() })

  it('handles nonuniform scale, reflection and huge translations without subtracting translated points', async () => {
    const mesh = input()
    mesh.transform[0] = -2
    mesh.transform[5] = 3
    mesh.transform[10] = 4
    mesh.transform[3] = 1e20
    expect(await cpuSurfaceArea(mesh, new AbortController().signal)).toBe(18)
  })

  it('rejects invalid indices, non-affine transforms and non-finite data', async () => {
    const mesh = input()
    mesh.indices[2] = 100
    await expect(cpuSurfaceArea(mesh, new AbortController().signal)).rejects.toThrow('invalid-compute-input')
    const projective = input()
    projective.transform[12] = 1
    await expect(cpuSurfaceArea(projective, new AbortController().signal)).rejects.toThrow()
    const invalid = input()
    invalid.vertices[0] = NaN
    await expect(cpuSurfaceArea(invalid, new AbortController().signal)).rejects.toThrow()
  })

  it('publishes a verified GPU result and falls back on disagreement or execution failure', async () => {
    const signal = new AbortController().signal
    const verified = await analyzeSurfaceArea(input(), signal, { id: 'webgpu-compute', surfaceArea: async () => 3.00001 })
    expect(verified.backend).toBe('webgpu-compute')
    expect(verified.fallback).toBeNull()
    const wrong = await analyzeSurfaceArea(input(), signal, { id: 'webgpu-compute', surfaceArea: async () => 6 })
    expect(wrong).toMatchObject({ backend: 'cpu', area: 3, fallback: 'verification-failed' })
    const failed = await analyzeSurfaceArea(input(), signal, { id: 'webgpu-compute', surfaceArea: async () => { throw new Error('device lost') } })
    expect(failed).toMatchObject({ backend: 'cpu', area: 3, fallback: 'gpu-failed' })
  })

  it('uses CPU when WebGPU is unavailable and never publishes on cancellation', async () => {
    vi.stubGlobal('navigator', {})
    expect(await analyzeSurfaceArea(input(), new AbortController().signal)).toMatchObject({ backend: 'cpu', area: 3, fallback: 'unavailable' })
    const controller = new AbortController()
    controller.abort()
    await expect(analyzeSurfaceArea(input(), controller.signal)).rejects.toThrow()
  })

  it('destroys a device arriving after cancellation without submitting work', async () => {
    let resolveDevice!: (device: GPUDevice) => void
    const requested = vi.fn(() => new Promise<GPUDevice>(resolve => { resolveDevice = resolve }))
    vi.stubGlobal('navigator', { gpu: { requestAdapter: async () => ({ requestDevice: requested }) } })
    const controller = new AbortController()
    const pending = webgpuSurfaceArea(input(), controller.signal)
    await Promise.resolve()
    expect(requested).toHaveBeenCalled()
    controller.abort()
    await expect(pending).rejects.toThrow()
    const destroy = vi.fn()
    resolveDevice({ destroy } as unknown as GPUDevice)
    await Promise.resolve()
    expect(destroy).toHaveBeenCalledOnce()
  })

  it('bounds a hung adapter request', async () => {
    vi.useFakeTimers()
    vi.stubGlobal('navigator', { gpu: { requestAdapter: () => new Promise(() => {}) } })
    const result = webgpuSurfaceArea(input(), new AbortController().signal)
    const assertion = expect(result).rejects.toThrow('gpu-failed')
    await vi.advanceTimersByTimeAsync(5000)
    await assertion
    expect(vi.getTimerCount()).toBe(0)
  })
})
