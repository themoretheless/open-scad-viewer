import { describe, expect, it, vi } from 'vitest'
import { WebGPURenderer } from '../src/services/webgpuRenderer'
import { SCENE_UNIFORM_LAYOUT } from '../src/services/shaders'

/**
 * Renderer-level checks for the key-light contact shadow: buildPipelines
 * creates the depth-map binding with a 1×1 inert dummy, enabling shadows
 * allocates the real map and issues a depth-only pass before the main pass,
 * and the disabled default keeps the frame (and scene uniform) byte-identical.
 */

interface FakeTexture { label: string; view: object; destroyed: boolean; destroy(): void }

function fakeTexture(label: string): FakeTexture & { createView(): object } {
  const view = { viewOf: label }
  return { label, view, destroyed: false, createView: () => view, destroy() { this.destroyed = true } }
}

function fakeMesh() {
  return {
    morph: undefined,
    morphSlot: {},
    vb: {},
    ib: {},
    ic: 3,
    bg: 'objBG',
    edgeIB: null,
    edgeIC: 0,
    alpha: 1,
    visible: true,
    shadingModel: 'phong',
    worldBounds: {
      center: [0, 0, 0],
      radius: 1e6, // never frustum-culled, whatever the default camera
      min: [-1, -1, -1],
      max: [1, 1, 1],
    },
  }
}

function fakeDevice(record: {
  textures: Array<{ size: number[]; format: string; usage: number }>
  samplers: unknown[]
  bindGroups: Array<{ layout: unknown; entries: unknown[] }>
  pipelines: Array<{ depthStencil?: { format: string; depthBiasSlopeScale?: number }; fragment?: unknown; layout?: { groups: number } }>
  passes: Array<{ colors: number; depthFormat: string | null; indexedDraws: number }>
  writes: Array<{ buffer: unknown; data: Float32Array }>
}) {
  const makePass = (descriptor: { colorAttachments: unknown[]; depthStencilAttachment?: { view: object } }) => {
    const pass = {
      descriptor,
      setPipeline: () => undefined,
      setBindGroup: () => undefined,
      setVertexBuffer: () => undefined,
      setIndexBuffer: () => undefined,
      draw: () => undefined,
      drawIndexed: () => undefined,
      executeBundles: () => undefined,
      setImmediates: () => undefined,
      end: () => undefined,
    }
    const entry = {
      colors: descriptor.colorAttachments.length,
      depthFormat: descriptor.depthStencilAttachment
        ? ((descriptor.depthStencilAttachment.view as { viewOf?: string }).viewOf ?? null)
        : null,
      indexedDraws: 0,
    }
    record.passes.push(entry)
    pass.drawIndexed = () => { entry.indexedDraws++ }
    return pass
  }
  return {
    limits: { maxTextureDimension2D: 8192, maxStorageBufferBindingSize: 1 << 27 },
    queue: {
      writeBuffer: (buffer: unknown, _offset: number, data: Float32Array) => {
        record.writes.push({ buffer, data: Float32Array.from(data) })
      },
      writeTexture: () => undefined,
      submit: () => undefined,
    },
    createTexture: (descriptor: { size: number[]; format: string; usage: number }) => {
      record.textures.push(descriptor)
      return fakeTexture(`tex${record.textures.length}`)
    },
    createSampler: (descriptor: unknown) => { const sampler = { descriptor }; record.samplers.push(sampler); return sampler },
    createBuffer: () => ({}),
    createBindGroupLayout: () => ({}),
    createPipelineLayout: (descriptor: { bindGroupLayouts: unknown[] }) => ({ groups: descriptor.bindGroupLayouts.length }),
    createShaderModule: () => ({}),
    createBindGroup: (descriptor: { layout: unknown; entries: unknown[] }) => {
      record.bindGroups.push(descriptor)
      return { bg: descriptor }
    },
    createRenderPipeline: (descriptor: { depthStencil?: { format: string; depthBiasSlopeScale?: number }; fragment?: unknown }) => {
      record.pipelines.push(descriptor)
      return { pipe: descriptor }
    },
    createCommandEncoder: () => ({ beginRenderPass: makePass, finish: () => ({}) }),
  }
}

