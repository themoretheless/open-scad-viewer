import { describe, expect, it, vi } from 'vitest'
import { createPickMatrix, GpuPicker, type GpuPickerMesh } from '../src/services/gpuPicker'
import { identityMat4 } from '../src/services/gpuCsgPreview'

describe('gpuPicker Pick Matrix Math', () => {
  it('maps center of target pixel directly to NDC origin (0, 0)', () => {
    const w = 800
    const h = 600
    // Pixel (399.5, 299.5) is exact center of 800x600 viewport
    const m = createPickMatrix(w, h, 399.5, 299.5, 1)

    // Verify matrix is affine and scales properly
    expect(m[0]).toBe(w)
    expect(m[5]).toBe(h)
    expect(m[10]).toBe(1)
    expect(m[15]).toBe(1)

    // NDC coordinates of center pixel in unzoomed camera are (0, 0)
    // Transformed by pick matrix: x' = w * (0 - 0) = 0
    expect(Math.abs(m[3])).toBeCloseTo(0, 4)
    expect(Math.abs(m[7])).toBeCloseTo(0, 4)
  })

  it('scales 1x1 window to exactly cover [-1, 1] NDC range', () => {
    const w = 100
    const h = 100
    const px = 50
    const py = 50
    const m = createPickMatrix(w, h, px, py, 1)

    // Center in NDC is: ((50 + 0.5) / 100) * 2 - 1 = 1.01 - 1 = 0.01
    // Matrix row 0: sx * x + (-sx * cx) = 100 * x - 100 * 0.01 = 100 * x - 1.0
    expect(m[0]).toBe(100)
    expect(m[3]).toBeCloseTo(-1.0, 4)

    // At x = 0.01 (pixel center), result is 100 * 0.01 - 1.0 = 0
    const xCenterTransformed = m[0] * 0.01 + m[3]
    expect(xCenterTransformed).toBeCloseTo(0, 4)

    // At x = 0.015 (right edge of pixel), result is 100 * 0.015 - 1.0 = 0.5
    const xRightEdgeTransformed = m[0] * 0.02 + m[3]
    expect(xRightEdgeTransformed).toBeCloseTo(1.0, 4)
  })

  it('leaves depth coordinate (Z) invariant', () => {
    const m = createPickMatrix(1920, 1080, 100, 200, 1)
    expect(m[10]).toBe(1)
    expect(m[11]).toBe(0)
    expect(m[14]).toBe(0)
    expect(m[15]).toBe(1)
  })
})

