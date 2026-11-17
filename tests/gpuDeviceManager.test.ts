import { describe, expect, it, vi } from 'vitest'
import { acquireWebGPUDevice, watchDeviceFailures } from '../src/render/gpuDeviceManager'

function fakeDevice() {
  let resolveLost: (info: { reason: GPUDeviceLostReason; message: string }) => void = () => {}
  const listeners: Array<(event: unknown) => void> = []
  const device = {
    destroy: vi.fn(),
    lost: new Promise(resolve => {
      resolveLost = resolve
    }),
    addEventListener: (_type: string, listener: (event: unknown) => void) => {
      listeners.push(listener)
    },
  }
  return {
    device: device as unknown as GPUDevice,
    destroy: device.destroy,
    lose: (reason: GPUDeviceLostReason, message: string) => resolveLost({ reason, message }),
    emitError: (message: string) => listeners.forEach(listener => listener({ error: { message } })),
  }
}

function setup(options: { adapter?: boolean; context?: boolean; configureThrows?: boolean } = {}) {
  const fake = fakeDevice()
  const configure = vi.fn(() => {
    if (options.configureThrows) throw new Error('configure failed')
  })
  const context = { configure } as unknown as GPUCanvasContext
  const adapter = { requestDevice: vi.fn(async () => fake.device) }
  const gpu = {
    requestAdapter: vi.fn(async () => (options.adapter === false ? null : adapter)),
    getPreferredCanvasFormat: () => 'rgba8unorm',
  } as unknown as GPU
  const canvas = {
    getContext: vi.fn(() => (options.context === false ? null : context)),
  } as unknown as HTMLCanvasElement
  return { fake, configure, context, adapter, gpu, canvas }
}

describe('acquireWebGPUDevice', () => {
  it('reports missing WebGPU without touching the canvas', async () => {
    const { canvas } = setup()
    const result = await acquireWebGPUDevice(canvas, { isCurrent: () => true, gpu: null })
    expect(result).toEqual({ kind: 'unavailable', reason: 'webgpu', message: expect.any(String) })
    expect(canvas.getContext).not.toHaveBeenCalled()
  })

  it('reports a missing adapter', async () => {
    const { canvas, gpu } = setup({ adapter: false })
    const result = await acquireWebGPUDevice(canvas, { isCurrent: () => true, gpu })
    expect(result).toMatchObject({ kind: 'unavailable', reason: 'adapter' })
  })

  it('returns a configured device, context and preferred format', async () => {
    const { canvas, gpu, fake, configure, context } = setup()
    const order: string[] = []
    const result = await acquireWebGPUDevice(canvas, {
      isCurrent: () => true,
      gpu,
      beforeDevice: async () => {
        order.push('warm')
      },
    })
    expect(order).toEqual(['warm'])
    expect(result).toEqual({ kind: 'ready', device: fake.device, context, format: 'rgba8unorm' })
    expect(configure).toHaveBeenCalledWith({ device: fake.device, format: 'rgba8unorm', alphaMode: 'premultiplied' })
    expect(fake.destroy).not.toHaveBeenCalled()
  })

  it('stops before requesting a device when superseded during warmup', async () => {
    const { canvas, gpu, adapter } = setup()
    let current = true
    const result = await acquireWebGPUDevice(canvas, {
      isCurrent: () => current,
      gpu,
      beforeDevice: async () => {
        current = false
      },
    })
    expect(result).toEqual({ kind: 'stale' })
    expect(adapter.requestDevice).not.toHaveBeenCalled()
  })

  it('treats a superseded init as stale even when the adapter is missing', async () => {
    const { canvas, gpu } = setup({ adapter: false })
    const result = await acquireWebGPUDevice(canvas, { isCurrent: () => false, gpu })
    expect(result).toEqual({ kind: 'stale' })
  })

  it('destroys a device acquired by a superseded init', async () => {
    const { canvas, gpu, adapter, fake } = setup()
    let current = true
    adapter.requestDevice.mockImplementation(async () => {
      current = false
      return fake.device
    })
    const result = await acquireWebGPUDevice(canvas, { isCurrent: () => current, gpu })
    expect(result).toEqual({ kind: 'stale' })
    expect(fake.destroy).toHaveBeenCalledTimes(1)
  })

  it('destroys the device when the canvas has no WebGPU context', async () => {
    const { canvas, gpu, fake } = setup({ context: false })
    const result = await acquireWebGPUDevice(canvas, { isCurrent: () => true, gpu })
    expect(result).toMatchObject({ kind: 'unavailable', reason: 'context' })
    expect(fake.destroy).toHaveBeenCalledTimes(1)
  })

  it('destroys the device and rethrows when configuring fails', async () => {
    const { canvas, gpu, fake } = setup({ configureThrows: true })
    await expect(acquireWebGPUDevice(canvas, { isCurrent: () => true, gpu })).rejects.toThrow('configure failed')
    expect(fake.destroy).toHaveBeenCalledTimes(1)
  })
})

describe('watchDeviceFailures', () => {
  it('forwards loss and uncaptured errors while current, and drops them after', async () => {
    const fake = fakeDevice()
    let current = true
    const onLost = vi.fn()
    const onError = vi.fn()
    watchDeviceFailures(fake.device, { isCurrent: () => current, onLost, onError })

    fake.emitError('bad pipeline')
    expect(onError).toHaveBeenCalledTimes(1)
    expect(onError.mock.calls[0]?.[0]).toBeInstanceOf(Error)
    expect((onError.mock.calls[0]?.[0] as Error).message).toBe('bad pipeline')

    current = false
    fake.emitError('late')
    fake.lose('destroyed', 'gone')
    await Promise.resolve()
    await Promise.resolve()
    expect(onError).toHaveBeenCalledTimes(1)
    expect(onLost).not.toHaveBeenCalled()
  })

  it('reports device loss with reason and message', async () => {
    const fake = fakeDevice()
    const onLost = vi.fn()
    watchDeviceFailures(fake.device, { isCurrent: () => true, onLost, onError: vi.fn() })
    fake.lose('unknown', 'driver reset')
    await vi.waitFor(() => expect(onLost).toHaveBeenCalledWith({ reason: 'unknown', message: 'driver reset' }))
  })
})
