/**
 * WebGPU device/surface acquisition and failure wiring, split out of the
 * renderer so the adapter -> device -> canvas context sequence (and its
 * stale-init and ownership rules) can be tested without a real GPU.
 */

export type DeviceUnavailableReason = 'webgpu' | 'adapter' | 'context'

export type DeviceAcquisition =
  | { kind: 'ready'; device: GPUDevice; context: GPUCanvasContext; format: GPUTextureFormat }
  | { kind: 'unavailable'; reason: DeviceUnavailableReason; message: string }
  /** A newer init superseded this one; nothing is left owned. */
  | { kind: 'stale' }

export interface AcquireDeviceOptions {
  /** False once a newer init or teardown superseded this attempt. */
  isCurrent: () => boolean
  /** Awaited between adapter and device requests (kernel warmup). */
  beforeDevice?: () => Promise<unknown>
  /** Injected for tests; defaults to `navigator.gpu`. */
  gpu?: GPU | null
}

const UNAVAILABLE_MESSAGES: Record<DeviceUnavailableReason, string> = {
  webgpu: 'WebGPU is unavailable in this environment.',
  adapter: 'No compatible WebGPU adapter was found.',
  context: 'The canvas could not create a WebGPU context.',
}

function unavailable(reason: DeviceUnavailableReason): DeviceAcquisition {
  return { kind: 'unavailable', reason, message: UNAVAILABLE_MESSAGES[reason] }
}

function defaultGpu(): GPU | null {
  return typeof navigator === 'undefined' ? null : (navigator.gpu ?? null)
}

/**
 * Requests an adapter and device and configures the canvas context. The
 * device is destroyed here on every path that does not return it (stale
 * init, missing context, a throw while configuring), so callers own a device
 * only through a `ready` result.
 */
export async function acquireWebGPUDevice(
  canvas: HTMLCanvasElement,
  options: AcquireDeviceOptions,
): Promise<DeviceAcquisition> {
  const gpu = options.gpu === undefined ? defaultGpu() : options.gpu
  if (!gpu) return unavailable('webgpu')

  const adapter = await gpu.requestAdapter()
  if (!options.isCurrent()) return { kind: 'stale' }
  if (!adapter) return unavailable('adapter')

  if (options.beforeDevice) {
    await options.beforeDevice()
    if (!options.isCurrent()) return { kind: 'stale' }
  }

  const device = await adapter.requestDevice()
  if (!options.isCurrent()) {
    device.destroy()
    return { kind: 'stale' }
  }

  try {
    const context = canvas.getContext('webgpu')
    if (!context) {
      device.destroy()
      return unavailable('context')
    }
    const format = gpu.getPreferredCanvasFormat()
    context.configure({ device, format, alphaMode: 'premultiplied' })
    return { kind: 'ready', device, context, format }
  } catch (error) {
    device.destroy()
    throw error
  }
}

export interface DeviceFailureHandlers {
  /** False once the renderer moved to another device or was destroyed. */
  isCurrent: () => boolean
  onLost: (info: { reason: GPUDeviceLostReason; message: string }) => void
  onError: (error: Error) => void
}

/**
 * Routes device loss and uncaptured errors (pipeline creation, texture
 * uploads outside the per-frame validation scope) to the handlers while the
 * device is still the current one.
 */
export function watchDeviceFailures(device: GPUDevice, handlers: DeviceFailureHandlers) {
  void device.lost.then(info => {
    if (!handlers.isCurrent()) return
    handlers.onLost({ reason: info.reason, message: info.message })
  })
  device.addEventListener?.('uncapturederror', event => {
    if (!handlers.isCurrent()) return
    handlers.onError(new Error((event as GPUUncapturedErrorEvent).error.message))
  })
}
