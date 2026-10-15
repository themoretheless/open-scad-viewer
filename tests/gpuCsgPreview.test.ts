import { describe, expect, it, vi } from 'vitest'
import {
  identityMat4,
  multiplyMat4,
  translationMat4,
  scaleMat4,
  rotationEulerMat4,
  normalMatrixFromModel,
  generateUnitCube,
  generateUnitCylinder,
  generateUnitSphere,
  createCsgPrimitive,
  flattenCsgNode,
  compileGpuCsgTree,
  GpuCsgPipelineManager,
  GpuCsgRenderer,
  type GpuCsgNode,
} from '../src/services/gpuCsgPreview'
import { WebGPURenderer } from '../src/services/webgpuRenderer'
import { compileOpenSCAD } from '../src/services/openscadCompiler'

describe('gpuCsgPreview math and primitives', () => {
  it('computes 4x4 matrix multiplication and translations correctly', () => {
    const ident = identityMat4()
    expect(ident[0]).toBe(1)
    expect(ident[5]).toBe(1)
    expect(ident[10]).toBe(1)
    expect(ident[15]).toBe(1)

    const t = translationMat4(10, 20, 30)
    expect(t[3]).toBe(10)
    expect(t[7]).toBe(20)
    expect(t[11]).toBe(30)

    const s = scaleMat4(2, 3, 4)
    const combined = multiplyMat4(t, s)
    // Row 0: [2, 0, 0, 10]
    expect(combined[0]).toBe(2)
    expect(combined[3]).toBe(10)
    // Row 1: [0, 3, 0, 20]
    expect(combined[5]).toBe(3)
    expect(combined[7]).toBe(20)
    // Row 2: [0, 0, 4, 30]
    expect(combined[10]).toBe(4)
    expect(combined[11]).toBe(30)
  })

  it('computes normal matrix from model transform', () => {
    const s = scaleMat4(2, 4, 8)
    const n = normalMatrixFromModel(s)
    // Normal matrix scales inverse: 1/2, 1/4, 1/8
    expect(n[0]).toBeCloseTo(0.5)
    expect(n[5]).toBeCloseTo(0.25)
    expect(n[10]).toBeCloseTo(0.125)
  })

  it('generates unit cube with 24 vertices and sharp face normals', () => {
    const cube = generateUnitCube(true)
    // 24 vertices * 6 floats (px, py, pz, nx, ny, nz) = 144 floats
    expect(cube.vertices.length).toBe(144)
    // 36 indices = 12 triangles
    expect(cube.indices.length).toBe(36)

    // Check bounds: min -0.5, max 0.5
    for (let i = 0; i < cube.vertices.length; i += 6) {
      const x = cube.vertices[i]
      const y = cube.vertices[i + 1]
      const z = cube.vertices[i + 2]
      expect(Math.abs(x)).toBeCloseTo(0.5)
      expect(Math.abs(y)).toBeCloseTo(0.5)
      expect(Math.abs(z)).toBeCloseTo(0.5)

      const nx = cube.vertices[i + 3]
      const ny = cube.vertices[i + 4]
      const nz = cube.vertices[i + 5]
      const normLen = Math.hypot(nx, ny, nz)
      expect(normLen).toBeCloseTo(1.0)
    }
  })

  it('generates unit cylinder with caps and side walls', () => {
    const cyl = generateUnitCylinder(16, true)
    expect(cyl.vertices.length).toBeGreaterThan(0)
    expect(cyl.indices.length).toBeGreaterThan(0)
    expect(cyl.indices.length % 3).toBe(0)

    // Check that top cap center has z = 0.5 and normal (0, 0, 1)
    expect(cyl.vertices[0]).toBe(0)
    expect(cyl.vertices[1]).toBe(0)
    expect(cyl.vertices[2]).toBe(0.5)
    expect(cyl.vertices[3]).toBe(0)
    expect(cyl.vertices[4]).toBe(0)
    expect(cyl.vertices[5]).toBe(1)
  })

  it('generates unit sphere with normalized vertex normals', () => {
    const sphere = generateUnitSphere(16, 8)
    expect(sphere.vertices.length).toBeGreaterThan(0)
    expect(sphere.indices.length).toBeGreaterThan(0)
    expect(sphere.indices.length % 3).toBe(0)

    // Verify all positions lie on unit sphere and match normals
    for (let i = 0; i < sphere.vertices.length; i += 6) {
      const x = sphere.vertices[i]
      const y = sphere.vertices[i + 1]
      const z = sphere.vertices[i + 2]
      const r = Math.hypot(x, y, z)
      expect(r).toBeCloseTo(1.0, 4)

      const nx = sphere.vertices[i + 3]
      const ny = sphere.vertices[i + 4]
      const nz = sphere.vertices[i + 5]
      expect(nx).toBeCloseTo(x, 4)
      expect(ny).toBeCloseTo(y, 4)
      expect(nz).toBeCloseTo(z, 4)
    }
  })
})