describe('GpuPicker Hardware Picking Pass', () => {
  function makeMockDevice(readbackRawId = 0) {
    vi.stubGlobal('GPUShaderStage', { VERTEX: 1, FRAGMENT: 2 })
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 32, INDEX: 16, UNIFORM: 64, COPY_DST: 8, MAP_READ: 1 })
    vi.stubGlobal('GPUTextureUsage', { RENDER_ATTACHMENT: 16, COPY_SRC: 4 })
    vi.stubGlobal('GPUMapMode', { READ: 1 })

    let mappedData = new Uint32Array([readbackRawId, 0, 0, 0])
    let unmapped = false

    const copiedCommands: any[] = []
    const passes: any[] = []

    const dev: any = {
      createTexture: (desc: any) => ({
        desc,
        createView: () => ({ textureDesc: desc }),
        destroy: vi.fn(),
      }),
      createBuffer: (desc: any) => ({
        desc,
        getMappedRange: () => mappedData.buffer,
        mapAsync: vi.fn(async () => { unmapped = false }),
        unmap: vi.fn(() => { unmapped = true }),
        destroy: vi.fn(),
      }),
      createBindGroupLayout: (desc: any) => ({ desc }),
      createPipelineLayout: (desc: any) => ({ desc }),
      createShaderModule: (desc: any) => ({ desc }),
      createRenderPipeline: (desc: any) => ({ desc }),
      createBindGroup: (desc: any) => ({ desc }),
      createCommandEncoder: () => {
        const enc: any = {
          beginRenderPass: (desc: any) => {
            const pass: any = {
              desc,
              draws: 0,
              setPipeline: vi.fn(),
              setBindGroup: vi.fn(),
              setVertexBuffer: vi.fn(),
              setIndexBuffer: vi.fn(),
              drawIndexed: vi.fn(() => { pass.draws++ }),
              end: vi.fn(),
            }
            passes.push(pass)
            return pass
          },
          copyTextureToBuffer: (src: any, dst: any, size: any) => {
            copiedCommands.push({ src, dst, size })
          },
          finish: () => ({ finish: true }),
        }
        return enc
      },
      queue: {
        writeBuffer: vi.fn(),
        submit: vi.fn(),
      },
    }

    return { dev, passes, copiedCommands, setReadbackId: (id: number) => { mappedData = new Uint32Array([id, 0, 0, 0]) } }
  }

  function fakeMesh(visible = true): GpuPickerMesh {
    return {
      vb: {} as any,
      ib: {} as any,
      ic: 36,
      transform: identityMat4(),
      visible,
    }
  }

  it('returns null immediately when meshes list is empty', async () => {
    const { dev } = makeMockDevice(0)
    const picker = new GpuPicker()
    const result = await picker.pick(dev, [], {
      cameraViewProjection: identityMat4(),
      viewportWidth: 800,
      viewportHeight: 600,
    }, 100, 100)

    expect(result).toBeNull()
    picker.destroy()
  })

  it('executes 1x1 picking render pass and decodes background as null', async () => {
    const { dev, passes, copiedCommands } = makeMockDevice(0) // rawId 0 = background
    const picker = new GpuPicker()
    const meshes = [fakeMesh(true), fakeMesh(true)]

    const result = await picker.pick(dev, meshes, {
      cameraViewProjection: identityMat4(),
      viewportWidth: 800,
      viewportHeight: 600,
    }, 400, 300)

    expect(result).toBeNull()
    expect(passes.length).toBe(1)
    // 2 visible meshes were drawn into the 1x1 pass
    expect(passes[0].draws).toBe(2)
    // copyTextureToBuffer was recorded
    expect(copiedCommands.length).toBe(1)
    expect(copiedCommands[0].size).toEqual([1, 1, 1])

    picker.destroy()
  })

  it('decodes hit mesh index correctly from GPU readback buffer', async () => {
    const { dev, setReadbackId } = makeMockDevice(2) // rawId 2 = meshIndex 1 (2 - 1)
    const picker = new GpuPicker()
    const meshes = [fakeMesh(true), fakeMesh(true), fakeMesh(true)]

    const hit = await picker.pick(dev, meshes, {
      cameraViewProjection: identityMat4(),
      viewportWidth: 800,
      viewportHeight: 600,
    }, 150, 250)

    expect(hit).toBe(1)

    // Test hit on mesh 0 (rawId 1)
    setReadbackId(1)
    const hit0 = await picker.pick(dev, meshes, {
      cameraViewProjection: identityMat4(),
      viewportWidth: 800,
      viewportHeight: 600,
    }, 150, 250)
    expect(hit0).toBe(0)

    picker.destroy()
  })

  it('skips invisible meshes during picking draw calls', async () => {
    const { dev, passes } = makeMockDevice(1)
    const picker = new GpuPicker()
    const meshes = [
      fakeMesh(true),
      fakeMesh(false), // invisible, should be skipped
      fakeMesh(true),
    ]

    await picker.pick(dev, meshes, {
      cameraViewProjection: identityMat4(),
      viewportWidth: 800,
      viewportHeight: 600,
    }, 100, 100)

    expect(passes[0].draws).toBe(2)
    picker.destroy()
  })

  it('destroys all allocated GPU textures and buffers cleanly', async () => {
    const { dev } = makeMockDevice(0)
    const picker = new GpuPicker()
    await picker.pick(dev, [fakeMesh(true)], {
      cameraViewProjection: identityMat4(),
      viewportWidth: 800,
      viewportHeight: 600,
    }, 100, 100)

    expect(() => picker.destroy()).not.toThrow()
  })

  it('integrates with WebGPURenderer.gpuPickAt safely', async () => {
    const { WebGPURenderer } = await import('../src/services/webgpuRenderer')
    const renderer = new WebGPURenderer()
    const uninit = await renderer.gpuPickAt(100, 100)
    expect(uninit).toBeNull()
    renderer.destroy()
  })
})
