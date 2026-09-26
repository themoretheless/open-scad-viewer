import { describe, expect, it, vi } from 'vitest'
import { WebGPURenderer } from '../src/services/webgpuRenderer'

/**
 * Renderer-level check that the section-cap pass is issued exactly when the
 * section plane clips a surfaced scene: the frame must draw opaque meshes
 * with the mesh pipeline, then redraw them with the front-culled cap pipeline.
 */

interface FakeInternals {
  canvas: unknown
  dev: unknown
  ctx: unknown
  initialized: boolean
  lost: boolean
  dead: boolean
  drawable: boolean
  gridVisible: boolean
  sceneUB: unknown
  sceneBG: unknown
  meshPipe: unknown
  meshPipeT: unknown
  meshImmediatePipeT: unknown
  sectionCapPipe: unknown
  sectionEnabled: boolean
  sectionNormal: [number, number, number]
  sectionOffset: number
  displayMode: string
  meshes: unknown[]
  render(): void
}

function fakeMesh() {
  return {
    morph: undefined,
    morphSlot: {},
    vb: {},
    ib: {},
    ic: 3,
    bg: {},
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

function harness() {
  vi.stubGlobal('GPUTextureUsage', { RENDER_ATTACHMENT: 16 })
  const pipelines: unknown[] = []
  const pass = {
    setPipeline: (pipeline: unknown) => { pipelines.push(pipeline) },
    setBindGroup: () => undefined,
    setVertexBuffer: () => undefined,
    setIndexBuffer: () => undefined,
    draw: () => undefined,
    drawIndexed: () => undefined,
    executeBundles: () => undefined,
    setImmediates: () => undefined,
    end: () => undefined,
  }
  const encoder = { beginRenderPass: () => pass, finish: () => ({}) }
  const dev = {
    limits: { maxTextureDimension2D: 8192, maxStorageBufferBindingSize: 1 << 27 },
    queue: { writeBuffer: () => undefined, submit: () => undefined },
    createTexture: () => ({ createView: () => ({}), destroy: () => undefined }),
    createCommandEncoder: () => encoder,
  }
  const renderer = new WebGPURenderer()
  const internal = renderer as unknown as FakeInternals
  internal.canvas = { width: 100, height: 100, clientWidth: 100, clientHeight: 100 }
  internal.dev = dev
  internal.ctx = { getCurrentTexture: () => ({ createView: () => ({}) }) }
  internal.initialized = true
  internal.lost = false
  internal.dead = false
  internal.gridVisible = false
  internal.sceneUB = {}
  internal.sceneBG = {}
  internal.meshPipe = 'mesh'
  internal.meshPipeT = 'meshT'
  internal.meshImmediatePipeT = null
  internal.sectionCapPipe = 'cap'
  internal.meshes = [fakeMesh()]
  internal.sectionNormal = [0, 0, 1]
  internal.sectionOffset = 0
  return { internal, pipelines }
}

describe('WebGPURenderer section-cap pass', () => {
  it('redraws opaque meshes with the cap pipeline after the surface pass when the section plane is enabled', () => {
    const { internal, pipelines } = harness()
    internal.sectionEnabled = true
    internal.render()
    const meshAt = pipelines.indexOf('mesh')
    const capAt = pipelines.indexOf('cap')
    expect(meshAt).toBeGreaterThanOrEqual(0)
    expect(capAt).toBeGreaterThan(meshAt)
  })

  it('skips the cap pass when the section plane is disabled', () => {
    const { internal, pipelines } = harness()
    internal.sectionEnabled = false
    internal.render()
    expect(pipelines).toContain('mesh')
    expect(pipelines).not.toContain('cap')
  })

  it('skips the cap pass in xray mode (surfaces route through the transparent pass)', () => {
    const { internal, pipelines } = harness()
    internal.sectionEnabled = true
    internal.displayMode = 'xray'
    internal.render()
    expect(pipelines).not.toContain('cap')
  })
})