describe('gpuCsgPreview tree flattening & AST compilation', () => {
  it('flattens simple primitives into single terms', () => {
    const cubeNode: GpuCsgNode = {
      type: 'primitive',
      primitive: createCsgPrimitive('cube'),
    }
    const terms = flattenCsgNode(cubeNode)
    expect(terms.length).toBe(1)
    expect(terms[0].positive.kind).toBe('cube')
    expect(terms[0].cutters.length).toBe(0)
  })

  it('flattens difference(cube, cylinder, sphere) into 1 term with 2 cutters', () => {
    const diffNode: GpuCsgNode = {
      type: 'operation',
      op: 'difference',
      children: [
        { type: 'primitive', primitive: createCsgPrimitive('cube') },
        { type: 'primitive', primitive: createCsgPrimitive('cylinder') },
        { type: 'primitive', primitive: createCsgPrimitive('sphere') },
      ],
    }
    const terms = flattenCsgNode(diffNode)
    expect(terms.length).toBe(1)
    expect(terms[0].positive.kind).toBe('cube')
    expect(terms[0].cutters.length).toBe(2)
    expect(terms[0].cutters[0].kind).toBe('cylinder')
    expect(terms[0].cutters[1].kind).toBe('sphere')
  })

  it('flattens union(diff1, diff2) into 2 independent terms', () => {
    const unionNode: GpuCsgNode = {
      type: 'operation',
      op: 'union',
      children: [
        {
          type: 'operation',
          op: 'difference',
          children: [
            { type: 'primitive', primitive: createCsgPrimitive('cube') },
            { type: 'primitive', primitive: createCsgPrimitive('cylinder') },
          ],
        },
        {
          type: 'operation',
          op: 'difference',
          children: [
            { type: 'primitive', primitive: createCsgPrimitive('sphere') },
            { type: 'primitive', primitive: createCsgPrimitive('cube') },
          ],
        },
      ],
    }
    const terms = flattenCsgNode(unionNode)
    expect(terms.length).toBe(2)
    expect(terms[0].positive.kind).toBe('cube')
    expect(terms[0].cutters.length).toBe(1)
    expect(terms[1].positive.kind).toBe('sphere')
    expect(terms[1].cutters.length).toBe(1)
  })

  it('compiles OpenSCAD source with difference and transforms into GpuCsgTree', () => {
    const source = `
      difference() {
        cube([20, 20, 20], center=true);
        translate([0, 0, 5]) cylinder(h=30, r=4, center=true);
      }
    `
    const ast = compileOpenSCAD(source)
    const tree = compileGpuCsgTree(ast)
    expect(tree.terms.length).toBe(1)
    expect(tree.terms[0].positive.kind).toBe('cube')
    expect(tree.terms[0].cutters.length).toBe(1)
    expect(tree.terms[0].cutters[0].kind).toBe('cylinder')

    // Cutter should have Z translation 5
    const cutterTransform = tree.terms[0].cutters[0].transform
    expect(cutterTransform[11]).toBe(5)
  })
})

