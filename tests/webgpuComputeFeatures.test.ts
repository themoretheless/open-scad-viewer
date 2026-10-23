import { afterEach, describe, expect, it, vi } from 'vitest'
import { runGpuCompute } from '../src/services/webgpuCompute'
import {
  closestPointOnNurbsCurve,
  computeNurbsCurveBoundingBox,
  evaluateNurbsCurveCurvature,
  evaluateNurbsCurveLength,
  subdivideNurbsCurvePolylineAdaptive,
  type NurbsCurve,
} from '../src/services/nurbsCurve'
import {
  closestPointOnNurbsSurface,
  evaluateNurbsSurfaceArea,
  evaluateNurbsSurfaceNormal,
  type NurbsSurface,
} from '../src/services/nurbsSurface'

describe('WebGPU compute feature negotiation', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('requests subgroup-size-control when WGSL uses @subgroup_size', async () => {
    vi.stubGlobal('GPUShaderStage', { COMPUTE: 4 })
    vi.stubGlobal('GPUBufferUsage', { STORAGE: 1, COPY_SRC: 2, MAP_READ: 4, COPY_DST: 8 })
    vi.stubGlobal('GPUMapMode', { READ: 1 })

    const requestDevice = vi.fn(async () => ({
      queue: { writeBuffer: vi.fn(), submit: vi.fn() },
      createShaderModule: vi.fn(() => ({})),
      createBindGroupLayout: vi.fn(() => ({})),
      createPipelineLayout: vi.fn(() => ({})),
      createComputePipeline: vi.fn(() => ({})),
      createCommandEncoder: vi.fn(() => ({
        beginComputePass: () => ({
          setPipeline: vi.fn(),
          setBindGroup: vi.fn(),
          dispatchWorkgroups: vi.fn(),
          end: vi.fn(),
        }),
        copyBufferToBuffer: vi.fn(),
        finish: vi.fn(() => ({})),
      })),
      createBuffer: vi.fn(() => ({
        destroy: vi.fn(),
        mapAsync: vi.fn(async () => undefined),
        getMappedRange: vi.fn(() => new ArrayBuffer(4)),
        unmap: vi.fn(),
      })),
      createBindGroup: vi.fn(() => ({})),
      destroy: vi.fn(),
    }))
    vi.stubGlobal('navigator', {
      gpu: {
        wgslLanguageFeatures: new Set(),
        requestAdapter: async () => ({
          features: new Set(['subgroups', 'subgroup-size-control']),
          requestDevice,
        }),
      },
    })

    await runGpuCompute({
      wgsl: 'enable subgroups;\nenable subgroup_size_control;\n@subgroup_size(32) @compute @workgroup_size(64) fn main() {}',
      entryPoint: 'main',
      dispatches: [{
        buffers: [{ binding: 0, data: new Float32Array(0), output: true }],
        outputBytes: 4,
        workgroups: [1, 1, 1],
      }],
    })

    expect(requestDevice).toHaveBeenCalledWith({ requiredFeatures: ['subgroups', 'subgroup-size-control'] })
  })

  it('rejects buffer_view shaders when WGSL language support is unavailable', async () => {
    const requestDevice = vi.fn()
    vi.stubGlobal('navigator', {
      gpu: {
        wgslLanguageFeatures: new Set(),
        requestAdapter: async () => ({
          features: new Set(),
          requestDevice,
        }),
      },
    })

    await expect(runGpuCompute({
      wgsl: 'requires buffer_view;\n@compute @workgroup_size(1) fn main() {}',
      entryPoint: 'main',
      dispatches: [{
        buffers: [{ binding: 0, data: new Float32Array(0), output: true }],
        outputBytes: 4,
        workgroups: [1, 1, 1],
      }],
    })).rejects.toThrow('WebGPU WGSL lacks required feature(s): buffer_view')
    expect(requestDevice).not.toHaveBeenCalled()
  })

  it('uses the first supported WGSL variant before creating the shader module', async () => {
    vi.stubGlobal('GPUShaderStage', { COMPUTE: 4 })
    vi.stubGlobal('GPUBufferUsage', { STORAGE: 1, COPY_SRC: 2, MAP_READ: 4, COPY_DST: 8 })
    vi.stubGlobal('GPUMapMode', { READ: 1 })

    const createShaderModule = vi.fn(() => ({}))
    const dispatchWorkgroups = vi.fn()
    vi.stubGlobal('navigator', {
      gpu: {
        wgslLanguageFeatures: new Set(['linear_indexing']),
        requestAdapter: async () => ({
          features: new Set(),
          requestDevice: async () => ({
            queue: { writeBuffer: vi.fn(), submit: vi.fn() },
            createShaderModule,
            createBindGroupLayout: vi.fn(() => ({})),
            createPipelineLayout: vi.fn(() => ({})),
            createComputePipeline: vi.fn(() => ({})),
            createCommandEncoder: vi.fn(() => ({
              beginComputePass: () => ({
                setPipeline: vi.fn(),
                setBindGroup: vi.fn(),
                dispatchWorkgroups,
                end: vi.fn(),
              }),
              copyBufferToBuffer: vi.fn(),
              finish: vi.fn(() => ({})),
            })),
            createBuffer: vi.fn(() => ({
              destroy: vi.fn(),
              mapAsync: vi.fn(async () => undefined),
              getMappedRange: vi.fn(() => new ArrayBuffer(4)),
              unmap: vi.fn(),
            })),
            createBindGroup: vi.fn(() => ({})),
            destroy: vi.fn(),
          }),
        }),
      },
    })

    const linear = 'requires linear_indexing;\n@compute @workgroup_size(1) fn main(@builtin(global_invocation_index) i: u32) {}'
    await runGpuCompute({
      wgsl: '@compute @workgroup_size(1) fn main() {}',
      wgslVariants: [
        { label: 'buffer-view', wgsl: 'requires buffer_view;\n@compute @workgroup_size(1) fn main() {}' },
        { label: 'linear', wgsl: linear },
      ],
      entryPoint: 'main',
      dispatches: [{
        buffers: [{ binding: 0, data: new Float32Array(0), output: true }],
        outputBytes: 4,
        workgroups: [1, 1, 1],
        variantWorkgroups: {
          linear: [4, 1, 1],
        },
      }],
    })

    expect(createShaderModule).toHaveBeenCalledWith({ code: linear })
    expect(dispatchWorkgroups).toHaveBeenCalledWith(4, 1, 1)
  })

  it('reuses GPUDevice and cached compute pipelines across multiple jobs without destroying device', async () => {
    vi.stubGlobal('GPUShaderStage', { COMPUTE: 4 })
    vi.stubGlobal('GPUBufferUsage', { STORAGE: 1, COPY_SRC: 2, MAP_READ: 4, COPY_DST: 8 })
    vi.stubGlobal('GPUMapMode', { READ: 1 })

    const createShaderModule = vi.fn(() => ({}))
    const createComputePipeline = vi.fn(() => ({}))
    const deviceDestroy = vi.fn()
    const requestDevice = vi.fn(async () => ({
      queue: { writeBuffer: vi.fn(), submit: vi.fn() },
      createShaderModule,
      createBindGroupLayout: vi.fn(() => ({})),
      createPipelineLayout: vi.fn(() => ({})),
      createComputePipeline,
      createCommandEncoder: vi.fn(() => ({
        beginComputePass: () => ({
          setPipeline: vi.fn(),
          setBindGroup: vi.fn(),
          dispatchWorkgroups: vi.fn(),
          end: vi.fn(),
        }),
        copyBufferToBuffer: vi.fn(),
        finish: vi.fn(() => ({})),
      })),
      createBuffer: vi.fn(() => ({
        destroy: vi.fn(),
        mapAsync: vi.fn(async () => undefined),
        getMappedRange: vi.fn(() => new ArrayBuffer(4)),
        unmap: vi.fn(),
      })),
      createBindGroup: vi.fn(() => ({})),
      destroy: deviceDestroy,
    }))

    const adapter = {
      features: new Set(),
      requestDevice,
    }
    vi.stubGlobal('navigator', {
      gpu: {
        wgslLanguageFeatures: new Set(),
        requestAdapter: async () => adapter,
      },
    })

    const job = {
      wgsl: '@compute @workgroup_size(1) fn main() {}',
      entryPoint: 'main',
      dispatches: [{
        buffers: [{ binding: 0, data: new Float32Array(0), output: true }],
        outputBytes: 4,
        workgroups: [1, 1, 1] as [number, number, number],
      }],
    }

    await runGpuCompute(job)
    expect(requestDevice).toHaveBeenCalledTimes(1)
    expect(createShaderModule).toHaveBeenCalledTimes(1)
    expect(createComputePipeline).toHaveBeenCalledTimes(1)
    expect(deviceDestroy).not.toHaveBeenCalled()

    // Second call with same adapter and job reuses device and pipeline
    await runGpuCompute(job)
    expect(requestDevice).toHaveBeenCalledTimes(1)
    expect(createShaderModule).toHaveBeenCalledTimes(1)
    expect(createComputePipeline).toHaveBeenCalledTimes(1)
    expect(deviceDestroy).not.toHaveBeenCalled()
  })

  it('evaluates NURBS curve length and curvature and NURBS surface normal and area', () => {
    const lineCurve: NurbsCurve = {
      degree: 1,
      knots: [0, 0, 1, 1],
      controlPoints: [
        [0, 0, 0],
        [3, 4, 0],
      ],
      weights: [1, 1],
    }
    const len = evaluateNurbsCurveLength(lineCurve, 16)
    expect(len).toBeCloseTo(5.0, 6)

    const lineCurv = evaluateNurbsCurveCurvature(lineCurve, 0.5)
    expect(lineCurv.curvature).toBeCloseTo(0, 6)
    expect(lineCurv.tangent?.[0]).toBeCloseTo(0.6, 6)
    expect(lineCurv.tangent?.[1]).toBeCloseTo(0.8, 6)

    // Quadratic rational quarter-circle of radius 2 in XY plane
    const r = 2
    const wMid = Math.SQRT1_2
    const arcCurve: NurbsCurve = {
      degree: 2,
      knots: [0, 0, 0, 1, 1, 1],
      controlPoints: [
        [r, 0, 0],
        [r, r, 0],
        [0, r, 0],
      ],
      weights: [1, wMid, 1],
    }
    const arcLen = evaluateNurbsCurveLength(arcCurve, 128)
    expect(arcLen).toBeCloseTo(Math.PI, 3)

    const arcCurv = evaluateNurbsCurveCurvature(arcCurve, 0.5)
    expect(arcCurv.curvature).toBeCloseTo(1 / r, 4)

    // Planar 2x3 rectangle surface in XY plane
    const rectSurface: NurbsSurface = {
      degreeU: 1,
      degreeV: 1,
      knotsU: [0, 0, 1, 1],
      knotsV: [0, 0, 1, 1],
      controlPoints: [
        [
          [0, 0, 0],
          [0, 3, 0],
        ],
        [
          [2, 0, 0],
          [2, 3, 0],
        ],
      ],
      weights: [
        [1, 1],
        [1, 1],
      ],
    }
    const normal = evaluateNurbsSurfaceNormal(rectSurface, 0.5, 0.5)
    expect(normal).not.toBeNull()
    expect(Math.abs(normal![2])).toBeCloseTo(1, 6)

    const area = evaluateNurbsSurfaceArea(rectSurface, 8, 8)
    expect(area).toBeCloseTo(6, 5)

    // Closest point on NURBS curve (line from (0,0,0) to (3,4,0))
    const cpCurve = closestPointOnNurbsCurve(lineCurve, [3, 0, 0], 32)
    // Projection of (3,0,0) onto direction (0.6, 0.8, 0) has length 1.8 -> u = 1.8 / 5 = 0.36, point = (1.08, 1.44, 0), distance = 2.4
    expect(cpCurve.u).toBeCloseTo(0.36, 4)
    expect(cpCurve.point[0]).toBeCloseTo(1.08, 4)
    expect(cpCurve.point[1]).toBeCloseTo(1.44, 4)
    expect(cpCurve.distance).toBeCloseTo(2.4, 4)

    // Closest point on NURBS surface ([0, 2] x [0, 3] in XY plane)
    const cpSurf = closestPointOnNurbsSurface(rectSurface, [1.2, 1.8, 4.0], 8)
    expect(cpSurf.u).toBeCloseTo(0.6, 4)
    expect(cpSurf.v).toBeCloseTo(0.6, 4)
    expect(cpSurf.point[0]).toBeCloseTo(1.2, 4)
    expect(cpSurf.point[1]).toBeCloseTo(1.8, 4)
    expect(cpSurf.point[2]).toBeCloseTo(0, 5)
    expect(cpSurf.distance).toBeCloseTo(4.0, 5)

    // NURBS curve bounding box and adaptive chord subdivision
    const bbox = computeNurbsCurveBoundingBox(arcCurve, 64)
    expect(bbox.min[0]).toBeCloseTo(0, 4)
    expect(bbox.min[1]).toBeCloseTo(0, 4)
    expect(bbox.max[0]).toBeCloseTo(2, 3)
    expect(bbox.max[1]).toBeCloseTo(2, 3)

    const linePoly = subdivideNurbsCurvePolylineAdaptive(lineCurve, 0.01, 8)
    expect(linePoly.length).toBeGreaterThanOrEqual(2)
    expect(linePoly[0]).toEqual([0, 0, 0])
    expect(linePoly[linePoly.length - 1]).toEqual([3, 4, 0])

    const arcPoly = subdivideNurbsCurvePolylineAdaptive(arcCurve, 0.005, 8)
    expect(arcPoly.length).toBeGreaterThan(4)
    for (const pt of arcPoly) {
      expect(Math.hypot(pt[0], pt[1])).toBeCloseTo(2, 2)
    }
  })
})
