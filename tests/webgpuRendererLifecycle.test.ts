import { afterEach, describe, expect, it, vi } from 'vitest'
import * as geometryKernel from '../src/services/geometry/kernel'
import {
  WebGPURenderer,
  type RendererLifecycleEvent,
} from '../src/services/webgpuRenderer'

describe('WebGPURenderer lifecycle reporting', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('does not allocate a device after teardown during kernel warmup',async()=>{
    let finish!:()=>void
    const pending=new Promise<void>(resolve=>{finish=resolve})
    const warmup=vi.spyOn(geometryKernel,'warmGeometryKernel').mockReturnValue(pending)
    const requestDevice=vi.fn()
    vi.stubGlobal('navigator',{gpu:{requestAdapter:async()=>({requestDevice})}})
    const renderer=new WebGPURenderer()
    try{
      const running=renderer.init({} as HTMLCanvasElement)
      await Promise.resolve()
      expect(warmup).toHaveBeenCalledOnce()
      renderer.destroy();finish()
      expect(await running).toBe(false)
      expect(requestDevice).not.toHaveBeenCalled()
    }finally{finish();renderer.destroy();warmup.mockRestore()}
  })

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

describe('first scene frame submission boundary', () => {
  it('does not report hidden frames or failed submissions, and reports a successful token once', () => {
    const renderer = new WebGPURenderer()
    const submitted = vi.fn()
    renderer.onFrameSubmitted = submitted
    const queueSubmit = vi.fn()
    const pass = { setPipeline() {}, setBindGroup() {}, end() {} }
    const internal = renderer as unknown as { drawable: boolean; render(): void; pendingFrameToken: number | null }
    Object.assign(internal, {
      canvas: { width: 640, height: 480 },
      dev: { queue: { writeBuffer() {}, submit: queueSubmit }, createCommandEncoder: () => ({ beginRenderPass: () => pass, finish: () => ({}) }) },
      ctx: { getCurrentTexture: () => ({ createView() {} }) },
      depth: { createView() {} }, sceneUB: {}, drawable: false,
      updateSize() {}, pendingFrameToken: 7,
    })
    internal.render()
    expect(submitted).not.toHaveBeenCalled()
    internal.drawable = true
    queueSubmit.mockImplementationOnce(() => { throw new Error('queue failed') })
    expect(() => internal.render()).toThrow('queue failed')
    expect(submitted).not.toHaveBeenCalled()
    expect(internal.pendingFrameToken).toBe(7)
    internal.render()
    expect(submitted).toHaveBeenCalledWith(7, expect.any(Number))
    internal.render()
    expect(submitted).toHaveBeenCalledTimes(1)
    renderer.onFrameSubmitted = () => { throw new Error('consumer failed') }
    internal.pendingFrameToken = 8
    expect(() => internal.render()).not.toThrow()
    expect(internal.pendingFrameToken).toBeNull()
  })
})