describe('gpuCsgPreview WebGPU pipelines and renderer', () => {
  function makeMockDevice() {
    vi.stubGlobal('GPUShaderStage', { VERTEX: 1, FRAGMENT: 2 })
    vi.stubGlobal('GPUBufferUsage', { VERTEX: 32, INDEX: 16, UNIFORM: 64, COPY_DST: 8, STORAGE: 128 })

    const createdPipelines: Array<{ descriptor: any }> = []
    const passes: Array<{
      desc: any
      pipeline?: any
      stencilRef?: number
      bindGroups: any[]
      drawCalls: number
    }> = []

    const dev: any = {
      createBindGroupLayout: (desc: any) => ({ desc }),
      createPipelineLayout: (desc: any) => ({ desc }),
      createShaderModule: (desc: any) => ({ desc }),
      createRenderPipeline: (desc: any) => {
        const pipe = { pipeDesc: desc }
        createdPipelines.push({ descriptor: desc })
        return pipe
      },
      createBuffer: (desc: any) => {
        let mapped: ArrayBuffer | null = null
        if (desc.mappedAtCreation) {
          mapped = new ArrayBuffer(desc.size)
        }
        return {
          desc,
          getMappedRange: () => mapped!,
          unmap: () => undefined,
          destroy: () => undefined,
        }
      },
      createBindGroup: (desc: any) => ({ desc }),
      queue: {
        writeBuffer: () => undefined,
      },
    }

    const encoder: any = {
      beginRenderPass: (desc: any) => {
        const currentPass: any = {
          desc,
          bindGroups: [],
          drawCalls: 0,
          setPipeline: (p: any) => { currentPass.pipeline = p },
          setBindGroup: (idx: number, bg: any) => { currentPass.bindGroups[idx] = bg },
          setVertexBuffer: () => undefined,
          setIndexBuffer: () => undefined,
          setStencilReference: (ref: number) => { currentPass.stencilRef = ref },
          drawIndexed: () => { currentPass.drawCalls++ },
          end: () => undefined,
        }
        passes.push(currentPass)
        return currentPass
      },
    }

    return { dev, encoder, createdPipelines, passes }
  }

  it('creates depth/stencil CSG pipelines with exact stencil parity configurations', () => {
    const { dev } = makeMockDevice()
    const pipelineManager = new GpuCsgPipelineManager('bgra8unorm')
    const pipelines = pipelineManager.getPipelines(dev)

    expect(pipelines.depthPipeline).toBeDefined()
    expect(pipelines.parityPipeline).toBeDefined()
    expect(pipelines.shadeBasePipeline).toBeDefined()
    expect(pipelines.shadeCutPipeline).toBeDefined()
    expect(pipelines.shadeIntersectPipeline).toBeDefined()

    // Pass 1: Depth pipeline: writes depth, culls back
    const depthDesc = (pipelines.depthPipeline as any).pipeDesc
    expect(depthDesc.depthStencil.format).toBe('depth24plus-stencil8')
    expect(depthDesc.depthStencil.depthWriteEnabled).toBe(true)
    expect(depthDesc.depthStencil.depthCompare).toBe('less')
    expect(depthDesc.primitive.cullMode).toBe('back')

    // Pass 2: Parity pipeline: depthWrite false, cullMode none, stencilFront increment-wrap, stencilBack decrement-wrap
    const parityDesc = (pipelines.parityPipeline as any).pipeDesc
    expect(parityDesc.depthStencil.depthWriteEnabled).toBe(false)
    expect(parityDesc.depthStencil.depthCompare).toBe('less')
    expect(parityDesc.primitive.cullMode).toBe('none')
    expect(parityDesc.depthStencil.stencilFront.passOp).toBe('increment-wrap')
    expect(parityDesc.depthStencil.stencilBack.passOp).toBe('decrement-wrap')

    // Pass 3: Shade base pipeline: depthCompare equal, stencilFront compare equal
    const shadeBaseDesc = (pipelines.shadeBasePipeline as any).pipeDesc
    expect(shadeBaseDesc.depthStencil.depthCompare).toBe('equal')
    expect(shadeBaseDesc.depthStencil.stencilFront.compare).toBe('equal')
    expect(shadeBaseDesc.primitive.cullMode).toBe('back')

    // Pass 4: Shade cut pipeline: cullMode front (inside cavity)
    const shadeCutDesc = (pipelines.shadeCutPipeline as any).pipeDesc
    expect(shadeCutDesc.primitive.cullMode).toBe('front')
    expect(shadeCutDesc.depthStencil.depthCompare).toBe('less')
  })

  it('renders CSG tree executing Goldfeather / SCS passes for subtracted terms', () => {
    const { dev, encoder, passes } = makeMockDevice()
    const renderer = new GpuCsgRenderer('bgra8unorm')

    const diffNode: GpuCsgNode = {
      type: 'operation',
      op: 'difference',
      children: [
        { type: 'primitive', primitive: createCsgPrimitive('cube') },
        { type: 'primitive', primitive: createCsgPrimitive('cylinder') },
      ],
    }
    const tree = { terms: flattenCsgNode(diffNode) }

    const fakeColorView = {} as any
    const fakeDepthStencilView = {} as any
    const opts = {
      viewProjection: identityMat4(),
      eyePosition: [0, 0, 100] as [number, number, number],
    }

    renderer.renderCsg(dev, encoder, fakeColorView, fakeDepthStencilView, tree, opts)

    // For a difference term:
    // Pass 1: Base Depth
    // Pass 2: Cutter Parity
    // Pass 3 & 4: Shade Base (with stencil ref 0) and Shade Cutters
    expect(passes.length).toBe(3) // 3 beginRenderPass calls (Pass 3 & 4 are combined in the shading pass)

    // Pass 1 checks
    expect(passes[0].pipeline).toBe(renderer.pipelineManager.getPipelines(dev).depthPipeline)
    expect(passes[0].drawCalls).toBe(1)

    // Pass 2 checks (Cutter parity)
    expect(passes[1].pipeline).toBe(renderer.pipelineManager.getPipelines(dev).parityPipeline)
    expect(passes[1].drawCalls).toBe(1) // 1 cutter

    // Pass 3 checks (Shade Base + Shade Cut)
    expect(passes[2].stencilRef).toBe(0)
    expect(passes[2].drawCalls).toBe(2) // 1 base draw + 1 cutter back-face draw

    renderer.destroy()
  })

  it('takes fast path (1 pass) for pure union primitives without cutters', () => {
    const { dev, encoder, passes } = makeMockDevice()
    const renderer = new GpuCsgRenderer('bgra8unorm')

    const unionNode: GpuCsgNode = {
      type: 'primitive',
      primitive: createCsgPrimitive('cube'),
    }
    const tree = { terms: flattenCsgNode(unionNode) }

    renderer.renderCsg(dev, encoder, {} as any, {} as any, tree, {
      viewProjection: identityMat4(),
      eyePosition: [0, 0, 100],
    })

    // Pure union primitive without cutters should take the 1-pass fast path!
    expect(passes.length).toBe(1)
    expect(passes[0].pipeline).toBe(renderer.pipelineManager.getPipelines(dev).shadeBasePipeline)
    expect(passes[0].drawCalls).toBe(1)

    renderer.destroy()
  })

  it('integrates seamlessly with WebGPURenderer lifecycle and render passes', () => {
    const { dev } = makeMockDevice()
    // Test WebGPURenderer methods
    const renderer = new WebGPURenderer()
    expect(renderer.getCsgPreviewTree()).toBeNull()

    const diffTree = {
      terms: flattenCsgNode({
        type: 'operation',
        op: 'difference',
        children: [
          { type: 'primitive', primitive: createCsgPrimitive('cube') },
          { type: 'primitive', primitive: createCsgPrimitive('cylinder') },
        ],
      }),
    }

    renderer.setCsgPreviewTree(diffTree)
    expect(renderer.getCsgPreviewTree()).toBe(diffTree)

    renderer.setCsgPreviewTree(null)
    expect(renderer.getCsgPreviewTree()).toBeNull()

    renderer.destroy()
  })
})
