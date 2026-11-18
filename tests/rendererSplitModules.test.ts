import { afterEach, describe, expect, it, vi } from 'vitest'
import { CanvasPicker, canvasPickPoint } from '../src/render/canvasPicker'
import { CsgDepthStencil } from '../src/render/csgDepthStencil'
import type { GpuPicker } from '../src/render/gpuPicker'
import { OverlayComposer, sameOverlayTarget } from '../src/render/overlayComposer'
import type { PickHit } from '../src/render/rendererContracts'
import { ResizeWatcher } from '../src/render/resizeWatcher'
import type { GMesh } from '../src/render/types'

afterEach(() => vi.unstubAllGlobals())

function fakeDevice() {
  const textures: { width: number; height: number; destroy: ReturnType<typeof vi.fn> }[] = []
  const buffers: { destroy: ReturnType<typeof vi.fn> }[] = []
  const device = {
    createTexture: vi.fn((desc: { size: [number, number] }) => {
      const texture = { width: desc.size[0], height: desc.size[1], destroy: vi.fn(), createView: vi.fn(() => ({})) }
      textures.push(texture)
      return texture
    }),
    createBuffer: vi.fn(() => {
      const buffer = { destroy: vi.fn() }
      buffers.push(buffer)
      return buffer
    }),
    queue: { writeBuffer: vi.fn() },
  }
  return { device: device as unknown as GPUDevice, raw: device, textures, buffers }
}

describe('CsgDepthStencil', () => {
  it('reuses the texture for the same size and reallocates on change', () => {
    vi.stubGlobal('GPUTextureUsage', { RENDER_ATTACHMENT: 16 })
    const { device, raw, textures } = fakeDevice()
    const stencil = new CsgDepthStencil()
    const first = stencil.ensure(device, 10, 20)
    expect(stencil.ensure(device, 10, 20)).toBe(first)
    expect(raw.createTexture).toHaveBeenCalledTimes(1)
    expect(raw.createTexture).toHaveBeenCalledWith({ size: [10, 20], format: 'depth24plus-stencil8', usage: 16 })
    stencil.ensure(device, 11, 20)
    expect(textures[0]!.destroy).toHaveBeenCalledTimes(1)
    expect(raw.createTexture).toHaveBeenCalledTimes(2)
  })

  it('release destroys the texture and forces a new allocation', () => {
    vi.stubGlobal('GPUTextureUsage', { RENDER_ATTACHMENT: 16 })
    const { device, raw, textures } = fakeDevice()
    const stencil = new CsgDepthStencil()
    stencil.release()
    stencil.ensure(device, 4, 4)
    stencil.release()
    expect(textures[0]!.destroy).toHaveBeenCalledTimes(1)
    stencil.ensure(device, 4, 4)
    expect(raw.createTexture).toHaveBeenCalledTimes(2)
  })
})

describe('CanvasPicker', () => {
  const rect = { left: 10, top: 20, width: 100, height: 50 }

  it('maps client points into the canvas and rejects points outside', () => {
    expect(canvasPickPoint(rect, 15, 25)).toEqual({ x: 5, y: 5 })
    expect(canvasPickPoint(rect, 9, 25)).toBeNull()
    expect(canvasPickPoint(rect, 110, 25)).toBeNull()
    expect(canvasPickPoint(rect, 15, 70)).toBeNull()
  })

  it('creates the picker lazily, passes the viewport, and destroys it', async () => {
    const pick = vi.fn(async () => 3)
    const destroy = vi.fn()
    const create = vi.fn(() => ({ pick, destroy }) as unknown as GpuPicker)
    const picker = new CanvasPicker(create)
    const { device } = fakeDevice()
    const scene = () => ({ cameraViewProjection: new Float32Array(16), sectionEnabled: false })
    expect(await picker.pickAt(device, rect, 0, 0, [], scene)).toBeNull()
    expect(create).not.toHaveBeenCalled()
    expect(await picker.pickAt(device, rect, 30, 40, [], scene)).toBe(3)
    await picker.pickAt(device, rect, 31, 40, [], scene)
    expect(create).toHaveBeenCalledTimes(1)
    expect(pick).toHaveBeenCalledWith(
      device,
      [],
      expect.objectContaining({ viewportWidth: 100, viewportHeight: 50 }),
      20,
      20,
    )
    picker.destroy()
    expect(destroy).toHaveBeenCalledTimes(1)
  })
})

describe('ResizeWatcher', () => {
  it('observes and disconnects, and is a no-op without ResizeObserver', () => {
    const watcher = new ResizeWatcher()
    vi.stubGlobal('ResizeObserver', undefined)
    watcher.observe({} as Element, () => {})
    watcher.disconnect()

    const observe = vi.fn()
    const disconnect = vi.fn()
    let callback: () => void = () => {}
    vi.stubGlobal(
      'ResizeObserver',
      class {
        constructor(cb: () => void) {
          callback = cb
        }
        observe = observe
        disconnect = disconnect
      },
    )
    const onResize = vi.fn()
    const element = {} as Element
    watcher.observe(element, onResize)
    expect(observe).toHaveBeenCalledWith(element)
    callback()
    expect(onResize).toHaveBeenCalledTimes(1)
    watcher.disconnect()
    watcher.disconnect()
    expect(disconnect).toHaveBeenCalledTimes(1)
  })
})

describe('OverlayComposer', () => {
  const hit = (meshIndex: number, faceId: number | null, triangleIndex: number) =>
    ({ meshIndex, faceId, triangleIndex }) as unknown as PickHit

  it('matches overlay targets per selection mode', () => {
    expect(sameOverlayTarget('object', [], hit(0, 1, 1), hit(0, 2, 2))).toBe(true)
    expect(sameOverlayTarget('object', [], hit(0, 1, 1), hit(1, 1, 1))).toBe(false)
    expect(sameOverlayTarget('face', [], hit(0, 1, 5), hit(0, 1, 6))).toBe(true)
    expect(sameOverlayTarget('face', [], hit(0, null, 5), hit(0, 1, 5))).toBe(true)
    expect(sameOverlayTarget('face', [], hit(0, null, 5), hit(0, null, 6))).toBe(false)
    expect(sameOverlayTarget('face', [], null, hit(0, 1, 1))).toBe(false)
  })

  it('skips rebuilds without a device or in object mode and releases slots', () => {
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 32, COPY_DST: 8 })
    const composer = new OverlayComposer()
    composer.selectionFaceSlot.count = 5
    composer.rebuildSelection({
      dev: null,
      selectionMode: 'face',
      meshes: [] as GMesh[],
      boundsRadius: 1,
      selectedHit: null,
      hoveredHit: null,
      isMeshVisible: () => true,
    })
    expect(composer.selectionFaceSlot.count).toBe(0)

    const { device, raw } = fakeDevice()
    composer.rebuildSourceHighlight({ dev: device, sourceHighlightId: null, meshes: [], isMeshVisible: () => true })
    expect(raw.createBuffer).not.toHaveBeenCalled()

    const buffer = { destroy: vi.fn() } as unknown as GPUBuffer
    Object.assign(composer.sourceLineSlot, { buffer, capacity: 64, count: 2 })
    composer.clearSelection()
    expect(composer.sourceLineSlot.buffer).toBe(buffer)
    composer.clearSourceHighlight()
    expect(composer.sourceLineSlot).toEqual({ buffer: null, capacity: 0, count: 0 })
    expect(buffer.destroy).toHaveBeenCalledTimes(1)
  })
})
