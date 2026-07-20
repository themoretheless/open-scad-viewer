import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  WebGPURenderer,
  type RendererLifecycleEvent,
} from '../src/services/webgpuRenderer'

describe('WebGPURenderer lifecycle reporting', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('reports typed initialization and availability states without changing init semantics', async () => {
    vi.stubGlobal('navigator', {})
    const renderer = new WebGPURenderer()
    const events: RendererLifecycleEvent[] = []
    renderer.onStatusChange = event => events.push(event)

    const ready = await renderer.init({} as HTMLCanvasElement)

    expect(ready).toBe(false)
    expect(events.map(event => event.status)).toEqual(['initializing', 'unavailable'])
    expect(renderer.currentStatus).toMatchObject({
      status: 'unavailable',
      reason: 'webgpu',
    })
  })

  it('surfaces frame failures and reports recovery after a successful retry', () => {
    let retry: FrameRequestCallback | null = null
    vi.stubGlobal('requestAnimationFrame', vi.fn((callback: FrameRequestCallback) => {
      retry = callback
      return 1
    }))
    const renderer = new WebGPURenderer()
    const events: RendererLifecycleEvent[] = []
    renderer.onStatusChange = event => events.push(event)
    const failure = new Error('surface expired')
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => undefined)
    const internal = renderer as unknown as {
      dead: boolean
      lost: boolean
      initialized: boolean
      render: () => void
      drawFrame: () => void
    }
    internal.dead = false
    internal.lost = false
    internal.initialized = true
    internal.render = () => { throw failure }

    internal.drawFrame()

    expect(renderer.currentStatus).toEqual({ status: 'error', phase: 'frame', error: failure })
    expect(consoleError).toHaveBeenCalledWith('[WebGPURenderer] frame failed', failure)
    expect(retry).not.toBeNull()

    internal.render = () => undefined
    ;(retry as FrameRequestCallback)(0)
    expect(events.map(event => event.status)).toEqual(['error', 'ready'])
    expect(renderer.currentStatus).toEqual({ status: 'ready' })
  })

  it('reports explicit destruction', () => {
    const renderer = new WebGPURenderer()
    const events: RendererLifecycleEvent[] = []
    renderer.onStatusChange = event => events.push(event)

    renderer.destroy()

    expect(events).toEqual([{ status: 'destroyed' }])
    expect(renderer.currentStatus).toEqual({ status: 'destroyed' })
  })

  it('rehydrates measurement state without aliasing recovery snapshots', () => {
    const renderer = new WebGPURenderer()
    const events: Array<{ points: number[][]; active: boolean }> = []
    renderer.onMeasurementChange = (measurement, active) => {
      events.push({ points: measurement?.points.map(point => [...point]) ?? [], active })
    }
    const snapshot = { points: [[1, 2, 3], [4, 6, 3]] as [number, number, number][], distance: 5 }

    expect(renderer.restoreMeasurement(snapshot, true)).toBe(true)
    snapshot.points[0][0] = 99
    expect(events.at(-1)).toEqual({ points: [[1, 2, 3], [4, 6, 3]], active: true })
    expect(renderer.restoreMeasurement({ points: [[Number.NaN, 0, 0]], distance: null }, false)).toBe(false)
  })
})