function recordShape() {
  return {
    textures: [] as Array<{ size: number[]; format: string; usage: number }>,
    samplers: [] as unknown[],
    bindGroups: [] as Array<{ layout: unknown; entries: unknown[] }>,
    pipelines: [] as Array<{ depthStencil?: { format: string; depthBiasSlopeScale?: number }; fragment?: unknown; layout?: { groups: number } }>,
    passes: [] as Array<{ colors: number; depthFormat: string | null; indexedDraws: number }>,
    writes: [] as Array<{ buffer: unknown; data: Float32Array }>,
  }
}

function stubGpuGlobals() {
  vi.stubGlobal('GPUTextureUsage', { TEXTURE_BINDING: 4, COPY_DST: 8, RENDER_ATTACHMENT: 16 })
  vi.stubGlobal('GPUShaderStage', { VERTEX: 1, FRAGMENT: 2 })
  vi.stubGlobal('GPUBufferUsage', { VERTEX: 32, INDEX: 16, UNIFORM: 64, COPY_DST: 8, STORAGE: 128 })
}

function harness() {
  stubGpuGlobals()
  const record = recordShape()
  const dev = fakeDevice(record)
  const renderer = new WebGPURenderer()
  const internal = renderer as unknown as Record<string, unknown> & { render(): void; buildPipelines(): void }
  internal.dev = dev
  // Real setup: shadow dummy, bind groups, vertex layouts; the draw-path
  // pipelines are replaced with string markers below for pass assertions.
  internal.buildPipelines()
  internal.canvas = { width: 100, height: 100, clientWidth: 100, clientHeight: 100 }
  internal.ctx = { getCurrentTexture: () => ({ createView: () => ({}) }) }
  internal.initialized = true
  internal.lost = false
  internal.dead = false
  internal.gridVisible = false
  internal.sceneUB = 'sceneUB'
  internal.sceneBG = 'sceneBG'
  internal.meshPipe = 'mesh'
  internal.meshPipeT = 'meshT'
  internal.meshImmediatePipeT = null
  internal.sectionCapPipe = 'cap'
  internal.shadowPipe = 'shadow'
  ;(internal.textures as Record<string, unknown>).shadowSampler = {}
  internal.meshes = [fakeMesh()]
  internal.bounds = { center: [0, 0, 0], radius: 10, min: [-1, -1, -1], max: [1, 1, 1] }
  return { renderer, internal, record }
}

