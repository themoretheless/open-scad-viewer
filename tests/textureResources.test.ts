import { afterEach, describe, expect, it, vi } from 'vitest'
import { SHADOW_MAP_SIZE, TextureResources } from '../src/services/textureResources'
import { fillObjectUniform } from '../src/services/uniformFill'
import { WebGPURenderer } from '../src/services/webgpuRenderer'

/**
 * Lifecycle checks for the renderer's texture resources: the shadow enable
 * flag survives re-initialization, failed loads report only while they are
 * still the selected preset, and the scene default material carries emissive.
 */

interface FakeTexture { size: number[]; format: string; destroyed: boolean; createView(): object; destroy(): void }

function fakeDevice() {
  const textures: FakeTexture[] = []
  const device = {
    queue: { writeTexture: () => undefined, copyExternalImageToTexture: () => undefined },
    createSampler: () => ({}),
    createTexture: (descriptor: { size: number[]; format: string }) => {
      const texture: FakeTexture = {
        size: descriptor.size,
        format: descriptor.format,
        destroyed: false,
        createView: () => ({ texture }),
        destroy() { this.destroyed = true },
      }
      textures.push(texture)
      return texture
    },
    createBindGroup: (descriptor: object) => descriptor,
  }
  return { device: device as unknown as GPUDevice, textures }
}

function stubGpuGlobals() {
  vi.stubGlobal('GPUTextureUsage', { TEXTURE_BINDING: 4, COPY_DST: 8, RENDER_ATTACHMENT: 16 })
}

const layouts = { matcapBGL: {}, envBGL: {}, shadowBGL: {} } as unknown as {
  matcapBGL: GPUBindGroupLayout; envBGL: GPUBindGroupLayout; shadowBGL: GPUBindGroupLayout
}

afterEach(() => { vi.unstubAllGlobals() })

describe('TextureResources', () => {
  it('re-init keeps enabled shadows on a real map instead of the 1×1 dummy', () => {
    stubGpuGlobals()
    const { device } = fakeDevice()
    const resources = new TextureResources({ getDevice: () => device, onTexturesRebound: () => undefined, onLoadError: () => undefined })
    resources.init(device, layouts)
    resources.setShadowsEnabled(true, device)
    // Device loss: teardown then init on the same instance, flag unchanged.
    resources.teardown()
    resources.init(device, layouts)
    expect(resources.shadowsEnabled).toBe(true)
    expect(resources.shadowTexture).not.toBe(resources.shadowDummyTexture)
    expect((resources.shadowTexture as unknown as FakeTexture).size).toEqual([SHADOW_MAP_SIZE, SHADOW_MAP_SIZE])
  })

  it('enabling before init gets the real map at init', () => {
    stubGpuGlobals()
    const { device } = fakeDevice()
    const resources = new TextureResources({ getDevice: () => device, onTexturesRebound: () => undefined, onLoadError: () => undefined })
    expect(resources.setShadowsEnabled(true, null)).toBe(false)
    resources.init(device, layouts)
    expect(resources.shadowTexture).not.toBe(resources.shadowDummyTexture)
  })

  it('reports a failed load only while it is still the selected preset', async () => {
    stubGpuGlobals()
    const { device } = fakeDevice()
    const errors: unknown[] = []
    const resources = new TextureResources({ getDevice: () => device, onTexturesRebound: () => undefined, onLoadError: error => errors.push(error) })
    resources.init(device, layouts)
    let failFirst: (() => void) | null = null
    vi.stubGlobal('createImageBitmap', vi.fn())
    vi.stubGlobal('fetch', vi.fn(() => new Promise((resolve, reject) => {
      if (!failFirst) failFirst = () => reject(new Error('offline'))
      else resolve({ ok: false, status: 404 })
    })))

    const superseded = resources.setMatcapTexture('studio')
    const current = resources.setMatcapTexture('clay')
    await current
    expect(errors).toHaveLength(1)
    expect(String(errors[0])).toContain('404')

    // The superseded load fails later: it is no longer the selection, so silent.
    failFirst!()
    await superseded
    expect(errors).toHaveLength(1)
  })
})

describe('renderer texture load errors', () => {
  it('surface through the lifecycle status so a later frame can clear them', () => {
    const renderer = new WebGPURenderer()
    const events: string[] = []
    renderer.onStatusChange = event => { events.push(event.status) }
    const hooks = (renderer as unknown as { textures: { hooks: { onLoadError(error: unknown): void } } }).textures.hooks
    hooks.onLoadError(new Error('Renderer: matcap fetch failed (404)'))
    expect(renderer.currentStatus.status).toBe('error')
    expect(events).toEqual(['error'])
  })
})

describe('fillObjectUniform default material', () => {
  it('writes the scene default emissive for meshes without their own material', () => {
    const uniform = new Float32Array(56)
    const identity = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]
    fillObjectUniform(uniform, identity, identity, [1, 1, 1, 1], 1, 0, undefined, {
      baseColor: [0.1, 0.1, 0.1], metallic: 0, roughness: 0.7, emissive: [1, 0.9, 0.6], alpha: 1,
    })
    expect([...uniform.slice(48, 51)].map(value => Number(value.toFixed(4)))).toEqual([1, 0.9, 0.6])
    expect(uniform[51]).toBeCloseTo(0.7)
  })
})