describe('WebGPURenderer contact shadows', () => {
  it('buildPipelines creates the shadow layout, a comparison sampler, and a 1×1 inert dummy', () => {
    const record = recordShape()
    stubGpuGlobals()
    const renderer = new WebGPURenderer()
    const internal = renderer as unknown as Record<string, unknown> & { buildPipelines(): void }
    internal.dev = fakeDevice(record)
    internal.buildPipelines()

    // The 1×1 depth dummy is bound so the shaders' sampling path is inert.
    const textures = internal.textures as Record<string, unknown>
    expect(record.textures).toContainEqual({ size: [1, 1], format: 'depth32float', usage: 4 })
    expect(textures.shadowBG).not.toBeNull()
    expect(textures.shadowDummyTexture).not.toBeNull()
    // meshShadow compiles to a depth-only pipeline (no fragment, depth32float).
    const shadowPipe = record.pipelines.find(pipeline => pipeline.depthStencil?.format === 'depth32float')
    expect(shadowPipe).toBeDefined()
    expect(shadowPipe!.fragment).toBeUndefined()
    // Its layout is scene+object only: the depth pass binds groups 0 and 1,
    // so a third (shadow) group would trip Dawn's "No bind group set" error.
    expect(shadowPipe!.layout!.groups).toBe(2)
    // Slope-scaled rasterizer bias keeps grazing-angle surfaces off their own
    // depth (ring-shaped self-shadowing "acne" on curved parts otherwise).
    expect(shadowPipe!.depthStencil!.depthBiasSlopeScale).toBeGreaterThan(0)
    // The lit surface pipelines keep the 3-group layout (scene+object+shadow).
    expect(record.pipelines.some(pipeline => pipeline.layout?.groups === 3)).toBe(true)
    // No real shadow map until shadows are enabled.
    expect(record.textures.some(t => t.size[0] === 1024)).toBe(false)
  })

  it('issues the depth-only shadow pass before the main pass when enabled', () => {
    const { renderer, internal, record } = harness()
    renderer.setShadowsEnabled(true)
    expect(record.textures).toContainEqual({
      size: [1024, 1024], format: 'depth32float', usage: 16 | 4,
    })
    internal.render()

    expect(record.passes).toHaveLength(2)
    const [shadow, main] = record.passes
    expect(shadow.colors).toBe(0)
    expect(main.colors).toBe(1)
    // The shadow pass draws into the real map, not the 1×1 dummy.
    expect(shadow.depthFormat).not.toContain('tex1')
  })

  it('omits the shadow pass and keeps the scene uniform byte-identical when disabled', () => {
    const { internal, record } = harness()
    internal.render()
    expect(record.passes).toHaveLength(1)
    expect(record.passes[0].colors).toBe(1)

    const write = record.writes.find(entry => entry.buffer === 'sceneUB')
    expect(write).toBeDefined()
    const data = write!.data
    expect(data).toHaveLength(SCENE_UNIFORM_LAYOUT.floats)
    // Shadow tail fully inert: lightVP zeros, params (0, texel, bias, strength).
    expect([...data.slice(SCENE_UNIFORM_LAYOUT.lightVPFloatOffset, SCENE_UNIFORM_LAYOUT.lightVPFloatOffset + 16)])
      .toEqual(new Array(16).fill(0))
    expect(data[SCENE_UNIFORM_LAYOUT.shadowFloatOffset]).toBe(0)
    expect(data[SCENE_UNIFORM_LAYOUT.shadowFloatOffset + 1]).toBeCloseTo(1 / 1024)
    // Legacy head unchanged: light and ambient still at floats 20..27.
    expect(data[20]).toBeCloseTo(0.55)
    expect(data[24]).toBeCloseTo(0.22)
  })

  it('fills the light VP and enable flag when enabled', () => {
    const { renderer, internal, record } = harness()
    renderer.setShadowsEnabled(true)
    internal.render()

    const write = record.writes.find(entry => entry.buffer === 'sceneUB')
    const data = write!.data
    expect(data[SCENE_UNIFORM_LAYOUT.shadowFloatOffset]).toBe(1)
    const lightVP = data.slice(SCENE_UNIFORM_LAYOUT.lightVPFloatOffset, SCENE_UNIFORM_LAYOUT.lightVPFloatOffset + 16)
    expect(lightVP.some(value => value !== 0)).toBe(true)
    // Orthographic projection: last column is translation-free in w.
    expect(lightVP[15]).toBeCloseTo(1)
  })

  it('toggling back off rebinds the dummy and restores the disabled frame', () => {
    const { renderer, internal, record } = harness()
    renderer.setShadowsEnabled(true)
    renderer.setShadowsEnabled(false)
    expect(record.passes).toHaveLength(0)
    internal.render()
    expect(record.passes).toHaveLength(1)
    const write = record.writes.find(entry => entry.buffer === 'sceneUB')
    expect(write!.data[SCENE_UNIFORM_LAYOUT.shadowFloatOffset]).toBe(0)
  })

  describe('shadow map caching', () => {
    const shadowPasses = (record: ReturnType<typeof recordShape>) =>
      record.passes.filter(pass => pass.colors === 0).length

    it('re-renders the shadow pass only when the scene changes, never on camera moves', () => {
      const { renderer, internal, record } = harness()
      renderer.setShadowsEnabled(true)

      internal.render()
      expect(shadowPasses(record)).toBe(1)

      // Static frames hit the cache: no shadow pass is issued again.
      internal.render()
      internal.render()
      expect(shadowPasses(record)).toBe(1)
      expect(record.passes).toHaveLength(4) // (shadow+main) + main + main

      // Camera movement does not invalidate a light-space depth map.
      ;(internal as unknown as { yaw: number }).yaw += 0.5
      ;(internal as unknown as { pitch: number }).pitch += 0.25
      internal.render()
      expect(shadowPasses(record)).toBe(1)
    })

    it('re-renders after a visibility change', () => {
      const { renderer, internal, record } = harness()
      // Visibility changes rebuild selection overlays; stub those side paths.
      internal.rebuildSelectionOverlays = () => undefined
      internal.rebuildSourceHighlightOverlay = () => undefined
      renderer.setShadowsEnabled(true)
      internal.render()
      expect(shadowPasses(record)).toBe(1)

      // Hiding the only mesh nulls scene bounds, which disables the shadow
      // sampling path entirely (no pass issued, as before caching).
      renderer.setMeshVisibility(0, false)
      internal.render()
      expect(shadowPasses(record)).toBe(1)

      // Showing it again bumps the epoch: one shadow re-render, then cached.
      renderer.setMeshVisibility(0, true)
      internal.render()
      expect(shadowPasses(record)).toBe(2)
      internal.render()
      expect(shadowPasses(record)).toBe(2)
    })

    it('re-renders while a morph animates, then caches again after it finishes', () => {
      const { renderer, internal, record } = harness()
      renderer.setShadowsEnabled(true)
      internal.render()
      expect(shadowPasses(record)).toBe(1)

      // A time-based morph moves vertices every frame: the map cannot cache.
      const mesh = internal.meshes[0] as { morph?: unknown }
      mesh.morph = { from: new Float32Array(9), target: new Float32Array(9), started: performance.now() }
      internal.render()
      internal.render()
      expect(shadowPasses(record)).toBe(3)

      // Morph retired (advanceGeometryAnimation also bumps the epoch): the
      // final geometry renders once more, then the cache holds.
      mesh.morph = undefined
      ;(internal as unknown as { invalidateShadowMap(): void }).invalidateShadowMap()
      internal.render()
      expect(shadowPasses(record)).toBe(4)
      internal.render()
      expect(shadowPasses(record)).toBe(4)
    })

    it('forces one re-render when shadows are toggled back on', () => {
      const { renderer, internal, record } = harness()
      renderer.setShadowsEnabled(true)
      internal.render()
      internal.render()
      expect(shadowPasses(record)).toBe(1)

      renderer.setShadowsEnabled(false)
      internal.render()
      expect(shadowPasses(record)).toBe(1)

      // Toggling on recreates the map: it must render once even if nothing
      // else changed, then cache again.
      renderer.setShadowsEnabled(true)
      internal.render()
      expect(shadowPasses(record)).toBe(2)
      internal.render()
      expect(shadowPasses(record)).toBe(2)
    })

    it('clears the map when the scene turns fully transparent, leaving no stale casters', () => {
      const { renderer, internal, record } = harness()
      // Restyling reads the vertex colour and writes the style uniform.
      Object.assign((internal.meshes as object[])[0], { color: [1, 1, 1, 1], ub: {} })
      renderer.setShadowsEnabled(true)
      internal.render()
      const casters = record.passes.filter(pass => pass.colors === 0)
      expect(casters.map(pass => pass.indexedDraws)).toEqual([1])

      // A translucent default (e.g. the Glass preset) moves every mesh out of
      // the opaque set: the map must be re-rendered empty, not kept from the
      // last opaque frame, or the grid keeps a shadow of nothing.
      renderer.setDefaultMaterial({ alpha: 0.4 })
      internal.render()
      const after = record.passes.filter(pass => pass.colors === 0)
      expect(after.map(pass => pass.indexedDraws)).toEqual([1, 0])
    })

    it('re-renders when the geometry epoch advances (setMeshes path)', () => {
      const { renderer, internal, record } = harness()
      renderer.setShadowsEnabled(true)
      internal.render()
      expect(shadowPasses(record)).toBe(1)

      // setMeshes bumps the epoch; simulated here through the same hook.
      ;(internal as unknown as { invalidateShadowMap(): void }).invalidateShadowMap()
      internal.render()
      expect(shadowPasses(record)).toBe(2)
    })
  })
})
